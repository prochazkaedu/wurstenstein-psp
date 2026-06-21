use parry2d::math::Pose;
use parry2d::shape::Ball;

use crate::assets::Assets;
use crate::playfield::{Playfield, PlayfieldPiece};
use crate::util::model::Transform;
use crate::util::transparent::TransparentRenderer;
use crate::util::algebra::Vector3;

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

use psp::sys::{self, SceKernelUtilsMt19937Context};

#[derive(Debug, Copy, Clone)]
pub enum PowerupKind {
	Health,
	Energy,
	Speed
}

impl PowerupKind {
	pub fn get_color(&self) -> [u8; 3] {
		match self {
			PowerupKind::Health => [255, 0, 0],
			PowerupKind::Energy => [128, 128, 255],
			PowerupKind::Speed => [0, 255, 0],
		}
	}
}

#[derive(PartialEq, Eq)]
enum PowerupState {
	Spawn,
	Floating,
	PickedUp,
	Gone,
}

pub struct Powerup {
	kind: PowerupKind,
	base_y: f32,
	transform: Transform,
	timer: f32,
	state: PowerupState,
}

impl Powerup {
	pub fn new(kind: PowerupKind, transform: Transform) -> Self {
		Self {
			kind,
			base_y: transform.position.y,
			transform,
			timer: 0.0,
			state: PowerupState::Spawn
		}
	}

	pub fn update(&mut self, dt: f32) {
		self.timer += dt;

		match self.state {
			PowerupState::Spawn => {
				self.transform.rotation.x += 4.0 * dt;
				let scale = (self.timer * 2.0).min(1.0);
				self.transform.scale.x = scale;
				self.transform.scale.y = scale;
				self.transform.scale.z = scale;

				if self.timer >= 0.5 {
					self.timer = 0.0;
					self.state = PowerupState::Floating;
				}
			},
			PowerupState::Floating => {
				self.transform.rotation.x += 1.0 * dt;
				self.transform.position.y = self.base_y + unsafe { psp::math::sinf(self.timer * 2.0) } * 0.3;
			},
			PowerupState::PickedUp => {
				self.transform.rotation.x += 8.0 * dt;
				let scale = (1.0 - self.timer * 2.0).max(0.0);
				self.transform.scale.x = scale;
				self.transform.scale.y = scale;
				self.transform.scale.z = scale;

				self.transform.position.y += dt * 3.0;

				if self.timer >= 0.5 {
					self.state = PowerupState::Gone;
				}
			},
			PowerupState::Gone => {
				self.transform.scale.x = 0.0;
				self.transform.scale.y = 0.0;
				self.transform.scale.z = 0.0;
			}
		}
	}
}

pub struct PowerupManager {
	powerups: Vec<Option<Powerup>>,
	rng: SceKernelUtilsMt19937Context,
	spawn_timer: f32,
}

impl PowerupManager {
	pub fn new() -> Self {
		let mut rng = SceKernelUtilsMt19937Context {
			count: 0, state: [0; _]
		};

		unsafe {
			sys::sceKernelUtilsMt19937Init(&mut rng, 548329438);
		}

		Self {
			powerups: vec![],
			rng,
			spawn_timer: 5.0,
		}
	}

	fn spawn_new_powerup<T: PlayfieldPiece>(&mut self, world: &Playfield<'_, T>) {
		let kind = unsafe { sys::sceKernelUtilsMt19937UInt(&mut self.rng) } % 3;

		let spawn_point_num = world.powerup_spawn_points.len();

		let idx = unsafe { sys::sceKernelUtilsMt19937UInt(&mut self.rng) } as usize % spawn_point_num;

		// Find a free slot for a powerup

		for off in 0..spawn_point_num {
			let idx = (idx + off) % spawn_point_num;

			if self.powerups[idx].is_some() {
				continue
			}

			let pos = world.powerup_spawn_points[idx];
			let pos = pos.map(|x| x as f32 * world.scale + world.scale * 0.5);

			let kind = match kind {
				0 => PowerupKind::Health,
				1 => PowerupKind::Energy,
				2 => PowerupKind::Speed,
				_ => unreachable!()
			};

			self.powerups[idx] = Some(Powerup::new(
				kind,
				Transform::origin().with_position(Vector3::new(pos[0], 1.5, pos[1]))
			));

			break
		}
	}

	pub fn update<T: PlayfieldPiece>(&mut self, world: &Playfield<'_, T>, dt: f32) {
		self.powerups.resize_with(world.powerup_spawn_points.len(), || None);

		self.spawn_timer -= dt;

		if self.spawn_timer < 0.0 {
			self.spawn_timer = 5.0;
			self.spawn_new_powerup(world);
		}

		for powerup in &mut self.powerups {
			let Some(power) = powerup else { continue };

			power.update(dt);

			if power.state == PowerupState::Gone {
				*powerup = None;
			}
		}
	}

	// pub fn update_point_lights(&self) {
	// 	let mut enabled_array = [0; 16];
	// 	let mut position_array = [[0.0; 3]; 16];
	// 	let mut diffuse_array = [[0.0; 3]; 16];
	// 	let mut specular_array = [[0.0; 3]; 16];
	//
	// 	for (idx, powerup) in self.powerups.iter().enumerate() {
	// 		if let Some(powerup) = powerup {
	// 			let color = powerup.kind.get_color().map(|x| x * powerup.transform.scale.x);
	//
	// 			enabled_array[idx] = 1;
	// 			position_array[idx] = powerup.transform.position.as_slice().try_into().unwrap();
	// 			diffuse_array[idx] = color;
	// 			specular_array[idx] = color;
	// 		}
	// 	}
	//
	// 	let position_array = position_array.concat();
	// 	let diffuse_array = diffuse_array.concat();
	// 	let specular_array = specular_array.concat();
	//
	// 	program.set_uniform_slice_i32_1("point_enabled", &enabled_array);
	// 	program.set_uniform_slice_f32_3("point_position", &position_array);
	// 	program.set_uniform_slice_f32_3("point_diffuse", &diffuse_array);
	// 	program.set_uniform_slice_f32_3("point_specular", &specular_array);
	// }

	pub fn render<'a>(&'a self, assets: &'a Assets, transparent: &mut TransparentRenderer<'a>) {
		for powerup in &self.powerups {
			let Some(powerup) = powerup else { continue };

			let color = powerup.kind.get_color();
			let color = [color[0], color[1], color[2], 100];

			let model = match powerup.kind {
				PowerupKind::Health => &assets.powerup_hp,
				PowerupKind::Energy => &assets.powerup_energy,
				PowerupKind::Speed => &assets.powerup_speed,
			};

			transparent.add_object(&powerup.transform, move || {
				model.draw_colored(&powerup.transform, &color);
			});
		}
	}

	pub fn get_collision_shapes(&self) -> Vec<Option<(Ball, Pose)>> {
		self.powerups.iter()
			.map(|x| if let Some(x) = &x && x.state == PowerupState::Floating { Some(x) } else { None })
			.map(|x| x.map(|x| (Ball::new(1.5), Pose::translation(x.transform.position.x, x.transform.position.z))))
			.collect::<Vec<_>>()
	}

	pub fn pick_up(&mut self, idx: usize) -> Option<PowerupKind> {
		self.powerups[idx].as_mut().map(|x| {
			x.state = PowerupState::PickedUp;
			x.timer = 0.0;
			x.kind
		})
	}
}

