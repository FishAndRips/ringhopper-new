use alloc::vec::Vec;
use alloc::boxed::Box;
use ringhopper_structs::{EditableTag, Parameters, TagPath};
use super::{ReadTagError, Tagset, WriteTagError};

pub struct MultiTagset<T: Tagset> {
    sets: Vec<T>
}

impl<T: Tagset> MultiTagset<T> {
    #[inline]
    pub const fn new(sets: Vec<T>) -> Self {
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
        self.sets.get_mut(set).expect("set out-of-bounds").write_tag(tag_path, tag, parameters)
    }
}

impl<T: Tagset> Tagset for MultiTagset<T> {
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        for i in self.sets.iter().rev() {
            match i.read_tag(tag_path, parameters) {
                Ok(n) => return Ok(n),
                Err(ReadTagError::NotFound) => continue,
                Err(e) => return Err(e)
            }
        }
        Err(ReadTagError::NotFound)
    }
    
    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError> {
        for i in self.sets.iter_mut().rev() {
            if i.has_tag(tag_path) {
                return i.write_tag(tag_path, tag, parameters);
            }
        }
        self.sets
            .first_mut()
            .expect("no tags directory")
            .write_tag(tag_path, tag, parameters)
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
}
