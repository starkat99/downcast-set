use downcast_rs::Downcast;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, TryReserveError, hash_map},
    hash::{BuildHasher, RandomState},
    iter::FusedIterator,
};

#[derive(Clone, Debug, Default)]
pub struct DowncastSet<D: ?Sized, S = RandomState> {
    map: HashMap<TypeId, Box<D>, S>,
}

impl<D: ?Sized> DowncastSet<D, RandomState> {
    pub fn new() -> Self {
        DowncastSet {
            map: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        DowncastSet {
            map: HashMap::with_capacity(capacity),
        }
    }
}

impl<D: ?Sized, S> DowncastSet<D, S> {
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    pub fn iter(&self) -> Iter<'_, D> {
        Iter {
            iter: self.map.values(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, D> {
        IterMut {
            iter: self.map.values_mut(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn drain(&mut self) -> Drain<'_, D> {
        Drain {
            drain: self.map.drain(),
        }
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut D) -> bool,
    {
        self.map.retain(|_, v| f(v.as_mut()))
    }

    pub fn clear(&mut self) {
        self.map.clear()
    }

    pub fn with_hasher(hash_builder: S) -> Self {
        DowncastSet {
            map: HashMap::with_hasher(hash_builder),
        }
    }

    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        DowncastSet {
            map: HashMap::with_capacity_and_hasher(capacity, hasher),
        }
    }

    pub fn hasher(&self) -> &S {
        self.map.hasher()
    }
}

impl<D, S> DowncastSet<D, S>
where
    D: ?Sized,
    S: BuildHasher,
{
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional)
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.map.try_reserve(additional)
    }

    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit()
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.map.shrink_to(min_capacity)
    }
}

impl<D, S> DowncastSet<D, S>
where
    D: ?Sized + Downcast,
    S: BuildHasher,
{
    pub fn contains<T: Any>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T: Downcast>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .map(|p| unsafe { &*(p.as_ref() as *const D as *const T) })
    }

    pub fn get_mut<T: Downcast>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .map(|p| unsafe { &mut *(p.as_mut() as *mut D as *mut T) })
    }

    pub fn insert<T: Into<Box<D>> + 'static>(&mut self, value: T) -> bool {
        self.map.insert(value.type_id(), value.into()).is_some()
    }

    pub fn replace<T: Into<Box<D>> + 'static>(&mut self, value: T) -> Option<T> {
        self.map
            .insert(value.type_id(), value.into())
            .map(|p| *unsafe { Box::from_raw(Box::into_raw(p) as *mut T) })
    }

    pub fn remove<T: Downcast>(&mut self) -> bool {
        self.map.remove(&TypeId::of::<T>()).is_some()
    }

    pub fn take<T: Downcast>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .map(|p| *unsafe { Box::from_raw(Box::into_raw(p) as *mut T) })
    }
}

impl<'a, T: ?Sized, S> IntoIterator for &'a DowncastSet<T, S> {
    type Item = &'a T;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: ?Sized, S> IntoIterator for &'a mut DowncastSet<T, S> {
    type Item = &'a mut T;

    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: ?Sized, S> IntoIterator for DowncastSet<T, S> {
    type Item = Box<T>;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.map.into_values(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Iter<'a, T: ?Sized + 'a> {
    iter: hash_map::Values<'a, TypeId, Box<T>>,
}

impl<'a, T: ?Sized + 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|p| p.as_ref())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.iter.count()
    }
}

impl<T: ?Sized> ExactSizeIterator for Iter<'_, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T: ?Sized> FusedIterator for Iter<'_, T> {}

#[derive(Debug, Default)]
pub struct IterMut<'a, T: ?Sized + 'a> {
    iter: hash_map::ValuesMut<'a, TypeId, Box<T>>,
}

impl<'a, T: ?Sized + 'a> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|p| p.as_mut())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.iter.count()
    }
}

impl<T: ?Sized> ExactSizeIterator for IterMut<'_, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T: ?Sized> FusedIterator for IterMut<'_, T> {}

#[derive(Debug, Default)]
pub struct IntoIter<T: ?Sized> {
    iter: hash_map::IntoValues<TypeId, Box<T>>,
}

impl<T: ?Sized> Iterator for IntoIter<T> {
    type Item = Box<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.iter.count()
    }
}

impl<T: ?Sized> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T: ?Sized> FusedIterator for IntoIter<T> {}

#[derive(Debug)]
pub struct Drain<'a, T: ?Sized + 'a> {
    drain: hash_map::Drain<'a, TypeId, Box<T>>,
}

impl<T: ?Sized> Iterator for Drain<'_, T> {
    type Item = Box<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.drain.next().map(|kvp| kvp.1)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.drain.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.drain.count()
    }
}

impl<T: ?Sized> ExactSizeIterator for Drain<'_, T> {
    fn len(&self) -> usize {
        self.drain.len()
    }
}

impl<T: ?Sized> FusedIterator for Drain<'_, T> {}
