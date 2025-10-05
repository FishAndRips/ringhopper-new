use funnel_web::vector::{Vector2D, Vector3D};
use crate::definitions::tag::gbxmodel::GBXModel;
use crate::definitions::tag::model::{Model, ModelDetailCutoff, ModelDetailNodeCount, ModelGeometryPart, ModelMarker, ModelNode, ModelRegion, ModelShaderReference};
use crate::{Bounds, EditableTag, Reflexive};
use crate::util::launder_reference_lifetime_mut;

pub trait ModelFns {
    fn node_list_checksum(&self) -> i32;
    fn detail_cutoff(&self) -> &ModelDetailCutoff;
    fn detail_cutoff_mut(&mut self) -> &mut ModelDetailCutoff;
    fn detail_node_count(&self) -> &ModelDetailNodeCount;
    fn detail_node_count_mut(&mut self) -> &mut ModelDetailNodeCount;
    fn base_map_scale(&self) -> &Vector2D;
    fn base_map_scale_mut(&mut self) -> &mut Vector2D;
    fn runtime_markers(&self) -> &Reflexive<ModelMarker>;
    fn runtime_markers_mut(&mut self) -> &mut Reflexive<ModelMarker>;
    fn nodes(&self) -> &Reflexive<ModelNode>;
    fn nodes_mut(&mut self) -> &mut Reflexive<ModelNode>;
    fn regions(&self) -> &Reflexive<ModelRegion>;
    fn regions_mut(&mut self) -> &mut Reflexive<ModelRegion>;
    fn shaders(&self) -> &Reflexive<ModelShaderReference>;
    fn shaders_mut(&mut self) -> &mut Reflexive<ModelShaderReference>;
    fn parts_iter(&self) -> impl Iterator<Item=ModelPartRef<'_>> where Self: Sized;
    fn parts_iter_mut(&mut self) -> impl Iterator<Item=ModelPartMut<'_>> where Self: Sized;
    fn get_blend_shared_normals(&self) -> bool;
    fn set_blend_shared_normals(&mut self, blend_shared_normals: bool);
    fn compute_bounding_box(&self) -> Bounds<Vector3D>;
    #[inline]
    fn compute_dimensions(&self) -> Vector3D {
        let b = self.compute_bounding_box();
        b.to - b.from
    }
}

fn compute_bounding_box(model: &impl ModelFns) -> Bounds<Vector3D> {
    let mut iterator = model.parts_iter()
        .map(|i| i.part.uncompressed_vertices.iter())
        .flatten();

    let Some(first_vertex) = iterator.next() else { return Bounds::default() };

    // Use the first vertex as a baseline
    let mut bounds_from = first_vertex.position;
    let mut bounds_to = first_vertex.position;

    for i in iterator {
        let position = i.position;

        bounds_from.x = bounds_from.x.min(position.x);
        bounds_from.y = bounds_from.y.min(position.y);
        bounds_from.z = bounds_from.z.min(position.z);

        bounds_to.x = bounds_to.x.max(position.x);
        bounds_to.y = bounds_to.y.max(position.y);
        bounds_to.z = bounds_to.z.max(position.z);
    }

    Bounds { from: bounds_from, to: bounds_to }
}

#[expect(missing_docs)]
pub struct ModelPartRef<'a> {
    pub part: &'a ModelGeometryPart,
    pub geometry_index: usize,
    pub part_index: usize,
    local_nodes: Option<([u8; 22], usize)>
}

#[expect(missing_docs)]
pub struct ModelPartMut<'a> {
    pub part: &'a mut ModelGeometryPart,
    pub geometry_index: usize,
    pub part_index: usize,
    local_nodes: Option<([u8; 22], usize)>
}

impl ModelPartRef<'_> {
    /// Resolve a node index, accounting for local nodes.
    ///
    /// Returns an error if the model is invalid.
    pub fn resolve_node_index(&self, node: usize) -> Result<usize, &'static str> {
        let Some(s) = self.get_local_nodes()? else {
            return Ok(node)
        };
        s.get(node).map(|i| *i as usize).ok_or("local node index (ref) requested out-of-bounds")
    }
    fn get_local_nodes(&self) -> Result<Option<&[u8]>, &'static str> {
        let Some(nodes) = self.local_nodes.as_ref() else { return Ok(None) };
        nodes.0
            .get(..nodes.1)
            .ok_or("local nodes (mut) out of bounds (local node count > max length)")
            .map(Some)
    }
}

impl ModelPartMut<'_> {
    /// Resolve a node index, accounting for local nodes.
    ///
    /// Returns an error if the model is invalid.
    pub fn resolve_node_index(&self, node: usize) -> Result<usize, &'static str> {
        let Some(s) = self.get_local_nodes()? else {
            return Ok(node)
        };
        s.get(node).map(|i| *i as usize).ok_or("local node index (ref) requested out-of-bounds")
    }
    fn get_local_nodes(&self) -> Result<Option<&[u8]>, &'static str> {
        let Some(nodes) = self.local_nodes.as_ref() else { return Ok(None) };
        nodes.0
            .get(..nodes.1)
            .ok_or("local nodes (mut) out of bounds (local node count > max length)")
            .map(Some)
    }
}

macro_rules! generate_modelfns_prelude {
    () => {
        #[inline]
        fn node_list_checksum(&self) -> i32 {
            self.node_list_checksum
        }

        #[inline]
        fn detail_cutoff(&self) -> &ModelDetailCutoff {
            &self.detail_cutoff
        }

        #[inline]
        fn detail_cutoff_mut(&mut self) -> &mut ModelDetailCutoff {
            &mut self.detail_cutoff
        }

        #[inline]
        fn detail_node_count(&self) -> &ModelDetailNodeCount {
            &self.detail_node_count
        }

        #[inline]
        fn detail_node_count_mut(&mut self) -> &mut ModelDetailNodeCount {
            &mut self.detail_node_count
        }

        #[inline]
        fn base_map_scale(&self) -> &Vector2D {
            &self.base_map_scale
        }

        #[inline]
        fn base_map_scale_mut(&mut self) -> &mut Vector2D {
            &mut self.base_map_scale
        }

        #[inline]
        fn runtime_markers(&self) -> &Reflexive<ModelMarker> {
            &self.runtime_markers
        }

        #[inline]
        fn runtime_markers_mut(&mut self) -> &mut Reflexive<ModelMarker> {
            &mut self.runtime_markers
        }

        #[inline]
        fn nodes(&self) -> &Reflexive<ModelNode> {
            &self.nodes
        }

        #[inline]
        fn nodes_mut(&mut self) -> &mut Reflexive<ModelNode> {
            &mut self.nodes
        }

        #[inline]
        fn regions(&self) -> &Reflexive<ModelRegion> {
            &self.regions
        }

        #[inline]
        fn regions_mut(&mut self) -> &mut Reflexive<ModelRegion> {
            &mut self.regions
        }

        #[inline]
        fn shaders(&self) -> &Reflexive<ModelShaderReference> {
            &self.shaders
        }

        #[inline]
        fn shaders_mut(&mut self) -> &mut Reflexive<ModelShaderReference> {
            &mut self.shaders
        }

        #[inline]
        fn get_blend_shared_normals(&self) -> bool {
            self.flags.blend_shared_normals
        }

        #[inline]
        fn set_blend_shared_normals(&mut self, blend_shared_normals: bool) {
            self.flags.blend_shared_normals = blend_shared_normals;
        }

        #[inline]
        fn compute_bounding_box(&self) -> Bounds<Vector3D> {
            compute_bounding_box(self)
        }
    };
}

impl ModelFns for GBXModel {
    generate_modelfns_prelude!();

    fn parts_iter(&self) -> impl Iterator<Item=ModelPartRef<'_>> where Self: Sized {
        let has_local_nodes = self.flags.parts_have_local_nodes;

        self.geometries
            .iter()
            .enumerate()
            .map(move |(geometry_index, geometry)| geometry.parts.iter().enumerate()
                .map(move |(part_index, part)| {
                    ModelPartRef {
                        geometry_index,
                        part_index,
                        part: &part.model_geometry_part,
                        local_nodes: has_local_nodes.then_some((part.local_node_indices, part.local_node_count as usize))
                    }
                })
            )
            .flatten()
    }

    fn parts_iter_mut(&mut self) -> impl Iterator<Item=ModelPartMut<'_>> where Self: Sized {
        let has_local_nodes = self.flags.parts_have_local_nodes;

        self.geometries
            .iter_mut()
            .enumerate()
            .map(move |(geometry_index, geometry)| geometry.parts.iter_mut().enumerate()
                .map(move |(part_index, part)| {
                    ModelPartMut {
                        geometry_index,
                        part_index,
                        part: &mut part.model_geometry_part,
                        local_nodes: has_local_nodes.then_some((part.local_node_indices, part.local_node_count as usize))
                    }
                })
            )
            .flatten()
    }
}

impl ModelFns for Model {
    generate_modelfns_prelude!();

    fn parts_iter(&self) -> impl Iterator<Item=ModelPartRef<'_>> where Self: Sized {
        self.geometries
            .iter()
            .enumerate()
            .map(|(geometry_index, geometry)| geometry.parts.iter().enumerate()
                .map(move |(part_index, part)| {
                    ModelPartRef {
                        geometry_index,
                        part_index,
                        part,
                        local_nodes: None
                    }
                })
            )
            .flatten()
    }

    fn parts_iter_mut(&mut self) -> impl Iterator<Item=ModelPartMut<'_>> where Self: Sized {
        self.geometries
            .iter_mut()
            .enumerate()
            .map(|(geometry_index, geometry)| geometry.parts.iter_mut().enumerate()
                .map(move |(part_index, part)| {
                    ModelPartMut {
                        geometry_index,
                        part_index,
                        part,
                        local_nodes: None
                    }
                })
            )
            .flatten()
    }
}

impl dyn EditableTag {
    pub fn downcast_model(&self) -> Option<&dyn ModelFns> {
        if let Some(d) = self.downcast_ref::<Model>() {
            Some(d as &dyn ModelFns)
        }
        else if let Some(d) = self.downcast_ref::<GBXModel>() {
            Some(d as &dyn ModelFns)
        }
        else {
            None
        }
    }
    pub fn downcast_model_mut(&mut self) -> Option<&mut dyn ModelFns> {
        // SAFETY: We're not going to mutably access `self` more than once.
        if let Some(d) = unsafe { launder_reference_lifetime_mut(self) }.downcast_mut::<Model>() {
            Some(d as &mut dyn ModelFns)
        }
        else if let Some(d) = self.downcast_mut::<GBXModel>() {
            Some(d as &mut dyn ModelFns)
        }
        else {
            None
        }
    }
}
