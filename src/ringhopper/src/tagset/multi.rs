use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use ringhopper_structs::{EditableTag, Parameters, TagPath};
use super::{ReadTagError, Tagset, TagsetDirectoryEntry, WriteTagError};

/// A virtual tagset composed of multiple tagsets.
///
/// Sets with lower indices take priority for both reading and writing.
///
/// If a new tag gets created, the first tagset will be written to.
#[repr(transparent)]
pub struct MultiTagset<T: Tagset> {
    sets: Vec<T>
}

/// Instantiate a [`MultiTagset`] with the given tagsets.
///
/// The tagsets can all be different types, as this internally creates a vector of `Box<dyn Tagset>`
/// types.
#[macro_export]
macro_rules! multi_tagset {
    [$($tagsets_other:expr), *] => {{
        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use $crate::tagset::Tagset;

        let mut tagsets: Vec<Box<dyn Tagset>> = Vec::new();
        $(tagsets.push(Box::new($tagsets_other));)*
        tagsets
    }}
}

impl<T: Tagset> MultiTagset<T> {
    /// Instantiate a new tagset.
    ///
    /// # Panics
    ///
    /// Panics if `sets.is_empty()`
    #[inline]
    pub const fn new(sets: Vec<T>) -> Self {
        assert!(!sets.is_empty());
        Self {
            sets
        }
    }
    #[inline]
    pub const fn get_sets(&self) -> &[T] {
        self.sets.as_slice()
    }
    #[inline]
    pub const fn get_sets_mut(&mut self) -> &mut [T] {
        self.sets.as_mut_slice()
    }
    #[inline]
    pub const fn get_sets_vec_mut(&mut self) -> &mut Vec<T> {
        &mut self.sets
    }
    #[inline]
    pub fn write_tag_to_set(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, set: usize, parameters: Parameters) -> Result<(), WriteTagError> {
        self.sets
            .get_mut(set)
            .expect("set out-of-bounds")
            .write_tag(tag_path, tag, parameters)
    }
    /// Return the original tagset vector.
    #[inline]
    pub fn into_sets(self) -> Vec<T> {
        self.sets
    }
}

impl<T: Tagset> Tagset for MultiTagset<T> {
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        for i in self.sets.iter() {
            match i.read_tag(tag_path, parameters) {
                Ok(n) => return Ok(n),
                Err(ReadTagError::NotFound) => continue,
                Err(e) => return Err(e)
            }
        }
        Err(ReadTagError::NotFound)
    }
    
    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError> {
        for i in self.sets.iter_mut() {
            if i.has_tag(tag_path) {
                return i.write_tag(tag_path, tag, parameters);
            }
        }
        self.write_tag_to_set(tag_path, tag, 0, parameters)
    }

    #[inline]
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        for i in self.sets.iter() {
            if i.has_tag(tag_path) {
                return true
            }
        }
        false
    }

    #[inline]
    fn is_writeable(&self) -> bool {
        self.sets.iter().any(|i| i.is_writeable())
    }

    #[inline]
    fn enumerate_directory(&self, dir: &str) -> Vec<TagsetDirectoryEntry> {
        let mut entries = BTreeSet::new();

        for i in &self.sets {
            entries.extend(i.enumerate_directory(dir))
        }

        entries.into_iter().collect()
    }

    #[inline]
    fn get_all_tags(&self) -> Vec<TagPath> {
        let mut entries = BTreeSet::new();

        for i in &self.sets {
            entries.extend(i.get_all_tags())
        }

        entries.into_iter().collect()
    }
}

#[cfg(test)]
mod test {
    use crate::tagset::TestTagset;

    #[test]
    fn multi_tagset_macro() {
        let tagset_1 = TestTagset::default();
        let tagset_2 = TestTagset::default();
        multi_tagset![tagset_1, tagset_2];
    }
}
