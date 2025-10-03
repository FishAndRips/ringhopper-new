use crate::definitions::engine::{Engine, EngineCompressionType};
use crate::definitions::tag::cache::{CEAFlags, CacheFileHeader, CacheFileHeaderPCDemo, CacheFileTag, CacheFileTagDataHeader, CacheFileTagDataHeaderExternalModels, CacheFileTagDataHeaderInternalModels};
use crate::definitions::tag::scenario::Scenario;
use crate::definitions::tag::TagGroup;
use crate::{Address, EditableTag, ForceBaseMemoryAddress, Parameters, SimpleWriteableData, TagPath, TagReference, WriteableData, WriteableDataError};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::format;
use byteorder::LittleEndian;
use core::cell::UnsafeCell;
use core::ffi::CStr;
use crate::compression::zlib_decompress;
use crate::definitions::tag::scenario_structure_bsp::ScenarioStructureBSP;

#[derive(Clone, Debug)]
#[expect(unused)]
pub struct ParsedCacheFile {
    decompressed_cache: Option<Vec<u8>>,

    tag_path_to_info: BTreeMap<Arc<TagPath>, Arc<TagInfo>>,
    tag_index_to_info: BTreeMap<usize, Arc<TagInfo>>,
    tag_paths_sorted: Vec<Arc<TagPath>>,
    tag_paths_ordered: Vec<Option<Arc<TagPath>>>,

    sections: BTreeMap<DataSectionType, core::ops::Range<usize>>,
    engine: &'static Engine,
    base_memory_address: u32,
    tag_count: usize,

    scenario_tag_index: usize,
    scenario_tag_data: Option<Box<Scenario>>,

    buffers: ParsedCacheFileBuffers
}

#[derive(Debug)]
#[expect(unused)]
pub(crate) struct TagInfo {
    /// Path of the tag
    pub tag_path: Arc<TagPath>,

    /// Index of the tag
    pub index: usize,

    /// Tag group of the tag
    pub tag_group: TagGroup,

    /// Section the tag's data is located
    pub section: UnsafeCell<DataSectionType>,

    /// Offset in the section
    pub offset: usize,

    /// Only the main struct is in the map; the rest is in Sounds
    pub external_sound: bool,
}

impl TagInfo {
    #[inline]
    const fn section(&self) -> DataSectionType {
        // SAFETY: Nothing else is able to mutably access this anymore.
        unsafe { *self.section.get() }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Ord, PartialOrd, Eq)]
#[expect(unused)]
pub(crate) enum DataSectionType {
    TagData,
    BSP(usize),
    Bitmaps,
    Sounds,
    Loc,
    ModelVertices,
    ModelTriangles,
    Unloaded
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

        let mut cache_buffer = buffers.cache.as_slice();

        let compressed_alignment = engine.compressed_data_alignment;
        let compressed_size = cache_buffer.len();

        if compressed_size % engine.compressed_data_alignment != 0 {
            return Err(LoadCacheFileError::CorruptMap { description: format!("Compressed size 0x{compressed_size:08X} does not align to compressed data alignment 0x{compressed_alignment:08X}") });
        }

        let mut decompressed_cache: Option<Vec<u8>> = None;
        match engine.compression_type {
            EngineCompressionType::Uncompressed => (),
            EngineCompressionType::Deflate => {
                decompressed_cache = Some(inflate_cache_file(buffers.cache.as_slice(), &header, engine)?);
                cache_buffer = decompressed_cache.as_ref().unwrap().as_slice();
            }
        };

        // if this overflows, it is wrong (since we managed to load the whole map in RAM somehow)
        let tag_data_offset = header.tag_data_offset as usize;
        let tag_data_end = tag_data_offset.checked_add(header.tag_data_size as usize).ok_or(LoadCacheFileError::CorruptHeader)?;
        let tag_data_range = tag_data_offset..tag_data_end;
        let tag_data = cache_buffer.get(tag_data_range.clone()).ok_or(LoadCacheFileError::CorruptHeader)?;
        let base_header = CacheFileTagDataHeader::read_tag_data::<LittleEndian>(
            tag_data,
            0,
            &mut 0,
            parameters
        )?;
        let tag_count = base_header.tag_count as usize;

        let Some(scenario_tag_index) = base_header.scenario_tag.index() else {
            return Err(LoadCacheFileError::CorruptMap { description: "Scenario tag index is null".to_owned() });
        };

        let mut sections = BTreeMap::new();
        sections.insert(DataSectionType::TagData, tag_data_range);

        if engine.has_external_models {
            let base_header_extended = CacheFileTagDataHeaderExternalModels::read_tag_data::<LittleEndian>(
                tag_data,
                0,
                &mut 0,
                parameters
            )?;

            let model_data_start = base_header_extended.model_data_file_offset as usize;
            let model_triangle_offset = base_header_extended.model_triangle_offset as usize;
            let model_data_size = base_header_extended.model_data_size as usize;

            if model_triangle_offset > model_data_size {
                return Err(LoadCacheFileError::CorruptMap {
                    description: format!("Can't find model data (model_triangle_offset 0x{model_data_start:08X} exceeds model_data_size 0x{model_data_size:08X})")
                })
            }

            let Some(model_data_end) = model_data_start.checked_add(model_data_size) else {
                // This is still a corrupt map because there is no way we could have loaded the map into RAM if this was true
                return Err(LoadCacheFileError::CorruptMap {
                    description: format!("Can't find model data (model_data_start 0x{model_data_start:08X} + model_data_size 0x{model_data_size:08X} overflows usize 0x{:08X})", usize::MAX)
                })
            };

            if cache_buffer.get(model_data_start..model_data_end).is_none() {
                return Err(LoadCacheFileError::CorruptMap {
                    description: "Can't find model data (model_data range not present in map)".to_string()
                })
            }

            let triangle_offset = model_data_start + base_header_extended.model_triangle_offset as usize;

            let vertices = model_data_start..triangle_offset;
            let triangles = triangle_offset..model_data_end;
            sections.insert(DataSectionType::ModelVertices, vertices);
            sections.insert(DataSectionType::ModelTriangles, triangles);
        }

        let tag_array_address = base_header.tag_array_address.0;
        let base_memory_address = find_base_memory_address(
            engine,
            &header,
            base_header.tag_array_address.0,
            parameters
        )?;
        debug_assert!(tag_array_address > base_memory_address, "bad base memory address calculation; tag array is before the tag data somehow");

        let tag_path_to_info = get_all_tags(
            parameters,
            engine,
            tag_data,
            base_memory_address,
            &base_header
        )?;

        let tag_index_to_info: BTreeMap<usize, Arc<TagInfo>> = tag_path_to_info
            .iter()
            .map(|i| (i.1.index, i.1.clone()))
            .collect();

        let Some(scenario_tag_info) = tag_index_to_info.get(&scenario_tag_index) else {
            return Err(LoadCacheFileError::CorruptMap {
                description: format!("Can't find scenario tag (scenario tag ID is wrong, no tag at index #{scenario_tag_index})")
            })
        };

        if scenario_tag_info.tag_group != TagGroup::Scenario {
            return Err(LoadCacheFileError::CorruptMap {
                description: format!("Can't find scenario tag (scenario tag at #{scenario_tag_index} is not a scenario but a {})", scenario_tag_info.tag_group)
            })
        }

        let mut cache_file = ParsedCacheFile {
            decompressed_cache,
            tag_paths_sorted: tag_path_to_info.keys().cloned().collect(),
            tag_paths_ordered: (0..tag_count).map(|i| tag_index_to_info.get(&i).map(|i| i.tag_path.clone())).collect(),
            tag_path_to_info,
            tag_index_to_info,
            engine,
            base_memory_address,
            scenario_tag_data: None,
            scenario_tag_index,
            sections,
            buffers,
            tag_count
        };

        let extracted_scenario_tag = cache_file.extract_tag(cache_file.scenario_tag_index, parameters)
            .map_err(|e| LoadCacheFileError::CorruptMap { description: format!("Failed to read scenario tag: {e}") })?;

        let scenario_tag = extracted_scenario_tag
            .downcast_ref::<Scenario>()
            .expect("scenario tag was not actually a scenario even though we checked this...");

        for (index, entry) in scenario_tag.structure_bsps.iter().enumerate() {
            match &entry.structure_bsp {
                TagReference::Unset(_) => {}
                TagReference::Set(path) => {
                    if path.group() != TagGroup::ScenarioStructureBSP {
                        return Err(LoadCacheFileError::CorruptMap { description: format!("BSP #{index} is not a scenario_structure_bsp but a {}", path.group()) });
                    }
                    let info = cache_file
                        .tag_path_to_info
                        .get(path)
                        .expect("should have worked...");

                    // SAFETY: We're the only thing that can access this right now
                    let section = unsafe { &mut *info.section.get() };
                    let new_section = DataSectionType::BSP(index);
                    *section = new_section;

                    let bsp_start = entry.bsp_start as usize;
                    let bsp_size = entry.bsp_size as usize;

                    let Some(bsp_end) = bsp_start.checked_add(entry.bsp_size as usize) else {
                        return Err(LoadCacheFileError::CorruptMap { description: format!("BSP #{index} overflows (0x{bsp_start:08X} + 0x{bsp_size:08X})") });
                    };

                    let bsp_range = bsp_start..bsp_end;
                    let Some(bsp_data) = cache_file.get_cache_buffer().get(bsp_range.clone()) else {
                        return Err(LoadCacheFileError::CorruptMap { description: format!("BSP #{index} is out-of-bounds for the cache file (0x{bsp_start:08X}..0x{bsp_end:08X})") });
                    };

                    let address_len = Address::length();
                    if bsp_size < Address::length() {
                        return Err(LoadCacheFileError::CorruptMap { description: format!("BSP #{index} has a bad size 0x{bsp_size:08X} (can't read the main BSP address)") });
                    }

                    let address = Address::read_tag_data_simple::<LittleEndian>(&bsp_data[0..address_len], Parameters::CACHE_FILES).expect("bsp address read fail!!!");
                    let Some(offset) = address.0.checked_sub(entry.bsp_address).map(|i| i as usize) else {
                        return Err(LoadCacheFileError::CorruptMap { description: format!("BSP #{index}'s address 0x{:08X} is out-of-bounds (left)", address.0) });
                    };

                    if bsp_data.get(offset..).and_then(|i| i.get(..ScenarioStructureBSP::base_length())).is_none() {
                        return Err(LoadCacheFileError::CorruptMap { description: format!("BSP #{index}'s address 0x{:08X} is out-of-bounds (right)", address.0) });
                    }

                    cache_file.sections.insert(new_section, bsp_range);
                }
            }
        }

        for i in &cache_file.tag_path_to_info {
            if i.1.section() == DataSectionType::BSP(usize::MAX) {
                return Err(LoadCacheFileError::CorruptMap { description: format!("BSP {} is not referenced by the scenario tag", i.0) });
            }
        }

        todo!();

        Ok(cache_file)
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
    pub const fn get_scenario_tag_index(&self) -> usize {
        self.scenario_tag_index
    }

    #[inline]
    pub const fn get_scenario_tag_data(&self) -> &Scenario {
        &*self.scenario_tag_data
            .as_ref()
            .expect("scenario tag not loaded")
    }

    #[inline]
    pub fn tag_path_to_tag_index(&self, tag_path: &TagPath) -> Option<usize> {
        self.tag_path_to_info.get(tag_path).map(|t| t.index)
    }

    #[expect(unused)]
    pub fn extract_tag(&self, index: usize, parameters: Parameters) -> Result<Box<dyn EditableTag>, WriteableDataError> {
        if let Some(scenario) = self.scenario_tag_data.as_ref() && self.scenario_tag_index == index {
            return Ok(scenario.clone_to_boxed_tag());
        }

        todo!("add extract_tag code")
    }

    #[inline]
    pub const fn get_engine(&self) -> &'static Engine {
        self.engine
    }

    /// Get the number of tag entries in the cache file.
    ///
    /// This is equivalent to `self.tag_paths().len()`. It may not
    ///
    /// If the cache file was pre-indexed (e.g. forged against another map) and there are gaps in
    /// the entries
    #[inline]
    pub const fn tag_entry_count(&self) -> usize {
        self.tag_count
    }

    #[inline]
    #[expect(unused)]
    pub(crate) fn get_section(&self, section: DataSectionType) -> Option<&[u8]> {
        let s = self.sections.get(&section)?.to_owned();
        Some(
            match section {
                DataSectionType::Loc => &self.buffers.loc.as_ref()?[s],
                DataSectionType::Bitmaps => &self.buffers.bitmaps.as_ref()?[s],
                DataSectionType::Sounds => &self.buffers.sounds.as_ref()?[s],
                _ => &self.get_cache_buffer()[s]
            }
        )
    }

    /// Get all tag paths in lexicographic order.
    #[inline]
    pub fn tag_paths_sorted(&self) -> &[Arc<TagPath>] {
        self.tag_paths_sorted.as_slice()
    }

    /// Get all tag paths in cache order.
    ///
    /// If the cache file was pre-indexed (e.g. forged against another map), it may contain gaps of
    /// `None` entries in between.
    #[inline]
    pub fn tag_paths(&self) -> &[Option<Arc<TagPath>>] {
        self.tag_paths_ordered.as_slice()
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

fn find_base_memory_address(
    engine: &Engine,
    header: &CacheFileHeader,
    tag_array_address: u32,
    parameters: Parameters
) -> Result<u32, LoadCacheFileError> {
    let header_size = if engine.has_external_models {
        CacheFileTagDataHeaderExternalModels::base_length()
    }
    else {
        CacheFileTagDataHeaderInternalModels::base_length()
    } as u32;

    let engine_base_memory_address = engine
        .base_memory_address
        .address;
    let eschaton_base_memory_address = tag_array_address
        .checked_sub(header_size)
        .ok_or(LoadCacheFileError::CorruptMap { description: "Underflowed the base memory address".to_owned() })?;

    match parameters.base_memory_address {
        Some(ForceBaseMemoryAddress::ForceFixed) => Ok(engine.base_memory_address.address),
        Some(ForceBaseMemoryAddress::ForceInferred) => Ok(eschaton_base_memory_address),
        None => {
            if engine.base_memory_address.inferred {
                // The tag data's base memory address is inferred based on the location of the tag array
                // which is assumed to immediately be after the header.
                Ok(eschaton_base_memory_address)
            }
            else {
                let expected_tag_array_address = engine_base_memory_address + header_size;

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
                    let description = format!(
                        "Incorrect tag array address (expected 0x{expected_tag_array_address:08X}, got 0x{tag_array_address:08X})"
                    );
                    let estimate = format!(
                        "(detected engine = \"{}\", expected address = 0x{engine_base_memory_address:08X}, estimated address = 0x{eschaton_base_memory_address:08X}, cache file build = \"{}\")",
                        engine.name,
                        header.build
                    );

                    let possible_range = engine_base_memory_address..engine_base_memory_address.saturating_add(header.tag_data_size);

                    return if header.build.as_str().is_empty() {
                        Err(LoadCacheFileError::MapDetectionError {
                            description: format!("{description} - Map appears to be for an untracked version of the game which may be using a different base memory address! {estimate}")
                        })
                    }
                    else if engine.is_fallback {
                        Err(LoadCacheFileError::MapDetectionError {
                            description: format!("{description} - Map appears to be for an unknown version of the game which may be using a different base memory address! {estimate}")
                        })
                    }
                    else if !possible_range.contains(&tag_array_address) {
                        Err(LoadCacheFileError::MapDetectionError {
                            description: format!("{description} - The map's base memory address is incorrect for what engine was detected {estimate}")
                        })
                    }
                    else {
                        Err(LoadCacheFileError::MapDetectionError {
                            description: format!("{description} - Map appears to have its tag array moved somewhere else in its tag space OR it has a different base memory address; it is possibly protected/corrupted, so we're refusing to load it! {estimate}")
                        })
                    };
                }
                Ok(engine_base_memory_address)
            }
        }
    }
}

fn get_all_tags(parameters: Parameters, engine: &Engine, tag_data: &[u8], base_memory_address: u32, base_header: &CacheFileTagDataHeader) -> Result<BTreeMap<Arc<TagPath>, Arc<TagInfo>>, LoadCacheFileError> {
    let get_tag_data = |address: u32, length: Option<usize>| -> Result<&[u8], LoadCacheFileError> {
        let offset = address.checked_sub(base_memory_address)
            .ok_or_else(|| LoadCacheFileError::CorruptMap {
                description: format!("Tried to get tag address 0x{address:08X} which is outside of the tag data...")
            })? as usize;

        tag_data.get(offset..)
            .and_then(|i| match length {
                Some(j) => i.get(0..j),
                None => Some(i)
            })
            .ok_or_else(|| LoadCacheFileError::CorruptMap {
                description: format!("Tried to get tag address 0x{address:08X} with length {length:?} but it overflowed...")
            })
    };

    let tag_count = base_header.tag_count as usize;
    let tag_entry_size = CacheFileTag::length();
    let tag_entries_size = tag_entry_size.checked_mul(tag_count).ok_or_else(|| LoadCacheFileError::CorruptMap { description: "bad tag count".to_string() })?;

    let mut all_tags: BTreeMap<Arc<TagPath>, Arc<TagInfo>> = BTreeMap::new();

    let tags = get_tag_data(
        base_header.tag_array_address.0,
        Some(tag_entries_size)
    )?;

    for tag_chunk_index in tags.chunks(tag_entry_size).enumerate().map(
        |(index, t)| CacheFileTag::read_tag_data_simple::<LittleEndian>(t, parameters)
            .map_err(|t| LoadCacheFileError::CorruptMap { description: format!("Bad tag entry #{index} - {t}") })
            .map(|t| (index, t))
    ) {
        let (index, tag_chunk) = tag_chunk_index?;

        // Ignore this; it is likely a MISSINGNO.
        if tag_chunk.tag_group == TagGroup::None {
            continue
        }

        // Check the tag ID
        if Some(index) != tag_chunk.id.index() {
            return Err(LoadCacheFileError::CorruptMap { description: format!("Bad tag entry #{index} - TagID 0x{:08X} does not correspond to its index of {index}", tag_chunk.id.as_u32()) });
        }

        let tag_path_bytes = get_tag_data(tag_chunk.path.0, None)?;
        let tag_path = {
            // Find the null terminator
            CStr::from_bytes_until_nul(tag_path_bytes).map_err(|_| LoadCacheFileError::CorruptMap {
                description: format!("Bad tag path (no null terminator) #{index}")
            })

                // To UTF-8
                .and_then(|t| t.to_str().map_err(|_| LoadCacheFileError::CorruptMap {
                    description: format!("Bad tag path (non-utf-8) #{index}")
                }))

                // Parse
                .and_then(|t| TagPath::from_path_without_extension(t, tag_chunk.tag_group).map_err(|e| LoadCacheFileError::CorruptMap {
                    description: format!("Bad tag path ({e}) #{index}")
                }))
        }?;

        if all_tags.contains_key(&tag_path) {
            return Err(LoadCacheFileError::CorruptMap {
                description: format!("Duplicate tag path #{index} - {tag_path}")
            })
        }

        let mut external_sound = false;
        let section = if engine.resource_maps.is_some_and(|i| i.externally_indexed_tags) {
            match tag_chunk.tag_group {
                TagGroup::Sound => {
                    external_sound = true;
                    DataSectionType::TagData
                },
                TagGroup::Bitmap => DataSectionType::Bitmaps,
                _ => DataSectionType::Loc
            }
        } else {
            match tag_chunk.tag_group {
                TagGroup::ScenarioStructureBSP => DataSectionType::BSP(usize::MAX),
                _ => DataSectionType::TagData
            }
        };

        let tag_path = Arc::new(tag_path);
        all_tags.insert(tag_path.clone(), Arc::new(TagInfo {
            tag_path,
            index,
            section: UnsafeCell::new(section),
            external_sound,
            offset: {
                match section {
                    DataSectionType::TagData => {
                        let Some(offset) = tag_chunk.data.0.checked_sub(base_memory_address) else {
                            return Err(LoadCacheFileError::CorruptMap { description: format!("Bad tag address #{index} - falls out of map (left)") })
                        };

                        let offset = offset as usize;
                        if tag_data.get(offset).is_none() {
                            return Err(LoadCacheFileError::CorruptMap { description: format!("Bad tag address #{index} - falls out of map (right)") })
                        }

                        offset
                    },

                    // We'll go back to get these in a moment
                    _ => usize::MAX
                }
            },
            tag_group: tag_chunk.tag_group
        }));
    }

    Ok(all_tags)
}

fn inflate_cache_file(cache_file: &[u8], header: &CacheFileHeader, engine: &Engine) -> Result<Vec<u8>, LoadCacheFileError> {
    let full_uncompressed_size = header.decompressed_size as usize;

    let padding = header.compression_padding as usize;

    if padding > engine.compressed_data_alignment {
        return Err(LoadCacheFileError::CorruptMap {
            description: format!("Decompression failed; compression padding 0x{padding:08X} is too big")
        })
    }

    let usable_size = cache_file.len().checked_sub(padding).expect("bad length (engine compressed_data_alignment check exploded)");
    let cache_file = &cache_file[..usable_size];

    let (header_data, compressed_data) = cache_file.split_at(CacheFileHeader::base_length());
    if full_uncompressed_size < header_data.len() {
        return Err(LoadCacheFileError::CorruptMap {
            description: format!("Decompression failed; uncompressed size 0x{full_uncompressed_size:08X} is smaller than a header and thus could not possibly be valid")
        })
    }

    let mut output = alloc::vec![0u8; full_uncompressed_size];
    let (output_header_data, output_decompressed_data) = output.split_at_mut(header_data.len());
    output_header_data.copy_from_slice(header_data);

    let expected_decompressed_length = output_decompressed_data.len();
    let actual_decompressed_length = zlib_decompress(compressed_data, output_decompressed_data)?;
    if actual_decompressed_length != expected_decompressed_length {
        return Err(LoadCacheFileError::CorruptMap {
            description: format!(
                "Decompression failed; decompressed size 0x{expected_decompressed_length:08X} != actual decompressed length 0x{actual_decompressed_length:08X}",
            )
        })
    }

    Ok(output)
}
