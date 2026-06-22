use psp::{SCREEN_WIDTH, SCREEN_HEIGHT};
use psp::math::sinf;
use psp::sys::{self, GuPrimitive, GuState, VertexType};

use crate::util::allocate_display_list;

use once_cell_no_std::OnceCell;

const WIDTH: usize = SCREEN_WIDTH as usize;
const HEIGHT: usize = SCREEN_HEIGHT as usize;

#[repr(C, align(4))]
#[derive(PartialEq)]
struct ColoredPoint {
	r: u8,
	g: u8,
	b: u8,
	a: u8,
	x: f32,
	y: f32,
	z: f32,
}

fn hash(seed: f32) -> f32 {
	let x = unsafe { sinf(seed) } * 43758.545;
	x - psp::math::floorf(x)
}

static HASH_CACHE: OnceCell<[f32; HEIGHT]> = OnceCell::new();

pub fn draw(time: f32) {
	let hashes = match HASH_CACHE.get() {
		Some(val) => val,
		None => {
			let mut arr = [0.0; HEIGHT];

			for (seed, y) in arr.iter_mut().enumerate() {
				*y = hash(seed as f32);
			}

			let _ = HASH_CACHE.set(arr);

			HASH_CACHE.get().unwrap()
		}
	};

	let mem = allocate_display_list(HEIGHT);

	for y in 0..HEIGHT {
		let seed_val = hashes[y];

		let speed = seed_val * 0.5 + 0.5;
		let x = speed * (time / 3.0 + 6.7);
		let x = (x * WIDTH as f32) as usize % WIDTH;

		mem[y] = ColoredPoint {
			r: (seed_val * 255.0) as u8,
			g: (seed_val * 255.0) as u8,
			b: (seed_val * 255.0) as u8,
			a: 255,
			x: x as f32,
			y: y as f32,
			z: 0.0
		};
	}

	unsafe {
		sys::sceGuDisable(GuState::Texture2D);

		sys::sceGumDrawArray(
			GuPrimitive::Points,
			VertexType::COLOR_8888 | VertexType::VERTEX_32BITF | VertexType::TRANSFORM_2D,
			HEIGHT as i32,
			core::ptr::null(),
			mem.as_ptr() as _,
		);
	}
}

