use funnel_web::id::TagID;
use crate::definitions::tag::cache::{CEAFlags, CacheFileHeader, CacheFileHeaderPCDemo, CacheFileTagDataHeader, CacheFileTagDataHeaderExternalModels, CacheFileTagDataHeaderInternalModels};
use crate::{EditableTag, ForceBaseMemoryAddress, Parameters, TagPath, WriteableData, WriteableDataError};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::string::String;
use alloc::borrow::ToOwned;
use alloc::string::ToString;
use byteorder::LittleEndian;
use crate::definitions::engine::{Engine, EngineCompressionType};
use crate::definitions::tag::scenario::Scenario;

#[derive(Clone, Debug)]
pub struct ParsedCacheFile {
    decompressed_cache: Option<Vec<u8>>,

    tags: BTreeMap<Arc<TagPath>, TagInfo>,
    tag_paths: Vec<Arc<TagPath>>,
    sections: BTreeMap<DataSectionType, DataSection>,
    engine: &'static Engine,

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
pub enum LoadCacheFileError {
    UnknownEngine,
    CorruptHeader,
    CorruptMap { description: String },
    MapDetectionError { description: String }
}

impl From<WriteableDataError> for LoadCacheFileError {
    fn from(value: WriteableDataError) -> Self {
        Self::CorruptMap { description: value.to_string() }
    }
}

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
        buffers: ParsedCacheFileBuffers,
        parameters: Parameters
    ) -> Result<Self, LoadCacheFileError> {
        let (engine, header) = Engine::read_header(&buffers.cache)
            .ok_or(LoadCacheFileError::UnknownEngine)?;

        let mut buffer_to_use = buffers.cache.as_slice();
        let mut decompressed_buffer: Vec<u8> = Vec::new();

        match engine.compression_type {
            EngineCompressionType::Uncompressed => (),
            EngineCompressionType::Deflate => todo!("DEFLATE")
        }

        let tag_data_offset = header.tag_data_offset as usize;

        // if this overflows, it is wrong (since we managed to load the whole map in RAM somehow)
        let tag_data_end = tag_data_offset.checked_add(header.tag_data_size as usize).ok_or(LoadCacheFileError::CorruptHeader)?;

        let tag_data = buffer_to_use.get(tag_data_offset..tag_data_end).ok_or(LoadCacheFileError::CorruptHeader)?;
        let base_header = CacheFileTagDataHeader::read_tag_data::<LittleEndian>(
            tag_data,
            0,
            &mut 0,
            parameters
        )?;

        let header_size = if engine.has_external_models {
            CacheFileTagDataHeaderExternalModels::base_length()
        }
        else {
            CacheFileTagDataHeaderInternalModels::base_length()
        } as u32;

        let mut base_memory_address = engine.base_memory_address.address;

        let tag_array_address = base_header.tag_array_address.0;
        let eschaton_base_memory_address = tag_array_address
            .checked_sub(header_size)
            .ok_or(LoadCacheFileError::CorruptMap { description: "Underflowed the base memory address".to_owned() })?;

        match parameters.base_memory_address {
            Some(ForceBaseMemoryAddress::ForceFixed) => base_memory_address = engine.base_memory_address.address,
            Some(ForceBaseMemoryAddress::ForceInferred) => base_memory_address = eschaton_base_memory_address,
            None => {
                if engine.base_memory_address.inferred {
                    // The tag data's base memory address is inferred based on the location of the tag array
                    // which is assumed to immediately be after the header.
                    base_memory_address = eschaton_base_memory_address;
                }
                else {
                    let expected_tag_array_address = base_memory_address + header_size;

                    // The tag data's base memory address is fixed for the target engine. We should
                    // still check, though, as bad things may happen if we proceed.
                    //
                    // If the tag array is not where we think it is, then one of three things are
                    // true about this map:
                    //
                    // - The map's tag array was moved by a map protector/corruptor.
                    //   Proceeding will DEFINITELY fail (for unrelated reasons).
                    //
                    // - The map was built with its tag array in a different location.
                    //   Proceeding MIGHT not fail.
                    //
                    // - The map is for an untracked beta build of the game we can't detect.
                    //   Proceeding will DEFINITELY fail (for this reason).
                    //
                    // The most likely case is the first one. There are a very, very small number of
                    // maps in the wild that have their tag array in a different location that
                    // aren't corrupted. To proceed, use `ForceBaseMemoryAddress::ForceFixed`.
                    //
                    // In case of the third one, there's no way to really discern untracked (i.e. no
                    // build string) versions of the game. If the beta is especially old, the
                    // definitions may very possibly not even work. Feel free to try your luck with
                    // `ForceBaseMemoryAddress::ForceInferred`.
                    if tag_array_address != expected_tag_array_address {
                        let description = alloc::format!(
                            "Incorrect tag array address (expected 0x{expected_tag_array_address:08X}, got 0x{tag_array_address:08X})"
                        );
                        let estimate = alloc::format!(
                            "(detected engine = \"{}\", expected address = 0x{base_memory_address:08X}, estimated address = 0x{eschaton_base_memory_address:08X}, cache file build = \"{}\")",
                            engine.name,
                            header.build
                        );

                        let possible_range = base_memory_address..base_memory_address.saturating_add(header.tag_data_size);

                        return if header.build.as_str().is_empty() {
                            Err(LoadCacheFileError::MapDetectionError {
                                description: alloc::format!("{description} - Map appears to be for an untracked version of the game which may be using a different base memory address! {estimate}")
                            })
                        }
                        else if engine.is_fallback {
                            Err(LoadCacheFileError::MapDetectionError {
                                description: alloc::format!("{description} - Map appears to be for an unknown version of the game which may be using a different base memory address! {estimate}")
                            })
                        }
                        else if !possible_range.contains(&tag_array_address) {
                            Err(LoadCacheFileError::MapDetectionError {
                                description: alloc::format!("{description} - The map's base memory address is incorrect for what engine was detected {estimate}")
                            })
                        }
                        else {
                            Err(LoadCacheFileError::MapDetectionError {
                                description: alloc::format!("{description} - Map appears to have its tag array moved somewhere else in its tag space OR it has a different base memory address; it is possibly protected/corrupted, so we're refusing to load it! {estimate}")
                            })
                        };
                    }
                }
            }
        }



        todo!()
    }

    #[inline]
    pub fn load_cache_file_from_cloned_slices<C: AsRef<[u8]>, B: AsRef<[u8]>, S: AsRef<[u8]>, L: AsRef<[u8]>>(
        cache_buffer: C,
        bitmaps_buffer: B,
        sounds_buffer: S,
        loc_buffer: L,
        parameters: Parameters
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
        }, parameters)
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
    pub const fn get_engine(&self) -> &'static Engine {
        self.engine
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
                _ => &self.get_cache_buffer()[s.range.clone()]
            }
        )
    }

    #[inline]
    pub fn tags(&self) -> &[Arc<TagPath>] {
        self.tag_paths.as_slice()
    }

    #[inline]
    fn get_cache_buffer(&self) -> &[u8] {
        self.decompressed_cache.as_ref().map(|i| i.as_slice()).unwrap_or(self.buffers.cache.as_slice())
    }
}

impl CacheFileHeaderPCDemo {
    pub const fn as_cache_file_header(self) -> CacheFileHeader {
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
            cea_flags: CEAFlags {
                uses_bitmap_data_from_tags: false,
                uses_sound_data_from_tags: false,
                disable_anniversary_mode: false
            },
            compression_padding: 0
        }
    }
}

impl CacheFileHeader {
    pub const fn as_pc_demo_cache_file_header(self) -> CacheFileHeaderPCDemo {
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
