use psp::sys::{ClearBuffer, CtrlButtons, FrontFaceDirection, GuState, GuSyncBehavior, GuSyncMode, LightComponent, LightType, MatrixMode, SceCtrlData, ScePspFMatrix4, ScePspFVector3};
use psp::{SCREEN_HEIGHT, SCREEN_WIDTH, sys};
use psp::vram_alloc::SimpleVramAllocator;

use crate::assets::Assets;
use crate::audio::{Audio, MusicRequest, SoundRequest};
use crate::playfield::EXAMPLE_MAZE;
use crate::controller::bullet::{BulletManager, BulletKind};
use crate::controller::camera::Camera;
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
use crate::util::algebra::{Matrix4, Vector2, Vector3};

pub struct App {
	assets: Assets,
	audio: Audio,
	perf: Perf,
	scene: Scene,
	params: Parameters,
	last_buttons: CtrlButtons
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
	stats: StatsLevel
}

impl Default for Parameters {
	fn default() -> Self {
		Self {
			flashlight_enabled: false,
			invincible: false,
			pov_camera: true,
			stats: StatsLevel::None
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsLevel {
	None,
	Fps,
	Overall,
	DetailedCpuDraw
}

struct Perf {
	start_time: u64,
	last_time: u64,
	last_update: u64,
	frame_start_time: u64,
	frame_cpu_status_time: u64,
	frame_cpu_background_time: u64,
	frame_cpu_init_3d_time: u64,
	frame_cpu_terrain_time: u64,
	frame_cpu_player_time: u64,
	frame_cpu_enemies_time: u64,
	frame_cpu_bullets_time: u64,
	frame_cpu_explosions_time: u64,
	frame_cpu_transparent_time: u64,
	frame_cpu_ui_time: u64,
	frame_cpu_draw_time: u64,
	frame_gpu_time: u64,
	fps: f32,
	accumulated_dt: f32,
	accumulated_count: usize
}

impl Default for Perf {
	fn default() -> Self {
		let mut start_time = 0;
		unsafe { sys::sceRtcGetCurrentTick(&mut start_time); }

		Self {
			start_time,
			last_time: start_time,
			last_update: start_time,
			frame_start_time: start_time,
			frame_cpu_status_time: start_time,
			frame_cpu_background_time: start_time,
			frame_cpu_init_3d_time: start_time,
			frame_cpu_terrain_time: start_time,
			frame_cpu_player_time: start_time,
			frame_cpu_enemies_time: start_time,
			frame_cpu_bullets_time: start_time,
			frame_cpu_explosions_time: start_time,
			frame_cpu_transparent_time: start_time,
			frame_cpu_ui_time: start_time,
			frame_cpu_draw_time: start_time,
			frame_gpu_time: start_time,
			fps: 0.0,
			accumulated_dt: 0.0,
			accumulated_count: 0
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

		App {
			assets,
			audio,
			scene,
			perf,
			params,
			last_buttons: CtrlButtons::empty()
		}
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
					(SCREEN_HEIGHT - 28) as f32,
					SCREEN_WIDTH as f32,
					SCREEN_HEIGHT as f32,
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

		if self.params.stats != StatsLevel::None {
			let tick = heapless::format!(64; "FPS: {:.1}", self.perf.fps).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 10, 30, HorizAlign::Left, white);
		}

		if self.params.stats == StatsLevel::DetailedCpuDraw {
			let tick = heapless::format!(64; "Background:{:6}", self.perf.frame_cpu_background_time - self.perf.frame_cpu_status_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 30, HorizAlign::Right, white);

			let tick = heapless::format!(64; "3D init:{:6}", self.perf.frame_cpu_init_3d_time - self.perf.frame_cpu_background_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 50, HorizAlign::Right, white);

			let tick = heapless::format!(64; "Terrain:{:6}", self.perf.frame_cpu_terrain_time - self.perf.frame_cpu_init_3d_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 70, HorizAlign::Right, white);

			let tick = heapless::format!(64; "Player:{:6}", self.perf.frame_cpu_player_time - self.perf.frame_cpu_terrain_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 90, HorizAlign::Right, white);

			let tick = heapless::format!(64; "Enemies:{:6}", self.perf.frame_cpu_enemies_time - self.perf.frame_cpu_player_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 110, HorizAlign::Right, white);

			let tick = heapless::format!(64; "Bullets:{:6}", self.perf.frame_cpu_bullets_time - self.perf.frame_cpu_enemies_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 130, HorizAlign::Right, white);

			let tick = heapless::format!(64; "Explosions:{:6}", self.perf.frame_cpu_explosions_time - self.perf.frame_cpu_bullets_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 150, HorizAlign::Right, white);

			let tick = heapless::format!(64; "Transparent:{:6}", self.perf.frame_cpu_transparent_time - self.perf.frame_cpu_explosions_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 170, HorizAlign::Right, white);

			let tick = heapless::format!(64; "UI:{:6}", self.perf.frame_cpu_ui_time - self.perf.frame_cpu_transparent_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 190, HorizAlign::Right, white);

			let tick = heapless::format!(64; "CPU draw:{:6}", self.perf.frame_cpu_draw_time - self.perf.frame_cpu_status_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 210, HorizAlign::Right, white);

			let tick = heapless::format!(64; "Total time:{:6}", self.perf.frame_gpu_time - self.perf.frame_start_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 230, HorizAlign::Right, white);
		}

		if self.params.stats == StatsLevel::Overall {
			let tick = heapless::format!(64; "CPU compute:{:6}", self.perf.frame_cpu_status_time - self.perf.frame_start_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 30, HorizAlign::Right, white);

			let tick = heapless::format!(64; "CPU draw:{:6}", self.perf.frame_cpu_draw_time - self.perf.frame_cpu_status_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 50, HorizAlign::Right, white);

			let tick = heapless::format!(64; "GPU draw:{:6}", self.perf.frame_gpu_time - self.perf.frame_cpu_draw_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 70, HorizAlign::Right, white);

			let tick = heapless::format!(64; "Total time:{:6}", self.perf.frame_gpu_time - self.perf.frame_start_time).unwrap();
			self.assets.font.draw_string(&tick, FontSize::SubTitle as u32, 480 - 10, 90, HorizAlign::Right, white);
		}
	}

	fn update_perf_data(&mut self, dt: f32) {
		self.perf.accumulated_dt += dt;
		self.perf.accumulated_count += 1;

		if self.perf.last_time as i64 - self.perf.last_update as i64 >= 500000 {
			let fps = (1.0 / self.perf.accumulated_dt) * self.perf.accumulated_count as f32;

			self.perf.accumulated_count = 0;
			self.perf.accumulated_dt = 0.0;

			self.perf.fps = fps;

			unsafe { sys::sceRtcGetCurrentTick(&mut self.perf.last_update); }
		}
	}

	pub fn main_loop(&mut self) {
		let mut new_time = 0;
		unsafe { sys::sceRtcGetCurrentTick(&mut new_time); }

		let dt = (new_time - self.perf.last_time) as f32 / 1000000.0;
		self.perf.last_time = new_time;

		let pad_data = &mut SceCtrlData::default();

		unsafe {
			sys::sceCtrlReadBufferPositive(pad_data, 1);

			let stick_dx = (pad_data.lx as f32) / 128.0 - 1.0;
			let stick_dy = (pad_data.ly as f32) / 128.0 - 1.0;

			let deadzone = 0.2;

			let stick_dx = if (-deadzone..=deadzone).contains(&stick_dx) { 0.0 } else { stick_dx };
			let stick_dy = if (-deadzone..=deadzone).contains(&stick_dy) { 0.0 } else { stick_dy };

			let mut camera_dx = 0.0;
			let camera_dy = 0.0;

			if pad_data.buttons.contains(CtrlButtons::LTRIGGER) {
				camera_dx -= 1.0;
			}

			if pad_data.buttons.contains(CtrlButtons::RTRIGGER) {
				camera_dx += 1.0;
			}

			// if(pad_data.buttons.contains(CtrlButtons::TRIANGLE)) {
			// 	camera_dy -= 1.0;
			// }
			//
			// if(pad_data.buttons.contains(CtrlButtons::CROSS)) {
			// 	camera_dy += 1.0;
			// }

			self.scene.camera.mouse_interact(camera_dx * 600.0 * dt, camera_dy * 600.0 * dt);

			if pad_data.buttons.contains(CtrlButtons::LEFT) {
				self.scene.camera.scroll_wheel_interact(-50.0 * dt);
			}

			if pad_data.buttons.contains(CtrlButtons::RIGHT) {
				self.scene.camera.scroll_wheel_interact(50.0 * dt);
			}

			let latched_buttons = pad_data.buttons.symmetric_difference(self.last_buttons).intersection(pad_data.buttons);

			if latched_buttons.contains(CtrlButtons::SELECT) {
				self.params.stats = match self.params.stats {
					StatsLevel::None => StatsLevel::Fps,
					StatsLevel::Fps => StatsLevel::Overall,
					StatsLevel::Overall => StatsLevel::DetailedCpuDraw,
					StatsLevel::DetailedCpuDraw => StatsLevel::None,
				}
			}

			match self.scene.state {
				SceneState::InGame { .. } => {
					self.params.flashlight_enabled = pad_data.buttons.contains(CtrlButtons::START);

					if latched_buttons.contains(CtrlButtons::SQUARE) {
						self.fire_bullet_from_player();
					}

					// if self.params.pov_camera {
						self.scene.player.x_movement = stick_dx;
						self.scene.player.y_movement = stick_dy;
						self.scene.player.jump = pad_data.buttons.contains(CtrlButtons::CROSS);
					// } else {
					// 	self.scene.camera.key_interact(Directions::Forward, pad_data.buttons.contains(CtrlButtons::TRIANGLE));
					// 	self.scene.camera.key_interact(Directions::Backward, pad_data.buttons.contains(CtrlButtons::CROSS));
					// 	self.scene.camera.key_interact(Directions::Left, pad_data.buttons.contains(CtrlButtons::SQUARE));
					// 	self.scene.camera.key_interact(Directions::Right, pad_data.buttons.contains(CtrlButtons::CIRCLE));
					// 	self.scene.camera.key_interact(Directions::Down, pad_data.buttons.contains(CtrlButtons::DOWN));
					// 	self.scene.camera.key_interact(Directions::Up, pad_data.buttons.contains(CtrlButtons::UP));
					// }
				},
				SceneState::Dead | SceneState::Title | SceneState::YoureWinner { .. } => {
					if latched_buttons.contains(CtrlButtons::SQUARE) {
						self.respawn();
					}
				}
			}

			self.last_buttons = pad_data.buttons;

			// TODO - POV camera toggle
		}

		self.update_state(dt);

		let mut frame_cpu_status_time = 0;
		unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_status_time); }

		self.init_drawing();

		let mut frame_cpu_background_time = frame_cpu_status_time;
		let mut frame_cpu_init_3d_time = frame_cpu_status_time;
		let mut frame_cpu_terrain_time = frame_cpu_status_time;
		let mut frame_cpu_player_time = frame_cpu_status_time;
		let mut frame_cpu_enemies_time = frame_cpu_status_time;
		let mut frame_cpu_bullets_time = frame_cpu_status_time;
		let mut frame_cpu_explosions_time = frame_cpu_status_time;
		let mut frame_cpu_transparent_time = frame_cpu_status_time;
		let mut frame_cpu_ui_time = frame_cpu_status_time;

		self.init_2d();
		background::draw(
			(self.perf.last_time - self.perf.start_time) as f32 / 1000000.0,
		);

		unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_background_time); }

		self.init_3d();

		unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_init_3d_time); }

		let view_mtx = self.scene.camera.get_view_matrix();

		if self.scene.state != SceneState::Title {
			self.assets.terrain.draw(&Transform::origin());

			unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_terrain_time); }

			self.assets.player.draw(self.scene.player.get_transform());
			if self.scene.player.get_stats().ammo > 0 {
				self.assets.sausage_tip.draw(self.scene.player.get_transform());
			}

			unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_player_time); }

			self.scene.enemies.render(&self.assets);

			unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_enemies_time); }

			self.scene.bullets.render(&self.assets);

			unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_bullets_time); }

			unsafe {
				sys::sceGuDisable(GuState::Lighting);
			}

			self.scene.explosions.render();

			unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_explosions_time); }

			let mut transparent = TransparentRenderer::new();

			self.scene.powerups.render(&self.assets, &mut transparent);

			transparent.render(view_mtx);

			unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_transparent_time); }
		}

		self.init_2d();
		self.redraw_ui();

		unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_ui_time); }

		let mut frame_cpu_draw_time = 0;
		unsafe { sys::sceRtcGetCurrentTick(&mut frame_cpu_draw_time); }

		self.end_drawing();

		let mut frame_gpu_draw_time = 0;
		unsafe { sys::sceRtcGetCurrentTick(&mut frame_gpu_draw_time); }

		self.vsync();

		self.perf.frame_start_time = new_time;
		self.perf.frame_cpu_status_time = frame_cpu_status_time;
		self.perf.frame_cpu_draw_time = frame_cpu_draw_time;
		self.perf.frame_gpu_time = frame_gpu_draw_time;
		self.perf.frame_cpu_background_time = frame_cpu_background_time;
		self.perf.frame_cpu_init_3d_time = frame_cpu_init_3d_time;
		self.perf.frame_cpu_terrain_time = frame_cpu_terrain_time;
		self.perf.frame_cpu_player_time = frame_cpu_player_time;
		self.perf.frame_cpu_enemies_time = frame_cpu_enemies_time;
		self.perf.frame_cpu_bullets_time = frame_cpu_bullets_time;
		self.perf.frame_cpu_explosions_time = frame_cpu_explosions_time;
		self.perf.frame_cpu_transparent_time = frame_cpu_transparent_time;
		self.perf.frame_cpu_ui_time = frame_cpu_ui_time;
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
			sys::sceGuEnable(GuState::Lighting);
			sys::sceGuEnable(GuState::CullFace);
			sys::sceGuFrontFace(FrontFaceDirection::CounterClockwise);

			sys::sceGumMatrixMode(MatrixMode::Projection);
			sys::sceGumLoadIdentity();
			sys::sceGumPerspective(self.scene.camera.get_zoom(), 16.0 / 9.0, 0.5, 1000.0);

			let view_mtx = core::mem::transmute::<Matrix4, ScePspFMatrix4>(self.scene.camera.get_view_matrix());
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
			// let shininess = [10.0];

			let none = 0x00000000;
			let full = 0xFFFFFFFF;
			let direction = ScePspFVector3 { x: 1.0, y: 5.0, z: 10.0 };

			sys::sceGuMaterial(LightComponent::AMBIENT, full);
			sys::sceGuMaterial(LightComponent::DIFFUSE, full);
			sys::sceGuMaterial(LightComponent::SPECULAR, full);
			// gl::Materialfv(gl::FRONT, gl::SHININESS, shininess.as_ptr());

			sys::sceGuAmbient(ambient);

			sys::sceGuLight(0, LightType::Directional, LightComponent::AMBIENT | LightComponent::DIFFUSE | LightComponent::SPECULAR, &direction);
			sys::sceGuLightColor(0, LightComponent::AMBIENT, ambient);
			sys::sceGuLightColor(0, LightComponent::DIFFUSE, diffuse);
			sys::sceGuLightColor(0, LightComponent::SPECULAR, specular);
			sys::sceGuLightAtt(0, 1.0, 0.0, 0.0);

			if self.params.flashlight_enabled {
				let mut transform = self.scene.player.get_transform().clone();
				transform.position.y += 1.0;

				let rotated = Vector2::new(0.0, 1.0).rotate(core::f32::consts::PI - transform.rotation.x);

				let position = ScePspFVector3 { x: transform.position.x, y: transform.position.y, z: transform.position.z };
				let direction = ScePspFVector3 { x: -rotated.x, y: 0.0, z: -rotated.y };

				sys::sceGuEnable(GuState::Light1);
				sys::sceGuLight(1, LightType::Spotlight, LightComponent::AMBIENT | LightComponent::DIFFUSE | LightComponent::SPECULAR, &position);
				sys::sceGuLightSpot(1, &direction, 0.0, 0.999);
				sys::sceGuLightColor(1, LightComponent::AMBIENT, none);
				sys::sceGuLightColor(1, LightComponent::DIFFUSE, full);
				sys::sceGuLightColor(1, LightComponent::SPECULAR, full);
				sys::sceGuLightAtt(1, 1.0, 0.0, 0.0);
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
		}
	}

	fn vsync(&self) {
		unsafe {
			sys::sceDisplayWaitVblankStart();
			sys::sceGuSwapBuffers();
		}
	}

	fn update_state(&mut self, dt: f32) {
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
						if self.scene.player.decrease_hp(damage) {
							self.audio.play_sound(SoundRequest::EnemyHit, None, 1.0);
						}
					}

					for bullet_idx in collision::check_player_with_bullet(&self.scene.player, &self.scene.bullets) {
						if self.scene.player.decrease_hp(2) {
							self.audio.play_sound(SoundRequest::EnemyHit, None, 1.0);
						}
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

		self.update_perf_data(dt);
	}
}

