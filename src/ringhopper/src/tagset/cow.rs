use ringhopper_structs::{EditableTag, TagPath};
use super::TagSet;
use alloc::boxed::Box;

/// Copy-on-write
///
/// Only reads from reader and writes to writer.
///
/// Mooooooooo!
pub struct Cow<R: TagSet, W: TagSet> {
    reader: R,
    writer: W
}

impl<R: TagSet, W: TagSet> Cow<R, W> {
    pub const fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

impl<R: TagSet, W: TagSet> TagSet for Cow<R, W> {
    fn read_tag(&self, tag_path: &TagPath) -> Option<Box<dyn EditableTag>> {
        self.reader.read_tag(tag_path)
    }
    fn write_tag(&self, tag_path: &TagPath, tag: &dyn EditableTag) {
        self.writer.write_tag(tag_path, tag)
    }
    fn has_tag(&self, tag_path: &TagPath) -> bool {
        self.reader.has_tag(tag_path)
    }
}
