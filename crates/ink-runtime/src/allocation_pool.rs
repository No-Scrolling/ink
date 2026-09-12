use rquickjs::allocator::{Allocator, RustAllocator};
use std::ptr;

const STEP: usize = 16;
const MAX_SMALL: usize = 2048;
const RETAINED_LIMIT: usize = 256 * 1024;

pub(super) struct AllocationPool {
    free: [*mut u8; MAX_SMALL / STEP],
    retained: usize,
}

impl AllocationPool {
    pub(super) fn new() -> Self {
        Self {
            free: [ptr::null_mut(); MAX_SMALL / STEP],
            retained: 0,
        }
    }
}

unsafe impl Allocator for AllocationPool {
    fn alloc(&mut self, size: usize) -> *mut u8 {
        if size == 0 || size > isize::MAX as usize - STEP {
            return ptr::null_mut();
        }
        if size > MAX_SMALL {
            return RustAllocator.alloc(size);
        }
        let capacity = size.div_ceil(STEP) * STEP;
        let head = &mut self.free[capacity / STEP - 1];
        if head.is_null() {
            return RustAllocator.alloc(capacity);
        }
        let pointer = *head;
        // Cached allocations hold their successor in the unused payload.
        unsafe {
            *head = pointer.cast::<*mut u8>().read();
        }
        self.retained -= capacity;
        pointer
    }

    fn calloc(&mut self, count: usize, size: usize) -> *mut u8 {
        let Some(size) = count.checked_mul(size) else {
            return ptr::null_mut();
        };
        let pointer = self.alloc(size);
        if !pointer.is_null() {
            unsafe {
                pointer.write_bytes(0, Self::usable_size(pointer));
            }
        }
        pointer
    }

    unsafe fn dealloc(&mut self, pointer: *mut u8) {
        if pointer.is_null() {
            return;
        }
        let capacity = unsafe { Self::usable_size(pointer) };
        if capacity > 0
            && capacity <= MAX_SMALL
            && capacity % STEP == 0
            && self.retained + capacity <= RETAINED_LIMIT
        {
            let head = &mut self.free[capacity / STEP - 1];
            unsafe {
                pointer.cast::<*mut u8>().write(*head);
            }
            *head = pointer;
            self.retained += capacity;
        } else {
            unsafe {
                RustAllocator.dealloc(pointer);
            }
        }
    }

    unsafe fn realloc(&mut self, pointer: *mut u8, size: usize) -> *mut u8 {
        if pointer.is_null() {
            return self.alloc(size);
        }
        if size == 0 {
            unsafe {
                self.dealloc(pointer);
            }
            return ptr::null_mut();
        }
        if size > isize::MAX as usize - STEP {
            return ptr::null_mut();
        }
        let previous = unsafe { Self::usable_size(pointer) };
        if previous > MAX_SMALL && size > MAX_SMALL {
            return unsafe { RustAllocator.realloc(pointer, size) };
        }
        if size <= previous && previous <= MAX_SMALL {
            return pointer;
        }
        let next = self.alloc(size);
        if !next.is_null() {
            unsafe {
                ptr::copy_nonoverlapping(pointer, next, previous.min(size));
                self.dealloc(pointer);
            }
        }
        next
    }

    unsafe fn usable_size(pointer: *mut u8) -> usize {
        unsafe { RustAllocator::usable_size(pointer) }
    }
}

impl Drop for AllocationPool {
    fn drop(&mut self) {
        for head in &mut self.free {
            while !head.is_null() {
                let pointer = *head;
                unsafe {
                    *head = pointer.cast::<*mut u8>().read();
                    RustAllocator.dealloc(pointer);
                }
            }
        }
    }
}
