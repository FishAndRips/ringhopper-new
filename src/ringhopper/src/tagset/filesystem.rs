use crate::tagset::{ReadTagError, Tagset, WriteTagError};
use alloc::boxed::Box;
use ringhopper_structs::{EditableTag, Parameters, TagPath};
use alloc::string::ToString;
use ringhopper_structs::definitions::read_editable_tag;

use std::path::PathBuf;

/// Represents a tags directory on a filesystem.
pub struct FilesystemTagset {
    path: PathBuf
}

impl FilesystemTagset {
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn tag_to_file_path(&self, tag: &TagPath) -> PathBuf {
        self.path.join(tag.path_display().to_string())
    }
}

impl Tagset for FilesystemTagset {
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        // This might panic if we run out of RAM.

        let path = self.tag_to_file_path(tag_path);
        let path_ref = &path;
        let data = std::fs::read(path_ref).map_err(|e| {
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
            .map_err(|e| WriteTagError::WriteError { description: e.to_string() })?;

        let path = self.tag_to_file_path(tag_path);
        std::fs::write(path, &data)
            .map_err(|e| WriteTagError::IOError { description: e.to_string() })
    }

    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.tag_to_file_path(tag_path).exists()
    }

    fn is_writeable(&self) -> bool {
        true
    }
}
