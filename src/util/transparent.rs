use crate::util::model::Transform;
use crate::util::algebra::{Matrix4, Vector3};

use psp::sys::{self, GuState};

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::vec;

pub struct TransparentObject<'a> {
	transform: &'a Transform,
	transformed_position: Vector3,
	render: Box<dyn FnOnce() + 'a>
}

pub struct TransparentRenderer<'a> {
	objects: Vec<TransparentObject<'a>>
}

impl<'a> TransparentRenderer<'a> {
	pub fn new() -> Self {
		Self {
			objects: vec![]
		}
	}

	pub fn add_object<F: FnOnce() + 'a>(&mut self, transform: &'a Transform, render: F) {
		self.objects.push(TransparentObject {
			transformed_position: Vector3::ZERO,
			transform,
			render: Box::new(render)
		});
	}

	pub fn render(mut self, view_mtx: Matrix4) {
		for obj in &mut self.objects {
			let mut pos = obj.transform.position;
			pos.x *= -1.0;
			obj.transformed_position = view_mtx.mul_vector3(pos);
		}

		self.objects.sort_by(|a, b| a.transformed_position.z.partial_cmp(&b.transformed_position.z).unwrap());

		unsafe {
			sys::sceGuEnable(GuState::Blend);
			sys::sceGuDepthMask(1);
			sys::sceGuDisable(GuState::CullFace);
		}

		for obj in self.objects {
			(obj.render)();
		}

		unsafe {
			sys::sceGuDisable(GuState::Blend);
			sys::sceGuDepthMask(0);
			sys::sceGuEnable(GuState::CullFace);
		}
	}
}

