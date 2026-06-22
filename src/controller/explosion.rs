use crate::util::algebra::Vector3;
use crate::util::allocate_display_list;

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

use psp::sys::{self, GuPrimitive, GuState, VertexType};

use once_cell_no_std::OnceCell;

const EXPLOSION_POINTS: usize = 256;

static HASH_CACHE: OnceCell<[[f32; 2]; EXPLOSION_POINTS]> = OnceCell::new();

fn hash(seed: f32) -> f32 {
	let x = unsafe { psp::math::sinf(seed) } * 43758.545;
	x - libm::floorf(x)
}

#[derive(PartialEq, Eq)]
enum ExplosionState {
	Running,
	Ended
}

pub struct Explosion {
	base_position: Vector3,
	state: ExplosionState,
	timer: f32
}

#[repr(C, align(4))]
#[derive(PartialEq)]
struct ColoredPoint {
	r: u8,
	g: u8,
	b: u8,
	a: u8,
	x: f32,
	y: f32,
	z: f32,
}

impl Explosion {
	fn update(&mut self, dt: f32) {
		match self.state {
			ExplosionState::Running => {
				self.timer += dt;
				if self.timer >= 1.0 {
					self.state = ExplosionState::Ended;
				}
			},
			ExplosionState::Ended => {}
		}
	}

	fn draw(&self) {
		let hashes = match HASH_CACHE.get() {
			Some(val) => val,
			None => {
				let mut arr = [[0.0, 0.0]; EXPLOSION_POINTS];

				for (seed, y) in arr.iter_mut().enumerate() {
					let dist = hash(seed as f32) * 2.0;
					let angle = hash(seed as f32 * 1234.0) * 2.0 * core::f32::consts::PI;

					let dx = unsafe { psp::math::cosf(angle) } * dist;
					let dz = unsafe { psp::math::sinf(angle) } * dist;

					*y = [ dx, dz ];
				}

				let _ = HASH_CACHE.set(arr);

				HASH_CACHE.get().unwrap()
			}
		};

		let mem = allocate_display_list(EXPLOSION_POINTS);

		for point in 0..EXPLOSION_POINTS {
			let seed = point as f32 / EXPLOSION_POINTS as f32;

			let [dx, dz] = hashes[point];

			let x = self.base_position.x + self.timer * dx;
			let y = self.base_position.y + -4.0 * self.timer * self.timer + 4.0 * self.timer;
			let z = self.base_position.z + self.timer * dz;

			mem[point] = ColoredPoint {
				r: 255,
				g: (seed * 255.0) as u8,
				b: 0,
				a: 255 - (self.timer * 255.0) as u8,
				x,
				y,
				z
			};
		}

		unsafe {
			sys::sceGuEnable(GuState::Blend);

			sys::sceGumDrawArray(
				GuPrimitive::Points,
				VertexType::COLOR_8888 | VertexType::VERTEX_32BITF | VertexType::TRANSFORM_3D,
				EXPLOSION_POINTS as i32,
				core::ptr::null(),
				mem.as_ptr() as _,
			);

			sys::sceGuDisable(GuState::Blend);
		}
	}
}

pub struct ExplosionManager {
	explosions: Vec<Option<Explosion>>
}

impl ExplosionManager {
	pub fn new() -> Self {
		Self {
			explosions: vec![]
		}
	}

	pub fn update(&mut self, dt: f32) {
		for explosion in &mut self.explosions {
			let Some(expl) = explosion else { continue };

			expl.update(dt);

			if expl.state == ExplosionState::Ended {
				*explosion = None;
			}
		}
	}

	pub fn add_explosion(&mut self, position: Vector3) {
		// Try to find an existing slot

		let new_explosion = Explosion {
			base_position: position,
			state: ExplosionState::Running,
			timer: 0.0,
		};

		for explosion in &mut self.explosions {
			if explosion.is_some() { continue }

			*explosion = Some(new_explosion);
			return
		}

		self.explosions.push(Some(new_explosion));
	}

	pub fn render(&self) {
		for explosion in &self.explosions {
			let Some(expl) = explosion else { continue };

			expl.draw();
		}
	}
}


