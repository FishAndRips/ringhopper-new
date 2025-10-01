use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::boxed::Box;
use ringhopper_structs::{EditableTag, Parameters, TagPath};
use crate::tagset::{ReadTagError, Tagset, TagsetDirectoryEntry, WriteTagError};
use alloc::borrow::ToOwned;

#[derive(Default)]
pub struct TestTagset {
    pub tags: BTreeMap<TagPath, Box<dyn EditableTag>>,
    pub writeable: bool
}

impl Clone for TestTagset {
    fn clone(&self) -> Self {
        let mut tags = BTreeMap::new();
        for i in &self.tags {
            tags.insert(i.0.clone(), i.1.clone_to_boxed_tag());
        }
        Self {
            tags,
            writeable: self.writeable
        }
    }
}

impl Tagset for TestTagset {
    fn read_tag(&self, tag_path: &TagPath, _parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        self.tags.get(tag_path).map(|t| t.clone_to_boxed_tag()).ok_or(ReadTagError::NotFound)
    }

    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, _parameters: Parameters) -> Result<(), WriteTagError> {
        self.tags.insert(tag_path.to_owned(), tag.clone_to_boxed_tag());
        Ok(())
    }

    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.tags.contains_key(tag_path)
    }

    fn is_writeable(&self) -> bool {
        true
    }

    fn enumerate_directory(&self, _dir: &str) -> Vec<TagsetDirectoryEntry> {
        unimplemented!()
    }

    fn get_all_tags(&self) -> Vec<TagPath> {
        self.tags.keys().cloned().collect()
    }
}
