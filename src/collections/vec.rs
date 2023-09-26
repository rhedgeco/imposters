use std::{any::TypeId, mem, ptr, slice};

use crate::{Imposter, ImposterDrop, RawMemory};

/// A type erased vector
#[derive(Debug)]
pub struct ImposterVec {
    typeid: TypeId,
    memory: RawMemory,
    len: usize,
    drop: Option<ImposterDrop>,
}

impl Drop for ImposterVec {
    #[inline]
    fn drop(&mut self) {
        self.clear()
    }
}

impl ImposterVec {
    /// Creates a new `ImposterVec` that can hold items of type `T`
    #[inline]
    pub fn new<T: 'static>() -> Self {
        Self {
            typeid: TypeId::of::<T>(),
            memory: RawMemory::new::<T>(),
            len: 0,
            drop: match mem::needs_drop::<T>() {
                false => None,
                true => Some(Imposter::drop_impl::<T>),
            },
        }
    }

    /// Creates a new `ImposterVec` with the initial value `imposter`
    #[inline]
    pub fn from_imposter(imposter: Imposter) -> Self {
        let mut memory = RawMemory::with_element_layout(imposter.layout());
        memory.resize(1);
        unsafe { memory.copy_to_index_unchecked(imposter.data().as_ptr(), 0) };

        Self {
            typeid: imposter.type_id(),
            memory,
            len: 1,
            drop: imposter.drop_fn(),
        }
    }

    /// Returns the [`TypeId`] of the items contained in this vec
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.typeid
    }

    /// Appends an [`Imposter`] to the end of the vector, returning `Ok(())`.
    ///
    /// If the imposter is not valid for this vec, it will be returned as `Err(Imposter)`
    #[inline]
    pub fn push_imposter(&mut self, imposter: Imposter) -> Result<(), Imposter> {
        if imposter.type_id() != self.typeid {
            return Err(imposter);
        }

        unsafe { self.push_imposter_unchecked(imposter) };
        Ok(())
    }

    /// Appends an [`Imposter`] to the end of the vector, returning `Ok(())`.
    ///
    /// # Safety
    /// the `imposter` type must match the type of this vec
    #[inline]
    pub unsafe fn push_imposter_unchecked(&mut self, imposter: Imposter) {
        self.push_raw_unchecked(imposter.data().as_ptr());
        imposter.dispose_and_forget();
    }

    /// Appends `item` to the end of the vector, returning `Ok(())`.
    ///
    /// If the item is not valid for this vec, it will be given back as `Some(T)`
    #[inline]
    pub fn push_item<T: 'static>(&mut self, item: T) -> Result<(), T> {
        if self.is_type::<T>() {
            return Err(item);
        }

        unsafe { self.push_item_unchecked(item) };
        Ok(())
    }

    /// Appends `item` to the end of the vector, returning `Ok(())`.
    ///
    /// # Safety
    /// type `T` must match this vecs type
    #[inline]
    pub unsafe fn push_item_unchecked<T: 'static>(&mut self, item: T) {
        let item_ptr = ptr::NonNull::from(&item).cast::<u8>().as_ptr();
        self.push_raw_unchecked(item_ptr);
        mem::forget(item);
    }

    /// Appends `item_ptr` to the end of the vector
    ///
    /// # Safety
    /// `item_ptr` must point to a type that matches this vec
    #[inline]
    pub unsafe fn push_raw_unchecked(&mut self, item_ptr: *mut u8) {
        let original_length = self.len;
        if original_length == self.memory.capacity() {
            let new_length = (self.memory.capacity() * 2).max(1);
            self.memory.resize(new_length);
        }

        self.memory.copy_to_index_unchecked(item_ptr, self.len);
        self.len += 1;
    }

    /// Returns a reference to the item of type `T` stored at `index` as `Some(&T)`
    ///
    /// If `T` does not match this vecs type, ot the index is out of bounds, returns `None`
    #[inline]
    pub fn get<T: 'static>(&self, index: usize) -> Option<&T> {
        if index >= self.len || self.is_type::<T>() {
            return None;
        }

        Some(unsafe { self.get_unchecked(index) })
    }

    /// Returns a reference to the item of type `T` stored at `index`
    ///
    /// # Safety
    /// - `T` must match this vecs type
    /// - `index` must be valid
    #[inline]
    pub unsafe fn get_unchecked<T: 'static>(&self, index: usize) -> &T {
        &*(self.memory.index_ptr_unchecked(index) as *mut T)
    }

    /// Returns a mutable reference to the item of type `T` stored at `index` as `Some(&T)`
    ///
    /// If `T` does not match this vecs type, ot the index is out of bounds, returns `None`
    #[inline]
    pub fn get_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len || self.is_type::<T>() {
            return None;
        }

        Some(unsafe { self.get_mut_unchecked(index) })
    }

    /// Returns a mutable reference to the item of type `T` stored at `index`
    ///
    /// # Safety
    /// - `T` must match this vecs type
    /// - `index` must be valid
    #[inline]
    pub unsafe fn get_mut_unchecked<T: 'static>(&mut self, index: usize) -> &mut T {
        &mut *(self.memory.index_ptr_unchecked(index) as *mut T)
    }

    /// Returns an untyped pointer to the item at `index` as `Some(*mut u8)`
    ///
    /// Returns `None` if the index is out of bounds
    #[inline]
    pub fn get_ptr(&self, index: usize) -> Option<*mut u8> {
        if index >= self.len {
            return None;
        }

        Some(unsafe { self.get_ptr_unchecked(index) })
    }

    /// Returns an untyped pointer to the item at `index`
    ///
    /// # Safety
    /// `index` must be in bounds for this vec
    #[inline]
    pub unsafe fn get_ptr_unchecked(&self, index: usize) -> *mut u8 {
        self.memory.index_ptr_unchecked(index)
    }

    /// Removes and returns the [`Imposter`] at `index`, swapping it with the last item in the vec
    ///
    /// Returns `None` if `index` is out of bounds
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> Option<Imposter> {
        if index >= self.len {
            return None;
        }

        Some(unsafe { self.swap_remove_unchecked(index) })
    }

    /// Removes and returns the [`Imposter`] at `index`, swapping it with the last item in the vec
    ///
    /// # Safety
    /// `index` must be valid for this vec
    #[inline]
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize) -> Imposter {
        let imposter = {
            let last_index = self.len - 1;
            self.memory.swap_unchecked(index, last_index);
            Imposter::from_raw(
                self.memory.copy_to_alloc_unchecked(last_index),
                self.typeid,
                self.memory.element_layout(),
                self.drop,
            )
        };

        self.len -= 1;
        imposter
    }

    /// Drops the value at `index` by swapping it with the last value, returning `true`
    ///
    /// Returns `false` if the index is out of bounds, and does not drop anything
    #[inline]
    pub fn swap_drop(&mut self, index: usize) -> bool {
        if index >= self.len {
            return false;
        }
        unsafe {
            let last_index = self.len - 1;
            self.memory.swap_unchecked(index, last_index);
            let removed = self.memory.index_ptr_unchecked(last_index);
            if let Some(drop) = self.drop {
                (drop)(removed);
            }
        }
        self.len -= 1;
        true
    }

    /// Clears all the elements in the vector, calling their drop function if necessary
    #[inline]
    pub fn clear(&mut self) {
        match self.len {
            0 => (),
            len => unsafe {
                self.len = 0;
                if let Some(drop) = self.drop {
                    let mut ptr = self.memory.ptr();
                    let data_size = self.memory.element_layout().size();
                    (drop)(ptr);
                    for _ in 0..(len - 1) {
                        ptr = ptr.add(data_size);
                        (drop)(ptr);
                    }
                }
            },
        }
    }

    /// Returns the number of items in the vec
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vec is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if `T` matches the internal item type
    #[inline]
    pub fn is_type<T: 'static>(&self) -> bool {
        TypeId::of::<T>() != self.typeid
    }

    /// Converts this `ImposterVec` into a typed [`Vec`]
    ///
    /// Returns `Err(Self)` if `T` does not match this vec's type
    #[inline]
    pub fn into_vec<T: 'static>(self) -> Result<Vec<T>, Self> {
        if self.is_type::<T>() {
            return Err(self);
        }

        Ok(unsafe { self.into_vec_unchecked() })
    }

    /// Converts this `ImposterVec` into a typed [`Vec`]
    ///
    /// # Safety
    /// type `T` must match this vecs type
    #[inline]
    pub unsafe fn into_vec_unchecked<T: 'static>(self) -> Vec<T> {
        Vec::from_raw_parts(
            self.memory.ptr() as *mut T,
            self.len,
            self.memory.capacity(),
        )
    }

    /// Returns this vec as a reference to slice of type `T`
    ///
    /// Returns `None` if `T` does not match this vecs type
    #[inline]
    pub fn as_slice<T: 'static>(&self) -> Option<&[T]> {
        if self.is_type::<T>() {
            return None;
        }

        Some(unsafe { self.as_slice_unchecked() })
    }

    /// Returns this vec as a reference to slice of type `T`
    ///
    /// # Safety
    /// type `T` must match this vecs type
    #[inline]
    pub unsafe fn as_slice_unchecked<T: 'static>(&self) -> &[T] {
        slice::from_raw_parts::<'_, T>(self.memory.ptr() as *const T, self.len)
    }

    /// Returns this vec as a mutable reference to slice of type `T`
    ///
    /// Returns `None` if `T` does not match this vecs type
    #[inline]
    pub fn as_slice_mut<T: 'static>(&mut self) -> Option<&mut [T]> {
        if self.is_type::<T>() {
            return None;
        }

        Some(unsafe { self.as_slice_mut_unchecked() })
    }

    /// Returns this vec as a mutable reference to slice of type `T`
    ///
    /// # Safety
    /// type `T` must match this vecs type
    #[inline]
    pub unsafe fn as_slice_mut_unchecked<T: 'static>(&mut self) -> &mut [T] {
        slice::from_raw_parts_mut::<'_, T>(self.memory.ptr() as *mut T, self.len)
    }

    /// Returns this vec as a pointer to a slice of type `T`
    ///
    /// Returns `None` if `T` does not match this vecs type
    #[inline]
    pub fn as_slice_ptr<T: 'static>(&self) -> Option<ptr::NonNull<[T]>> {
        if self.is_type::<T>() {
            return None;
        }

        Some(unsafe { self.as_slice_ptr_unchecked() })
    }

    /// Returns this vec as a pointer to a slice of type `T`
    ///
    /// # Safety
    /// type `T` must match this vecs type
    #[inline]
    pub unsafe fn as_slice_ptr_unchecked<T: 'static>(&self) -> ptr::NonNull<[T]> {
        let slice = slice::from_raw_parts_mut::<'_, T>(self.memory.ptr() as *mut T, self.len);
        ptr::NonNull::new_unchecked(slice as *mut [T])
    }

    /// Returns an iterator over all the elements of this vec
    ///
    /// This iterator will use untyped pointer references to each item.
    /// If you want a typed iterater, first use `as_slice<T>` or `as_slice_mut<T>` and iterate over the slice instead.
    #[inline]
    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }
}

/// An iterator over the raw pointers in a [`ImposterVec`]
pub struct Iter<'a> {
    vec: &'a ImposterVec,
    index: usize,
}

impl<'a> Iter<'a> {
    fn new(vec: &'a ImposterVec) -> Self {
        Self { vec, index: 0 }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = *mut u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.vec.len() {
            return None;
        }

        Some(unsafe { self.vec.memory.index_ptr_unchecked(self.index) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Test1(u32);

    #[test]
    fn new_imposter_vec() {
        let vec = ImposterVec::new::<Test1>();
        assert!(vec.is_empty());
        let vec = ImposterVec::from_imposter(Imposter::new(Test1(42)));
        assert!(vec.len() == 1);
    }

    #[test]
    fn push_imposter_vec() {
        let mut vec = ImposterVec::new::<Test1>();
        vec.push_item(Test1(42)).unwrap();
        vec.push_imposter(Imposter::new(Test1(43))).ok().unwrap();
        assert!(vec.len() == 2);
    }

    #[test]
    fn swap_drop_vec() {
        let mut vec = ImposterVec::from_imposter(Imposter::new(Test1(42)));
        vec.push_item(Test1(43)).unwrap();
        vec.push_item(Test1(44)).unwrap();
        vec.swap_drop(1);
        assert!(vec.len() == 2);
        assert!(!vec.swap_drop(2));
        assert!(vec.len() == 2);
        vec.swap_drop(0);
        assert!(vec.len() == 1);
        vec.swap_drop(0);
        assert!(vec.len() == 0);
    }

    #[test]
    fn swap_remove_vec() {
        let mut vec = ImposterVec::from_imposter(Imposter::new(Test1(42)));
        vec.push_item(Test1(43)).unwrap();
        vec.push_item(Test1(44)).unwrap();
        assert!(vec.swap_remove(3).is_none());
        let test = vec.swap_remove(1).unwrap().downcast::<Test1>().unwrap();
        assert!(test.0 == 43);
        let test = vec.swap_remove(0).unwrap().downcast::<Test1>().unwrap();
        assert!(test.0 == 42);
        let test = vec.swap_remove(0).unwrap().downcast::<Test1>().unwrap();
        assert!(test.0 == 44);
    }
}
