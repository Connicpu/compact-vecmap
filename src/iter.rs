use VecMap;

use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr;

use index_pool::iter::IndexIter;

pub(crate) fn keys<'a, T: 'a>(map: &'a VecMap<T>) -> Keys<'a, T> {
    Keys {
        inner: map.indices.all_indices(),
        _marker: PhantomData,
    }
}

pub(crate) fn values<'a, T: 'a>(map: &'a VecMap<T>) -> Values<'a, T> {
    Values {
        inner: map.indices.all_indices(),
        map: map,
    }
}

pub(crate) fn values_mut<'a, T: 'a>(map: &'a mut VecMap<T>) -> ValuesMut<'a, T> {
    ValuesMut {
        inner: map.indices.all_indices(),
        map: map,
        _marker: PhantomData,
    }
}

pub(crate) fn iter<'a, T: 'a>(map: &'a VecMap<T>) -> Iter<'a, T> {
    Iter {
        inner: map.indices.all_indices(),
        map: map,
    }
}

pub(crate) fn iter_mut<'a, T: 'a>(map: &'a mut VecMap<T>) -> IterMut<'a, T> {
    IterMut {
        inner: map.indices.all_indices(),
        map: map,
        _marker: PhantomData,
    }
}

pub(crate) fn drain<'a, T: 'a>(map: &'a mut VecMap<T>) -> Drain<'a, T> {
    let ptr = map as *mut _;
    Drain {
        inner: ManuallyDrop::new(map.indices.all_indices()),
        map: ptr,
        _marker: PhantomData,
    }
}

#[derive(Clone)]
pub struct Keys<'a, T: 'a> {
    inner: IndexIter<'a>,
    _marker: PhantomData<&'a VecMap<T>>,
}

impl<'a, T: 'a> Iterator for Keys<'a, T> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[derive(Clone)]
pub struct Values<'a, T: 'a> {
    inner: IndexIter<'a>,
    map: &'a VecMap<T>,
}

impl<'a, T: 'a> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|i| unsafe { &*self.map.eptr(i) })
    }
}

pub struct ValuesMut<'a, T: 'a> {
    inner: IndexIter<'a>,
    map: &'a VecMap<T>,
    _marker: PhantomData<&'a mut VecMap<T>>,
}

impl<'a, T: 'a> Iterator for ValuesMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|i| unsafe { &mut *self.map.eptr(i) })
    }
}

#[derive(Clone)]
pub struct Iter<'a, T: 'a> {
    inner: IndexIter<'a>,
    map: &'a VecMap<T>,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|i| (i, unsafe { &*self.map.eptr(i) }))
    }
}

pub struct IterMut<'a, T: 'a> {
    inner: IndexIter<'a>,
    map: &'a VecMap<T>,
    _marker: PhantomData<&'a mut VecMap<T>>,
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|i| (i, unsafe { &mut *self.map.eptr(i) }))
    }
}

pub struct Drain<'a, T: 'a> {
    inner: ManuallyDrop<IndexIter<'a>>,
    map: *mut VecMap<T>,
    _marker: PhantomData<&'a mut VecMap<T>>,
}

impl<'a, T: 'a> Iterator for Drain<'a, T> {
    type Item = (usize, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|i| (i, unsafe { ptr::read((*self.map).eptr(i)) }))
    }
}

impl<'a, T: 'a> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        struct ClearOnDrop<'a, T: 'a>(&'a mut VecMap<T>);
        impl<'a, T: 'a> Drop for ClearOnDrop<'a, T> {
            fn drop(&mut self) {
                self.0.indices.clear();
            }
        }
        unsafe {
            let self_ = ClearOnDrop(&mut *self.map);
            for key in &mut *self.inner {
                ptr::drop_in_place(self_.0.eptr(key));
            }
            ptr::drop_in_place(&mut *self.inner);
        }
    }
}
