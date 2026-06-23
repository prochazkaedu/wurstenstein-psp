use psp::sys::TexturePixelFormat;
use psp::vram_alloc::SimpleVramAllocator;

use ab_glyph::{Font as _, FontRef, ScaleFont};

use crate::util::rectangle::{self, Texture};

struct Glyph {
	tex_x: u32,
	tex_y: u32,
	tex_w: u32,
	x_off: i32,
	y_off: i32,
	w: u32,
	h: u32
}

const MIN_CHAR: usize = 0x20;
const MAX_CHAR: usize = 0x7E;
const CHAR_NUM: usize = MAX_CHAR - MIN_CHAR + 1;

struct RenderedFont {
	char_map: heapless::Vec<Glyph, CHAR_NUM>,
	texture: &'static [u32],
	scale: u32,
	tex_w: u32,
	tex_h: u32,
}

impl RenderedFont {
	pub fn new(font: &FontRef, scale: u32, alloc: &SimpleVramAllocator) -> Self {
		let font = font.into_scaled(scale as f32);

		let mut overflow = false;
		let mut w = 0;
		let mut h = 0;
		let mut total_h = 0;

		for c in MIN_CHAR..=MAX_CHAR {
			let glyph = font.glyph_id(char::from_u32(c as u32).unwrap()).with_scale(scale as f32);

			if let Some(q) = font.outline_glyph(glyph) {
				let cw = q.px_bounds().width() as u32;

				if w + cw > 512 {
					overflow = true;
					w = 0;
					total_h += h;
					h = 0;
				}

				w += cw;
				h = h.max(q.px_bounds().height() as u32);
			}
		}

		total_h += h;

		let tex_w = if overflow {
			512
		} else {
			2u32.pow(w.ilog2() + 1)
		};
		let tex_h = 2u32.pow(total_h.ilog2() + 1);

		let texture = alloc.alloc_texture_pixels(
			tex_w,
			tex_h,
			TexturePixelFormat::Psm8888,
		);

		let texture = unsafe {
			core::slice::from_raw_parts_mut(
				texture.as_mut_ptr_direct_to_vram() as *mut u32,
				tex_w as usize * tex_h as usize
			)
		};

		let mut char_map = heapless::Vec::new();

		let mut base_x = 0;
		let mut curr_y = 0;
		let mut base_y = 0;

		for c in MIN_CHAR..=MAX_CHAR {
			let id = font.glyph_id(char::from_u32(c as u32).unwrap());
			let glyph = id.with_scale(scale as f32);

			if let Some(q) = font.outline_glyph(glyph.clone()) {
				let w = q.px_bounds().width() as u32;
				let h = q.px_bounds().height() as u32;

				let cw = q.px_bounds().width() as u32;

				if base_x + cw > 512 {
					base_x = 0;
					base_y += curr_y;
					curr_y = 0;
				}

				let _ = char_map.push(Glyph {
					tex_x: base_x,
					tex_y: base_y,
					x_off: font.h_side_bearing(id) as i32,
					y_off: q.px_bounds().min.y as i32,
					w: font.h_advance(id) as u32,
					tex_w: w,
					h
				});

				q.draw(|x, y, c| {
					assert!(x < w);
					assert!(y < h);
					texture[((y + base_y) * tex_w + x + base_x) as usize] = (((c * 255.0) as u32) << 24) | 0xFFFFFF;
					// texture[(y * tex_w + x + base_x) as usize] = 0xFFFFFFFF;
				});

				base_x += cw;
				curr_y = curr_y.max(q.px_bounds().height() as u32);
			} else {
				let _ = char_map.push(Glyph {
					tex_x: 0,
					tex_y: 0,
					x_off: font.h_side_bearing(id) as i32,
					y_off: 0,
					w: font.h_advance(id) as u32,
					tex_w: 0,
					h: 0
				});
			}
		}

		Self {
			char_map,
			texture,
			scale,
			tex_w,
			tex_h,
		}
	}
}

pub struct Font<'a> {
	fonts: heapless::Vec<RenderedFont, 3>,
	font: FontRef<'a>,
}

pub enum HorizAlign {
	Left,
	Center,
	Right
}

impl<'a> Font<'a> {
	pub fn new(ttf: &'a [u8]) -> Self {
		let font = FontRef::try_from_slice(ttf).unwrap();

		Self {
			fonts: heapless::Vec::new(),
			font,
		}
	}

	pub fn register_scale(&mut self, scale: u32, alloc: &SimpleVramAllocator) {
		let _ = self.fonts.push(RenderedFont::new(&self.font, scale, alloc));
	}

	pub fn draw_string(&self, string: &str, scale: u32, x: i32, y: i32, align: HorizAlign, color: &[u8; 4]) -> i32 {
		let x = match align {
			HorizAlign::Left => x,
			HorizAlign::Center => x - self.calculate_string_width(string, scale) / 2,
			HorizAlign::Right => x - self.calculate_string_width(string, scale),
		};

		let mut acc = 0;

		for c in string.chars() {
			acc += self.draw_character(c, scale, x + acc, y, color);
		}

		acc
	}

	pub fn calculate_string_width(&self, string: &str, scale: u32) -> i32 {
		string.chars()
			.map(|c| self.calculate_char_width(c, scale))
			.sum()
	}

	pub fn calculate_char_width(&self, c: char, scale: u32) -> i32 {
		let Some(font) = self.fonts.iter().find(|x| x.scale == scale) else {
			return 0;
		};

		let Some(idx) = (c as usize).checked_sub(MIN_CHAR) else {
			return 0;
		};

		let Some(c) = font.char_map.get(idx) else {
			return 0;
		};

		c.w as i32
	}

	pub fn draw_character(&self, c: char, scale: u32, x: i32, y: i32, color: &[u8; 4]) -> i32 {
		let Some(font) = self.fonts.iter().find(|x| x.scale == scale) else {
			return 0;
		};

		let Some(idx) = (c as usize).checked_sub(MIN_CHAR) else {
			return 0;
		};

		let Some(c) = font.char_map.get(idx) else {
			return 0;
		};

		let tex_dimensions = &[
			c.tex_x as u16,
			c.tex_y as u16,
			(c.tex_x + c.tex_w) as u16,
			(c.tex_y + c.h) as u16,
		];

		let dimensions = &[
			(x + c.x_off) as f32,
			(y + c.y_off) as f32,
			(x + c.x_off + c.tex_w as i32) as f32,
			(y + c.y_off + c.h as i32) as f32,
		];

		rectangle::colored_and_textured(dimensions, color, tex_dimensions, &Texture {
			ptr: font.texture.as_ptr() as usize,
			width: font.tex_w as usize,
			height: font.tex_h as usize,
			linear_filter: false
		});

		c.w as i32
	}
}


