pub mod background;
pub mod font;
pub mod rectangle;

pub fn allocate_display_list<T: Sized>(count: usize) -> &'static mut [T] {
	unsafe {
		let ptr = psp::sys::sceGuGetMemory((count * core::mem::size_of::<T>()) as i32) as *mut T;
		core::slice::from_raw_parts_mut(ptr, count)
	}
}

