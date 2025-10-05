use crate::WriteableData;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};
use combarc::CombArc;

/// Reflexives are a resizeable array of items.
#[derive(Clone, PartialEq, Debug, Default)]
#[repr(transparent)]
pub struct Reflexive<T: WriteableData + Clone> {
    data: CombArc<Vec<T>>
}

impl<T: WriteableData + Clone> Reflexive<T> {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { data: CombArc::new(Vec::new()) }
    }
    
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self { data: CombArc::new(Vec::with_capacity(capacity)) }
    }
    
    #[inline]
    #[must_use]
    pub fn with_vec(vec: Vec<T>) -> Self {
        Self { data: CombArc::new(vec) }
    }
    
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> Vec<T> {
        CombArc::make_inner(self.data)
    }
}

impl<T: WriteableData + Clone> Deref for Reflexive<T> {
    type Target = Vec<T>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: WriteableData + Clone> DerefMut for Reflexive<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: WriteableData + Clone> IntoIterator for Reflexive<T> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into_inner().into_iter()
    }
}

impl<'a, T: WriteableData + Clone> IntoIterator for &'a Reflexive<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a, T: WriteableData + Clone> IntoIterator for &'a mut Reflexive<T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}
