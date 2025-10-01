use crate::tagset::{ReadTagError, Tagset, TagsetDirectoryEntry, WriteTagError};
use alloc::boxed::Box;
use ringhopper_structs::{is_path_separator, EditableTag, Parameters, TagPath, HALO_PATH_SEPARATOR, HALO_PATH_SEPARATOR_STR};
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::borrow::ToOwned;
use alloc::string::String;
use ringhopper_structs::definitions::tag::{read_editable_tag, TagGroup};

#[cfg(feature = "minxp")]
mod fs_mod {
    pub use minxp::path::PathBuf;
    pub use minxp::fs;
    pub use minxp::path;
}

#[cfg(all(feature = "std", not(feature = "minxp")))]
mod fs_mod {
    pub use std::path::PathBuf;
    pub use std::fs;
    pub use std::path;
}

use fs_mod::*;

/// Represents a tags directory on a filesystem.
pub struct FilesystemTagset {
    path: PathBuf
}

impl FilesystemTagset {
    #[inline]
    pub fn new<P: AsRef<path::Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    #[inline]
    pub fn tag_to_file_path(&self, tag: &TagPath) -> PathBuf {
        self.path.join(tag.path_display().to_string())
    }
}

impl Tagset for FilesystemTagset {
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        // This might panic if we run out of RAM.

        let path = self.tag_to_file_path(tag_path);
        let path_ref = &path;
        let data = fs::read(path_ref).map_err(|e| {
            if path_ref.exists() {
                ReadTagError::IOError { description: e.to_string() }
            }
            else {
                ReadTagError::NotFound
            }
        })?;

        read_editable_tag(&data, parameters)
            .map_err(|e| ReadTagError::ParseError { description: e.to_string() })
    }

    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError> {
        let data = tag.write_tag_to_vec(parameters)
            .map_err(|e| WriteTagError::SerializeError { description: e.to_string() })?;

        let path = self.tag_to_file_path(tag_path);
        fs::write(path, &data)
            .map_err(|e| WriteTagError::IOError { description: e.to_string() })
    }

    #[inline]
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.tag_to_file_path(tag_path).exists()
    }

    #[inline]
    fn is_writeable(&self) -> bool {
        true
    }

    fn enumerate_directory(&self, dir: &str) -> Vec<TagsetDirectoryEntry> {
        let new_path = dir.replace(is_path_separator, path::MAIN_SEPARATOR_STR);
        let full_path = self.path.join(&new_path);

        let Ok(dir) = full_path.read_dir() else {
            return Vec::new()
        };

        let mut entries = Vec::new();

        for i in dir.filter_map(Result::ok) {
            let Ok(metadata) = i.metadata() else {
                continue
            };

            let path = i.path();
            let Some(filename) = path.file_name().and_then(|i| i.to_str()) else {
                continue
            };

            if metadata.is_dir() {
                entries.push(TagsetDirectoryEntry::Directory { filename: filename.to_owned() });
            }
            else if metadata.is_file() {
                let Some(extension) = path.extension().and_then(|i| i.to_str()) else {
                    continue
                };
                if TagGroup::from_str(extension).is_none() {
                    continue
                }
                entries.push(TagsetDirectoryEntry::TagFile {
                    filename: filename.to_owned(),
                    path: {
                        let mut p = new_path.clone();
                        if !p.ends_with(HALO_PATH_SEPARATOR) && !p.is_empty() {
                            p += HALO_PATH_SEPARATOR_STR;
                        }
                        p += filename;
                        let Ok(p) = TagPath::from_path_with_extension(&p) else {
                            continue
                        };
                        p
                    }
                });
            }
        }

        entries.dedup();
        entries.sort();

        entries
    }
    fn get_all_tags(&self) -> Vec<TagPath> {
        let mut entries = Vec::new();

        fn recursively_get_all_tags(tagset: &FilesystemTagset, dir: &str, entries: &mut Vec<TagPath>) {
            let to_prepend = if dir.is_empty() { String::new() } else { alloc::format!("{dir}{}", path::MAIN_SEPARATOR) };

            for i in tagset.enumerate_directory(dir) {
                match i {
                    TagsetDirectoryEntry::Directory { filename } => {
                        recursively_get_all_tags(tagset, &alloc::format!("{to_prepend}{filename}{}", path::MAIN_SEPARATOR), entries);
                    }
                    TagsetDirectoryEntry::TagFile { path, .. } => {
                        entries.push(path);
                    }
                }
            }
        }

        recursively_get_all_tags(self, "", &mut entries);

        entries
    }
}
