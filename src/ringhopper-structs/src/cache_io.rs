use funnel_web::id::TagID;
use crate::definitions::cache::{CacheFileHeader, CacheFileHeaderPCDemo};
use crate::{EditableTag, Parameters, TagPath, WriteableDataError};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::sync::Arc;
use crate::definitions::scenario::Scenario;

#[derive(Clone, Debug)]
pub struct ParsedCacheFile {
    tags: BTreeMap<Arc<TagPath>, TagInfo>,
    tag_paths: Vec<Arc<TagPath>>,
    sections: BTreeMap<DataSectionType, DataSection>,

    scenario_tag_id: TagID,
    scenario_tag_data: Box<Scenario>,

    buffers: ParsedCacheFileBuffers
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

#[derive(Clone, Debug)]
pub enum LoadCacheFileError {}

#[derive(Clone, Debug)]
pub struct ParsedCacheFileBuffers {
    pub cache: Vec<u8>,
    pub bitmaps: Option<Vec<u8>>,
    pub sounds: Option<Vec<u8>>,
    pub loc: Option<Vec<u8>>
}

impl ParsedCacheFile {
    #[inline]
    #[expect(unused)]
    pub fn load_cache_file_from_buffers(
        buffers: ParsedCacheFileBuffers
    ) -> Result<Self, LoadCacheFileError> {
        todo!()
    }

    #[inline]
    pub fn load_cache_file_from_cloned_slices<C: AsRef<[u8]>, B: AsRef<[u8]>, S: AsRef<[u8]>, L: AsRef<[u8]>>(
        cache_buffer: C,
        bitmaps_buffer: B,
        sounds_buffer: S,
        loc_buffer: L
    ) -> Result<Self, LoadCacheFileError> {
        let bitmaps_buffer = bitmaps_buffer.as_ref();
        let sounds_buffer = sounds_buffer.as_ref();
        let loc_buffer = loc_buffer.as_ref();

        fn none_if_empty(buffer: &[u8]) -> Option<Vec<u8>> {
            if buffer.is_empty() {
                None
            }
            else {
                Some(buffer.to_vec())
            }
        }

        Self::load_cache_file_from_buffers(ParsedCacheFileBuffers {
            cache: cache_buffer.as_ref().to_vec(),
            bitmaps: none_if_empty(bitmaps_buffer),
            sounds: none_if_empty(sounds_buffer),
            loc: none_if_empty(loc_buffer)
        })
    }

    #[inline]
    pub fn to_inner_buffers(self) -> ParsedCacheFileBuffers {
        self.buffers
    }

    #[inline]
    pub const fn get_scenario_tag_id(&self) -> TagID {
        self.scenario_tag_id
    }

    #[inline]
    pub const fn get_scenario_tag_data(&self) -> &Scenario {
        &*self.scenario_tag_data
    }

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
        let s = self.sections.get(&section)?;
        Some(
            match section {
                DataSectionType::Loc => &self.buffers.loc.as_ref()?[s.range.clone()],
                DataSectionType::Bitmaps => &self.buffers.bitmaps.as_ref()?[s.range.clone()],
                DataSectionType::Sounds => &self.buffers.sounds.as_ref()?[s.range.clone()],
                _ => &self.buffers.cache[s.range.clone()]
            }
        )
    }

    #[inline]
    pub fn tags(&self) -> &[Arc<TagPath>] {
        self.tag_paths.as_slice()
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
