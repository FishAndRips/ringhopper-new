use core::cmp::Ordering;
use alloc::collections::VecDeque;
use funnel_web::id::Index;
use funnel_web::string::String32;
use crate::definitions::tag::gbxmodel::GBXModel;
use crate::definitions::tag::model::{Model, ModelDetailNodeCount, ModelMarker, ModelMarkerInstance, ModelRegionPermutationMarker};
use crate::postprocess::Action;
use crate::{ModelFns, PostprocessError, PostprocessState, TagPath};
use funnel_web::vector::{Matrix4x3, Vector3D};
use tinyvec::TinyVec;
use crate::util::launder_reference_lifetime_mut;

pub(crate) fn postprocess_gbxmodel(model: &mut GBXModel, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    postprocess_model_fns(model, action, tag_path, state)?;
    Ok(())
}

pub(crate) fn postprocess_model(model: &mut Model, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    postprocess_model_fns(model, action, tag_path, state)?;
    Ok(())
}

fn postprocess_model_fns<M: ModelFns + 'static>(model: &mut M, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    // This step should be done first
    verify_vertices_present(model, action, state)?;

    generate_node_matrices(model, action)?;
    null_out_part_indices(model, action);
    blend_shared_normals(model, action);
    flip_lod_cutoffs(model, action);
    compute_centroid_node_indices(model, action)?;
    move_markers(model, action)?;
    calculate_node_counts(model, action);

    // TODO: we need to put all vertices in their pointers rather than being tag data. can't do that
    //       in here because we can't just allocate stuff and put them on the tag data structs, so
    //       this will need to be a map compilation step
    Ok(())
}

fn calculate_node_counts<M: ModelFns + 'static>(model: &mut M, action: Action) {
    if !action.postprocess() {
        return
    }

    let mut node_count = ModelDetailNodeCount::default();
    for lod in 0..5 {
        // not technically a node count but just the highest node index (although it is probably
        // correct)
        let mut highest_node_index = 0;

        for region in model.regions() {
            for permutation in &region.permutations {
                let Some(geometry) = permutation.get_geometry_index_for_lod_index(lod) else {
                    continue;
                };
                for part in model.parts_iter().filter(|i| i.geometry_index == geometry) {
                    for vertex in &part.part.uncompressed_vertices {
                        highest_node_index = highest_node_index
                            .max(vertex.node0_index.index().unwrap_or(0))
                            .max(vertex.node1_index.index().unwrap_or(0));
                    }
                }
            }
        }

        *node_count.get_node_count_mut(lod).expect("0..5 should match all nodes") = highest_node_index as u16;
    }
}

fn move_markers<M: ModelFns + 'static>(model: &mut M, action: Action) -> Result<(), PostprocessError> {
    if action.postprocess() {
        if !model.runtime_markers().is_empty() {
            // This should be stripped from loose tags, but we don't do it because of inaccurate tag
            // extractors leaving them in here, or memelords editing tags directly in "expert mode"
            // Guerilla, or even official MCC tags (the moa) having them set here.
            //
            // It should be noted that having runtime markers defined in loose tags is completely
            // broken, and re-importing tags with runtime markers pre-defined won't remove them
            // (stale markers) and, when building the map, tool.exe will keep both sets of markers.
            //
            // Suffice it is to say: We don't support these tags. Just bludgeon them!
            fail_postprocess!("Model already has runtime markers defined. This tag needs bludgeoned!");
        }

        let mut runtime_markers = alloc::collections::BTreeMap::<[u8; 32], ModelMarker>::new();
        let node_count = model.nodes().len();

        for (r, region) in model.regions().iter().enumerate() {
            for (p, permutation) in region.permutations.iter().enumerate() {
                if permutation.markers.is_empty() {
                    continue
                }

                if p > 127 || r > 127 || node_count > 127 {
                    fail_postprocess!("Can't add markers from permutation/region index >127 or node index >127")
                }

                for marker in &permutation.markers {
                    let runtime_marker_set = loop {
                        let name = *permutation.name.bytes();
                        let mut name_case_insensitive = name;
                        name_case_insensitive.make_ascii_lowercase();

                        match runtime_markers.get_mut(&name_case_insensitive) {
                            Some(n) => break n,
                            None => {
                                runtime_markers.insert(name_case_insensitive, ModelMarker {
                                    name: String32::from_bytes(name_case_insensitive).expect(";-;"),
                                    ..ModelMarker::default()
                                });
                            }
                        }
                    };

                    runtime_marker_set.instances.push(ModelMarkerInstance {
                        region_index: r as u8,
                        permutation_index: p as u8,
                        node_index: marker.node_index.0.0 as u8,
                        translation: marker.translation,
                        rotation: marker.rotation,
                    })
                }
            }
        }

        let runtime_markers_output = model.runtime_markers_mut();
        for mut m in runtime_markers.into_iter().map(|i| i.1) {
            m.instances.sort_by(|a, b| {
                let region = a.region_index.cmp(&b.region_index);
                if region != Ordering::Equal {
                    return region
                }

                let permutation = a.permutation_index.cmp(&b.permutation_index);
                if permutation != Ordering::Equal {
                    return permutation
                }

                a.node_index.cmp(&b.node_index)
            });

            runtime_markers_output.push(m);
        }
    }
    else if action.unpostprocess() {
        // This may not result in the same order as the original tag file, but it should still
        // result in the same map.
        for m in core::mem::take(model.runtime_markers_mut()) {
            for (i, instance) in m.instances.into_iter().enumerate() {
                let Some(r) = model.regions_mut().get_mut(instance.region_index as usize) else {
                    fail_postprocess!("Can't unpostprocess model because runtime marker #{i} of {} has an invalid region", m.name)
                };
                let Some(p) = r.permutations.get_mut(instance.permutation_index as usize) else {
                    fail_postprocess!("Can't unpostprocess model because runtime marker #{i} of {} has an invalid permutation", m.name)
                };
                p.markers.push(ModelRegionPermutationMarker {
                    name: m.name,
                    node_index: if instance.node_index == 0xFF { Index::new() } else { Index(instance.node_index as u16) }.into(),
                    rotation: instance.rotation,
                    translation: instance.translation,
                });
            }
        }
    }

    Ok(())
}

fn compute_centroid_node_indices<M: ModelFns + 'static>(model: &mut M, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let mut all_weights = TinyVec::<[f32; 64]>::new();
    let node_count = model.nodes().len();

    for m in model.parts_iter_mut() {
        all_weights.clear();
        all_weights.resize(node_count, 0.0);

        m.part.centroid_primary_node = Index(0);
        m.part.centroid_secondary_node = Index(0);
        m.part.centroid_primary_weight = 0.0;
        m.part.centroid_secondary_weight = 0.0;

        for v in &m.part.uncompressed_vertices {
            for (n, weight) in [(v.node0_index.index(), v.node0_weight), (v.node1_index.index(), v.node1_weight)] {
                if let Some(n) = n {
                    if weight < 0.0 {
                        // prevent division by 0 (tool.exe doesn't do this lol)
                        fail_postprocess!("Vertex contains a negative weight {weight}");
                    }
                    let Some(w) = all_weights.get_mut(n) else {
                        fail_postprocess!("Vertex contains an invalid node index #{n}");
                    };
                    *w += weight;
                }
            }
        }

        for (n, weight) in all_weights.iter().copied().enumerate() {
            if weight > m.part.centroid_primary_weight {
                m.part.centroid_secondary_weight = m.part.centroid_primary_weight;
                m.part.centroid_secondary_node = m.part.centroid_primary_node;
                m.part.centroid_primary_weight = weight;
                m.part.centroid_primary_node = Index(n as u16);
            }
            else if weight > m.part.centroid_secondary_weight {
                m.part.centroid_secondary_weight = weight;
                m.part.centroid_secondary_node = Index(n as u16);
            }
        }

        if m.part.centroid_primary_weight != 0.0 {
            let sum = m.part.centroid_primary_weight + m.part.centroid_secondary_weight;
            m.part.centroid_primary_weight /= sum;
            m.part.centroid_secondary_weight /= sum;
        }
        else {
            m.part.centroid_primary_weight = 1.0;
        }
    }

    Ok(())
}

fn flip_lod_cutoffs<M: ModelFns + 'static>(model: &mut M, action: Action) {
    if !action.postprocess() && !action.unpostprocess() {
        return
    }

    let cutoffs = model.detail_cutoff_mut();
    core::mem::swap(&mut cutoffs.super_high, &mut cutoffs.super_low);
    core::mem::swap(&mut cutoffs.high, &mut cutoffs.low);
}

fn generate_node_matrices<M: ModelFns + 'static>(model: &mut M, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let nodes = model.nodes_mut();
    if nodes.is_empty() {
        fail_postprocess!("No nodes present in model!");
    }

    for (n, node) in nodes.iter_mut().enumerate() {
        if !node.default_translation.is_valid() {
            fail_postprocess!("Invalid translation for node #{n}");
        }
        if !node.default_rotation.is_valid() {
            fail_postprocess!("Invalid translation for node #{n}");
        }
        node.default_matrix = Matrix4x3::from_point_and_quaternion(node.default_translation, node.default_rotation);
    }

    let mut is_multiplied = TinyVec::<[bool; 64]>::with_capacity(nodes.len());
    is_multiplied.resize(nodes.len(), false);

    let mut node_indices = VecDeque::<usize>::with_capacity(nodes.len());
    node_indices.push_back(0); // push the base node

    while let Some(node_index) = node_indices.pop_front() {
        match is_multiplied.get_mut(node_index) {
            Some(true) => fail_postprocess!("Node #{node_index} is referenced more than once"),
            Some(n) => *n = true,
            None => fail_postprocess!("Node #{node_index} is not valid but is referenced")
        }

        // SAFETY: This reference won't leave here. We need to do this to also get the parent node
        //         without copying everything.
        let node = unsafe { launder_reference_lifetime_mut(&mut nodes[node_index]) };

        if node_index != 0 {
            let Some(parent) = node.parent_node_index.index() else {
                fail_postprocess!("Node #{node_index} is not the first node but is orphaned");
            };
            if is_multiplied.get(parent) != Some(&true) {
                fail_postprocess!("Node #{node_index} has a parent #{parent} that hasn't been processed yet");
            }
            let parent_node = nodes.get(parent).expect("Parent node doesn't exist, yet we modified it?");

            // it's a transformation of the parent node's transformation matrix
            node.default_matrix = parent_node.default_matrix * node.default_matrix;
        }
        else if !node.parent_node_index.is_null() {
            fail_postprocess!("The base node (#{node_index}) has parents nodes (this is invalid - there must be exactly one base, root node)");
        }

        if let Some(next_sibling) = node.next_sibling_node_index.index() {
            if node_index == 0 {
                fail_postprocess!("The base node (#{node_index}) has sibling nodes (this is invalid - there must be exactly one base, root node)");
            }
            node_indices.push_front(next_sibling);
        }
        if let Some(first_child) = node.first_child_node_index.index() {
            node_indices.push_front(first_child);
        }
    }

    if let Some(unmodified) = is_multiplied.iter().position(|i| *i == false) {
        fail_postprocess!("Node #{unmodified} did not have node matrices generated (no parent node?)")
    }

    for node in nodes {
        node.default_matrix = node.default_matrix.inverted();
    }

    Ok(())
}

fn verify_vertices_present<M: ModelFns + 'static>(model: &mut M, action: Action, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }
    let needs_compressed = state.engine().uses_compressed_models;
    for m in model.parts_iter() {
        if m.part.uncompressed_vertices.is_empty() {
            fail_postprocess!("Part #{} of geometry #{} is missing uncompressed vertices", m.part_index, m.geometry_index);
        }
        if !m.part.compressed_vertices.is_empty() && m.part.compressed_vertices.len() != m.part.uncompressed_vertices.len() {
            fail_postprocess!("Part #{} of geometry #{} has a mismatched compressed vertices count", m.part_index, m.geometry_index);
        }
        if needs_compressed && m.part.compressed_vertices.is_empty() {
            // TODO: Make a function to generate compressed vertices on the fly here
            fail_postprocess!("Part #{} of geometry #{} is missing compressed vertices (required by the target engine)", m.part_index, m.geometry_index);
        }
    }
    Ok(())
}

// A hack to make some models look better by memeing their normals up
//
// Useful for when you fucked up making your models and don't want to fix them.
//
// This is required for quite a few stock tags, apparently.
fn blend_shared_normals<M: ModelFns + 'static>(model: &mut M, action: Action) {
    if !model.get_blend_shared_normals() {
        return
    }

    // Prevent further floating point destruction.
    //
    // A lot of extraction tools don't do this, and this can result in slight generational loss.
    if action.postprocess() && action.unpostprocess() {
        model.set_blend_shared_normals(false);
    }

    if !action.postprocess() {
        // Blending shared normals cannot be undone, and we only want to do it when postprocessing.
        return
    }

    // This will be used for checking what vertices are "close enough" to be blended.
    //
    // This is a bit odd, as we'd probably want vertices to be exact, but as this is anyway a very
    // terrible hack, the jank must stay as-is.
    let dimensions = model.compute_dimensions();
    let scaled_dimensions = dimensions * 0.001;

    let regions = model.regions();

    // An implementation difference from tool.exe: tool.exe uses a stack-allocated array and errors
    // after 256 entries.
    //
    // We don't expect this to happen in most tags, but there is no point crashing, either, as the
    // game isn't going to care about the blend shared normals flag after postprocessing. Luckily we
    // have this fancy TinyVec crate.
    let mut matching_vertices_normals: TinyVec<[*mut Vector3D; 256]> = TinyVec::new();

    // We have to do it this way because floating point operations are non-commutative
    //
    // Blending is weighted per-region
    for lod in 0..5 {
        for (r1, region1) in regions.iter().enumerate() {
            for (p1, permutation1) in region1.permutations.iter().enumerate() {
                let Some(g1) = permutation1.get_geometry_index_for_lod_index(lod) else {
                    continue;
                };

                for part1 in model.parts_iter().filter(|i| i.geometry_index == g1) {
                    for (v1, vertex1) in part1.part.uncompressed_vertices.iter().enumerate() {
                        matching_vertices_normals.clear();
                        let mut normal = Vector3D::ZEROED;

                        for (r2, region2) in regions.iter().enumerate().skip(r1) {
                            let is_same_region = r2 == r1;
                            let mut region_normal = Vector3D::ZEROED;

                            for (p2, permutation2) in region2.permutations.iter().enumerate().skip(if is_same_region { p1 } else { 0 }) {
                                let is_same_permutation = is_same_region && p2 == p1;

                                let Some(g2) = permutation2.get_geometry_index_for_lod_index(lod) else {
                                    continue
                                };

                                for part2 in model.parts_iter().filter(|i| i.geometry_index == g2).skip_while(|i| is_same_permutation && i.part_index < part1.part_index) {
                                    let is_same_part = is_same_permutation && part1.part_index == part2.part_index;

                                    for vertex2 in part2.part.uncompressed_vertices.iter().skip(if is_same_part { v1 } else { 0 }) {
                                        let difference = vertex2.position - vertex1.position;

                                        if difference.x > scaled_dimensions.x { continue }
                                        if difference.y > scaled_dimensions.y { continue }
                                        if difference.z > scaled_dimensions.z { continue }

                                        let normal = &vertex2.normal;

                                        // A bit of a possible oversight: We don't check if we've added this vertex previously.
                                        //
                                        // If the model has fewer than five LoDs, then we may blend the same vertex multiple times,
                                        // resulting in some possibly weird weighting.
                                        //
                                        // Again, this function is horrible, so the jank must stay.

                                        region_normal += *normal;
                                        matching_vertices_normals.push(normal as *const Vector3D as *mut Vector3D);
                                    }
                                }
                            }

                            // OK if normalized() fails (it can fail in tool.exe)
                            normal += region_normal.normalized().unwrap_or(region_normal);
                        }

                        let magnitude = normal.magnitude();
                        if magnitude > 0.0 {
                            continue
                        }

                        let Some(blended_normal) = normal.normalized() else {
                            continue;
                        };

                        for i in &matching_vertices_normals {
                            // SAFETY: The model is only being mutably accessed in this one function, and
                            // we've not modified anything when getting these pointers, so they should still
                            // be valid. As such, this *shouldn't* lead to any UB.
                            //
                            // Unfortunately Rust has no way of knowing, plus it is possible for the same
                            // pointer to be stored multiple times. The "safe" way would probably result
                            // in multiple slow loops.
                            //
                            // Not great, but what can you do?
                            unsafe { **i = blended_normal };
                        }
                    }
                }
            }
        }
    }

    // Recompress
    //
    // tool.exe does NOT do this, thus everything we did would've been completely discarded on Xbox,
    // one of many tool.exe bugs
    for m in model.parts_iter_mut() {
        let uncompressed_vertices = &m.part.uncompressed_vertices;
        let compressed_vertices = &mut m.part.compressed_vertices;

        for (uncompressed, compressed) in uncompressed_vertices.iter().zip(compressed_vertices.iter_mut()) {
            compressed.normal = uncompressed.normal.compress();
        }
    }
}

fn null_out_part_indices(model: &mut impl ModelFns, action: Action) {
    // FIXME: Do not call this "filthy part index" but just "part index".

    let from;
    let to;

    // Older builds of tool.exe null each value individually.
    //
    // The tool.exe for the HCEAEK was fixed to do it if both are zero. However, this still does
    // not actually work because the renderer was never fixed to handle it correctly.
    if action.postprocess() {
        from = 0;
        to = u8::MAX;
    }
    else if action.unpostprocess() {
        from = u8::MAX;
        to = 0;
    }
    else {
        return
    }

    for part in model.parts_iter_mut() {
        let part = part.part;
        if part.next_filthy_part_index == from && part.prev_filthy_part_index == from {
            part.next_filthy_part_index = to;
            part.prev_filthy_part_index = to;
        }
    }
}
