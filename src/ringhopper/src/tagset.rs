//! Defines tag sets (tags directories, maps, etc.)

mod multi;
mod cache;
mod cow;

use alloc::boxed::Box;
use ringhopper_structs::{EditableTag, TagPath};

pub trait TagSet {
    fn read_tag(&self, tag_path: &TagPath) -> Option<Box<dyn EditableTag>>;
    fn write_tag(&self, tag_path: &TagPath, tag: &dyn EditableTag);
    fn has_tag(&self, tag_path: &TagPath) -> bool;
}

pub use multi::*;
pub use cow::*;
pub use cache::*;
