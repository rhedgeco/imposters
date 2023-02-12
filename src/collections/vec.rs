use std::{alloc::dealloc, any::TypeId, mem, ptr};

use crate::{Imposter, ImposterDrop, MemoryBuilder};

pub struct ImposterVec {
    typeid: TypeId,
    memory: MemoryBuilder,
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
            memory: MemoryBuilder::new::<T>(),
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
            memory: MemoryBuilder::from_layout(imposter.layout()),
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

        unsafe {
            self.push_raw_unchecked(imposter.data().as_ptr());
            dealloc(imposter.data().as_ptr(), imposter.layout());
            mem::forget(imposter);
        }

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
        unsafe {
            self.push_raw_unchecked(item_ptr);
        }

        mem::forget(item);
        None
    }

    unsafe fn push_raw_unchecked(&mut self, item_ptr: *mut u8) {
        let len = self.len;
        if len == self.memory.capacity() {
            let new_len = (self.memory.capacity() * 2).max(1);
            self.memory.resize(new_len);
            self.len = new_len;
        }

        let data_size = self.memory.layout().size();
        let end = self.memory.ptr().add(len * data_size);
        ptr::copy_nonoverlapping(item_ptr, end, data_size);
    }

    /// Drops the value at `index` by swapping it with the last value
    pub fn swap_drop(&mut self, index: usize) {
        if index >= self.len {
            panic!("Index out of bounds");
        }

        let data_size = self.memory.layout().size();
        let last_offset = (self.len - 1) * data_size;
        let drop_offset = index * data_size;
        unsafe {
            let last = self.memory.ptr().add(last_offset);
            let drop = self.memory.ptr().add(drop_offset);
            ptr::swap_nonoverlapping(last, drop, data_size);
            if let Some(drop) = self.drop {
                (drop)(last);
            }
        }
        self.len -= 1;
    }

    /// Clears all the elements in the vector, calling their drop function if necessary
    pub fn clear(&mut self) {
        match self.len {
            0 => return,
            len => unsafe {
                self.len = 0;
                if let Some(drop) = self.drop {
                    let mut ptr = self.memory.ptr();
                    let data_size = self.memory.layout().size();
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
}
