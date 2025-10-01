use ringhopper_structs::{is_path_separator, EditableTag, Parameters, ParsedCacheFile, TagPath, HALO_PATH_SEPARATOR, HALO_PATH_SEPARATOR_STR};
use crate::tagset::{ReadTagError, Tagset, TagsetDirectoryEntry, WriteTagError};
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::collections::BTreeSet;
use alloc::borrow::ToOwned;
use alloc::sync::Arc;

impl Tagset for ParsedCacheFile {
    #[inline]
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        let tag_id = self.tag_path_to_tag_id(tag_path)
            .ok_or(ReadTagError::NotFound)?;
        self.extract_tag(tag_id, parameters)
            .map_err(|e| ReadTagError::ParseError { description: e.to_string() })
    }

    #[inline]
    fn write_tag(&mut self, _tag_path: &TagPath, _tag: &dyn EditableTag, _parameters: Parameters) -> Result<(), WriteTagError> {
        Err(WriteTagError::ReadOnlyTagset)
    }

    #[inline]
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.tag_path_to_tag_id(tag_path).is_some()
    }

    #[inline]
    fn is_writeable(&self) -> bool {
        false
    }

    fn enumerate_directory(&self, dir: &str) -> Vec<TagsetDirectoryEntry> {
        let mut dir_matched = dir.replace(is_path_separator, HALO_PATH_SEPARATOR_STR);
        if !dir_matched.ends_with(HALO_PATH_SEPARATOR_STR) && !dir_matched.is_empty() {
            dir_matched += HALO_PATH_SEPARATOR_STR;
        }

        let mut entries = BTreeSet::new();

        for i in self.tags() {
            let path = i.path();
            let Some((prefix, suffix)) = path.split_at_checked(dir_matched.len()) else {
                continue
            };

            if prefix != dir_matched {
                continue
            }

            let mut entry_iterator = suffix.split(HALO_PATH_SEPARATOR);
            let Some(stem) = entry_iterator.next() else {
                continue
            };
            if entry_iterator.next().is_some() {
                entries.insert(TagsetDirectoryEntry::Directory {
                    filename: stem.to_owned()
                });
            }
            else {
                let filename = alloc::format!("{stem}.{}", i.group().as_str());
                
                entries.insert(TagsetDirectoryEntry::TagFile {
                    filename,
                    path: Arc::as_ref(&i).to_owned()
                });
            }
        }

        entries.into_iter().collect()
    }

    #[inline]
    fn get_all_tags(&self) -> Vec<TagPath> {
        self.tags().iter().map(|i| Arc::as_ref(&i).to_owned()).collect()
    }
}
