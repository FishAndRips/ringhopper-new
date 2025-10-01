//! Contains all definition-derived data.
#![allow(unused)]

pub const HEAD_FOURCC: u32 = 0x68656164;
pub const FOOT_FOURCC: u32 = 0x666F6F74;
pub const HEAD_FOURCC_OBFUSCATED: u32 = 0x45686564;
pub const FOOT_FOURCC_OBFUSCATED: u32 = 0x47666F74;

/// Contains all tag definitions as well as all tag groups.
pub mod tag {
    ringhopper_structs_codegen::generate_tag_group_enum!();

    use crate::*;
    use alloc::vec::Vec;
    use funnel_web::vector::*;
    use funnel_web::color::*;
    use funnel_web::string::*;
    use funnel_web::id::*;
    use funnel_web::rectangle::*;
    use alloc::string::String;
    use alloc::boxed::Box;
    use byteorder::ByteOrder;

    ringhopper_structs_codegen::generate_tag_data_defs!();
}

/// Contains all engine definitions.
pub mod engine {
    use crate::{Parameters, Strictness};
    use crate::definitions::{FOOT_FOURCC, FOOT_FOURCC_OBFUSCATED, HEAD_FOURCC, HEAD_FOURCC_OBFUSCATED};
    use crate::definitions::tag::cache::CacheFileHeader;

    #[derive(Debug)]
    pub struct Engine {
        pub name: &'static str,
        pub display_name: &'static str,
        pub version: &'static str,
        pub cache_file_version: u32,
        pub build: Option<EngineBuild>,
        pub compression_type: EngineCompressionType,

        pub max_tag_space: usize,
        pub max_script_nodes: usize,
        pub cache_file_size_limits: EngineCacheFileSizeLimits,

        pub base_memory_address: EngineBaseMemoryAddress,
        pub data_alignment: usize,
        pub bitmaps: EngineBitmap,
        pub has_external_models: bool,
        pub uses_compressed_models: bool,
        pub allows_external_bsps: bool,
        pub has_obfuscated_header_layout: bool,
        pub resource_maps: Option<EngineSupportedResourceMaps>,
        pub required_tags: EngineRequiredTags,

        pub is_build_target: bool,
        pub is_fallback: bool,
        pub is_cache_default: bool,
        pub is_custom: bool
    }

    #[derive(Debug)]
    pub struct EngineBuild {
        pub main: &'static str,
        pub enforced: bool,
        pub aliases: &'static [&'static str]
    }

    #[derive(Debug)]
    pub struct EngineBitmap {
        pub swizzled: bool,
        pub texture_dimension_must_modulo_block_size: bool,
        pub cubemap_faces_stored_separately: bool,
        pub alignment: usize
    }

    #[derive(Debug)]
    pub struct EngineCacheFileSizeLimits {
        pub user_interface: usize,
        pub singleplayer: usize,
        pub multiplayer: usize
    }

    #[derive(Debug)]
    pub struct EngineBaseMemoryAddress {
        pub address: u32,
        pub inferred: bool
    }

    #[derive(Debug)]
    pub enum EngineCompressionType {
        Uncompressed,
        Deflate
    }

    #[derive(Debug)]
    pub struct EngineSupportedResourceMaps {
        externally_indexed_tags: bool
    }

    #[derive(Debug)]
    pub struct EngineRequiredTags {
        pub all: &'static [&'static str],
        pub user_interface: &'static [&'static str],
        pub singleplayer: &'static [&'static str],
        pub multiplayer: &'static [&'static str]
    }

    ringhopper_structs_codegen::generate_engine_defs!();

    impl Engine {
        /// Get the engine by its shorthand/name.
        pub fn get(engine: &str) -> Option<&'static Self> {
            ALL_ENGINES.binary_search_by(|i| i.name.cmp(engine))
                .ok()
                .map(|i| &ALL_ENGINES[i])
        }

        /// Match the first 2048 bytes of a cache file's header to an engine.
        pub fn read_header(bytes: &[u8]) -> Option<(&'static Self, CacheFileHeader)> {
            use crate::SimpleWriteableData;
            use super::tag;
            use tag::cache::{CacheFileHeader, CacheFileHeaderPCDemo};
            use alloc::vec::Vec;

            let header_bytes = bytes.get(0x0..0x800)?;
            let parameters = Parameters {
                strictness: Strictness::Strict,
                ..Parameters::CACHE_FILES
            };

            let mut unobfuscated_header = CacheFileHeader::read_tag_data_simple::<byteorder::LittleEndian>(header_bytes, parameters)
                .ok();
            let mut obfuscated_header = CacheFileHeaderPCDemo::read_tag_data_simple::<byteorder::LittleEndian>(header_bytes, parameters)
                .ok()
                .map(|i| i.as_cache_file_header());

            if unobfuscated_header.is_some_and(|h| h.head_fourcc != HEAD_FOURCC || h.foot_fourcc != FOOT_FOURCC) {
                unobfuscated_header = None;
            }

            if obfuscated_header.is_some_and(|h| h.head_fourcc != HEAD_FOURCC_OBFUSCATED || h.foot_fourcc != FOOT_FOURCC_OBFUSCATED) {
                obfuscated_header = None;
            }

            let mut candidates: Vec<&'static Engine> = Vec::new();
            let header: CacheFileHeader;

            if let Some(h) = unobfuscated_header {
                candidates.extend(ALL_ENGINES.iter().filter(|e| !e.has_obfuscated_header_layout));
                header = h;
            }
            else if let Some(h) = obfuscated_header {
                candidates.extend(ALL_ENGINES.iter().filter(|e| e.has_obfuscated_header_layout));
                header = h;
            }
            else {
                return None
            }

            let mut exact_match: Option<&'static Engine> = None;

            candidates.retain(|i| {
                if exact_match.is_some() {
                    return false;
                }

                if header.cache_version != i.cache_file_version {
                    return false
                }

                if let Some(b) = &i.build {
                    let build_str = header.build.as_str();
                    if b.main == build_str || b.aliases.contains(&build_str) {
                        exact_match = Some(i);
                        return true;
                    }
                    else if b.enforced {
                        return false;
                    }
                }

                true
            });

            let check_for_match = (|| {
                if exact_match.is_some() {
                    return exact_match
                }

                let first = Some(candidates.iter().copied().next()?);

                if candidates.len() == 1 {
                    return first;
                }

                if let Some(c) = candidates.iter().copied().find(|i| i.is_build_target) {
                    return Some(c)
                }

                if let Some(c) = candidates.iter().copied().find(|i| !i.is_fallback) {
                    return Some(c)
                }

                first
            })()?;

            Some((check_for_match, header))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::definitions::engine::{Engine, ALL_ENGINES};

    #[test]
    fn header_check() {
        let custom_edition = include_bytes!("definitions/bloodgulch_custom_edition_header.bin");
        assert_eq!(Engine::read_header(custom_edition).expect("could not match custom edition").0.name, "pc-custom", "Halo Custom Edition's header matched the wrong engine");

        let trial = include_bytes!("definitions/bloodgulch_trial_header.bin");
        assert_eq!(Engine::read_header(trial).expect("could not match halo trial").0.name, "pc-demo", "Halo Trial's header matched the wrong engine");
    }

    #[test]
    fn all_engines_can_be_found_by_get_engine() {
        for i in ALL_ENGINES {
            let Some(e) = Engine::get(i.name) else {
                panic!("Failed to find engine {}", i.name);
            };
            assert_eq!(i.name, e.name, "Found an engine {} but it was actually {}", i.name, e.name);
        }
    }
}
