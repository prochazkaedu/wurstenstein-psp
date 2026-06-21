use psp::sys::{ClearBuffer, CtrlButtons, FrontFaceDirection, GuState, GuSyncBehavior, GuSyncMode, LightComponent, LightType, MatrixMode, SceCtrlData, ScePspFVector3};
use psp::{SCREEN_HEIGHT, SCREEN_WIDTH, sys};
use psp::vram_alloc::SimpleVramAllocator;

use core::num::NonZeroU32;
use core::time::Duration;

use crate::assets::Assets;
use crate::audio::{Audio, MusicRequest, SoundRequest};
use crate::playfield::EXAMPLE_MAZE;
use crate::controller::bullet::{BulletManager, BulletKind};
use crate::controller::camera::{Camera, Directions};
use crate::controller::collision;
use crate::controller::enemy::EnemyManager;
use crate::controller::explosion::ExplosionManager;
use crate::controller::player::{PlayerAction, PlayerController, MAX_AMMO, MAX_HEALTH};
use crate::controller::powerup::{PowerupManager, PowerupKind};
use crate::util::background;
use crate::util::font::HorizAlign;
use crate::util::model::Transform;
use crate::util::rectangle;
use crate::util::transparent::TransparentRenderer;
use crate::util::algebra::{Vector2, Vector3};

pub struct App {
	assets: Assets,
	audio: Audio,
	perf: Perf,
	scene: Scene,
	params: Parameters,
}

#[derive(PartialEq)]
enum SceneState {
	Title,
	InGame { timer: f32, kills: usize },
	Dead,
	YoureWinner { kills: usize }
}

struct Scene {
	state: SceneState,
	camera: Camera,
	player: PlayerController,
	bullets: BulletManager,
	enemies: EnemyManager,
	powerups: PowerupManager,
	explosions: ExplosionManager,
}

impl Scene {
	pub fn new(assets: &Assets) -> Self {
		let spawn = EXAMPLE_MAZE.player_spawn_point.map(|x| (x as f32 + 0.5) * EXAMPLE_MAZE.scale);
		let player = PlayerController::new(Transform::origin().with_position(Vector3::new(spawn[0], 0.0, spawn[1])), assets.player_bounding_box.clone());

		let mut camera = Camera::new(Vector3::ZERO);
		camera.set_pov(true);

		let bullets = BulletManager::new(assets.bullet_bounding_box.clone());

		let enemies = EnemyManager::new();

		let powerups = PowerupManager::new();

		let explosions = ExplosionManager::new();

		Self {
			state: SceneState::InGame { timer: 120.0, kills: 0 },
			camera,
			player,
			bullets,
			enemies,
			powerups,
			explosions,
		}
	}
}

struct Parameters {
	flashlight_enabled: bool,
	invincible: bool,
	pov_camera: bool,
}

impl Default for Parameters {
	fn default() -> Self {
		Self {
			flashlight_enabled: false,
			invincible: false,
			pov_camera: true,
		}
	}
}

struct Perf {
	start_time: f32,
	last_time: f32,
}

impl Default for Perf {
	fn default() -> Self {
		let start_time = 0.0;

		Self {
			start_time,
			last_time: start_time,
		}
	}
}

enum FontSize {
	Title = 50,
	SubTitle = 20,
}

impl App {
	pub fn init(
		vram_allocator: &SimpleVramAllocator
	) -> Self {
		let files = ::assets::Assets::parse_from_data(include_bytes!("..//assets.bin")).unwrap();

		let mut audio = Audio::init(files.music, files.sounds);

		let mut assets = Assets::init(files.models, files.textures);

		assets.font.register_scale(FontSize::Title as u32, vram_allocator);
		assets.font.register_scale(FontSize::SubTitle as u32, vram_allocator);

		let mut scene = Scene::new(&assets);
		scene.state = SceneState::Title;

		let perf = Perf::default();
		let params = Parameters::default();

		audio.play_music(MusicRequest::Title);

		let mut app = App {
			assets,
			audio,
			scene,
			perf,
			params,
		};

		app
	}

	fn respawn(&mut self) {
		self.scene = Scene::new(&self.assets);
		self.audio.play_music(MusicRequest::InGame);
	}

	fn update_camera(&mut self, dt: f32) {
		let pitch_range = if self.params.pov_camera {
			-89.9..=-15.0
		} else {
			-89.9..=89.9
		};

		self.scene.camera.set_pov(self.params.pov_camera);
		self.scene.camera.set_pitch_range(pitch_range);
		self.scene.camera.set_target(self.scene.player.get_transform().position);
		self.scene.camera.update_position(dt);
	}

	fn fire_bullet_from_player(&mut self) {
		match self.scene.state {
			SceneState::Title => {
				self.respawn();
			}
			SceneState::InGame { .. } => {
				if self.scene.player.fire_bullet() {
					self.audio.play_sound(SoundRequest::PlayerShoot, None, 1.0);
					self.scene.bullets.spawn_bullet(self.scene.player.get_transform().clone(), 15.0, BulletKind::FromPlayer);
				}
			},
			_ => {}
		}
	}

	fn redraw_ui(&mut self) {
		let w = SCREEN_WIDTH;
		let h = SCREEN_HEIGHT;

		let white = &[255, 255, 255, 255];
		let red = &[255, 0, 0, 255];
		let green = &[0, 255, 0, 255];
		let blue = &[128, 128, 255, 255];

		match self.scene.state {
			SceneState::Title => {
				self.assets.font.draw_string("Wurstenstein 3D", FontSize::Title as u32, w as i32 / 2, h as i32 / 2 - 15, HorizAlign::Center, white);
				self.assets.font.draw_string("Shoot to start", FontSize::SubTitle as u32, w as i32 / 2, h as i32 / 2 + 40, HorizAlign::Center, white);
			},
			SceneState::InGame { timer, kills } => {
				let dimensions = &[
					0.0,
					0.0,
					1.0,
					45.0 / h as f32
				];

				rectangle::colored(dimensions, &[40, 40, 40, 192]);

				let stats = self.scene.player.get_stats();

				let health = heapless::format!(64; "Health: {}/{} ", stats.health, MAX_HEALTH).unwrap();
				let speed = if stats.speed_timer > 0.0 {
					heapless::format!(64; "Speed: turbo {:.1}s ", stats.speed_timer)
				} else {
					heapless::format!(64; "Speed: normal ")
				}.unwrap();
				let ammo = heapless::format!(64; "Ammo: {}/{}", stats.ammo, MAX_AMMO).unwrap();

				let remaining = timer as u32;
				let timer = heapless::format!(64; " {:02}:{:02}", remaining / 60, remaining % 60).unwrap();

				let kills = heapless::format!(64; "Kills: {}", kills).unwrap();

				let mut acc = 8;

				acc += self.assets.font.draw_string(&health, FontSize::SubTitle as u32, acc, h as i32 - 8, HorizAlign::Left, red);
				acc += self.assets.font.draw_string(&speed, FontSize::SubTitle as u32, acc, h as i32 - 8, HorizAlign::Left, green);
				self.assets.font.draw_string(&ammo, FontSize::SubTitle as u32, acc, h as i32 - 8, HorizAlign::Left, blue);

				let mut acc = w as i32 - 8;

				acc -= self.assets.font.draw_string(&timer, FontSize::SubTitle as u32, acc, h as i32 - 8, HorizAlign::Right, white);
				self.assets.font.draw_string(&kills, FontSize::SubTitle as u32, acc, h as i32 - 8, HorizAlign::Right, white);
			},
			SceneState::YoureWinner { kills } => {
				rectangle::colored(&[0.0, 0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32], &[0, 255, 0, 160]);

				let kills = heapless::format!(128; "{kills} bad guys were eliminated.").unwrap();

				self.assets.font.draw_string("You win!", FontSize::Title as u32, w as i32 / 2, h as i32 / 2 - 30, HorizAlign::Center, white);
				self.assets.font.draw_string(&kills, FontSize::SubTitle as u32, w as i32 / 2, h as i32 / 2 + 30, HorizAlign::Center, white);
				self.assets.font.draw_string("Shoot to respawn", FontSize::SubTitle as u32, w as i32 / 2, h as i32 / 2 + 60, HorizAlign::Center, white);
			},
			SceneState::Dead => {
				rectangle::colored(&[0.0, 0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32], &[255, 0, 0, 160]);

				self.assets.font.draw_string("You are dead", FontSize::Title as u32, w as i32 / 2, h as i32 / 2 - 15, HorizAlign::Center, white);
				self.assets.font.draw_string("Shoot to respawn", FontSize::SubTitle as u32, w as i32 / 2, h as i32 / 2 + 40, HorizAlign::Center, white);
			}
		}
	}

	pub fn main_loop(&mut self) {
		let pad_data = &mut SceCtrlData::default();

		unsafe {
			sys::sceCtrlReadBufferPositive(pad_data, 1);

			let stick_dx = (pad_data.lx as f32) / 128.0 - 1.0;
			let stick_dy = (pad_data.ly as f32) / 128.0 - 1.0;

			self.scene.camera.mouse_interact(stick_dx * 10.0, stick_dy * 10.0);

			if pad_data.buttons.contains(CtrlButtons::LEFT) {
				self.scene.camera.scroll_wheel_interact(-5.0);
			}

			if pad_data.buttons.contains(CtrlButtons::RIGHT) {
				self.scene.camera.scroll_wheel_interact(5.0);
			}

			match self.scene.state {
				SceneState::InGame { .. } => {
					self.params.flashlight_enabled = pad_data.buttons.contains(CtrlButtons::LTRIGGER);

					if pad_data.buttons.contains(CtrlButtons::RTRIGGER) {
						self.fire_bullet_from_player();
					}

					if self.params.pov_camera {
						self.scene.player.move_left = pad_data.buttons.contains(CtrlButtons::SQUARE);
						self.scene.player.move_right = pad_data.buttons.contains(CtrlButtons::CIRCLE);
						self.scene.player.move_forward = pad_data.buttons.contains(CtrlButtons::TRIANGLE);
						self.scene.player.move_backward = pad_data.buttons.contains(CtrlButtons::CROSS);
						self.scene.player.jump = pad_data.buttons.contains(CtrlButtons::DOWN);
					} else {
						self.scene.camera.key_interact(Directions::Forward, pad_data.buttons.contains(CtrlButtons::TRIANGLE));
						self.scene.camera.key_interact(Directions::Backward, pad_data.buttons.contains(CtrlButtons::CROSS));
						self.scene.camera.key_interact(Directions::Left, pad_data.buttons.contains(CtrlButtons::SQUARE));
						self.scene.camera.key_interact(Directions::Right, pad_data.buttons.contains(CtrlButtons::CIRCLE));
						self.scene.camera.key_interact(Directions::Down, pad_data.buttons.contains(CtrlButtons::DOWN));
						self.scene.camera.key_interact(Directions::Up, pad_data.buttons.contains(CtrlButtons::UP));
					}
				},
				SceneState::Dead | SceneState::Title | SceneState::YoureWinner { .. } => {
					if pad_data.buttons.contains(CtrlButtons::RTRIGGER) {
						self.respawn();
					}
				}
			}

			// TODO - POV camera toggle
		}



		self.update_state();

		self.init_drawing();

		self.init_2d();
		background::draw(
			self.perf.last_time - self.perf.start_time,
		);

		self.init_3d();

		let view_mtx = self.scene.camera.get_view_matrix();

		if self.scene.state != SceneState::Title {
			self.assets.terrain.draw(&Transform::origin());

			self.assets.player.draw(self.scene.player.get_transform());
			if self.scene.player.get_stats().ammo > 0 {
				self.assets.sausage_tip.draw(self.scene.player.get_transform());
			}

			self.scene.enemies.render(&self.assets);
			self.scene.bullets.render(&self.assets);

			unsafe {
				sys::sceGuDisable(GuState::Lighting);
			}

			self.scene.explosions.render();

			let mut transparent = TransparentRenderer::new();

			self.scene.powerups.render(&self.assets, &mut transparent);

			transparent.render(view_mtx);
		}

		self.init_2d();
		self.redraw_ui();

		self.end_drawing();
	}

	fn init_2d(&self) {
		unsafe {
			sys::sceGuDisable(GuState::DepthTest);
			sys::sceGuDisable(GuState::Lighting);
			sys::sceGuFrontFace(FrontFaceDirection::Clockwise);
		}
	}

	fn init_3d(&self) {
		unsafe {
			sys::sceGuEnable(GuState::DepthTest);
			// sys::sceGuEnable(GuState::Lighting);
			sys::sceGuEnable(GuState::CullFace);
			sys::sceGuFrontFace(FrontFaceDirection::CounterClockwise);

			sys::sceGumMatrixMode(MatrixMode::Projection);
			sys::sceGumLoadIdentity();
			sys::sceGumPerspective(45.0, 16.0 / 9.0, 0.5, 1000.0);

			let view_mtx = unsafe { core::mem::transmute(self.scene.camera.get_view_matrix()) };
			sys::sceGumMatrixMode(MatrixMode::View);
			sys::sceGumLoadMatrix(&view_mtx);

			sys::sceGumMatrixMode(sys::MatrixMode::Model);
			sys::sceGumLoadIdentity();

			sys::sceGuEnable(GuState::Light0);
			sys::sceGuDisable(GuState::Light1);
			sys::sceGuDisable(GuState::Light2);
			sys::sceGuDisable(GuState::Light3);

			let ambient = 0xFF484848;
			let diffuse = 0xFF646464;
			let specular = 0xFF646464;
			let shininess = [10.0];

			let none = 0x00000000;
			let full = 0xFFFFFFFF;
			let direction = ScePspFVector3 { x: 1.0, y: 5.0, z: 10.0 };

			sys::sceGuMaterial(LightComponent::AMBIENT, full);
			sys::sceGuMaterial(LightComponent::DIFFUSE, full);
			sys::sceGuMaterial(LightComponent::SPECULAR, full);
			// gl::Materialfv(gl::FRONT, gl::SHININESS, shininess.as_ptr());

			sys::sceGuLight(0, LightType::Directional, LightComponent::AMBIENT | LightComponent::DIFFUSE | LightComponent::SPECULAR, &direction);
			sys::sceGuLightColor(0, LightComponent::AMBIENT, ambient);
			sys::sceGuLightColor(0, LightComponent::DIFFUSE, diffuse);
			sys::sceGuLightColor(0, LightComponent::SPECULAR, specular);
			sys::sceGuLightAtt(0, 0.0, 0.0, 0.0);

			if self.params.flashlight_enabled {
				let mut transform = self.scene.player.get_transform().clone();
				transform.position.y += 1.0;

				let rotated = Vector2::new(0.0, 1.0).rotate(core::f32::consts::PI - transform.rotation.x);

				// sys::sceGuEnable(GuState::Light1);
				//
				// let spot_dir = [rotated.x, 0.0, rotated.y, 1.0];
				// let position = [transform.position.x, transform.position.y, transform.position.z, 1.0];
				//
				// gl::Lightfv(gl::LIGHT1, gl::POSITION, position.as_ptr());
				// gl::Lightfv(gl::LIGHT1, gl::SPOT_DIRECTION, spot_dir.as_ptr());
				// gl::Lightf(gl::LIGHT1, gl::SPOT_CUTOFF, 1.5);
				// gl::Lightf(gl::LIGHT1, gl::SPOT_EXPONENT, 0.0);
				// gl::Lightfv(gl::LIGHT1, gl::AMBIENT, none.as_ptr());
				// gl::Lightfv(gl::LIGHT1, gl::DIFFUSE, full.as_ptr());
				// gl::Lightfv(gl::LIGHT1, gl::SPECULAR, full.as_ptr());
			}
		}
	}

	fn init_drawing(&self) {
		unsafe {
			sys::sceGuClearColor(0xff000000);
			sys::sceGuClearDepth(0);
			sys::sceGuClear(ClearBuffer::COLOR_BUFFER_BIT | ClearBuffer::DEPTH_BUFFER_BIT);
		}
	}

	fn end_drawing(&self) {
		unsafe {
			sys::sceGuFinish();
			sys::sceGuSync(GuSyncMode::Finish, GuSyncBehavior::Wait);

			sys::sceDisplayWaitVblankStart();
			sys::sceGuSwapBuffers();
		}
	}

	fn update_state(&mut self) {
		let dt = 0.01666;
		self.perf.last_time += dt;

		if let SceneState::InGame { timer, kills } = &mut self.scene.state {
			if !self.scene.player.is_dead() {
				self.scene.player.has_contact_with_world = collision::check_with_ground(&self.scene.player, &EXAMPLE_MAZE);

				if let Some(idx) = collision::check_with_powerups(&self.scene.player, &self.scene.powerups) && let Some(kind) = self.scene.powerups.pick_up(idx) {
					self.scene.player.pick_up_powerup(kind);

					match kind {
						PowerupKind::Health => self.audio.play_sound(SoundRequest::PowerupHpPickup, None, 1.0),
						PowerupKind::Energy => self.audio.play_sound(SoundRequest::PowerupEnergyPickup, None, 1.0),
						PowerupKind::Speed => self.audio.play_sound(SoundRequest::PowerupSpeedPickup, None, 1.0),
					}
				}

				if !self.params.invincible {
					if let Some(idx) = collision::check_with_enemies(&self.scene.player, &self.scene.enemies) && let Some(damage) = self.scene.enemies.collide_with_player(idx) {
						// if self.scene.player.decrease_hp(damage) {
						// 	self.audio.play_sound(SoundRequest::EnemyHit, None, 1.0);
						// }
					}

					for bullet_idx in collision::check_player_with_bullet(&self.scene.player, &self.scene.bullets) {
						// if self.scene.player.decrease_hp(2) {
						// 	self.audio.play_sound(SoundRequest::EnemyHit, None, 1.0);
						// }
						self.scene.bullets.despawn_bullet(bullet_idx);
					}
				}

				for (enemy_idx, bullet_idx) in collision::check_enemies_with_bullets(&self.scene.enemies, &self.scene.bullets) {
					*kills += 1;
					if let Some(transform) = self.scene.enemies.get_transform(enemy_idx) {
						self.audio.play_sound(SoundRequest::EnemyDeath, Some(transform.position), 10.0);
					}

					self.scene.enemies.collide_with_bullet(enemy_idx);
					self.scene.bullets.despawn_bullet(bullet_idx);
				}

				self.scene.player.update_yaw(self.scene.camera.get_yaw_pitch().0);

				*timer = (*timer - dt).max(0.0);

				if *timer == 0.0 {
					self.scene.state = SceneState::YoureWinner { kills: *kills };
					self.audio.play_music(MusicRequest::Win);
				}
			}

			if let Some(action) = self.scene.player.update(&EXAMPLE_MAZE, dt) {
				match action {
					PlayerAction::Jumped => {
						self.audio.play_sound(SoundRequest::PlayerJump, None, 1.0);
					},
					PlayerAction::StartedDying => {
						self.audio.play_sound(SoundRequest::PlayerDeath, None, 1.0);
						self.audio.play_music(MusicRequest::Stop);
					},
					PlayerAction::Dead => {
						self.audio.play_sound(SoundRequest::PlayerExplosion, None, 1.0);
						self.scene.explosions.add_explosion(self.scene.player.get_transform().position);
						self.audio.play_music(MusicRequest::Death);
						self.scene.state = SceneState::Dead;
					}
				}
			}

			self.update_camera(dt);

			let pos = self.scene.player.get_transform().position;
			let rot = self.scene.player.get_transform().rotation.x;

			self.audio.update_position(pos, rot);

			self.scene.powerups.update(&EXAMPLE_MAZE, dt);
			let enemy_updates = self.scene.enemies.update(&EXAMPLE_MAZE, dt);
			for bullet_pos in enemy_updates.shot_bullets {
				let angle = {
					let bullet_pos = Vector2::new(bullet_pos.x, bullet_pos.z);

					let player_pos = self.scene.player.get_transform().position;
					let player_pos = Vector2::new(player_pos.x, player_pos.z);

					let diff = player_pos - bullet_pos;

					let base = Vector2::new(0.0, 1.0);

					let mul = if base.cross(diff) >= 0.0 {
						1.0
					} else {
						-1.0
					};

					Vector2::new(0.0, 1.0).angle(diff) * mul
				};

				let transform = Transform::origin().with_position(bullet_pos).with_rotation(Vector3::new(core::f32::consts::PI - angle, 0.0, 0.0));

				self.scene.bullets.spawn_bullet(transform, 15.0, BulletKind::FromEnemy);
			}
			for gone_pos in enemy_updates.enemies_gone {
				self.audio.play_sound(SoundRequest::EnemyExplosion, Some(gone_pos), 10.0);
				self.scene.explosions.add_explosion(gone_pos);
			}
			self.scene.bullets.update(dt);
		}

		self.scene.explosions.update(dt);
	}
	
	fn update_shader_data(&mut self) {
		// self.scene.powerups.update_point_lights(program);
	}
}

