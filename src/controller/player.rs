use parry2d::math::Pose;
use parry2d::shape::Cuboid;

use crate::assets::BoundingBox;
use crate::playfield::{Playfield, PlayfieldPiece};
use crate::controller::powerup::PowerupKind;
use crate::util::model::Transform;
use crate::util::algebra::Vector2;

#[derive(PartialEq, Eq)]
enum PlayerState {
	Spawning,
	Alive,
	StartedDying,
	Dead
}

pub struct PlayerController {
	pub x_movement: f32,
	pub y_movement: f32,
	pub jump: bool,
	pub has_contact_with_world: bool,
	timer: f32,
	state: PlayerState,
	health: usize,
	ammo: usize,
	powerup_speed_timer: f32,
	fall_jump_triggered: bool,
	force_jump: bool,
	damage_timeout: f32,
	already_reported_death: bool,
	bounding_box: BoundingBox,
	gravity: f32,
	xz_force: [f32; 2],
	transform: Transform
}

pub enum PlayerAction {
	Jumped,
	StartedDying,
	Dead
}

pub struct PlayerStats {
	pub health: usize,
	pub ammo: usize,
	pub speed_timer: f32
}

pub const MAX_AMMO: usize = 10;
pub const MAX_HEALTH: usize = 10;
pub const MAX_SPEED_TIMER: f32 = 10.0;

impl PlayerController {
	pub fn new(spawn: Transform, bounding_box: BoundingBox) -> Self {
		Self {
			x_movement: 0.0,
			y_movement: 0.0,
			jump: false,
			has_contact_with_world: true,
			timer: 0.0,
			state: PlayerState::Spawning,
			health: MAX_HEALTH,
			ammo: MAX_AMMO,
			powerup_speed_timer: 0.0,
			fall_jump_triggered: false,
			force_jump: false,
			damage_timeout: 0.0,
			already_reported_death: false,
			bounding_box,
			gravity: 0.0,
			xz_force: [0.0, 0.0],
			transform: spawn
		}
	}

	pub fn update_yaw(&mut self, yaw: f32) {
		self.transform.rotation.x = -(yaw + 90.0).to_radians();
	}

	pub fn update<T: PlayfieldPiece>(&mut self, world: &Playfield<'_, T>, dt: f32) -> Option<PlayerAction> {
		self.timer += dt;

		match self.state {
			PlayerState::Spawning => {
				self.state = PlayerState::Alive;
				None
			},
			PlayerState::Alive => {
				let mut action = None;

				let max_speed: f32 = if self.powerup_speed_timer > 0.0 { 10.0 } else { 5.0 };

				const ACCEL: f32 = 20.0;
				const BASE_GRAVITY: f32 = 10.0;
				const BASE_GRAVITY_ACCEL: f32 = 40.0;

				// Update XZ coordinates

				let accel = ACCEL * dt;

				let mut xz_force = self.xz_force;

				if self.x_movement < 0.0 {
					xz_force[0] = f32::max(xz_force[0] - accel, max_speed * self.x_movement);
				} else if xz_force[0] < 0.0 {
					xz_force[0] = f32::min(xz_force[0] + accel, 0.0);
				}

				if self.x_movement > 0.0 {
					xz_force[0] = f32::min(xz_force[0] + accel, max_speed * self.x_movement);
				} else if xz_force[0] > 0.0 {
					xz_force[0] = f32::max(xz_force[0] - accel, 0.0);
				}

				if self.y_movement < 0.0 {
					xz_force[1] = f32::max(xz_force[1] - accel, max_speed * self.y_movement);
				} else if xz_force[1] < 0.0 {
					xz_force[1] = f32::min(xz_force[1] + accel, 0.0);
				}

				if self.y_movement > 0.0 {
					xz_force[1] = f32::min(xz_force[1] + accel, max_speed * self.y_movement);
				} else if xz_force[1] > 0.0 {
					xz_force[1] = f32::max(xz_force[1] - accel, 0.0);
				}

				self.xz_force = xz_force;

				let rotated = Vector2::from_array(xz_force).rotate(-self.transform.rotation.x);

				self.transform.position.x += rotated.x * dt;
				self.transform.position.z += rotated.y * dt;

				// Fall to death if the player is not touching the world, or if the player has fallen enough (we allow a little edgebug to make the game more fair)

				let floor = if !self.has_contact_with_world || self.transform.position.y < -world.height {
					world.death_barrier
				} else {
					if self.transform.position.y < -0.01 && !self.fall_jump_triggered {
						self.fall_jump_triggered = true;
						self.force_jump = true;
					}

					0.0
				};

				// Allow jumping if at (or very near) floor level

				if self.jump && self.transform.position.y <= floor + 0.01 {
					self.force_jump = true;
				}

				self.jump = false;

				if self.force_jump {
					self.gravity = -BASE_GRAVITY;
					action = Some(PlayerAction::Jumped);
				}

				self.force_jump = false;

				// Update Y coordinate

				self.transform.position.y = f32::max(floor, self.transform.position.y - self.gravity * dt * 2.0);
				
				if self.transform.position.y == floor && self.gravity >= 0.0 {
					// If at floor level, reset gravity
					self.gravity = 0.0;
					self.fall_jump_triggered = false;
				} else {
					self.gravity = f32::min(self.gravity + BASE_GRAVITY_ACCEL * dt, BASE_GRAVITY);
				}

				if self.transform.position.y <= world.death_barrier + 0.01 && !self.already_reported_death {
					self.state = PlayerState::StartedDying;
					self.already_reported_death = false;
					self.timer = 0.0;
				}

				self.damage_timeout -= dt;

				self.powerup_speed_timer -= dt;

				action
			},
			PlayerState::StartedDying => {
				if self.timer >= 0.5 {
					self.timer = 0.0;
					self.already_reported_death = false;
					self.state = PlayerState::Dead;
					None?
				}

				self.health = 0;
				self.ammo = 0;
				self.powerup_speed_timer = 0.0;

				self.transform.rotation.x += 8.0 * dt;
				let scale = (1.0 - self.timer * 2.0).max(0.0);
				self.transform.scale.x = scale;
				self.transform.scale.y = scale;
				self.transform.scale.z = scale;

				if self.already_reported_death { None? }
				self.already_reported_death = true;
				Some(PlayerAction::StartedDying)
			},
			PlayerState::Dead => {
				self.transform.scale.x = 0.0;
				self.transform.scale.y = 0.0;
				self.transform.scale.z = 0.0;

				if self.already_reported_death { None? }
				self.already_reported_death = true;
				Some(PlayerAction::Dead)
			}
		}
	}

	pub fn is_dead(&self) -> bool {
		self.state == PlayerState::StartedDying || self.state == PlayerState::Dead
	}

	pub fn get_transform(&self) -> &Transform {
		&self.transform
	}

	pub fn get_collision_shape(&self) -> (Cuboid, Pose) {
		self.bounding_box.get_collision_shape()
	}

	pub fn pick_up_powerup(&mut self, kind: PowerupKind) {
		match kind {
			PowerupKind::Health => {
				self.health = (self.health + 5).min(MAX_HEALTH);
			},
			PowerupKind::Speed => {
				self.powerup_speed_timer = MAX_SPEED_TIMER;
			},
			PowerupKind::Energy => {
				self.ammo = MAX_AMMO;
			}
		}
	}

	pub fn get_stats(&self) -> PlayerStats {
		PlayerStats {
			health: self.health,
			ammo: self.ammo,
			speed_timer: self.powerup_speed_timer.max(0.0)
		}
	}

	pub fn decrease_hp(&mut self, amount: usize) -> bool {
		if self.damage_timeout > 0.0 {
			return false;
		}

		self.health = self.health.saturating_sub(amount);
		self.damage_timeout = 0.5;

		if self.health == 0 {
			self.state = PlayerState::StartedDying;
			self.already_reported_death = false;
			self.timer = 0.0;
		}

		true
	}

	pub fn fire_bullet(&mut self) -> bool {
		if self.ammo == 0 {
			return false;
		}

		self.ammo -= 1;
		true
	}
}

