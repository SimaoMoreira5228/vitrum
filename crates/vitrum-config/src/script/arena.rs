use std::cell::Cell;

pub struct Arena {
	buf: Vec<u8>,
	pos: Cell<usize>,
}

impl Arena {
	pub fn new(capacity: usize) -> Self {
		Self {
			buf: vec![0u8; capacity],
			pos: Cell::new(0),
		}
	}

	pub fn alloc<T>(&self, val: T) -> &mut T {
		let layout = std::alloc::Layout::new::<T>();
		let align = layout.align();
		let size = layout.size();

		let pos = self.pos.get();
		let aligned = (pos + align - 1) & !(align - 1);
		let end = aligned + size;

		if end > self.buf.len() {
			panic!("Arena overflow: need {} bytes, have {} remaining", size, self.buf.len() - pos);
		}

		self.pos.set(end);

		unsafe {
			let ptr = self.buf.as_ptr().add(aligned) as *mut T;
			ptr.write(val);
			&mut *ptr
		}
	}

	pub fn alloc_slice<T>(&self, items: &[T]) -> &mut [T] {
		let layout = std::alloc::Layout::array::<T>(items.len()).expect("slice too large");
		let align = layout.align();
		let size = layout.size();

		let pos = self.pos.get();
		let aligned = (pos + align - 1) & !(align - 1);
		let end = aligned + size;

		if end > self.buf.len() {
			panic!("Arena overflow: need {} bytes, have {} remaining", size, self.buf.len() - pos);
		}

		self.pos.set(end);

		unsafe {
			let ptr = self.buf.as_ptr().add(aligned) as *mut T;
			for (i, item) in items.iter().enumerate() {
				std::ptr::write(ptr.add(i), std::ptr::read(item as *const T));
			}
			std::slice::from_raw_parts_mut(ptr, items.len())
		}
	}

	pub fn alloc_str<'a>(&'a self, s: &str) -> &'a str {
		let bytes = s.as_bytes();
		let slice = self.alloc_slice(bytes);
		unsafe { std::str::from_utf8_unchecked(slice) }
	}

	pub fn used(&self) -> usize {
		self.pos.get()
	}

	pub fn reset(&mut self) {
		self.pos.set(0);
	}
}

impl Default for Arena {
	fn default() -> Self {
		Self::new(64 * 1024)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_alloc_int() {
		let arena = Arena::new(1024);
		let val = arena.alloc(42i64);
		assert_eq!(*val, 42);
	}

	#[test]
	fn test_alloc_slice() {
		let arena = Arena::new(1024);
		let slice = arena.alloc_slice(&[1, 2, 3, 4, 5]);
		assert_eq!(slice, &[1, 2, 3, 4, 5]);
	}

	#[test]
	fn test_alloc_str() {
		let arena = Arena::new(1024);
		let s = arena.alloc_str("hello world");
		assert_eq!(s, "hello world");
	}
}
