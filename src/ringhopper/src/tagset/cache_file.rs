use ringhopper_structs::{EditableTag, Parameters, ParsedCacheFile, TagPath};
use crate::tagset::{ReadTagError, Tagset, WriteTagError};
use alloc::boxed::Box;
use alloc::string::ToString;

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
}
