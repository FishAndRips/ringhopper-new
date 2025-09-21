use alloc::vec::Vec;
use alloc::collections::TryReserveError;
use core::iter::once;
use byteorder::ByteOrder;
use funnel_web::id::TagID;
use alloc::string::String;
use funnel_web::crc::CRC32;
use crate::{Address, Parameters, Reflexive, SimpleWriteableData, Strictness, TagPath, TagReference, MAX_PATH_LEN};
use crate::definitions::tag::TagFileHeader;
use crate::definitions::TagGroup;
use crate::simple_io::{ReflexiveC, TagDataC, TagReferenceC};

pub trait MainTagStruct: WriteableData {
    fn tag_group() -> TagGroup;
}

pub trait WriteableData: Sized {
    fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError>;
    fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError>;
    #[must_use]
    fn base_length() -> usize;
}

#[derive(Debug, Clone, PartialEq)]
pub enum WriteableDataError {
    OutOfBounds { offset_requested: usize, size_requested: usize },
    Other { description: &'static str },
    LowMemory
}

impl From<TryReserveError> for WriteableDataError {
    fn from(_value: TryReserveError) -> Self {
        Self::LowMemory
    }
}

impl<T: SimpleWriteableData> WriteableData for T {
    #[inline]
    fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, _cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError> {
        Self::read_tag_data_simple::<B>(&tag_data[offset..add_offsets(offset, Self::length(), tag_data.len())?], parameters)
            .map_err(|description| WriteableDataError::Other { description })
    }
    #[inline]
    fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError> {
        let tag_data_len = tag_data.len();
        self.write_tag_data_simple::<B>(&mut tag_data[offset..add_offsets(offset, Self::length(), tag_data_len)?], parameters);
        Ok(())
    }
    #[inline]
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
                let path_len = path.len();
                debug_assert!(path_len < MAX_PATH_LEN);

                let path_size = path_len as u32;
                TagReferenceC {
                    group: tag_path.group(),
                    path_pointer: Address::default(),
                    path_size,
                    tag_id: TagID::new()
                }.write_tag_data::<B>(tag_data, offset, parameters)?;

                tag_data.try_reserve(path_len + 1)?;
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

        tag_data.try_reserve(total_base_length)?;

        let tag_data_start = tag_data.len();
        let new_tag_data_end = tag_data_start + total_base_length;
        tag_data.resize(new_tag_data_end, 0);

        ReflexiveC {
            count: count_u32,
            address: Address::default(),
            unused: 0
        }.write_tag_data::<B>(tag_data, offset, parameters)?;

        for (offset, element) in (tag_data_start..new_tag_data_end).step_by(base_length).zip(self.iter()) {
            element.write_tag_data::<B>(tag_data, offset, parameters)?;
        }

        Ok(())
    }

    #[inline]
    fn base_length() -> usize {
        0xC
    }
}

impl WriteableData for Vec<u8> {
    fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError> {
        let data = TagDataC::read_tag_data::<B>(tag_data, offset, cursor, parameters)?;
        let length = data.length as usize;
        if length == 0 {
            return Ok(Vec::new());
        }

        let cursor_base = *cursor;
        let cursor_end = add_offsets(cursor_base, length, tag_data.len())?;
        *cursor = cursor_end;

        Ok(tag_data[cursor_base..cursor_end].to_vec())
    }

    fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError> {
        tag_data.try_reserve(self.len())?;

        let length = u32::try_from(self.len())
            .ok()
            .ok_or(WriteableDataError::Other { description: "Maximum data size of 4 GiB reached/exceeded" })?;

        tag_data.try_reserve(length as usize)?;

        TagDataC {
            length,
            ..Default::default()
        }.write_tag_data::<B>(tag_data, offset, parameters)?;

        tag_data.extend_from_slice(self.as_slice());

        Ok(())
    }

    #[inline]
    fn base_length() -> usize {
        0x14
    }
}

impl WriteableData for String {
    fn read_tag_data<B: ByteOrder>(tag_data: &[u8], offset: usize, cursor: &mut usize, parameters: Parameters) -> Result<Self, WriteableDataError> {
        let data = TagDataC::read_tag_data::<B>(tag_data, offset, cursor, parameters)?;
        let length = data.length as usize;

        let cursor_base = *cursor;
        let cursor_end = add_offsets(cursor_base, length, tag_data.len())?;
        *cursor = cursor_end;

        let data = &tag_data[cursor_base..cursor_end];
        parse_utf16_string(data, parameters)
    }

    fn write_tag_data<B: ByteOrder>(&self, tag_data: &mut Vec<u8>, offset: usize, parameters: Parameters) -> Result<(), WriteableDataError> {
        let length_bytes = encode_utf16_null_terminated_string(self)
            .count()
            .checked_mul(size_of::<u16>())
            .and_then(|v| u32::try_from(v).ok())
            .ok_or(WriteableDataError::Other { description: "Maximum string size reached/exceeded 4 GiB when encoded into UTF-16" })?;

        tag_data.try_reserve(length_bytes as usize)?;

        TagDataC {
            length: length_bytes,
            ..Default::default()
        }.write_tag_data::<B>(tag_data, offset, parameters)?;

        let mut null_byte_added = false;
        for b in encode_utf16_null_terminated_string(self) {
            if null_byte_added {
                // TODO: On relaxed mode, we can probably just cut the string off here
                return Err(WriteableDataError::Other { description: "Invalid string data (interior null bytes detected)" });
            }
            if b == 0 {
                null_byte_added = true;
            }
            tag_data.extend_from_slice(&b.to_le_bytes());
        }

        Ok(())
    }

    #[inline]
    fn base_length() -> usize {
        0x14
    }
}

fn parse_utf16_string(data_with_null_terminator: &[u8], parameters: Parameters) -> Result<String, WriteableDataError> {
    if !data_with_null_terminator.len().is_multiple_of(2) {
        return Err(WriteableDataError::Other { description: "UTF-16 string has improper length" })
    }
    if data_with_null_terminator.is_empty() {
        return Err(WriteableDataError::Other { description: "UTF-16 string has no data (thus is not null-terminated)" })
    }

    let (data, null_terminator) = data_with_null_terminator.split_at(data_with_null_terminator.len() - 2);
    if null_terminator != [0,0] && parameters.strictness > Strictness::Relaxed {
        // On the game, this null terminator will be nulled out at runtime anyway, but such tags
        // are quite dangerous.
        return Err(WriteableDataError::Other { description: "UTF-16 string is not null terminated" })
    };

    if data.is_empty() {
        return Ok(String::new())
    }

    // SAFETY: We know that this is fine because we checked that data.len() is divisible by 2.
    let chunks = unsafe { data.as_chunks_unchecked::<2>() };

    let wchar_iterator = || chunks
        .iter()
        .copied()
        .map(u16::from_le_bytes);

    let mut null_terminator_encountered = false;
    let mut len = 0usize;

    for c in char::decode_utf16(wchar_iterator()) {
        let Ok(c) = c else {
            return Err(WriteableDataError::Other { description: "Invalid UTF-16" });
        };
        if null_terminator_encountered {
            if parameters.strictness > Strictness::Relaxed {
                return Err(WriteableDataError::Other { description: "Interior null byte detected" });
            }
            break;
        }
        if c == '\x00' {
            null_terminator_encountered = true;
        }
        len = len.checked_add(c.len_utf8())
            // super unlikely but you never know
            .ok_or(WriteableDataError::LowMemory)?;
    }

    let mut string = String::new();
    string.try_reserve(len)?;

    for c in char::decode_utf16(wchar_iterator()) {
        let c = c.expect("was checked in an earlier loop to be valid UTF-16");
        string.push(c);
    }

    Ok(string)
}

fn encode_utf16_null_terminated_string(string: &str) -> impl Iterator<Item = u16> {
    string.encode_utf16().chain(once(0))
}

pub fn read_tag_file<T: MainTagStruct>(data: &[u8], parameters: Parameters) -> Result<T, WriteableDataError> {
    let header_size = TagFileHeader::length();
    if data.len() < header_size {
        return Err(WriteableDataError::Other { description: "invalid tag file (no header)" })
    };

    let mut cursor = header_size;
    let header = TagFileHeader::read_tag_data::<byteorder::BigEndian>(data, 0x0, &mut cursor, parameters)?;
    let group = T::tag_group();
    if header.tag_group != group {
        return Err(WriteableDataError::Other { description: "invalid tag file (wrong group)" })
    }
    if header.version != group.version() {
        return Err(WriteableDataError::Other { description: "invalid tag file (wrong version for tag group)" })
    }

    // should not have changed
    debug_assert_eq!(cursor, header_size);

    let base_length = T::base_length();
    let base_offset = cursor;
    cursor = add_offsets(cursor, base_length, data.len())?;

    if parameters.strictness > Strictness::Relaxed {
        if header.tag_data_offset as usize != base_offset {
            return Err(WriteableDataError::Other { description: "invalid tag file (bad tag data offset)" })
        }

        let mut crc = CRC32::new();
        crc.update(&data[base_offset..]);
        if crc.crc() != header.crc32 {
            return Err(WriteableDataError::Other { description: "invalid tag file (wrong CRC32 - tag is corrupted)" })
        }
    }

    T::read_tag_data::<byteorder::BigEndian>(data, base_offset, &mut cursor, parameters)
}

pub fn write_tag_file<T: MainTagStruct>(tag_data: &T, parameters: Parameters) -> Result<Vec<u8>, WriteableDataError> {
    let mut final_data = Vec::new();

    let header_size = TagFileHeader::length();
    let initial_size = header_size.checked_add(T::base_length()).expect("base_length of header and tag group overflow; that is bad");
    final_data.try_reserve(initial_size)?;
    final_data.resize(initial_size, 0);

    tag_data.write_tag_data::<byteorder::BigEndian>(&mut final_data, header_size, parameters)?;

    let tag_group = T::tag_group();
    let final_header = TagFileHeader {
        tag_group,
        crc32: {
            let mut crc = CRC32::new();
            crc.update(&final_data[header_size..]);
            crc.crc()
        },
        tag_data_offset: header_size as u32,
        internal_size: 0,
        version: tag_group.version(),
        first_internal_index: 0,
        second_internal_index: 0xFF,
        blam_fourcc: 0x626C616D,
    };

    final_header.write_tag_data_simple::<byteorder::BigEndian>(&mut final_data[..header_size], parameters);

    Ok(final_data)
}
