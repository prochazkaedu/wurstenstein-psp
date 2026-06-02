use psp::{SCREEN_WIDTH, SCREEN_HEIGHT};
use psp::Align16;
use psp::math::sinf;
use psp::sys::{
	self, ScePspFVector3, DisplayPixelFormat, GuContextType, GuSyncMode, GuSyncBehavior,
	GuPrimitive, TextureFilter, TextureEffect, TextureColorComponent,
	FrontFaceDirection, ShadingModel, GuState, TexturePixelFormat, DepthFunc,
	VertexType, ClearBuffer, MipmapLevel,
};

const WIDTH: usize = SCREEN_WIDTH as usize;
const HEIGHT: usize = SCREEN_HEIGHT as usize;

#[repr(C, align(4))]
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
	let x = unsafe { sinf(seed) } * 43758.5453123;
	x - libm::floorf(x)
}

extern crate alloc;

pub fn draw(time: f32) {
	let mut points: Align16<heapless::Vec::<ColoredPoint, HEIGHT>> = Align16(heapless::Vec::new());

	for y in 0..HEIGHT {
		let seed_val = hash(y as f32);

		let speed = seed_val * 0.5 + 0.5;
		let x = speed * (time / 3.0 + 6.7) + hash(y as f32 * 1.234);
		let x = x - libm::floorf(x);

		let _ = points.0.push(ColoredPoint {
			r: (seed_val * 255.0) as u8,
			g: (seed_val * 255.0) as u8,
			b: (seed_val * 255.0) as u8,
			a: 255,
			x: x * WIDTH as f32,
			y: y as f32,
			z: 0.0
		});
	}

	unsafe {
		sys::sceGumDrawArray(
			GuPrimitive::Points,
			VertexType::COLOR_8888 | VertexType::VERTEX_32BITF | VertexType::TRANSFORM_2D,
			points.0.len() as i32,
			core::ptr::null(),
			points.0.as_ptr() as _,
		);
	}

	// unsafe {
	// 	gl::PointSize(3.0);
	// 	gl::Begin(gl::POINTS);
	//
	// 	for y in 0..screen_h {
	// 		let seed_val = hash(y as f32);
	//
	// 		let speed = seed_val * 0.5 + 0.5;
	// 		let x = (speed * (time / 3.0 + 6.7) + hash(y as f32 * 1.234)).fract();
	//
	// 		gl::Color3f(seed_val, seed_val, seed_val);
	// 		gl::Vertex3f(x, y as f32 / screen_h as f32, 0.0);
	// 	}
	//
	// 	gl::End();
	// }
}

