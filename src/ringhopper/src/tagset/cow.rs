use ringhopper_structs::{EditableTag, Parameters, TagPath};
use super::{ReadTagError, Tagset, WriteTagError};
use alloc::boxed::Box;

/// Copy-on-write
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
    pub const fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
    pub const fn get_reader(&self) -> &R {
        &self.reader
    }
    pub const fn get_reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }
    pub const fn get_writer(&self) -> &W {
        &self.writer
    }
    pub const fn get_writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

impl<R: Tagset, W: Tagset> Tagset for CowTagset<R, W> {
    fn read_tag(&self, tag_path: &TagPath, parameters: Parameters) -> Result<Box<dyn EditableTag>, ReadTagError> {
        self.reader.read_tag(tag_path, parameters)
    }
    fn write_tag(&mut self, tag_path: &TagPath, tag: &dyn EditableTag, parameters: Parameters) -> Result<(), WriteTagError> {
        if self.writer.is_writeable() {
            self.writer.write_tag(tag_path, tag, parameters)
        }
        else {
            Err(WriteTagError::ReadOnlyTagset)
        }
    }
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.reader.has_tag(tag_path)
    }
    fn is_writeable(&self) -> bool {
        self.writer.is_writeable()
    }
}
