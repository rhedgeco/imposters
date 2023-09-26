use std::{
    alloc::{dealloc, Layout},
    any::TypeId,
    mem, ptr,
};

pub type ImposterDrop = unsafe fn(ptr: *mut u8);

/// # ඞ IMPOSTER ඞ
///
/// A type erased wrapper around any kind of data
#[derive(Debug)]
pub struct Imposter {
    data: ptr::NonNull<u8>,
    typeid: TypeId,
    layout: Layout,
    drop: Option<ImposterDrop>,
}

impl Drop for Imposter {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let ptr = self.data.as_ptr();
            if let Some(drop) = self.drop {
                (drop)(ptr);
            }

            if self.layout.size() != 0 {
                dealloc(ptr, self.layout);
            }
        }
    }
}

impl<T: 'static> From<Box<T>> for Imposter {
    fn from(item: Box<T>) -> Self {
        let data = unsafe {
            let data_ptr = Box::into_raw(item);
            ptr::NonNull::new_unchecked(data_ptr).cast::<u8>()
        };

        Self {
            data,
            typeid: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop: match mem::needs_drop::<T>() {
                false => None,
                true => Some(Self::drop_impl::<T>),
            },
        }
    }
}

impl Imposter {
    /// Creates a new imposter containing `item`
    #[inline]
    pub fn new<T: 'static>(item: T) -> Self {
        Box::new(item).into()
    }

    pub(crate) unsafe fn from_raw(
        data: ptr::NonNull<u8>,
        typeid: TypeId,
        layout: Layout,
        drop: Option<ImposterDrop>,
    ) -> Self {
        Self {
            data,
            typeid,
            layout,
            drop,
        }
    }

    /// Downcasts the data in this imposter to an owned type `T`.
    ///
    /// If `T` does not match the internal type, the imposter is returned in `Err`
    #[inline]
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        self.downcast_box().map(|b| *b)
    }

    /// Downcasts the data in this imposter to an owned type `T`.
    ///
    /// # Safety
    /// - `T` must match the internal type
    pub unsafe fn downcast_unchecked<T: 'static>(self) -> T {
        *self.downcast_box_unchecked()
    }

    /// Downcasts the data in this imposter to an owned type `Box<T>`.
    ///
    /// If `T` does not match the internal type, the imposter is returned in `Err`
    pub fn downcast_box<T: 'static>(self) -> Result<Box<T>, Self> {
        if self.has_type_id::<T>() {
            return Err(self);
        }

        // SAFETY:
        // raw pointer type is checked before conversion
        Ok(unsafe { self.downcast_box_unchecked() })
    }

    /// Downcasts the data in this imposter to an owned type `Box<T>`.
    ///
    /// # Safety
    /// - `T` must match the internal type
    pub unsafe fn downcast_box_unchecked<T: 'static>(self) -> Box<T> {
        let item = unsafe { Box::from_raw(self.data.as_ptr() as *mut T) };
        mem::forget(self);
        item
    }

    /// Downcasts the data in this imposter to type `&T`.
    ///
    /// If `T` does not match the internal type, `None` is returned.
    #[inline]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        if self.has_type_id::<T>() {
            return None;
        }

        // SAFETY:
        // raw pointer type is checked before conversion
        Some(unsafe { self.downcast_ref_unchecked() })
    }

    /// Downcasts the data in this imposter to type `&T`.
    ///
    /// # Safety
    /// - `T` must match the internal type
    pub unsafe fn downcast_ref_unchecked<T: 'static>(&self) -> &T {
        &*(self.data.as_ptr() as *mut T)
    }

    /// Downcasts the data in this imposter to type `&mut T`.
    ///
    /// If `T` does not match the internal type, `None` is returned.
    #[inline]
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        if self.has_type_id::<T>() {
            return None;
        }

        // SAFETY:
        // raw pointer type is checked before conversion
        Some(unsafe { self.downcast_mut_unchecked() })
    }

    /// Downcasts the data in this imposter to type `&mut T`.
    ///
    /// # Safety
    /// - `T` must match the internal type
    pub unsafe fn downcast_mut_unchecked<T: 'static>(&mut self) -> &mut T {
        &mut *(self.data.as_ptr() as *mut T)
    }

    /// Disposes of this imposter and deallocates the data it points to ***without*** calling its destructor
    pub fn dispose_and_forget(self) {
        unsafe { dealloc(self.data.as_ptr(), self.layout) };
        mem::forget(self);
    }

    /// Returns a reference to the internal data pointer
    #[inline]
    pub fn data(&self) -> ptr::NonNull<u8> {
        self.data
    }

    /// Returns a reference to the internal type id
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.typeid
    }

    /// Returns true if `T` matches the internal type
    pub fn has_type_id<T: 'static>(&self) -> bool {
        self.typeid != TypeId::of::<T>()
    }

    /// Returns a reference to the internal layout
    #[inline]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Returns a reference to the internal drop function
    #[inline]
    pub fn drop_fn(&self) -> Option<ImposterDrop> {
        self.drop
    }

    /// This is the function used if data needs to be dropped inside a imposter
    #[inline]
    pub(crate) unsafe fn drop_impl<T>(ptr: *mut u8) {
        ptr::drop_in_place(ptr as *mut T);
    }
}

#[cfg(test)]
mod tests {
    use droptest::{assert_drop, DropRegistry};

    use super::*;

    struct Test1(u32);
    struct Test2(u64);

    #[test]
    fn new_imposter() {
        let test1 = Test1(42);
        let imposter = Imposter::new(test1);
        assert!(unsafe { &*(imposter.data.as_ptr() as *mut Test1) }.0 == 42);
    }

    #[test]
    fn imposter_downcast() {
        let test1 = Test1(42);
        let imposter = Imposter::new(test1);
        assert!(imposter.downcast_ref::<Test1>().is_some());
        assert!(imposter.downcast_ref::<Test2>().is_none());
        let test1 = imposter.downcast::<Test1>().unwrap();
        assert!(test1.0 == 42);
    }

    #[test]
    fn drop_imposter() {
        let test1 = Test1(42);
        let imposter = Imposter::new(test1);

        let registry = DropRegistry::default();
        let guard = registry.new_guard_for(imposter);
        let guard_id = guard.id();

        drop(guard);
        assert_drop!(registry, guard_id);
    }
}
