use parry2d::math::Pose;
use parry2d::shape::Ball;

use crate::assets::Assets;
use crate::playfield::{Playfield, PlayfieldPiece};
use crate::util::model::Transform;
use crate::util::algebra::Vector3;

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

use psp::sys::{self, SceKernelUtilsMt19937Context};

pub enum EnemyKind {
	Apple,
	Pear
}

#[derive(PartialEq, Eq)]
enum EnemyState {
	Spawn,
	Idle,
	Despawn,
	Gone
}

pub struct Enemy {
	kind: EnemyKind,
	transform: Transform,
	timer: f32,
	state: EnemyState
}

impl Enemy {
	pub fn new(kind: EnemyKind, transform: Transform) -> Self {
		Self {
			kind,
			transform,
			timer: 0.0,
			state: EnemyState::Spawn
		}
	}

	fn update(&mut self, dt: f32) -> bool {
		self.timer += dt;

		match self.state {
			EnemyState::Spawn => {
				self.transform.rotation.x += 4.0 * dt;
				let scale = (self.timer * 2.0).min(1.0);
				self.transform.scale.x = scale;
				self.transform.scale.y = scale;
				self.transform.scale.z = scale;

				if self.timer >= 0.5 {
					self.timer = 0.0;
					self.state = EnemyState::Idle;
				}
				false
			},
			EnemyState::Idle => {
				if self.timer >= 0.5 {
					self.timer = 0.0;
					true
				} else {
					false
				}
			},
			EnemyState::Despawn => {
				self.transform.rotation.x += 8.0 * dt;
				let scale = (1.0 - self.timer * 2.0).max(0.0);
				self.transform.scale.x = scale;
				self.transform.scale.y = scale;
				self.transform.scale.z = scale;

				if self.timer >= 0.5 {
					self.state = EnemyState::Gone;
				}
				false
			},
			EnemyState::Gone => {
				self.transform.scale.x = 0.0;
				self.transform.scale.y = 0.0;
				self.transform.scale.z = 0.0;
				false
			}
		}
	}
}

pub struct EnemyManager {
	enemies: Vec<Option<Enemy>>,
	rng: SceKernelUtilsMt19937Context,
	spawn_timer: f32,
}

pub struct EnemyUpdate {
	pub shot_bullets: Vec<Vector3>,
	pub enemies_gone: Vec<Vector3>,
}

impl EnemyManager {
	pub fn new() -> Self {
		let mut rng = SceKernelUtilsMt19937Context {
			count: 0, state: [0; _]
		};

		unsafe {
			sys::sceKernelUtilsMt19937Init(&mut rng, 548329438);
		}

		Self {
			enemies: vec![],
			rng,
			spawn_timer: 3.0,
		}
	}

	fn spawn_new_enemy<T: PlayfieldPiece>(&mut self, world: &Playfield<'_, T>) {
		let kind = unsafe { sys::sceKernelUtilsMt19937UInt(&mut self.rng) } % 2;

		let spawn_point_num = world.enemy_spawn_points.len();

		let idx = unsafe { sys::sceKernelUtilsMt19937UInt(&mut self.rng) } as usize % spawn_point_num;

		// Find a free slot for an enemy

		for off in 0..spawn_point_num {
			let idx = (idx + off) % spawn_point_num;

			if self.enemies[idx].is_some() {
				continue
			}

			let pos = world.enemy_spawn_points[idx];
			let pos = pos.map(|x| x as f32 * world.scale + world.scale * 0.5);

			let kind = match kind {
				0 => EnemyKind::Apple,
				1 => EnemyKind::Pear,
				_ => unreachable!()
			};

			self.enemies[idx] = Some(Enemy::new(
				kind,
				Transform::origin().with_position(Vector3::new(pos[0], 0.0, pos[1]))
			));

			break
		}
	}

	pub fn update<T: PlayfieldPiece>(&mut self, world: &Playfield<'_, T>, dt: f32) -> EnemyUpdate {
		self.enemies.resize_with(world.enemy_spawn_points.len(), || None);

		self.spawn_timer -= dt;

		if self.spawn_timer < 0.0 {
			self.spawn_timer = 2.0;
			self.spawn_new_enemy(world);
		}

		let mut shot_bullets = vec![];
		let mut enemies_gone = vec![];

		for enemy in &mut self.enemies {
			let Some(enemak) = enemy else { continue };

			if enemak.update(dt) {
				shot_bullets.push(enemak.transform.position);
			}

			if enemak.state == EnemyState::Gone {
				enemies_gone.push(enemak.transform.position);
				*enemy = None;
			}
		}

		EnemyUpdate {
			shot_bullets,
			enemies_gone
		}
	}

	pub fn render(&self, assets: &Assets) {
		for enemy in &self.enemies {
			let Some(enemy) = enemy else { continue };

			let model = match enemy.kind {
				EnemyKind::Apple => &assets.apple,
				EnemyKind::Pear => &assets.pear,
			};

			model.draw(&enemy.transform);
		}
	}

	pub fn get_collision_shapes(&self) -> Vec<Option<(Ball, Pose)>> {
		self.enemies.iter()
			.map(|x| if let Some(x) = &x && x.state == EnemyState::Idle { Some(x) } else { None })
			.map(|x| x.map(|x| (Ball::new(1.0), Pose::translation(x.transform.position.x, x.transform.position.z))))
			.collect::<Vec<_>>()
	}

	pub fn get_collision_shapes_as_targets(&self) -> Vec<Option<(Ball, Pose)>> {
		self.enemies.iter()
			.map(|x| if let Some(x) = &x && x.state == EnemyState::Idle { Some(x) } else { None })
			.map(|x| x.map(|x| (Ball::new(1.5), Pose::translation(x.transform.position.x, x.transform.position.z))))
			.collect::<Vec<_>>()
	}

	pub fn get_transform(&self, idx: usize) -> Option<&Transform> {
		self.enemies[idx].as_ref().map(|x| &x.transform)
	}

	pub fn collide_with_bullet(&mut self, idx: usize) {
		if let Some(enemy) = self.enemies[idx].as_mut() {
			enemy.timer = 0.0;
			enemy.state = EnemyState::Despawn;
		}
	}

	pub fn collide_with_player(&mut self, idx: usize) -> Option<usize> {
		self.enemies[idx].as_mut().map(|x| {
			match x.kind {
				EnemyKind::Apple => 3,
				EnemyKind::Pear => 3
			}
		})
	}
}

