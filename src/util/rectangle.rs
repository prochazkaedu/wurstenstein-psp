use psp::sys::{
	self, ScePspFVector3, DisplayPixelFormat, GuContextType, GuSyncMode, GuSyncBehavior,
	GuPrimitive, TextureFilter, TextureEffect, TextureColorComponent,
	FrontFaceDirection, ShadingModel, GuState, TexturePixelFormat, DepthFunc,
	VertexType, ClearBuffer, MipmapLevel,
};

use crate::util::allocate_display_list;

#[repr(C, align(4))]
struct Vertex {
	x: f32,
	y: f32,
	z: f32,
}

#[repr(C, align(4))]
struct VertexTex {
	u: u16,
	v: u16,
	x: f32,
	y: f32,
	z: f32,
}

pub fn colored(position: &[f32; 4], color: &[u8; 4]) {
	unsafe {
		sys::sceGuEnable(GuState::Blend);

		sys::sceGuColor(u32::from_le_bytes(*color));

		let mut mem = allocate_display_list(4);

		mem[0] = Vertex { x: position[2], y: position[3], z: 0.0 };
		mem[1] = Vertex { x: position[0], y: position[3], z: 0.0 };
		mem[2] = Vertex { x: position[2], y: position[1], z: 0.0 };
		mem[3] = Vertex { x: position[0], y: position[1], z: 0.0 };

		sys::sceGumDrawArray(
			GuPrimitive::TriangleStrip,
			VertexType::VERTEX_32BITF | VertexType::TRANSFORM_2D,
			4,
			core::ptr::null(),
			mem.as_ptr() as _,
		);

		sys::sceGuDisable(GuState::Blend);
	}
}

pub fn colored_and_textured(position: &[f32; 4], color: &[u8; 4], texcoords: &[u16; 4]) {
	unsafe {
		sys::sceGuEnable(GuState::Blend);
		sys::sceGuEnable(GuState::Texture2D);

		sys::sceGuColor(u32::from_le_bytes(*color));

		let mut mem = allocate_display_list(4);

		mem[0] = VertexTex { u: texcoords[2], v: texcoords[3], x: position[2], y: position[3], z: 0.0 };
		mem[1] = VertexTex { u: texcoords[0], v: texcoords[3], x: position[0], y: position[3], z: 0.0 };
		mem[2] = VertexTex { u: texcoords[2], v: texcoords[1], x: position[2], y: position[1], z: 0.0 };
		mem[3] = VertexTex { u: texcoords[0], v: texcoords[1], x: position[0], y: position[1], z: 0.0 };

		sys::sceGumDrawArray(
			GuPrimitive::TriangleStrip,
			VertexType::TEXTURE_16BIT | VertexType::VERTEX_32BITF | VertexType::TRANSFORM_2D,
			4,
			core::ptr::null(),
			mem.as_ptr() as _,
		);

		sys::sceGuDisable(GuState::Blend);
		sys::sceGuDisable(GuState::Texture2D);
	}
}

