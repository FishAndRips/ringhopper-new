//! Defines tag sets (tags directories, maps, etc.)

mod multi;
mod cache;
mod cow;
#[cfg(any(feature = "std", feature = "minxp"))]
mod filesystem;
mod cache_file;
#[cfg(test)]
mod test;

use alloc::boxed::Box;
use ringhopper_structs::{EditableTag, Parameters, TagPath};
use alloc::string::String;
use alloc::vec::Vec;

/// Defines a set of tags which can be used by various Ringhopper functions.
pub trait Tagset {
    /// Read a tag from the tagset.
    ///
    /// Returns `Err` if an error occurred.
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError>;

    /// Write a tag to the tagset.
    ///
    /// Returns `Err` if an error occurred.
    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError>;

    /// Return `true` if the tagset contains a tag at the given tag path.
    fn has_tag(&self, tag_path: &TagPath) -> bool;

    /// Return `true` if the tagset can be written to.
    ///
    /// If `false`, then `write_tag` will likely always fail and should not be called.
    fn is_writeable(&self) -> bool;

    /// Find all tags and directories at the given path `dir`.
    ///
    /// This path may use either `/` (Unix path separator), `\\` (Halo path separator), or, if `std`
    /// is enabled, the system's native path separator.
    fn enumerate_directory(&self, dir: &str) -> Vec<TagsetDirectoryEntry>;

    /// Get all tags in the tagset.
    ///
    /// You can alternatively use `enumerate_directory` to do this, but this may provide a more
    /// efficient way of doing this.
    fn get_all_tags(&self) -> Vec<TagPath>;
}

/// An entry returned by [`Tagset::enumerate_directory`].
#[derive(Clone, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub enum TagsetDirectoryEntry {
    /// A tag file was found.
    TagFile {
        /// Filename of the tag.
        filename: String,

        /// Full path of the tag.
        path: TagPath
    },

    /// A directory was found.
    Directory {
        /// Filename of the directory.
        filename: String
    }
}

/// Indicates a tag write error.
#[derive(Clone, Debug)]
pub enum ReadTagError {
    /// The tag was not found.
    NotFound,

    /// A parsing error occurred.
    ParseError {
        /// Error description.
        ///
        /// Usually this is the error converted to a string.
        description: String
    },
    /// An IO error occurred.
    IOError {
        /// Error description.
        ///
        /// Usually this is the error converted to a string.
        description: String
    }
}

/// Indicates a tag read error.
#[derive(Clone, Debug)]
pub enum WriteTagError {
    /// The tagset is read-only.
    ReadOnlyTagset,

    /// A serialization error occurred.
    SerializeError {
        /// Error description.
        ///
        /// Usually this is the error converted to a string.
        description: String
    },

    /// An IO error occurred.
    IOError {
        /// Error description.
        ///
        /// Usually this is the error converted to a string.
        description: String
    }
}

pub use multi::*;
pub use cow::*;
pub use cache::*;
#[cfg(any(feature = "std", feature = "minxp"))]
pub use filesystem::*;
#[cfg(test)]
pub(crate) use test::*;
