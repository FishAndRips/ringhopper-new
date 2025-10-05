//! Contains all definition-derived data.
#![allow(unused)]

use crate::build_fourcc;

pub const HEAD_FOURCC: u32 = build_fourcc("head");
pub const FOOT_FOURCC: u32 = build_fourcc("foot");
pub const HEAD_FOURCC_OBFUSCATED: u32 = build_fourcc("Ehed");
pub const FOOT_FOURCC_OBFUSCATED: u32 = build_fourcc("Gfot");

/// Contains all tag definitions as well as all tag groups.
pub mod tag {
    use alloc::borrow::Cow;
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
    use super::engine::Engine;
    use combarc::CombArc;
    use funnel_web::float::FloatOps;

    ringhopper_structs_codegen::generate_tag_group_enum!();
    ringhopper_structs_codegen::generate_tag_data_defs!();

    impl ScenarioTriggerVolume {
        /// Test to see if a point falls inside of the trigger volume.
        #[must_use]
        pub fn test_point(&self, point: Vector3D) -> bool {
            let parameters = self.parameters;

            match self._type {
                ScenarioTriggerVolumeType::Fixed => {
                    let [
                    _,_,_,
                    _,_,_,
                    _,_,_,
                    x_from,x_to,y_from, y_to, z_from, z_to
                    ] = parameters;

                    let b = Rectangle3D {
                        x_from, x_to, y_from, y_to, z_from, z_to
                    };

                    b.test_point(point)
                }
                ScenarioTriggerVolumeType::Rotational => {
                    let [
                    _,_,_,
                    forward_x,forward_y,forward_z,
                    up_x,up_y,up_z,
                    pos_x,pos_y,pos_z,
                    extent_x,extent_y,extent_z,
                    ] = parameters;

                    let forward = Vector3D {
                        x: forward_x,
                        y: forward_y,
                        z: forward_z,
                    };

                    let up = Vector3D {
                        x: up_x,
                        y: up_y,
                        z: up_z,
                    };

                    let pos = Vector3D {
                        x: pos_x,
                        y: pos_y,
                        z: pos_z,
                    };

                    let extent = Vector3D {
                        x: extent_x,
                        y: extent_y,
                        z: extent_z,
                    };

                    test_rotated_bounding_box(forward, up, pos, extent, point)
                }
            }
        }
    }

    impl ModelVertexUncompressed {
        #[inline]
        #[must_use]
        pub fn compress(self) -> ModelVertexCompressed {
            // TODO: Change definitions to use CompressedVector2D type directly and deal with the
            //       consequences.
            let compressed_texture_cords = self.texture_coords.compress();

            ModelVertexCompressed {
                position: self.position,
                normal: self.normal.compress(),
                binormal: self.binormal.compress(),
                tangent: self.tangent.compress(),
                // node1_weight is derived from node0_weight
                node0_weight: self.node0_weight.fw_compress_clamped(),
                // this is multiplied by 3 upfront (for performance?)
                node0_index: (self.node0_index.0 as i16).wrapping_mul(3) as i8,
                node1_index: (self.node1_index.0 as i16).wrapping_mul(3) as i8,
                texture_coordinate_u: compressed_texture_cords.x,
                texture_coordinate_v: compressed_texture_cords.y
            }
        }
    }

    impl ModelVertexCompressed {
        #[inline]
        #[must_use]
        pub fn decompress(self) -> ModelVertexUncompressed {
            let node0_weight = self.node0_weight.decompress();
            ModelVertexUncompressed {
                position: self.position,
                normal: self.normal.decompress(),
                binormal: self.binormal.decompress(),
                tangent: self.tangent.decompress(),
                node0_weight,
                node1_weight: 1.0 - node0_weight,
                node0_index: Index(((self.node0_index) / 3) as u16),
                node1_index: Index(((self.node1_index) / 3) as u16),
                texture_coords: Vector2D {
                    x: self.texture_coordinate_u.decompress(),
                    y: self.texture_coordinate_v.decompress()
                }
            }
        }
    }

    impl ModelRegionPermutation {
        /// Get the geometry index for the LoD index.
        ///
        /// Returns `None` if `lod_index > 4` or there is no geometry.
        pub const fn get_geometry_index_for_lod_index(&self, lod_index: usize) -> Option<usize> {
            match lod_index {
                0 => self.super_low.index(),
                1 => self.low.index(),
                2 => self.medium.index(),
                3 => self.high.index(),
                4 => self.super_high.index(),
                _ => None
            }
        }
    }

    impl ModelDetailNodeCount {
        /// Get the node count.
        ///
        /// Returns `None` if `lod_index > 4`.
        pub const fn get_node_count_mut(&mut self, lod_index: usize) -> Option<&mut u16> {
            match lod_index {
                0 => Some(&mut self.super_low),
                1 => Some(&mut self.low),
                2 => Some(&mut self.medium),
                3 => Some(&mut self.high),
                4 => Some(&mut self.super_high),
                _ => None
            }
        }

        /// Get the node count.
        ///
        /// Returns `None` if `lod_index > 4`.
        pub const fn get_node_count(&self, lod_index: usize) -> Option<u16> {
            match lod_index {
                0 => Some(self.super_low),
                1 => Some(self.low),
                2 => Some(self.medium),
                3 => Some(self.high),
                4 => Some(self.super_high),
                _ => None
            }
        }
    }

    /// Return the object type of the tag group.
    #[inline]
    pub fn object_type_of_tag_group(group: TagGroup) -> Option<ObjectType> {
        match group {
            TagGroup::Biped => Some(ObjectType::Biped),
            TagGroup::Vehicle => Some(ObjectType::Vehicle),
            TagGroup::Weapon => Some(ObjectType::Weapon),
            TagGroup::Equipment => Some(ObjectType::Equipment),
            TagGroup::Garbage => Some(ObjectType::Garbage),
            TagGroup::Projectile => Some(ObjectType::Projectile),
            TagGroup::Scenery => Some(ObjectType::Scenery),
            TagGroup::DeviceMachine => Some(ObjectType::DeviceMachine),
            TagGroup::DeviceControl => Some(ObjectType::DeviceControl),
            TagGroup::DeviceLightFixture => Some(ObjectType::DeviceLightFixture),
            TagGroup::Placeholder => Some(ObjectType::Placeholder),
            TagGroup::SoundScenery => Some(ObjectType::SoundScenery),
            group => None
        }
    }

    /// Return the shader type of the tag group.
    #[inline]
    pub fn shader_type_of_tag_group(group: TagGroup) -> Option<(Option<ShaderTypeXbox>, ShaderTypePC)> {
        match group {
            TagGroup::ShaderEnvironment => Some((Some(ShaderTypeXbox::Environment), ShaderTypePC::Environment)),
            TagGroup::ShaderModel => Some((Some(ShaderTypeXbox::Model), ShaderTypePC::Model)),
            TagGroup::ShaderTransparentGeneric => Some((Some(ShaderTypeXbox::TransparentGeneric), ShaderTypePC::TransparentGeneric)),
            TagGroup::ShaderTransparentChicago => Some((Some(ShaderTypeXbox::TransparentChicago), ShaderTypePC::TransparentChicago)),
            TagGroup::ShaderTransparentChicagoExtended => Some((None, ShaderTypePC::TransparentChicagoExtended)),
            TagGroup::ShaderTransparentGlass => Some((Some(ShaderTypeXbox::TransparentGlass), ShaderTypePC::TransparentGlass)),
            TagGroup::ShaderTransparentMeter => Some((Some(ShaderTypeXbox::TransparentMeter), ShaderTypePC::TransparentMeter)),
            TagGroup::ShaderTransparentPlasma => Some((Some(ShaderTypeXbox::TransparentPlasma), ShaderTypePC::TransparentPlasma)),
            TagGroup::ShaderTransparentWater => Some((Some(ShaderTypeXbox::TransparentWater), ShaderTypePC::TransparentWater)),
            _ => None,
        }
    }
}

/// Contains all engine definitions.
pub mod engine {
    use core::ops::RangeInclusive;
    use crate::{Parameters, Strictness};
    use crate::definitions::{FOOT_FOURCC, FOOT_FOURCC_OBFUSCATED, HEAD_FOURCC, HEAD_FOURCC_OBFUSCATED};
    use crate::definitions::tag::cache::CacheFileHeader;
    use crate::definitions::tag::scenario::ScenarioType;

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
        pub compressed_data_alignment: usize,
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
        pub is_custom: bool,

        pub grenade_limits: EngineGrenadeLimits,
        pub minimum_weapons: usize
    }

    #[derive(Debug)]
    pub struct EngineGrenadeLimits {
        pub singleplayer: RangeInclusive<u8>,
        pub multiplayer: RangeInclusive<u8>,
        pub user_interface: RangeInclusive<u8>
    }

    impl EngineGrenadeLimits {
        #[inline]
        pub fn get_limit_for_scenario(&self, scenario_type: ScenarioType) -> RangeInclusive<u8> {
            match scenario_type {
                ScenarioType::Singleplayer => self.singleplayer.clone(),
                ScenarioType::Multiplayer => self.multiplayer.clone(),
                ScenarioType::UserInterface => self.user_interface.clone()
            }
        }
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

    #[derive(Copy, Clone, Debug)]
    pub struct EngineSupportedResourceMaps {
        pub externally_indexed_tags: bool
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
