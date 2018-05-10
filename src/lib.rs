#![feature(alloc)]

extern crate alloc;
extern crate index_pool;

use alloc::raw_vec::RawVec;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::mem;
use std::mem::ManuallyDrop;
use std::ptr;
use std::usize;
use std::ops::{Index, IndexMut};

use index_pool::IndexPool;

pub mod iter;

pub struct VecMap<T> {
    vec: RawVec<T>,
    indices: IndexPool,
}

impl<T> VecMap<T> {
    pub fn new() -> Self {
        VecMap {
            vec: RawVec::new(),
            indices: IndexPool::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        VecMap {
            vec: RawVec::with_capacity(capacity),
            indices: IndexPool::new(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.vec.cap()
    }

    pub fn reserve_len(&mut self, len: usize) {
        if len <= self.vec.cap() {
            return;
        }

        let max = self.indices.maximum();
        self.vec.reserve(max, len - max);
    }

    pub fn reserve_len_exact(&mut self, len: usize) {
        if len <= self.vec.cap() {
            return;
        }

        let max = self.indices.maximum();
        self.vec.reserve_exact(max, len - max);
    }

    pub fn keys(&self) -> iter::Keys<T> {
        iter::keys(self)
    }

    pub fn values(&self) -> iter::Values<T> {
        iter::values(self)
    }

    pub fn values_mut(&mut self) -> iter::ValuesMut<T> {
        iter::values_mut(self)
    }

    pub fn iter(&self) -> iter::Iter<T> {
        iter::iter(self)
    }

    pub fn iter_mut(&mut self) -> iter::IterMut<T> {
        iter::iter_mut(self)
    }

    pub fn drain(&mut self) -> iter::Drain<T> {
        iter::drain(self)
    }

    pub fn append(&mut self, other: &mut Self) {
        for (i, value) in other.drain() {
            self.insert(i, value);
        }
    }

    pub fn split_off(&mut self, at: usize) -> Self {
        struct DeallocOnDrop<T>(ManuallyDrop<VecMap<T>>);
        impl<T> Drop for DeallocOnDrop<T> {
            fn drop(&mut self) {
                unsafe {
                    self.0.vec.dealloc_buffer();
                }
            }
        }

        // There *shouldn't* be any panics in here, but just to be safe
        let mut other = DeallocOnDrop(ManuallyDrop::new(Self::with_capacity(
            self.indices.maximum(),
        )));

        for i in self.indices.all_indices_after(at) {
            other.0.insert(i, unsafe { mem::uninitialized() });
        }

        for i in other.0.indices.all_indices() {
            let _res = self.indices.return_id(i);
            debug_assert!(_res.is_ok());
            unsafe {
                let value = ptr::read(self.eptr(i));
                ptr::write(other.0.eptr(i), value);
            }
        }

        let md = unsafe { ptr::read(&mut other.0) };
        mem::forget(self);
        ManuallyDrop::into_inner(md)
    }

    pub fn len(&self) -> usize {
        self.indices.in_use()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.in_use() == 0
    }

    pub fn clear(&mut self) {
        struct ClearOnDrop<'a, T: 'a>(&'a mut VecMap<T>);
        impl<'a, T: 'a> Drop for ClearOnDrop<'a, T> {
            fn drop(&mut self) {
                self.0.indices.clear();
            }
        }
        let self_ = ClearOnDrop(self);
        for i in self_.0.indices.all_indices() {
            unsafe {
                ptr::drop_in_place(self_.0.eptr(i));
            }
        }
    }

    pub fn contains_key(&self, index: usize) -> bool {
        !self.indices.is_free(index)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if self.indices.is_free(index) {
            None
        } else {
            Some(unsafe { &*self.eptr(index) })
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if self.indices.is_free(index) {
            None
        } else {
            Some(unsafe { &mut *self.eptr(index) })
        }
    }

    pub fn add(&mut self, value: T) -> usize {
        let index = self.indices.new_id();
        if index + 1 == self.indices.maximum() {
            self.indices
                .return_id(index)
                .expect("I was just given this ID");
            self.ensure_growth(index);
            assert_eq!(index, self.indices.new_id());
        }
        unsafe {
            ptr::write(self.eptr(index), value);
        }
        index
    }

    pub fn insert(&mut self, index: usize, value: T) -> Option<T> {
        if self.indices.is_free(index) {
            self.ensure_growth(index);
            self.indices
                .request_id(index)
                .expect("I already verified it's free");
            unsafe {
                ptr::write(self.eptr(index), value);
            }
            None
        } else {
            unsafe { Some(mem::replace(&mut *self.eptr(index), value)) }
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if self.indices.is_free(index) {
            None
        } else {
            self.indices
                .return_id(index)
                .expect("Just verified index is not free");
            Some(unsafe { ptr::read(self.eptr(index)) })
        }
    }

    fn ensure_growth(&mut self, idx: usize) {
        let needed = idx + 1;
        let cap = self.vec.cap();

        if cap == 0 {
            self.vec.reserve(0, needed);
            return;
        }

        if needed < cap {
            return;
        }

        if needed < cap * 2 {
            self.vec.double();
            return;
        }

        let max = self.indices.maximum();
        self.vec.reserve(max, needed - max);
    }

    unsafe fn eptr(&self, idx: usize) -> *mut T {
        self.vec.ptr().offset(idx as isize)
    }
}

impl<T> Drop for VecMap<T> {
    fn drop(&mut self) {
        self.clear();
        unsafe {
            self.vec.dealloc_buffer();
        }
    }
}

impl<T> Default for VecMap<T> {
    fn default() -> Self {
        VecMap::new()
    }
}

impl<T> Hash for VecMap<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (i, v) in self.iter() {
            i.hash(state);
            v.hash(state);
        }
    }
}

impl<T> Clone for VecMap<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut other = VecMap::with_capacity(self.indices.maximum());
        for (i, v) in self.iter() {
            other.insert(i, v.clone());
        }
        other
    }
}

impl<T> PartialEq for VecMap<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<T> Eq for VecMap<T>
where
    T: Eq,
{
}

impl<T> PartialOrd for VecMap<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<T> Ord for VecMap<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<T> Debug for VecMap<T>
where
    T: Debug,
{
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_map().entries(self.iter()).finish()
    }
}

impl<T> FromIterator<(usize, T)> for VecMap<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (usize, T)>,
    {
        let iter = iter.into_iter();
        let mut map = VecMap::with_capacity(iter.size_hint().0);
        for (i, v) in iter {
            map.insert(i, v);
        }
        map
    }
}

impl<'a, T> IntoIterator for &'a VecMap<T>
where
    T: 'a,
{
    type Item = (usize, &'a T);
    type IntoIter = iter::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut VecMap<T>
where
    T: 'a,
{
    type Item = (usize, &'a mut T);
    type IntoIter = iter::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> Extend<(usize, T)> for VecMap<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (usize, T)>,
    {
        for (i, v) in iter {
            self.insert(i, v);
        }
    }
}

impl<'a, T> Extend<(usize, &'a T)> for VecMap<T>
where
    T: Copy + 'a,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (usize, &'a T)>,
    {
        for (i, v) in iter {
            self.insert(i, *v);
        }
    }
}

impl<T> Index<usize> for VecMap<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        self.get(i).expect("key not present")
    }
}

impl<'a, T: 'a> Index<&'a usize> for VecMap<T> {
    type Output = T;
    fn index(&self, i: &'a usize) -> &T {
        self.get(*i).expect("key not present")
    }
}

impl<T> IndexMut<usize> for VecMap<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        self.get_mut(i).expect("key not present")
    }
}

impl<'a, T: 'a> IndexMut<&'a usize> for VecMap<T> {
    fn index_mut(&mut self, i: &'a usize) -> &mut T {
        self.get_mut(*i).expect("key not present")
    }
}
