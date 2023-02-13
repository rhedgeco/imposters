use std::{
    alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout},
    ptr::{self, NonNull},
};

pub struct RawMemory {
    ptr: ptr::NonNull<u8>,
    capacity: usize,
    element_layout: Layout,
}

impl Drop for RawMemory {
    fn drop(&mut self) {
        if self.capacity == 0 {
            return;
        }

        let array_size = self.element_layout.size() * self.capacity;
        let array_align = self.element_layout.align();
        unsafe {
            dealloc(
                self.ptr.as_ptr(),
                Layout::from_size_align_unchecked(array_size, array_align),
            );
        }
    }
}

impl RawMemory {
    pub fn new<T: 'static>() -> Self {
        Self {
            ptr: ptr::NonNull::<T>::dangling().cast(),
            capacity: 0,
            element_layout: Layout::new::<T>(),
        }
    }

    pub fn with_element_layout(layout: Layout) -> Self {
        Self {
            ptr: Self::create_dangling_ptr(&layout),
            capacity: 0,
            element_layout: layout,
        }
    }

    #[inline]
    pub fn index_ptr(&mut self, index: usize) -> *mut u8 {
        self.panic_out_of_bounds(index);
        unsafe { self.index_ptr_unchecked(index) }
    }

    #[inline]
    pub unsafe fn index_ptr_unchecked(&mut self, index: usize) -> *mut u8 {
        self.ptr().add(index * self.element_layout.size())
    }

    #[inline]
    pub fn copy_to_index(&mut self, src: *const u8, index: usize) {
        self.panic_out_of_bounds(index);
        unsafe { self.copy_to_index_unchecked(src, index) };
    }

    #[inline]
    pub unsafe fn copy_to_index_unchecked(&mut self, src: *const u8, index: usize) {
        let index_ptr = self.index_ptr_unchecked(index);
        ptr::copy_nonoverlapping(src, index_ptr, self.element_layout.size())
    }

    #[inline]
    pub fn copy_to_alloc(&self, index: usize) -> ptr::NonNull<u8> {
        self.panic_out_of_bounds(index);
        unsafe { self.copy_to_alloc_unchecked(index) }
    }

    #[inline]
    pub unsafe fn copy_to_alloc_unchecked(&self, index: usize) -> ptr::NonNull<u8> {
        let index_ptr = self.ptr.as_ptr().add(index * self.element_layout.size());
        let new_ptr = alloc(self.element_layout);
        if new_ptr.is_null() {
            handle_alloc_error(self.element_layout);
        }
        ptr::copy_nonoverlapping(index_ptr, new_ptr, self.element_layout.size());
        NonNull::new_unchecked(new_ptr)
    }

    #[inline]
    pub fn swap(&mut self, x: usize, y: usize) {
        self.panic_out_of_bounds(x);
        self.panic_out_of_bounds(y);
        unsafe { self.swap_unchecked(x, y) };
    }

    #[inline]
    pub unsafe fn swap_unchecked(&mut self, x: usize, y: usize) {
        if x == y {
            return;
        }

        let element_size = self.element_layout.size();
        let array_ptr = self.ptr();
        ptr::swap_nonoverlapping(
            array_ptr.add(x * element_size),
            array_ptr.add(y * element_size),
            element_size,
        );
    }

    pub fn resize(&mut self, new_capacity: usize) {
        if self.capacity == new_capacity || self.element_layout.size() == 0 {
            return;
        }

        let old_memory_layout = self.memory_layout();
        self.ptr = if new_capacity == 0 {
            unsafe { dealloc(self.ptr(), old_memory_layout) };
            Self::create_dangling_ptr(&self.element_layout)
        } else {
            let new_memory_size = self
                .element_layout
                .size()
                .checked_mul(new_capacity)
                .expect("memory overflow");
            unsafe {
                let new_memory_layout =
                    Layout::from_size_align_unchecked(new_memory_size, self.element_layout.align());
                if self.capacity == 0 {
                    ptr::NonNull::new(alloc(new_memory_layout))
                } else {
                    ptr::NonNull::new(realloc(self.ptr(), old_memory_layout, new_memory_size))
                }
                .unwrap_or_else(|| handle_alloc_error(new_memory_layout))
            }
        };

        self.capacity = new_capacity;
    }

    #[inline]
    pub fn ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn element_layout(&self) -> Layout {
        self.element_layout
    }

    #[inline]
    pub fn memory_layout(&self) -> Layout {
        unsafe {
            Layout::from_size_align_unchecked(
                self.element_layout
                    .size()
                    .checked_mul(self.capacity)
                    .expect("memory overflow"),
                self.element_layout.align(),
            )
        }
    }

    #[inline]
    fn panic_out_of_bounds(&self, index: usize) {
        if index >= self.capacity {
            panic!("index out of bounds");
        }
    }

    /// Creates a dangling pointer with a specified layout
    ///
    /// # Safety
    /// This pointer is dangling and invalid.
    /// This is not inherently unsafe, unless the pointer is dereferenced.
    /// This pointer should only be used to `alloc` new memory with the same alignment.
    #[inline]
    fn create_dangling_ptr(layout: &Layout) -> ptr::NonNull<u8> {
        #[cfg(miri)]
        {
            // Use special miri dangling pointer
            // this allows miri to track dangling pointers better
            layout.dangling()
        }
        #[cfg(not(miri))]
        unsafe {
            ptr::NonNull::new_unchecked(layout.align() as *mut u8)
        }
    }
}
