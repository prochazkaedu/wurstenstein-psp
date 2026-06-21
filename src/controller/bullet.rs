use parry2d::math::{Rot2, Pose};
use parry2d::shape::Cuboid;

use crate::assets::{Assets, BoundingBox};
use crate::util::model::Transform;
use crate::util::algebra::Vector2;

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

#[derive(PartialEq, Eq)]
enum BulletState {
	Flying,
	Despawn,
	Gone,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BulletKind {
	FromPlayer,
	FromEnemy,
}

pub struct Bullet {
	kind: BulletKind,
	transform: Transform,
	velocity: f32,
	timer: f32,
	state: BulletState
}

impl Bullet {
	fn update(&mut self, dt: f32) {
		let vector = Vector2::new(0.0, -1.0).rotate(-self.transform.rotation.x) * dt * self.velocity;

		self.transform.position.x += vector.x;
		self.transform.position.z += vector.y;

		self.timer += dt;

		match self.state {
			BulletState::Flying => {
				if self.timer >= 3.0 {
					self.timer = 0.0;
					self.state = BulletState::Despawn;
				}
			},
			BulletState::Despawn => {
				let scale = (1.0 - self.timer * 4.0).max(0.0);
				self.transform.scale.x = scale;
				self.transform.scale.y = scale;
				self.transform.scale.z = scale;

				if self.timer >= 0.25 {
					self.state = BulletState::Gone;
				}
			},
			BulletState::Gone => {
				self.transform.scale.x = 0.0;
				self.transform.scale.y = 0.0;
				self.transform.scale.z = 0.0;
			}
		}
	}
}

pub struct BulletManager {
	bullets: Vec<Option<Bullet>>,
	bounding_box: BoundingBox,
}

impl BulletManager {
	pub fn new(bounding_box: BoundingBox) -> Self {
		Self {
			bullets: vec![],
			bounding_box
		}
	}

	pub fn render(&self, assets: &Assets) {
		for bullet in &self.bullets {
			let Some(bullet) = bullet else { continue };

			assets.sausage_bullet.draw(&bullet.transform);
		}
	}

	pub fn update(&mut self, dt: f32) {
		for bullet in &mut self.bullets {
			let Some(sausage) = bullet else { continue };

			sausage.update(dt);

			if sausage.state == BulletState::Gone {
				*bullet = None;
			}
		}
	}

	pub fn spawn_bullet(&mut self, transform: Transform, velocity: f32, kind: BulletKind) {
		// Try to find an existing slot

		let new_bullet = Bullet {
			kind,
			transform,
			velocity,
			timer: 0.0,
			state: BulletState::Flying
		};

		for bullet in &mut self.bullets {
			if bullet.is_some() { continue }

			*bullet = Some(new_bullet);
			return
		}

		self.bullets.push(Some(new_bullet));
	}

	pub fn get_collision_shapes(&self) -> Vec<Option<(BulletKind, Cuboid, Pose)>> {
		self.bullets.iter()
			.map(|x| {
				let Some(x) = &x else { None? };

				match x.kind {
					BulletKind::FromEnemy => if x.state == BulletState::Flying && x.timer > 0.2 { Some(x) } else { None },
					BulletKind::FromPlayer => if x.state == BulletState::Flying { Some(x) } else { None },
				}
			})
			.map(|x| x.map(|x| {
				let (shape, pose) = self.bounding_box.get_collision_shape();
				let vect = Vector2::new(pose.translation[0], pose.translation[1]).rotate(-x.transform.rotation.x);
				let mut pose = Pose::translation(x.transform.position.x + vect.x, x.transform.position.z + vect.y);
				pose.rotation = Rot2::from_angle(-x.transform.rotation.x);
				(x.kind, shape, pose)
			}))
			.collect::<Vec<_>>()
	}

	pub fn despawn_bullet(&mut self, idx: usize) {
		if let Some(x) = &mut self.bullets[idx] {
			x.timer = 0.0;
			x.state = BulletState::Despawn;
		}
	}
}

