#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::collapsible_if)]

mod app;

mod assets;

mod audio;

mod controller;

mod util;

mod playfield;

use psp::Align16;
use psp::sys::{
	self, BlendFactor, BlendOp, CtrlMode, DepthFunc, DisplayPixelFormat, FrontFaceDirection, GuContextType, GuState, GuSyncBehavior, GuSyncMode, ShadingModel, TextureColorComponent, TextureEffect, TextureFilter, TexturePixelFormat
};
use psp::vram_alloc::get_vram_allocator;
use psp::{BUF_WIDTH, SCREEN_WIDTH, SCREEN_HEIGHT};

use crate::app::App;

psp::module!("wurstenstein_3d", 1, 1);

static mut LIST: Align16<[u32; 0x40000]> = Align16([0; 0x40000]);

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

	sys::sceGumLoadIdentity();

	sys::sceGuInit();

	sys::sceGuStart(GuContextType::Direct, &raw mut LIST.0 as *mut _);
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
	sys::sceGuFrontFace(FrontFaceDirection::CounterClockwise);
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

	sys::sceCtrlSetSamplingCycle(0);
	sys::sceCtrlSetSamplingMode(CtrlMode::Analog);

	let mut app = App::init(&allocator);

	loop {
		sys::sceGuStart(GuContextType::Direct, &raw mut LIST.0 as *mut _);

		app.main_loop();
	}
}
