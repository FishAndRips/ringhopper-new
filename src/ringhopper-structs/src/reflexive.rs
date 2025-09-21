use crate::WriteableData;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

/// Reflexives are a resizeable array of items.
#[derive(Clone, PartialEq, Debug, Default)]
#[repr(transparent)]
pub struct Reflexive<T: WriteableData> {
    data: Vec<T>
}

impl<T: WriteableData> Reflexive<T> {
    #[inline]
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self { data: Vec::with_capacity(capacity) }
    }
}

impl<T: WriteableData> Deref for Reflexive<T> {
    type Target = Vec<T>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: WriteableData> DerefMut for Reflexive<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: WriteableData> IntoIterator for Reflexive<T> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, T: WriteableData> IntoIterator for &'a Reflexive<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a, T: WriteableData> IntoIterator for &'a mut Reflexive<T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}
