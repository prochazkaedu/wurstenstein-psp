pub struct Arena<T, const N: usize> {
	inner: [T; N],
	idx: usize
}

impl<T, const N: usize> Arena<T, N> {
	pub const fn init() -> Self {
		Self {
			inner: unsafe { core::mem::zeroed() },
			idx: 0
		}
	}

	pub fn clear(&mut self) {
		self.idx = 0;
	}

	pub fn get_memory(&mut self, size: usize) -> &mut [T] {
		let out = &mut self.inner[self.idx..self.idx+size];
		self.idx += size;
		out
	}

	pub fn push(&mut self, val: T) {
		self.inner[self.idx] = val;
		self.idx += 1;
	}

	pub fn len(&self) -> usize {
		self.idx
	}

	pub fn get_top(&self) -> Option<&T> {
		if self.idx == 0 { None? }

		Some(&self.inner[self.idx - 1])
	}

	pub fn get_all(&self) -> &[T] {
		&self.inner[..self.idx]
	}
}

