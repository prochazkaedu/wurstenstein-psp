#![allow(unused)]

use core::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};
use psp::math::{acosf, cosf, sinf, hypotf};

#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
	pub x: f32,
	pub y: f32
}

impl Vector2 {
	pub const ZERO: Self = Self::new(0.0, 0.0);
	pub const UNIT: Self = Self::new(1.0, 1.0);

	pub const fn new(x: f32, y: f32) -> Self {
		Self { x, y }
	}

	pub const fn from_array(arr: [f32; 2]) -> Self {
		Self::new(arr[0], arr[1])
	}

	pub fn length(self) -> f32 {
		hypotf(self.x, self.y)
	}

	pub fn normalize(self) -> Self {
		self / self.length()
	}

	pub fn rotate(self, angle: f32) -> Self {
		let cos = unsafe { cosf(angle) };
		let sin = unsafe { sinf(angle) };

		Self {
			x: self.x * cos - self.y * sin,
			y: self.x * sin + self.y * cos
		}
	}

	pub fn cross(self, other: Self) -> f32 {
		self.x * other.y - self.y * other.x
	}

	pub fn dot(self, other: Self) -> f32 {
		self.x * other.x + self.y * other.y
	}

	pub fn angle(self, other: Self) -> f32 {
		let vec1 = self / self.length();
		let vec2 = other / other.length();
		acosf(vec1.dot(vec2))
	}

	pub const fn as_array(self) -> [f32; 2] {
		[self.x, self.y]
	}
}

impl Add<Vector2> for Vector2 {
	type Output = Self;

	fn add(self, rhs: Vector2) -> Self::Output {
		Self {
			x: self.x + rhs.x,
			y: self.y + rhs.y
		}
	}
}

impl Sub<Vector2> for Vector2 {
	type Output = Self;

	fn sub(self, rhs: Vector2) -> Self::Output {
		Self {
			x: self.x - rhs.x,
			y: self.y - rhs.y
		}
	}
}

impl Mul<f32> for Vector2 {
	type Output = Self;

	fn mul(self, rhs: f32) -> Self::Output {
		Self {
			x: self.x * rhs,
			y: self.y * rhs
		}
	}
}

impl Div<f32> for Vector2 {
	type Output = Self;

	fn div(self, rhs: f32) -> Self::Output {
		Self {
			x: self.x / rhs,
			y: self.y / rhs
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Vector3 {
	pub x: f32,
	pub y: f32,
	pub z: f32
}

impl Vector3 {
	pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
	pub const UNIT: Self = Self::new(1.0, 1.0, 1.0);

	pub const fn new(x: f32, y: f32, z: f32) -> Self {
		Self { x, y, z }
	}

	pub const fn from_array(arr: [f32; 3]) -> Self {
		Self::new(arr[0], arr[1], arr[2])
	}

	pub fn length(self) -> f32 {
		hypotf(hypotf(self.x, self.y), self.z)
	}

	pub fn normalize(self) -> Self {
		self / self.length()
	}

	pub fn cross(self, other: Self) -> Vector3 {
		Self::new(
			self.y * other.z - self.z * other.y,
			self.z * other.x - self.x * other.z,
			self.x * other.y - self.y * other.x,
		)
	}

	pub fn dot(self, other: Self) -> f32 {
		self.x * other.x + self.y * other.y + self.z * other.z
	}

	pub const fn as_array(self) -> [f32; 3] {
		[self.x, self.y, self.z]
	}
}

impl Add<Vector3> for Vector3 {
	type Output = Self;

	fn add(self, rhs: Vector3) -> Self::Output {
		Self {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
			z: self.z + rhs.z
		}
	}
}

impl AddAssign<Vector3> for Vector3 {
	fn add_assign(&mut self, rhs: Vector3) {
		self.x += rhs.x;
		self.y += rhs.y;
		self.z += rhs.z;
	}
}

impl Sub<Vector3> for Vector3 {
	type Output = Self;

	fn sub(self, rhs: Vector3) -> Self::Output {
		Self {
			x: self.x - rhs.x,
			y: self.y - rhs.y,
			z: self.z - rhs.z
		}
	}
}

impl SubAssign<Vector3> for Vector3 {
	fn sub_assign(&mut self, rhs: Vector3) {
		self.x -= rhs.x;
		self.y -= rhs.y;
		self.z -= rhs.z;
	}
}

impl Mul<f32> for Vector3 {
	type Output = Self;

	fn mul(self, rhs: f32) -> Self::Output {
		Self {
			x: self.x * rhs,
			y: self.y * rhs,
			z: self.z * rhs
		}
	}
}

impl Div<f32> for Vector3 {
	type Output = Self;

	fn div(self, rhs: f32) -> Self::Output {
		Self {
			x: self.x / rhs,
			y: self.y / rhs,
			z: self.z / rhs
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Matrix4 {
	pub inner: [f32; 16]
}

impl Matrix4 {
	const ZERO: Self = Self { inner: [0.0; _] };

	pub fn new(inner: [f32; 16]) -> Self {
		Self { inner }
	}

	pub fn look_at(eye: Vector3, center: Vector3, up: Vector3) -> Self {
		let f = (center - eye).normalize();
		let u = up.normalize();
		let s = f.cross(u).normalize();
		let u = s.cross(f);

		Self {
			inner: [
				s.x, u.x, -f.x, 0.0,
				s.y, u.y, -f.y, 0.0,
				s.z, u.z, -f.z, 0.0,
				-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0
			]
		}
	}

	pub fn as_array(self) -> [f32; 16] {
		self.inner
	}

	pub fn mul_vector3(&self, vec: Vector3) -> Vector3 {
		Vector3::new(
			self.inner[0] * vec.x + self.inner[1] * vec.y + self.inner[2] * vec.z + self.inner[3],
			self.inner[4] * vec.x + self.inner[5] * vec.y + self.inner[6] * vec.z + self.inner[7],
			self.inner[8] * vec.x + self.inner[9] * vec.y + self.inner[10] * vec.z + self.inner[11],
		)
	}
}

