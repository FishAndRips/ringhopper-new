use ringhopper_structs::{EditableTag, Parameters, TagPath};
use super::{ReadTagError, Tagset, TagsetDirectoryEntry, WriteTagError};
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Copy-on-write tagset.
///
/// Only reads from reader and writes to writer.
///
/// If `!writer.is_writable()`, then writes will not be performed.
///
/// 🐄 Mooooooooo! 🐄
pub struct CowTagset<R: Tagset, W: Tagset> {
    reader: R,
    writer: W
}

impl<R: Tagset, W: Tagset> CowTagset<R, W> {
    #[inline]
    pub const fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
    #[inline]
    pub const fn get_reader(&self) -> &R {
        &self.reader
    }
    #[inline]
    pub const fn get_reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }
    #[inline]
    pub const fn get_writer(&self) -> &W {
        &self.writer
    }
    #[inline]
    pub const fn get_writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

impl<R: Tagset, W: Tagset> Tagset for CowTagset<R, W> {
    #[inline]
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        self.reader.read_tag(tag_path, parameters)
    }
    #[inline]
    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError> {
        if self.writer.is_writeable() {
            self.writer.write_tag(tag_path, tag, parameters)
        }
        else {
            Err(WriteTagError::ReadOnlyTagset)
        }
    }
    #[inline]
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.reader.has_tag(tag_path)
    }
    #[inline]
    fn is_writeable(&self) -> bool {
        self.writer.is_writeable()
    }
    #[inline]
    fn enumerate_directory(&self, dir: &str) -> Vec<TagsetDirectoryEntry> {
        self.reader.enumerate_directory(dir)
    }
    #[inline]
    fn get_all_tags(&self) -> Vec<TagPath> {
        self.reader.get_all_tags()
    }
}
