use funnel_web::collision_bsp::{BSP2DNodeReference, CollisionBSP2DNode, CollisionBSP2DNodeIndex, CollisionBSP3DNode, CollisionBSP3DNodeIndex, CollisionBSPEdge, CollisionBSPFunctions, CollisionBSPLeaf, CollisionBSPSurface, CollisionBSPSurfaceFlags, CollisionBSPVertex};
use funnel_web::vector::Plane3D;
use crate::definitions::tag::model_collision_geometry::ModelCollisionGeometryBSP;

impl CollisionBSPFunctions for ModelCollisionGeometryBSP {
    fn get_3d_node(&self, node: usize) -> Option<CollisionBSP3DNode> {
        self.bsp3d_nodes.get(node).map(|i| CollisionBSP3DNode {
            plane_index: i.plane as usize,
            front_child: CollisionBSP3DNodeIndex(i.front_child),
            back_child: CollisionBSP3DNodeIndex(i.back_child),
        })
    }

    fn get_3d_node_count(&self) -> usize {
        self.bsp3d_nodes.len()
    }

    fn get_plane(&self, plane: usize) -> Option<Plane3D> {
        self.planes.get(plane).map(|i| i.plane)
    }

    fn get_plane_count(&self) -> usize {
        self.planes.len()
    }

    fn get_leaf(&self, leaf: usize) -> Option<CollisionBSPLeaf> {
        self.leaves.get(leaf).map(|i| CollisionBSPLeaf {
            contains_double_sided_surfaces: i.flags.contains_double_sided_surfaces,
            bsp_2d_node_reference_count: i.bsp2d_reference_count as usize,
            bsp_2d_node_reference_start: i.first_bsp2d_reference as usize
        })
    }

    fn get_leaf_count(&self) -> usize {
        self.leaves.len()
    }

    fn get_2d_node_reference(&self, node: usize) -> Option<BSP2DNodeReference> {
        self.bsp2d_references.get(node).map(|i| BSP2DNodeReference {
            plane: i.plane as usize,
            node: CollisionBSP2DNodeIndex(i.bsp2d_node)
        })
    }

    fn get_2d_node_reference_count(&self) -> usize {
        self.bsp2d_references.len()
    }

    fn get_2d_node(&self, node: usize) -> Option<CollisionBSP2DNode> {
        self.bsp2d_nodes.get(node).map(|i| CollisionBSP2DNode {
            plane: i.plane,
            left_child: CollisionBSP2DNodeIndex(i.left_child),
            right_child: CollisionBSP2DNodeIndex(i.right_child),
        })
    }

    fn get_2d_node_count(&self) -> usize {
        self.bsp2d_nodes.len()
    }

    fn get_surface(&self, surface: usize) -> Option<CollisionBSPSurface> {
        self.surfaces.get(surface).map(|i| CollisionBSPSurface {
            plane: i.plane as usize,
            first_edge: 0,
            flags: CollisionBSPSurfaceFlags {
                two_sided: i.flags.two_sided,
                invisible: i.flags.invisible,
                climbable: i.flags.climbable,
                breakable: i.flags.breakable,
            },
            material: i.material.0,
            breakable_surface_index: None,
        })
    }

    fn get_surface_count(&self) -> usize {
        self.surfaces.len()
    }

    fn get_edge(&self, edge: usize) -> Option<CollisionBSPEdge> {
        self.edges.get(edge).map(|v| CollisionBSPEdge {
            start_vertex: v.start_vertex as usize,
            end_vertex: v.end_vertex as usize,
            forward_edge: v.forward_edge as usize,
            reverse_edge: v.reverse_edge as usize,
            left_surface: v.left_surface as usize,
            right_surface: v.right_surface as usize,
        })
    }

    fn get_edge_count(&self) -> usize {
        self.edges.len()
    }

    fn get_vertex(&self, vertex: usize) -> Option<CollisionBSPVertex> {
        self.vertices.get(vertex).map(|v| CollisionBSPVertex {
            point: v.point,
            first_edge: v.first_edge as usize,
        })
    }

    fn get_vertex_count(&self) -> usize {
        self.vertices.len()
    }
}
