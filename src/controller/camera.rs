use core::ops::RangeInclusive;

use crate::util::algebra::{Matrix4, Vector3};

pub enum Directions {
	Left,
	Right,
	Up,
	Down,
	Forward,
	Backward,
}

#[derive(Debug, Clone)]
pub struct Camera {
	position: Vector3,
	front: Vector3,
	up: Vector3,
	right: Vector3,
	world_up: Vector3,

	use_pov: bool,
	target: Vector3,
	distance: f32,

	move_forward: bool,
	move_backward: bool,
	move_left: bool,
	move_right: bool,
	move_up: bool,
	move_down: bool,
	move_fast: bool,

	yaw: f32,
	pitch: f32,
	pitch_range: RangeInclusive<f32>,
	speed: f32,
	sensitivity: f32,
	zoom: f32,
}

impl Camera {
	pub fn new(position: Vector3) -> Self {
		let yaw = 45.0f32;
		let pitch = -23.0f32;
		let world_up = Vector3::new(0.0, 1.0, 0.0);
		let front = Camera::calc_front(yaw, pitch);
		let right = Camera::calc_right(front, world_up);
		let up = Camera::calc_up(right, front);

		Self {
			position,
			front,
			up,
			right,
			world_up,
			use_pov: false,
			target: Vector3::ZERO,
			distance: 20.0f32,
			move_forward: false,
			move_backward: false,
			move_left: false,
			move_right: false,
			move_up: false,
			move_down: false,
			move_fast: false,
			yaw,
			pitch,
			pitch_range: -89.9..=89.9,
			speed: 2.5f32,
			sensitivity: 0.1f32,
			zoom: 45.0f32,
		}
	}

	pub fn set_pov(&mut self, enabled: bool) {
		self.use_pov = enabled;
	}

	pub fn set_target(&mut self, target: Vector3) {
		self.target = target;
	}

	pub fn set_pitch_range(&mut self, range: RangeInclusive<f32>) {
		if self.pitch < *range.start() {
			self.pitch = *range.start();
		}

		if self.pitch > *range.end() {
			self.pitch = *range.end();
		}

		self.pitch_range = range;
		self.update_vectors();
	}

	pub fn get_zoom(&self) -> f32 {
		self.zoom
	}

	pub fn get_position(&self) -> Vector3 {
		if self.use_pov {
			self.target - self.front * self.distance
		} else {
			self.position
		}
	}

	pub fn get_front(&self) -> &Vector3 {
		&self.front
	}

	pub fn get_yaw_pitch(&self) -> (f32, f32) {
		(self.yaw, self.pitch)
	}

	pub fn get_view_matrix(&self) -> Matrix4 {
		if self.use_pov {
			let eye = self.target - self.front * self.distance;
			Matrix4::look_at(eye, self.target, self.up)
		} else {
			Matrix4::look_at(self.position, self.position + self.front, self.up)
		}
	}

	pub fn move_fast(&mut self, fast: bool) {
		self.move_fast = fast;
	}

	pub fn key_interact(&mut self, direction: Directions, pressed: bool) {
		match direction {
			Directions::Left => self.move_left = pressed,
			Directions::Right => self.move_right = pressed,
			Directions::Up => self.move_up = pressed,
			Directions::Down => self.move_down = pressed,
			Directions::Forward => self.move_forward = pressed,
			Directions::Backward => self.move_backward = pressed,
		}
	}

	pub fn update_position(&mut self, dt: f32) {
		let dt = if self.move_fast { dt * 3.0 } else { dt };

		if self.use_pov {
			let orbit_speed = 67.0f32;

			if self.move_left {
				self.yaw -= orbit_speed * dt;
				self.update_vectors();
			}

			if self.move_right {
				self.yaw += orbit_speed * dt;
				self.update_vectors();
			}

			if self.move_forward {
				self.distance = (self.distance - self.speed * dt).max(0.5);
			}

			if self.move_backward {
				self.distance += self.speed * dt;
			}

			if self.move_up {
				self.pitch = (self.pitch + orbit_speed * dt).clamp(*self.pitch_range.start(), *self.pitch_range.end());
				self.update_vectors();
			}

			if self.move_down {
				self.pitch = (self.pitch - orbit_speed * dt).clamp(*self.pitch_range.start(), *self.pitch_range.end());
				self.update_vectors();
			}
		} else {
			if self.move_forward {
				self.position += self.front * self.speed * dt;
			}

			if self.move_left {
				self.position -= self.right * self.speed * dt;
			}

			if self.move_right {
				self.position += self.right * self.speed * dt;
			}

			if self.move_up {
				self.position += self.up * self.speed * dt;
			}

			if self.move_down {
				self.position -= self.up * self.speed * dt;
			}

			if self.move_backward {
				self.position -= self.front * self.speed * dt;
			}
		}
	}

	pub fn mouse_interact(&mut self, dx: f32, dy: f32) {
		self.yaw += dx * self.sensitivity;
		self.pitch = (self.pitch - dy * self.sensitivity).clamp(*self.pitch_range.start(), *self.pitch_range.end());
		self.update_vectors();
	}

	pub fn scroll_wheel_interact(&mut self, delta: f32) {
		self.zoom = (self.zoom + delta).clamp(30.0, 150.0);
	}

	fn update_vectors(&mut self) {
		self.front = Camera::calc_front(self.yaw, self.pitch);
		self.right = Camera::calc_right(self.front, self.world_up);
		self.up = Camera::calc_up(self.right, self.front);
	}

	fn calc_front(yaw: f32, pitch: f32) -> Vector3 {
		let ya = yaw.to_radians();
		let pa = pitch.to_radians();

		let pa_cos = unsafe { psp::math::cosf(pa) };
		let pa_sin = unsafe { psp::math::sinf(pa) };
		let ya_cos = unsafe { psp::math::cosf(ya) };
		let ya_sin = unsafe { psp::math::sinf(ya) };

		Vector3::new(ya_cos * pa_cos, pa_sin, ya_sin * pa_cos).normalize()
	}

	fn calc_right(front: Vector3, world_up: Vector3) -> Vector3 {
		front.cross(world_up).normalize()
	}

	fn calc_up(right: Vector3, front: Vector3) -> Vector3 {
		right.cross(front).normalize()
	}
}

