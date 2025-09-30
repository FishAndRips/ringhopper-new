//! Defines tag sets (tags directories, maps, etc.)

mod multi;
mod cache;
mod cow;
#[cfg(any(feature = "std", feature = "minxp"))]
mod filesystem;
mod cache_file;

use alloc::boxed::Box;
use ringhopper_structs::{EditableTag, Parameters, TagPath};
use alloc::string::String;
use alloc::vec::Vec;

pub trait Tagset {
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError>;
    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError>;
    fn has_tag(&self, tag_path: &TagPath) -> bool;
    fn is_writeable(&self) -> bool;
    fn enumerate_directory(&self, dir: &str) -> Vec<TagsetDirectoryEntry>;
    fn get_all_tags(&self) -> Vec<TagPath>;
}

#[derive(Clone, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub enum TagsetDirectoryEntry {
    TagFile {
        filename: String,
        path: TagPath
    },
    Directory {
        filename: String
    }
}

#[derive(Clone, Debug)]
pub enum ReadTagError {
    NotFound,
    ParseError { description: String },
    IOError { description: String }
}

#[derive(Clone, Debug)]
pub enum WriteTagError {
    ReadOnlyTagset,
    WriteError { description: String },
    IOError { description: String }
}

pub use multi::*;
pub use cow::*;
pub use cache::*;
#[cfg(any(feature = "std", feature = "minxp"))]
pub use filesystem::*;
