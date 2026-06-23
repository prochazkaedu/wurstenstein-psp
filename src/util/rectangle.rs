use psp::sys::{self, GuPrimitive, GuState, MipmapLevel, TextureFilter, TexturePixelFormat, VertexType};

use crate::util::{allocate_display_list, arena::Arena};

struct ColoredRectangle {
	color: u32,
	position: [f32; 4]
}

struct TexturedRectangle {
	color: u32,
	position: [f32; 4],
	texcoords: [u16; 4]
}

#[derive(Clone)]
pub struct Texture {
	pub ptr: usize,
	pub width: usize,
	pub height: usize,
	pub linear_filter: bool
}

struct TextureReference {
	rectangle_arena_idx: usize,
	texture: Texture
}

static mut COLORED_ARENA: Arena<ColoredRectangle, 128> = Arena::init();
static mut TEXTURED_ARENA: Arena<TexturedRectangle, 4096> = Arena::init();
static mut TEXTURE_REF_ARENA: Arena<TextureReference, 16> = Arena::init();

#[repr(C, align(4))]
struct Vertex {
	color: u32,
	x: f32,
	y: f32,
	z: f32,
}

#[repr(C, align(4))]
struct VertexTex {
	u: u16,
	v: u16,
	color: u32,
	x: f32,
	y: f32,
	z: f32,
}

pub fn colored(position: &[f32; 4], color: &[u8; 4]) {
	#[allow(static_mut_refs)]
	unsafe {
		COLORED_ARENA.push(ColoredRectangle {
			color: u32::from_le_bytes(*color),
			position: *position
		});
	}
}

pub fn colored_and_textured(position: &[f32; 4], color: &[u8; 4], texcoords: &[u16; 4], texture: &Texture) {
	#[allow(static_mut_refs)]
	unsafe {
		let idx = TEXTURED_ARENA.len();

		let last_tex = TEXTURE_REF_ARENA.get_top();

		if matches!(last_tex, Some(t) if t.texture.ptr != texture.ptr) || last_tex.is_none() {
			TEXTURE_REF_ARENA.push(TextureReference {
				rectangle_arena_idx: idx,
				texture: texture.clone()
			})
		}

		TEXTURED_ARENA.push(TexturedRectangle {
			color: u32::from_le_bytes(*color),
			position: *position,
			texcoords: *texcoords
		});
	}
}

pub fn draw_all() {
	#[allow(static_mut_refs)]
	unsafe {
		sys::sceGuEnable(GuState::Blend);

		let colored = COLORED_ARENA.get_all();

		let mem = allocate_display_list(6 * colored.len());

		for (idx, rect) in colored.iter().enumerate() {
			let idx = idx * 6;
			mem[idx    ] = Vertex { color: rect.color, x: rect.position[2], y: rect.position[3], z: 0.0 };
			mem[idx + 1] = Vertex { color: rect.color, x: rect.position[0], y: rect.position[3], z: 0.0 };
			mem[idx + 2] = Vertex { color: rect.color, x: rect.position[2], y: rect.position[1], z: 0.0 };
			mem[idx + 3] = Vertex { color: rect.color, x: rect.position[2], y: rect.position[1], z: 0.0 };
			mem[idx + 4] = Vertex { color: rect.color, x: rect.position[0], y: rect.position[3], z: 0.0 };
			mem[idx + 5] = Vertex { color: rect.color, x: rect.position[0], y: rect.position[1], z: 0.0 };
		}

		sys::sceGumDrawArray(
			GuPrimitive::Triangles,
			VertexType::COLOR_8888 | VertexType::VERTEX_32BITF | VertexType::TRANSFORM_2D,
			6 * colored.len() as i32,
			core::ptr::null(),
			mem.as_ptr() as _,
		);

		COLORED_ARENA.clear();

		sys::sceGuEnable(GuState::Texture2D);

		let texture_refs = TEXTURE_REF_ARENA.get_all();
		let textured = TEXTURED_ARENA.get_all();

		let mut last_idx = 0;

		for i in 0..texture_refs.len() {
			let top = texture_refs.get(i + 1).map(|x| x.rectangle_arena_idx).unwrap_or(textured.len());

			let tex = &texture_refs[i];

			sys::sceGuTexMode(TexturePixelFormat::Psm8888, 0, 0, 0);
			let (min, mag) = match tex.texture.linear_filter {
				true => (TextureFilter::Linear, TextureFilter::Linear),
				false => (TextureFilter::Nearest, TextureFilter::Nearest)
			};
			sys::sceGuTexFilter(min, mag);
			sys::sceGuTexImage(MipmapLevel::None, tex.texture.width as i32, tex.texture.height as i32, tex.texture.width as i32, tex.texture.ptr as _);

			let mem = allocate_display_list(6 * (top - last_idx));

			for idx in last_idx..top {
				let rect = &textured[idx];
				let idx = (idx - last_idx) * 6;
				mem[idx    ] = VertexTex { u: rect.texcoords[2], v: rect.texcoords[3], color: rect.color, x: rect.position[2], y: rect.position[3], z: 0.0 };
				mem[idx + 1] = VertexTex { u: rect.texcoords[0], v: rect.texcoords[3], color: rect.color, x: rect.position[0], y: rect.position[3], z: 0.0 };
				mem[idx + 2] = VertexTex { u: rect.texcoords[2], v: rect.texcoords[1], color: rect.color, x: rect.position[2], y: rect.position[1], z: 0.0 };
				mem[idx + 3] = VertexTex { u: rect.texcoords[2], v: rect.texcoords[1], color: rect.color, x: rect.position[2], y: rect.position[1], z: 0.0 };
				mem[idx + 4] = VertexTex { u: rect.texcoords[0], v: rect.texcoords[3], color: rect.color, x: rect.position[0], y: rect.position[3], z: 0.0 };
				mem[idx + 5] = VertexTex { u: rect.texcoords[0], v: rect.texcoords[1], color: rect.color, x: rect.position[0], y: rect.position[1], z: 0.0 };
			}

			sys::sceGumDrawArray(
				GuPrimitive::Triangles,
				VertexType::COLOR_8888 | VertexType::TEXTURE_16BIT | VertexType::VERTEX_32BITF | VertexType::TRANSFORM_2D,
				6 * (top - last_idx) as i32,
				core::ptr::null(),
				mem.as_ptr() as _,
			);

			last_idx = top;
		}

		TEXTURED_ARENA.clear();
		TEXTURE_REF_ARENA.clear();

		sys::sceGuDisable(GuState::Texture2D);

		sys::sceGuDisable(GuState::Blend);
	}
}

