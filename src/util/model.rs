use crate::util::algebra::Vector3;

use psp::sys::{
	self, BlendFactor, BlendOp, ClearBuffer, DepthFunc, DisplayPixelFormat, FrontFaceDirection, GuContextType, GuPrimitive, GuState, GuSyncBehavior, GuSyncMode, GuTexWrapMode, MipmapLevel, ScePspFVector3, ShadingModel, TextureColorComponent, TextureEffect, TextureFilter, TexturePixelFormat, VertexType
};

extern crate alloc;
use alloc::vec::Vec;

struct ModelTexture {
	texture: Vec<u8>,
	start_offset: usize,
	width: usize,
	height: usize,
	linear_filter: bool
}

#[derive(Clone)]
pub struct Transform {
	pub position: Vector3,
	pub scale: Vector3,
	/// Yaw, Pitch, Roll
	pub rotation: Vector3
}

impl Transform {
	pub fn origin() -> Self {
		Self {
			position: Vector3::new(0.0, 0.0, 0.0),
			scale: Vector3::new(1.0, 1.0, 1.0),
			rotation: Vector3::new(0.0, 0.0, 0.0)
		}
	}

	pub fn with_position(mut self, position: Vector3) -> Self {
		self.position = position;
		self
	}

	pub fn with_scale(mut self, scale: Vector3) -> Self {
		self.scale = scale;
		self
	}

	pub fn with_rotation(mut self, rotation: Vector3) -> Self {
		self.rotation = rotation;
		self
	}
}

pub struct Model {
	vtx: Option<assets::Model>,
	tex: Option<ModelTexture>,
	scale: Vector3
}

impl Model {
	pub fn new() -> Self {
		Self {
			vtx: None,
			tex: None,
			scale: Vector3::UNIT
		}
	}

	pub fn add_mesh(&mut self, model: assets::Model) {
		self.vtx = Some(model);
	}

	pub fn add_texture(&mut self, tex: assets::Texture, linear_filter: bool) {
		let mut texture = Vec::with_capacity(tex.width * tex.height * 4 + 16);

		let start_offset = match texture.as_ptr() as usize & 0xF {
			0 => 0,
			x => 0x10 - x
		};

		for x in 0..start_offset {
			texture.push(0);
		}

		assert_eq!((texture.as_ptr() as usize + texture.len()) & 0xF, 0);

		texture.extend_from_slice(&tex.bytes);

		self.tex = Some(ModelTexture {
			texture,
			start_offset,
			width: tex.width,
			height: tex.height,
			linear_filter
		});
	}

	pub fn with_mesh(mut self, mesh: assets::Model) -> Self {
		self.add_mesh(mesh);
		self
	}

	pub fn with_texture(mut self, tex: assets::Texture, linear_filter: bool) -> Self {
		self.add_texture(tex, linear_filter);
		self
	}

	pub fn with_scale(mut self, scale: Vector3) -> Self {
		self.scale = scale;
		self
	}

	pub fn draw(&self, transform: &Transform) {
		self.draw_colored(transform, &[255; 4]);
	}

	pub fn draw_colored(&self, transform: &Transform, color: &[u8; 4]) {
		unsafe {
			if let Some(tex) = &self.tex {
				sys::sceGuTexMode(TexturePixelFormat::Psm8888, 0, 0, 0);
				// assert!(tex.tex.bytes.as_ptr() as u32 & 0xF == 0);
				sys::sceGuTexImage(MipmapLevel::None, tex.width as i32, tex.height as i32, tex.width as i32, tex.texture[tex.start_offset..].as_ptr() as *const _);
			}

			if let Some(vtx) = &self.vtx {
				sys::sceGumPushMatrix();

				sys::sceGumTranslate(&ScePspFVector3 {
					x: transform.position.x,
					y: transform.position.y,
					z: transform.position.z,
				});
				sys::sceGumScale(&ScePspFVector3 {
					x: transform.scale.x * self.scale.x,
					y: transform.scale.y * self.scale.y,
					z: transform.scale.z * self.scale.z,
				});
				sys::sceGumRotateZ(transform.rotation.z);
				sys::sceGumRotateY(transform.rotation.x);
				sys::sceGumRotateX(transform.rotation.y);

				if self.tex.is_some() {
					sys::sceGuEnable(GuState::Texture2D);
				}

				sys::sceGuEnable(GuState::Blend);
				sys::sceGuColor(u32::from_le_bytes(*color));

				sys::sceGumDrawArray(
					GuPrimitive::Triangles,
					VertexType::TEXTURE_32BITF | VertexType::NORMAL_32BITF | VertexType::VERTEX_32BITF | VertexType::INDEX_16BIT | VertexType::TRANSFORM_3D,
					vtx.indices.len() as i32,
					vtx.indices.as_ptr() as _,
					vtx.vertices.as_ptr() as _
				);

				if self.tex.is_some() {
					sys::sceGuDisable(GuState::Texture2D);
				}

				sys::sceGuDisable(GuState::Blend);

				sys::sceGumPopMatrix();
			}
		}
	}
}
