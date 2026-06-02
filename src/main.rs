#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

mod util;

use core::{ptr, f32::consts::PI};
use psp::Align16;
use psp::sys::{
	self, BlendFactor, BlendOp, ClearBuffer, DepthFunc, DisplayPixelFormat, FrontFaceDirection, GuContextType, GuPrimitive, GuState, GuSyncBehavior, GuSyncMode, GuTexWrapMode, MipmapLevel, ScePspFVector3, ShadingModel, TextureColorComponent, TextureEffect, TextureFilter, TexturePixelFormat, VertexType
};
use psp::vram_alloc::get_vram_allocator;
use psp::{BUF_WIDTH, SCREEN_WIDTH, SCREEN_HEIGHT};

psp::module!("sample_cube", 1, 1);

// Both width and height, this is a square image.
const IMAGE_SIZE: usize = 128;

// The image data *must* be aligned to a 16 byte boundary.
static FERRIS: Align16<[u8; IMAGE_SIZE * IMAGE_SIZE * 4]> = Align16(*include_bytes!("../ferris.bin"));

static mut LIST: Align16<[u32; 0x40000]> = Align16([0; 0x40000]);

#[repr(C, align(4))]
struct Vertex {
	u: f32,
	v: f32,
	x: f32,
	y: f32,
	z: f32,
}

static VERTICES: Align16<[Vertex; 12 * 3]> = Align16([
	Vertex { u: 0.0, v: 0.0, x: -1.0, y: -1.0, z:  1.0}, // 0
	Vertex { u: 1.0, v: 0.0, x: -1.0, y:  1.0, z:  1.0}, // 4
	Vertex { u: 1.0, v: 1.0, x:  1.0, y:  1.0, z:  1.0}, // 5

	Vertex { u: 0.0, v: 0.0, x: -1.0, y: -1.0, z:  1.0}, // 0
	Vertex { u: 1.0, v: 1.0, x:  1.0, y:  1.0, z:  1.0}, // 5
	Vertex { u: 0.0, v: 1.0, x:  1.0, y: -1.0, z:  1.0}, // 1

	Vertex { u: 0.0, v: 0.0, x: -1.0, y: -1.0, z: -1.0}, // 3
	Vertex { u: 1.0, v: 0.0, x:  1.0, y: -1.0, z: -1.0}, // 2
	Vertex { u: 1.0, v: 1.0, x:  1.0, y:  1.0, z: -1.0}, // 6

	Vertex { u: 0.0, v: 0.0, x: -1.0, y: -1.0, z: -1.0}, // 3
	Vertex { u: 1.0, v: 1.0, x:  1.0, y:  1.0, z: -1.0}, // 6
	Vertex { u: 0.0, v: 1.0, x: -1.0, y:  1.0, z: -1.0}, // 7

	Vertex { u: 0.0, v: 0.0, x:  1.0, y: -1.0, z: -1.0}, // 0
	Vertex { u: 1.0, v: 0.0, x:  1.0, y: -1.0, z:  1.0}, // 3
	Vertex { u: 1.0, v: 1.0, x:  1.0, y:  1.0, z:  1.0}, // 7

	Vertex { u: 0.0, v: 0.0, x:  1.0, y: -1.0, z: -1.0}, // 0
	Vertex { u: 1.0, v: 1.0, x:  1.0, y:  1.0, z:  1.0}, // 7
	Vertex { u: 0.0, v: 1.0, x:  1.0, y:  1.0, z: -1.0}, // 4

	Vertex { u: 0.0, v: 0.0, x: -1.0, y: -1.0, z: -1.0}, // 0
	Vertex { u: 1.0, v: 0.0, x: -1.0, y:  1.0, z: -1.0}, // 3
	Vertex { u: 1.0, v: 1.0, x: -1.0, y:  1.0, z:  1.0}, // 7

	Vertex { u: 0.0, v: 0.0, x: -1.0, y: -1.0, z: -1.0}, // 0
	Vertex { u: 1.0, v: 1.0, x: -1.0, y:  1.0, z:  1.0}, // 7
	Vertex { u: 0.0, v: 1.0, x: -1.0, y: -1.0, z:  1.0}, // 4

	Vertex { u: 0.0, v: 0.0, x: -1.0, y:  1.0, z: -1.0}, // 0
	Vertex { u: 1.0, v: 0.0, x:  1.0, y:  1.0, z: -1.0}, // 1
	Vertex { u: 1.0, v: 1.0, x:  1.0, y:  1.0, z:  1.0}, // 2

	Vertex { u: 0.0, v: 0.0, x: -1.0, y:  1.0, z: -1.0}, // 0
	Vertex { u: 1.0, v: 1.0, x:  1.0, y:  1.0, z:  1.0}, // 2
	Vertex { u: 0.0, v: 1.0, x: -1.0, y:  1.0, z:  1.0}, // 3

	Vertex { u: 0.0, v: 0.0, x: -1.0, y: -1.0, z: -1.0}, // 4
	Vertex { u: 1.0, v: 0.0, x: -1.0, y: -1.0, z:  1.0}, // 7
	Vertex { u: 1.0, v: 1.0, x:  1.0, y: -1.0, z:  1.0}, // 6

	Vertex { u: 0.0, v: 0.0, x: -1.0, y: -1.0, z: -1.0}, // 4
	Vertex { u: 1.0, v: 1.0, x:  1.0, y: -1.0, z:  1.0}, // 6
	Vertex { u: 0.0, v: 1.0, x:  1.0, y: -1.0, z: -1.0}, // 5
]);

fn psp_main() {
	unsafe { psp_main_inner() }
}

unsafe fn psp_main_inner() {
	psp::enable_home_button();

	let allocator = get_vram_allocator().unwrap();
	let fbp0 = allocator.alloc_texture_pixels(BUF_WIDTH, SCREEN_HEIGHT, TexturePixelFormat::Psm8888);
	let fbp1 = allocator.alloc_texture_pixels(BUF_WIDTH, SCREEN_HEIGHT, TexturePixelFormat::Psm8888);
	let zbp = allocator.alloc_texture_pixels(BUF_WIDTH, SCREEN_HEIGHT, TexturePixelFormat::Psm4444);
	// Attempting to free the three VRAM chunks at this point would give a
	// compile-time error since fbp0, fbp1 and zbp are used later on
	//allocator.free_all();

	let font = include_bytes!("../notosansmono-ascii.ttf");
	let mut font = crate::util::font::Font::new(font);

	font.register_scale(20, &allocator);
	font.register_scale(50, &allocator);

	sys::sceGumLoadIdentity();

	sys::sceGuInit();

	sys::sceGuStart(GuContextType::Direct, &raw mut LIST.0 as *mut [u32; 0x40000] as *mut _);
	sys::sceGuDrawBuffer(DisplayPixelFormat::Psm8888, fbp0.as_mut_ptr_from_zero() as _, BUF_WIDTH as i32);
	sys::sceGuDispBuffer(SCREEN_WIDTH as i32, SCREEN_HEIGHT as i32, fbp1.as_mut_ptr_from_zero() as _, BUF_WIDTH as i32);
	sys::sceGuDepthBuffer(zbp.as_mut_ptr_from_zero() as _, BUF_WIDTH as i32);
	sys::sceGuOffset(2048 - (SCREEN_WIDTH / 2), 2048 - (SCREEN_HEIGHT / 2));
	sys::sceGuViewport(2048, 2048, SCREEN_WIDTH as i32, SCREEN_HEIGHT as i32);
	sys::sceGuDepthRange(65535, 0);
	sys::sceGuScissor(0, 0, SCREEN_WIDTH as i32, SCREEN_HEIGHT as i32);
	sys::sceGuEnable(GuState::ScissorTest);
	sys::sceGuDepthFunc(DepthFunc::GreaterOrEqual);
	sys::sceGuEnable(GuState::DepthTest);
	sys::sceGuFrontFace(FrontFaceDirection::Clockwise);
	sys::sceGuShadeModel(ShadingModel::Smooth);
	sys::sceGuEnable(GuState::CullFace);
	sys::sceGuEnable(GuState::Texture2D);
	sys::sceGuEnable(GuState::ClipPlanes);
	sys::sceGuBlendFunc(
		BlendOp::Add,
		BlendFactor::SrcAlpha,
		BlendFactor::OneMinusSrcAlpha,
		0,
		0,
	);
	sys::sceGuTexFunc(TextureEffect::Modulate, TextureColorComponent::Rgba);
	sys::sceGuTexFilter(TextureFilter::Nearest, TextureFilter::Nearest);
	sys::sceGuTexScale(1.0, 1.0);
	sys::sceGuTexOffset(0.0, 0.0);
	sys::sceGuFinish();
	sys::sceGuSync(GuSyncMode::Finish, GuSyncBehavior::Wait);

	sys::sceDisplayWaitVblankStart();

	sys::sceGuDisplay(true);

	// run sample

	let mut val = 0.0;

	loop {
		sys::sceGuStart(GuContextType::Direct, &raw mut LIST.0 as *mut [u32; 0x40000] as *mut _);

		// clear screen
		sys::sceGuClearColor(0xff000000);
		sys::sceGuClearDepth(0);
		sys::sceGuClear(ClearBuffer::COLOR_BUFFER_BIT | ClearBuffer::DEPTH_BUFFER_BIT);

		// setup matrices for cube

		sys::sceGumMatrixMode(sys::MatrixMode::Projection);
		sys::sceGumLoadIdentity();
		sys::sceGumPerspective(75.0, 16.0 / 9.0, 0.5, 1000.0);

		sys::sceGumMatrixMode(sys::MatrixMode::View);
		sys::sceGumLoadIdentity();

		sys::sceGumMatrixMode(sys::MatrixMode::Model);
		sys::sceGumLoadIdentity();

		{
			let pos = ScePspFVector3 { x: 0.0, y: 0.0, z: -2.5 };
			let rot = ScePspFVector3 {
				x: val * 0.79 * (PI / 180.0),
				y: val * 0.98 * (PI / 180.0),
				z: val * 1.32 * (PI / 180.0),
			};

			sys::sceGumTranslate(&pos);
			sys::sceGumRotateXYZ(&rot);
		}

		sys::sceGuDisable(GuState::DepthTest);
		sys::sceGuEnable(GuState::Blend);

		sys::sceGuDisable(GuState::Texture2D);
		crate::util::background::draw(val);

		crate::util::rectangle::colored(&[10.0, 10.0, 20.0, 20.0], &[255, 0, 0, 128]);

		// setup texture

		sys::sceGuTexMode(TexturePixelFormat::Psm8888, 0, 0, 0);
		sys::sceGuTexImage(MipmapLevel::None, 128, 128, 128, &FERRIS as *const _ as *const _);

		// draw cube

		sys::sceGuColor(0x80FFFFFF);
		sys::sceGuEnable(GuState::Texture2D);
		sys::sceGumDrawArray(
			GuPrimitive::Triangles,
			VertexType::TEXTURE_32BITF | VertexType::VERTEX_32BITF | VertexType::TRANSFORM_3D,
			12 * 3,
			core::ptr::null(),
			&VERTICES as *const Align16<_> as _,
		);

		sys::sceGuEnable(GuState::Texture2D);
		crate::util::rectangle::colored_and_textured(&[0.0, 0.0, 128.0, 128.0], &[255, 255, 255, 255], &[0, 0, 128, 128]);

		font.draw_string("Wurstenstein 3D", 50, 240, 120, crate::util::font::HorizAlign::Center, &[255, 255, 255, 255]);

		sys::sceGuEnable(GuState::DepthTest);
		sys::sceGuDisable(GuState::Blend);

		sys::sceGuFinish();
		sys::sceGuSync(GuSyncMode::Finish, GuSyncBehavior::Wait);

		sys::sceDisplayWaitVblankStart();
		sys::sceGuSwapBuffers();

		val += 0.01666;
	}

	// sys::sceGuTerm();
	// psp::sys::sceKernelExitGame();
}
