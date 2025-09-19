use alloc::vec::Vec;
use byteorder::ByteOrder;
use funnel_web::id::TagID;
use crate::{Address, Parameters, Reflexive, SimpleWriteableData, TagPath, TagReference};
use crate::simple_io::{ReflexiveC, TagReferenceC};

pub trait WriteableData: Sized {
    fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError>;
    fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError>;
    fn base_length() -> usize;
}

#[derive(Debug, Clone, PartialEq)]
pub enum WriteableDataError {
    OutOfBounds { offset_requested: usize, size_requested: usize },
    Other { description: &'static str }
}

impl<T: SimpleWriteableData> WriteableData for T {
    #[inline(always)]
    fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, _cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError> {
        Self::read_tag_data_simple::<B>(&tag_data[offset..add_offsets(offset, Self::length(), tag_data.len())?], parameters)
            .map_err(|description| WriteableDataError::Other { description })
    }
    #[inline(always)]
    fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError> {
        let tag_data_len = tag_data.len();
        Ok(self.write_tag_data_simple::<B>(&mut tag_data[offset..add_offsets(offset, Self::length(), tag_data_len)?], parameters))
    }
    #[inline(always)]
    fn base_length() -> usize {
        Self::length()
    }
}

pub(crate) fn add_offsets(offset: usize, size: usize, buffer_size: usize) -> Result<usize, WriteableDataError> {
    offset.checked_add(size)
        .and_then(|c| if c > buffer_size { None } else { Some(c) })
        .ok_or(WriteableDataError::OutOfBounds { offset_requested: offset, size_requested: size })
}

impl WriteableData for TagReference {
    fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError> {
        let tag_reference = TagReferenceC::read_tag_data::<B>(tag_data, offset, cursor, parameters)?;
        let len = tag_reference.path_size as usize;
        if len == 0 {
            return Ok(TagReference::Unset(tag_reference.group))
        }

        let end = add_offsets(*cursor, len, tag_data.len())?;
        let end_with_null_terminator = add_offsets(end, 1, tag_data.len())?;

        let Some(tag_path_str) = core::ffi::CStr::from_bytes_with_nul(&tag_data[*cursor..end_with_null_terminator])
            .ok()
            .and_then(|s| s.to_str().ok()) else {
            return Err(WriteableDataError::Other { description: "tag reference is invalid or not properly null terminated" })
        };

        *cursor = end_with_null_terminator;

        TagPath::from_path_without_extension(tag_path_str, tag_reference.group)
            .map_err(|description| WriteableDataError::Other { description })
            .map(TagReference::Set)
    }
    fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError> {
        match self {
            TagReference::Unset(group) => TagReferenceC {
                group: *group,
                path_pointer: Address::default(),
                path_size: 0,
                tag_id: TagID::new()
            }.write_tag_data::<B>(tag_data, offset, parameters),

            TagReference::Set(tag_path) => {
                let path = tag_path.path();
                let path_size = u32::try_from(path.len())
                    .ok()
                    .ok_or(WriteableDataError::Other { description: "tag path length overflows a 32-bit size" })?;

                TagReferenceC {
                    group: tag_path.group(),
                    path_pointer: Address::default(),
                    path_size,
                    tag_id: TagID::new()
                }.write_tag_data::<B>(tag_data, offset, parameters)?;

                tag_data.extend_from_slice(path.as_bytes());
                tag_data.push(0);

                Ok(())
            }
        }
    }
    fn base_length() -> usize {
        0x10
    }
}

impl<T: WriteableData> WriteableData for Reflexive<T> {
    fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError> {
        let base = ReflexiveC::read_tag_data::<B>(tag_data, offset, cursor, parameters)?;
        if base.count == 0 {
            return Ok(Self::new())
        }
        let count = base.count as usize;

        let base_length = T::base_length();
        let total_base_length = base_length
            .checked_mul(count)
            .ok_or(WriteableDataError::Other { description: "Number of elements times base_length exceeds usize::MAX" })?;

        let cursor_base = *cursor;
        let cursor_end = add_offsets(cursor_base, total_base_length, tag_data.len())?;
        *cursor = cursor_end;

        let mut reserved = Reflexive::<T>::with_capacity(count);
        for c in (cursor_base..cursor_end).step_by(base_length) {
            reserved.push(T::read_tag_data::<B>(tag_data, c, cursor, parameters)?);
        }
        Ok(reserved)
    }

    fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError> {
        if self.is_empty() {
            return ReflexiveC {
                count: 0, address: Address::default(), unused: 0
            }.write_tag_data::<B>(tag_data, offset, parameters);
        }

        let count = self.len();
        let count_u32 = u32::try_from(count)
            .ok()
            .ok_or(WriteableDataError::Other { description: "Number of elements exceeds u32::MAX" })?;

        let base_length = T::base_length();
        let total_base_length = base_length
            .checked_mul(count)
            .ok_or(WriteableDataError::Other { description: "Number of elements times base_length exceeds usize::MAX" })?;

        let tag_data_start = tag_data.len();
        let new_tag_data_end = tag_data_start
            .checked_add(total_base_length)
            .ok_or(WriteableDataError::Other { description: "Number of elements times base_length plus tag_data.len() exceeds usize::MAX" })?;

        ReflexiveC {
            count: count_u32,
            address: Address::default(),
            unused: 0
        }.write_tag_data::<B>(tag_data, offset, parameters)?;

        tag_data.resize(new_tag_data_end, 0);
        for (offset, element) in (tag_data_start..new_tag_data_end).step_by(base_length).zip(self.iter()) {
            element.write_tag_data::<B>(tag_data, offset, parameters)?;
        }

        Ok(())
    }

    fn base_length() -> usize {
        0xC
    }
}
