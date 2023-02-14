use std::{any::TypeId, mem, ptr};

use crate::{Imposter, ImposterDrop, RawMemory};

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
    pub fn from_imposter(imposter: Imposter) -> Self {
        let mut vec = Self {
            typeid: imposter.type_id(),
            memory: RawMemory::with_element_layout(imposter.layout()),
            len: 0,
            drop: imposter.drop_fn(),
        };
        vec.push_imposter(imposter);
        vec
    }

    /// Appends an [`Imposter`] to the end of the vector, returning `None`.
    ///
    /// If the imposter is not valid for this vec, it will be returned as `Some(Imposter)`
    pub fn push_imposter(&mut self, imposter: Imposter) -> Option<Imposter> {
        if imposter.type_id() != self.typeid {
            return Some(imposter);
        }

        unsafe { self.push_raw_unchecked(imposter.data().as_ptr()) };
        imposter.forget();
        None
    }

    /// Appends `item` to the end of the vector, returning `None`.
    ///
    /// If the item is not valid for this vec, it will be returned as `Some(T)`
    pub fn push_item<T: 'static>(&mut self, item: T) -> Option<T> {
        if TypeId::of::<T>() != self.typeid {
            return Some(item);
        }

        let item_ptr = ptr::NonNull::from(&item).cast::<u8>().as_ptr();
        unsafe { self.push_raw_unchecked(item_ptr) };
        mem::forget(item);
        None
    }

    unsafe fn push_raw_unchecked(&mut self, item_ptr: *mut u8) {
        let original_length = self.len;
        if original_length == self.memory.capacity() {
            let new_length = (self.memory.capacity() * 2).max(1);
            self.memory.resize(new_length);
        }

        self.memory.copy_to_index_unchecked(item_ptr, self.len);
        self.len += 1;
    }

    pub fn get<T: 'static>(&self, index: usize) -> Option<&T> {
        if index >= self.len || TypeId::of::<T>() != self.typeid {
            return None;
        }

        unsafe {
            let index_ptr = self.memory.index_ptr_unchecked(index);
            Some(&*(index_ptr as *mut T))
        }
    }

    pub fn get_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len || TypeId::of::<T>() != self.typeid {
            return None;
        }

        unsafe {
            let index_ptr = self.memory.index_ptr_unchecked(index);
            Some(&mut *(index_ptr as *mut T))
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> Option<Imposter> {
        if index >= self.len {
            return None;
        }

        Some(unsafe { self.swap_remove_unchecked(index) })
    }

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

    /// Drops the value at `index` by swapping it with the last value
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
        return true;
    }

    /// Clears all the elements in the vector, calling their drop function if necessary
    pub fn clear(&mut self) {
        match self.len {
            0 => return,
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
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vec is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        vec.push_item(Test1(42));
        vec.push_imposter(Imposter::new(Test1(43)));
        assert!(vec.len() == 2);
    }

    #[test]
    fn swap_drop_vec() {
        let mut vec = ImposterVec::from_imposter(Imposter::new(Test1(42)));
        vec.push_item(Test1(43));
        vec.push_item(Test1(44));
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
        vec.push_item(Test1(43));
        vec.push_item(Test1(44));
        assert!(vec.swap_remove(3).is_none());
        let test = vec.swap_remove(1).unwrap().downcast::<Test1>().unwrap();
        assert!(test.0 == 43);
        let test = vec.swap_remove(0).unwrap().downcast::<Test1>().unwrap();
        assert!(test.0 == 42);
        let test = vec.swap_remove(0).unwrap().downcast::<Test1>().unwrap();
        assert!(test.0 == 44);
    }
}
