use downcast_rs::Downcast;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, TryReserveError, hash_map},
    hash::{BuildHasher, RandomState},
    iter::FusedIterator,
};

pub trait Upcast<T>: Downcast
where
    T: ?Sized + Any,
{
    fn upcast(self) -> Box<T>;
}

pub trait UncheckedDowncast<T>: Any
where
    T: Downcast,
{
    unsafe fn downcast_ref_unchecked(&self) -> &T;

    unsafe fn downcast_mut_unchecked(&mut self) -> &mut T;

    unsafe fn downcast_unchecked(self: Box<Self>) -> Box<T>;
}

impl<T> Upcast<dyn Any> for T
where
    T: Any,
{
    fn upcast(self) -> Box<dyn Any> {
        Box::new(self)
    }
}

impl<T> UncheckedDowncast<T> for dyn Any
where
    T: Any,
{
    // TODO replace unwrap_unchecked when *_unchecked versions are stabilized
    unsafe fn downcast_ref_unchecked(&self) -> &T {
        self.downcast_ref().unwrap()
    }

    unsafe fn downcast_mut_unchecked(&mut self) -> &mut T {
        self.downcast_mut().unwrap()
    }

    unsafe fn downcast_unchecked(self: Box<Self>) -> Box<T> {
        self.downcast().unwrap()
    }
}

#[macro_export]
macro_rules! impl_casts {
    ($trait:ident) => {
        impl<T> $crate::Upcast<dyn $trait> for T
        where
            T: $trait,
        {
            fn upcast(self) -> Box<dyn $trait> {
                Box::new(self)
            }
        }

        impl<T> $crate::UncheckedDowncast<T> for dyn $trait
        where
            T: $trait,
        {
            // TODO replace unwrap_unchecked when *_unchecked versions are stabilized
            unsafe fn downcast_ref_unchecked(&self) -> &T {
                self.as_any().downcast_ref().unwrap()
            }

            unsafe fn downcast_mut_unchecked(&mut self) -> &mut T {
                self.as_any_mut().downcast_mut().unwrap()
            }

            unsafe fn downcast_unchecked(self: Box<Self>) -> Box<T> {
                self.into_any().downcast().unwrap()
            }
        }
    };
}

#[derive(Debug, Default)]
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
    D: ?Sized + Any,
    S: BuildHasher,
{
    pub fn contains<T>(&self) -> bool
    where
        T: Upcast<D>,
        D: UncheckedDowncast<T>,
    {
        self.map.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T>(&self) -> Option<&T>
    where
        T: Upcast<D>,
        D: UncheckedDowncast<T>,
    {
        self.map
            .get(&TypeId::of::<T>())
            .map(|p| unsafe { p.as_ref().downcast_ref_unchecked() })
    }

    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Upcast<D>,
        D: UncheckedDowncast<T>,
    {
        self.map
            .get_mut(&TypeId::of::<T>())
            .map(|p| unsafe { p.as_mut().downcast_mut_unchecked() })
    }

    pub fn insert<T>(&mut self, value: T) -> bool
    where
        T: Upcast<D>,
        D: UncheckedDowncast<T>,
    {
        !self.map.contains_key(&value.type_id())
            && self.map.insert(value.type_id(), value.upcast()).is_none()
    }

    pub fn replace<T: Upcast<D>>(&mut self, value: T) -> Option<T>
    where
        T: Upcast<D>,
        D: UncheckedDowncast<T>,
    {
        self.map
            .insert(value.type_id(), value.upcast())
            .map(|p| *unsafe { p.downcast_unchecked() })
    }

    pub fn remove<T>(&mut self) -> bool
    where
        T: Any,
        D: UncheckedDowncast<T>,
    {
        self.map.remove(&TypeId::of::<T>()).is_some()
    }

    pub fn take<T>(&mut self) -> Option<T>
    where
        T: Upcast<D>,
        D: UncheckedDowncast<T>,
    {
        self.map
            .remove(&TypeId::of::<T>())
            .map(|p| *unsafe { p.downcast_unchecked() })
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

#[cfg(test)]
mod tests {
    use super::*;

    trait TestTrait: Downcast {
        fn value(&self) -> i32;
    }
    impl_casts!(TestTrait);

    // impl<T> Downcast<dyn TestTrait> for T
    // where
    //     T: TestTrait + Any,
    // {
    //     fn into_dyn_box(self: Box<Self>) -> Box<dyn TestTrait> {
    //         self
    //     }
    // }

    #[derive(Clone, Debug, PartialEq)]
    struct A(i32);

    impl TestTrait for A {
        fn value(&self) -> i32 {
            self.0
        }
    }

    #[derive(Debug, PartialEq)]
    struct B(i32);

    impl TestTrait for B {
        fn value(&self) -> i32 {
            self.0
        }
    }

    #[test]
    fn downcast_set_basics() {
        let mut set: DowncastSet<dyn TestTrait> = DowncastSet::new();

        assert!(set.is_empty());
        assert!(set.get::<A>().is_none());
        assert!(set.get::<B>().is_none());

        assert!(!set.contains::<A>());
        assert!(set.insert(A(1)));
        assert!(!set.contains::<B>());
        assert!(set.insert(B(2)));
        assert!(!set.insert(B(3)));

        assert_eq!(set.len(), 2);
        assert!(set.contains::<A>());
        assert!(set.contains::<B>());
        assert_eq!(set.get::<A>(), Some(&A(1)));
        assert_eq!(set.get::<B>(), Some(&B(2)));

        for item in &set {
            if item.as_any().is::<A>() {
                assert_eq!(item.value(), 1);
            } else {
                assert!(item.as_any().is::<B>());
                assert_eq!(item.value(), 2);
            }
        }

        set.get_mut::<A>().unwrap().0 = 3;
        assert_eq!(set.get::<A>(), Some(&A(3)));

        assert_eq!(set.replace(B(4)), Some(B(2)));
        assert_eq!(set.get::<B>(), Some(&B(4)));
        assert_eq!(set.len(), 2);

        assert!(set.remove::<B>());
        assert!(!set.remove::<B>());
        assert!(!set.contains::<B>());
        assert_eq!(set.len(), 1);

        assert_eq!(set.take::<A>(), Some(A(3)));
        assert!(set.is_empty());
    }
}
