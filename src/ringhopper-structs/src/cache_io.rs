use funnel_web::id::TagID;
use crate::definitions::cache::{CacheFileHeader, CacheFileHeaderPCDemo};
use crate::{EditableTag, Parameters, TagPath, WriteableDataError};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct ParsedCacheFile {
    tags: BTreeMap<TagPath, TagInfo>,
    sections: BTreeMap<DataSectionType, DataSection>,

    cache_data: Vec<u8>,
    bitmaps_data: Vec<u8>,
    sounds_data: Vec<u8>,
    loc_data: Vec<u8>
}

#[derive(Copy, Clone, Debug)]
#[expect(unused)]
pub(crate) struct TagInfo {
    /// ID of the tag
    pub tag_id: TagID,

    /// Section the tag's data is located
    pub section: DataSectionType,

    /// Offset in the section
    pub offset: usize
}

#[derive(Copy, Clone, PartialEq, Debug, Ord, PartialOrd, Eq)]
#[expect(unused)]
pub(crate) enum DataSectionType {
    TagData,
    BSP(usize),
    Bitmaps,
    Sounds,
    Loc,
    Unloaded
}

#[derive(Clone, Debug)]
pub(crate) struct DataSection {
    pub range: core::ops::Range<usize>
}

impl ParsedCacheFile {
    #[inline]
    pub fn tag_path_to_tag_id(&self, tag_path: &TagPath) -> Option<TagID> {
        self.tags.get(tag_path).map(|t| t.tag_id)
    }
    #[expect(unused)]
    pub fn extract_tag(&self, tag_id: TagID, parameters: Parameters) -> Result<Box<dyn EditableTag>, WriteableDataError> {
        todo!()
    }
    #[inline]
    #[expect(unused)]
    pub(crate) fn get_section(&self, section: DataSectionType) -> Option<&[u8]> {
        self.sections
            .get(&section)
            .map(|s| match section {
                DataSectionType::Loc => &self.loc_data[s.range.clone()],
                DataSectionType::Bitmaps => &self.bitmaps_data[s.range.clone()],
                DataSectionType::Sounds => &self.sounds_data[s.range.clone()],
                _ => &self.cache_data[s.range.clone()]
            })
    }
}

impl CacheFileHeaderPCDemo {
    #[expect(unused)]
    pub(crate) fn as_cache_file_header(self) -> CacheFileHeader {
        CacheFileHeader {
            map_type: self.map_type,
            head_fourcc: self.head_fourcc,
            tag_data_size: self.tag_data_size,
            tag_data_offset: self.tag_data_offset,
            build: self.build,
            cache_version: self.cache_version,
            name: self.name,
            crc32: self.crc32,
            decompressed_size: self.decompressed_size,
            foot_fourcc: self.foot_fourcc,
            ..Default::default()
        }
    }
}

impl CacheFileHeader {
    #[expect(unused)]
    pub(crate) fn as_pc_demo_cache_file_header(self) -> CacheFileHeaderPCDemo {
        CacheFileHeaderPCDemo {
            map_type: self.map_type,
            head_fourcc: self.head_fourcc,
            tag_data_size: self.tag_data_size,
            tag_data_offset: self.tag_data_offset,
            build: self.build,
            cache_version: self.cache_version,
            name: self.name,
            crc32: self.crc32,
            decompressed_size: self.decompressed_size,
            foot_fourcc: self.foot_fourcc
        }
    }
}
