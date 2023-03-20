use std::{
    alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout},
    ptr::{self, NonNull},
};

/// A bare bones memory management abstraction for the imposters library
pub struct RawMemory {
    ptr: ptr::NonNull<u8>,
    capacity: usize,
    element_layout: Layout,
}

impl Drop for RawMemory {
    #[inline]
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
    /// Returns a new RawMemory struct that should hold items of type `T`
    #[inline]
    pub fn new<T: 'static>() -> Self {
        Self {
            ptr: ptr::NonNull::<T>::dangling().cast(),
            capacity: 0,
            element_layout: Layout::new::<T>(),
        }
    }

    /// Returns a new RawMemory struct with a given item `layout`
    #[inline]
    pub fn with_element_layout(layout: Layout) -> Self {
        Self {
            ptr: Self::create_dangling_ptr(&layout),
            capacity: 0,
            element_layout: layout,
        }
    }

    /// Returns a pointer to the given `index`
    ///
    /// # Safety
    /// `index` must be in bounds
    #[inline]
    pub unsafe fn index_ptr_unchecked(&self, index: usize) -> *mut u8 {
        self.ptr().add(index * self.element_layout.size())
    }

    /// Copies data from `src` into the given `index`
    ///
    /// # Safety
    /// `src` data type must match the type for this memory
    /// `index` must be in bounds
    #[inline]
    pub unsafe fn copy_to_index_unchecked(&mut self, src: *const u8, index: usize) {
        let index_ptr = self.index_ptr_unchecked(index);
        ptr::copy_nonoverlapping(src, index_ptr, self.element_layout.size())
    }

    /// Allocates new memory and copies the item at `index` to that location
    ///
    /// # Safety
    /// `index` must be in bounds
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

    /// Swaps the items at `x` and `y`
    ///
    /// # Safety
    /// `x` and `y` must both be in bounds
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

    /// Resizes this block of memory to match `new_capacity`
    ///
    /// If shrinking, this will technically forget the items at the end of the memory.
    /// Those items will not be dropped. While this may be unfavorable it is not technically undefined
    /// as [`std::mem::forget`] is also marked as safe.
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

    /// Returns a pointer to the beginning of this memory block
    #[inline]
    pub fn ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Returns the current capacity of this memory block
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the associated element layout of this memory block
    #[inline]
    pub fn element_layout(&self) -> Layout {
        self.element_layout
    }

    /// Returns the layout for the entirety of this memory block
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

    /// Creates a dangling pointer with a specified layout.
    /// This is abstracted to allow for MIRI to make smarter pointer checks.
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
