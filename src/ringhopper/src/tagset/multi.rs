use alloc::vec::Vec;
use alloc::boxed::Box;
use ringhopper_structs::{EditableTag, TagPath};
use super::TagSet;

pub struct MultiTagSet<T: TagSet> {
    sets: Vec<T>
}

impl<T: TagSet> MultiTagSet<T> {
    pub const fn new(sets: Vec<T>) -> Self {
        Self {
            sets
        }
    }
    pub fn write_tag_to_set(&self, tag_path: &TagPath, tag: &dyn EditableTag, set: usize) {
        self.sets.get(set).expect("set out-of-bounds").write_tag(tag_path, tag)
    }
}

impl<T: TagSet> TagSet for MultiTagSet<T> {
    fn read_tag(&self, tag_path: &TagPath) -> Option<Box<dyn EditableTag>> {
        for i in self.sets.iter().rev() {
            if let Some(t) = i.read_tag(tag_path) {
                return Some(t)
            }
        }
        None
    }
    fn write_tag(&self, tag_path: &TagPath, tag: &dyn EditableTag) {
        for i in self.sets.iter().rev() {
            if i.has_tag(tag_path) {
                return i.write_tag(tag_path, tag);
            }
        }
        self.sets
            .first()
            .expect("no tags directory")
            .write_tag(tag_path, tag)
    }
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        for i in self.sets.iter() {
            if i.has_tag(tag_path) {
                return true
            }
        }
        false
    }
}
