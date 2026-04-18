use crate::definitions::tag::detail_object_collection::DetailObjectCollection;
use crate::definitions::tag::enums::MaterialType;
use crate::definitions::tag::fog::Fog;
use crate::definitions::tag::scenario::{Scenario, ScenarioDecal};
use crate::definitions::tag::scenario_structure_bsp::{ScenarioStructureBSP, ScenarioStructureBSPDetailObjectData, ScenarioStructureBSPPathfindingSurface, ScenarioStructureBSPRuntimeDecal};
use crate::definitions::tag::shader::Shader;
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, Reflexive, TagPath};
use alloc::vec::Vec;
use core::cmp::Ordering;
use funnel_web::collision_bsp::CollisionBSPFunctions;
use funnel_web::constants::DETAIL_OBJECT_WORLD_UNITS_PER_CELL;
use funnel_web::float::FloatOps;
use funnel_web::id::Index;
use funnel_web::vector::{Angle, Euler2D, Vector3D};
use tinyvec::ArrayVec;

pub fn postprocess_scenario_structure_bsp(bsp: &mut ScenarioStructureBSP, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        do_postprocess_scenario_structure_bsp(bsp, state)?;
    }
    else if action.unpostprocess() {
        unpostprocess_bsp_detail_object_data(bsp, state)
    }

    Ok(())
}

fn do_postprocess_scenario_structure_bsp(bsp: &mut ScenarioStructureBSP, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    check_all_indices(bsp)?;
    postprocess_bsp_collision_materials(bsp, state)?;
    postprocess_bsp_detail_object_data(bsp, state)?;
    postprocess_bsp_fog_plane_material(bsp, state)?;
    postprocess_bsp_decals(bsp, state)?;
    build_pathfinding_surfaces(bsp);
    Ok(())
}

fn check_all_indices(bsp: &ScenarioStructureBSP) -> Result<(), PostprocessError> {
    assert_postprocess!(bsp.collision_bsp.len() == 1, "BSP does not have exactly one collision BSP.");

    let Some(collision_bsp) = bsp.collision_bsp.get(0) else {
        fail_postprocess!("No collision BSP present in BSP.");
    };

    if let Err(e) = collision_bsp.bounds_check() {
        fail_postprocess!("Collision BSP contains errors: {e}");
    }

    // TODO: The rest of the checks should be automated.

    Ok(())
}

fn postprocess_bsp_collision_materials(bsp: &mut ScenarioStructureBSP, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for (material_index, material) in bsp.collision_materials.iter_mut().enumerate() {
        let Some(material_shader) = material.shader.get() else {
            fail_postprocess!("Material #{material_index} has no shader set");
        };

        let shader = state.read_tag_group::<Shader>(material_shader).expect("not a shader");
        material.material = shader.physics.material_type;
    }
    Ok(())
}

fn postprocess_bsp_detail_object_data(bsp: &mut ScenarioStructureBSP, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if bsp.detail_objects.is_empty() {
        bsp.detail_objects.push(Default::default());
    }

    assert_postprocess!(bsp.detail_objects.len() <= 1, "BSPs can only contain at most one set of detail object data.");

    let scenario_tag = state.scenario_tag();
    let detail_data = &mut bsp.detail_objects[0];

    let has_valid_data = !scenario_tag.detail_object_collection_palette.is_empty()
        && !detail_data.cells.is_empty()
        && !detail_data.counts.is_empty()
        && !detail_data.instances.is_empty();

    if !has_valid_data {
        if !detail_data.cells.is_empty() || !detail_data.counts.is_empty() || !detail_data.instances.is_empty() {
            if scenario_tag.detail_object_collection_palette.is_empty() {
                fail_postprocess!("Detail object data is defined, but no palette is present on the scenario");
            }
            else {
                fail_postprocess!("Missing some detail object data");
            }
        }
        else {
            return Ok(())
        }
    };

    detail_data.bullshit = 1;

    if detail_data.z_reference_vectors.is_empty() {
        return Ok(())
    }

    modify_z_reference_vectors(detail_data, state, false)?;

    Ok(())
}

fn unpostprocess_bsp_detail_object_data(bsp: &mut ScenarioStructureBSP, state: &mut dyn PostprocessState) {
    for i in &mut bsp.detail_objects {
        modify_z_reference_vectors(i, state, false)
            .expect("modify_z_reference_vectors should be infallible when unpostprocessing");
    }
}

fn modify_z_reference_vectors(
    detail_data: &mut ScenarioStructureBSPDetailObjectData,
    state: &dyn PostprocessState,
    unpostprocess: bool
) -> Result<(), PostprocessError> {
    if detail_data.z_reference_vectors.is_empty() {
        return Ok(())
    }

    let scenario_tag = state.scenario_tag();
    let offsets = get_detail_object_z_offsets(scenario_tag, state);

    for (cell_index, cell) in detail_data.cells.iter().enumerate() {
        let mut count_offset = 0;
        for (layer_index, offset) in offsets.iter().copied().enumerate().take(MAX_DETAIL_OBJECT_LAYERS) {
            let Some(offset) = offset else {
                continue
            };
            if cell.valid_layers_flags & (1u32 >> layer_index) == 0 {
                continue
            }
            let Some(vector_index) = (cell.count_index as usize)
                .checked_add(count_offset) else {
                if unpostprocess {
                    continue
                }
                else {
                    fail_postprocess!("Layer #{layer_index} of cell #{cell_index} has an out-of-bounds vector index that overflows usize");
                }
            };

            let Some(vector) = detail_data.z_reference_vectors.get_mut(vector_index) else {
                if unpostprocess {
                    continue;
                }
                else {
                    fail_postprocess!("Layer #{layer_index} of cell #{cell_index} has an out-of-bounds vector index {vector_index}");
                }
            };
            vector.z_reference_l -= offset;
            count_offset += 1;
        }
    }

    Ok(())
}

const MAX_DETAIL_OBJECT_LAYERS: usize = 32;

fn get_detail_object_z_offsets(scenario: &Scenario, state: &dyn PostprocessState) -> ArrayVec<[Option<f32>; MAX_DETAIL_OBJECT_LAYERS]> {
    let mut offsets = ArrayVec::new();
    for detail_object in scenario.detail_object_collection_palette.iter().take(MAX_DETAIL_OBJECT_LAYERS) {
        let Some(d) = detail_object.reference.get() else {
            offsets.push(None);
            continue
        };
        let detail_object = state.read_tag_group::<DetailObjectCollection>(d).expect("not a detail object collection");
        offsets.push(Some(detail_object.global_z_offset / DETAIL_OBJECT_WORLD_UNITS_PER_CELL));
    }
    offsets
}

fn postprocess_bsp_fog_plane_material(bsp: &mut ScenarioStructureBSP, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for plane in &mut bsp.fog_planes {
        plane.material_type = None;

        let region = bsp.fog_regions[plane.front_region.index().expect("bsp fog plane region was null")];
        let Some(fog) = region.fog.index() else {
            continue
        };
        let fog = &bsp.fog_palette[fog];
        let Some(fog_path) = fog.fog.get() else {
            continue
        };
        let fog_tag = state.read_tag_group::<Fog>(fog_path).expect("not a fog tag");

        if fog_tag.flags.is_water {
            plane.material_type = Some(MaterialType::Water);
        }
    }
    Ok(())
}

fn postprocess_bsp_decals(bsp: &mut ScenarioStructureBSP, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    // First we need to initialize the cluster indices
    for cluster in &mut bsp.clusters {
        cluster.decal_count = 0;
        cluster.first_decal_index = Index::new();
    }

    // Next, do we need to actually do anything?
    let scenario = state.scenario_tag();
    if scenario.decals.is_empty() {
        return Ok(())
    }

    // Sort!
    let mut decals_sorted = sort_decals(bsp, &scenario.decals);
    decals_sorted.retain(|decal| {
        // No cluster index
        if !decal.cluster_index.is_null() {
            return false
        }

        // Skip decals that are null
        let Some(decal_palette_index) = decal.adjusted_decal.decal_type.index() else {
            return false
        };
        let Some(decal_type) = scenario.decal_palette.get(decal_palette_index) else {
            // can't trust the scenario tag yet as it hasn't been postprocessed
            return false
        };
        if decal_type.reference.is_unset() {
            return false
        }

        true
    });

    // Check if it's within limits
    let decal_limit = 6 * 1024;
    assert_postprocess!(
        decals_sorted.len() <= decal_limit,
        "BSP exceeds the maximum number of decals ({} > {decal_limit})", decals_sorted.len()
    );

    let mut runtime_decals = Vec::with_capacity(decals_sorted.len());
    runtime_decals.extend(decals_sorted.iter().map(|i| ScenarioStructureBSPRuntimeDecal{
        position: i.adjusted_decal.position,
        yaw: i.adjusted_decal.yaw,
        pitch: i.adjusted_decal.pitch,
        decal_type: i.adjusted_decal.decal_type.0
    }));
    bsp.runtime_decals = Reflexive::with_vec(runtime_decals);

    // No decals in this BSP
    if bsp.runtime_decals.is_empty() {
        return Ok(())
    }

    let mut current_cluster = None;
    let mut first_decal_index = None;

    let mut snip_cluster = |current_cluster: Option<usize>, first_decal_index: Option<Index>, first_next_decal_index: usize| {
        if let Some(first_cluster) = current_cluster {
            let first_decal_index = first_decal_index
                .expect("no first decal index specified, but current cluster was specified");

            let first_decal_index_usize = first_decal_index
                .index()
                .expect("first decal index was None somehow");

            let decal_count = u16::try_from(
                first_next_decal_index
                    .checked_sub(first_decal_index_usize)
                    .expect("first_next_decal_index minus first_decal_index_usize underflowed")
            ).expect("we have too many decals somehow");

            let cluster = &mut bsp.clusters[first_cluster];
            cluster.decal_count = decal_count;
            cluster.first_decal_index = first_decal_index;
        }
    };

    for (decal_index, decal) in decals_sorted.iter().enumerate() {
        let cluster_index = decal.cluster_index.index().expect("first_cluster was None somehow even though we excluded it");

        if current_cluster != Some(cluster_index) {
            snip_cluster(current_cluster, first_decal_index, decal_index);
            first_decal_index = Some(Index::from_usize(decal_index).expect("decal count was checked"));
            current_cluster = Some(cluster_index);
        }
    }

    snip_cluster(current_cluster, first_decal_index, decals_sorted.len());
    Ok(())
}

fn build_pathfinding_surfaces(bsp: &mut ScenarioStructureBSP) {
    let collision_model = &bsp.collision_bsp[0];
    let mut pathfinding_surfaces: Vec<ScenarioStructureBSPPathfindingSurface> = Vec::with_capacity(bsp.surfaces.len());

    for surface in &collision_model.surfaces {
        let plane = &collision_model.planes[surface.plane as usize];

        let mut data = 0u8;
        if plane.plane.vector.z >= 0.71 {
            data = 0x40 | if surface.flags.breakable { 0x80 } else { 0x00 };
        }

        pathfinding_surfaces.push(ScenarioStructureBSPPathfindingSurface { data });
    }

    bsp.pathfinding_surfaces = Reflexive::with_vec(pathfinding_surfaces);
}

fn apply_decal_offset(decal: &ScenarioDecal) -> Vector3D {
    let euler_angle = Euler2D {
        yaw: Angle((decal.yaw as f32) * (f32::FW_PI / 127.0)),
        pitch: Angle((decal.pitch as f32) * (f32::FW_PI / 127.0)),
    };

    decal.position.apply_offset(euler_angle.as_vector(), 0.1)
}

struct SortedDecal {
    cluster_index: Index,
    adjusted_decal: ScenarioDecal
}

fn sort_decals(bsp: &ScenarioStructureBSP, reflexive: &Reflexive<ScenarioDecal>) -> Vec<SortedDecal> {
    let collision_bsp = &bsp.collision_bsp[0];
    let mut sorted = Vec::with_capacity(reflexive.len());

    for i in reflexive {
        let offset = apply_decal_offset(i);
        let cluster_index = collision_bsp.leaf_index_for_point_3d(offset)
            .ok()
            .flatten()
            .and_then(|i| bsp.leaves.get(i))
            .map(|i| i.cluster)
            .unwrap_or_default();

        sorted.push(SortedDecal {
            cluster_index,
            adjusted_decal: ScenarioDecal {
                position: offset,
                ..*i
            }
        });
    }

    // note: the original implementation uses qsort (introsort on MSVC) and just compares cluster
    // indices; this may result in a different order from Rust's driftsort when cluster indices are
    // equal but the rest of the decal is not
    //
    // as such, we compare the entire decal if the cluster indices are not the same, as while this
    // won't match tool.exe, it will always consistently sort into the same thing

    sorted.sort_by(|l,r| {
        match l.cluster_index.cmp(&r.cluster_index) {
            Ordering::Equal => compare_scenario_decal(&l.adjusted_decal, &r.adjusted_decal),
            otherwise => otherwise
        }
    });

    sorted
}

fn compare_scenario_decal(a: &ScenarioDecal, b: &ScenarioDecal) -> Ordering {
    // tool.exe doesn't do any extra comparisons here, but we want to ensure that maps built by
    // ringhopper will have the same decal order on build regardless of how tool.exe shuffled it

    match a.decal_type.cmp(&b.decal_type) {
        Ordering::Equal => (),
        other => return other
    };
    match a.position.z.total_cmp(&b.position.z) {
        Ordering::Equal => (),
        other => return other
    };
    match a.position.y.total_cmp(&b.position.y) {
        Ordering::Equal => (),
        other => return other
    };
    match a.position.x.total_cmp(&b.position.x) {
        Ordering::Equal => (),
        other => return other
    };
    match a.yaw.cmp(&b.yaw) {
        Ordering::Equal => (),
        other => return other
    };
    a.pitch.cmp(&b.pitch)
}
