//! Defines tag sets (tags directories, maps, etc.)

mod multi;
mod cache;
mod cow;
#[cfg(feature = "std")]
mod filesystem;

use alloc::boxed::Box;
use ringhopper_structs::{EditableTag, Parameters, TagPath};
use alloc::string::String;

pub trait Tagset {
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError>;
    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError>;
    fn has_tag(&self, tag_path: &TagPath) -> bool;
    fn is_writeable(&self) -> bool;
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
#[cfg(feature = "std")]
pub use filesystem::*;
