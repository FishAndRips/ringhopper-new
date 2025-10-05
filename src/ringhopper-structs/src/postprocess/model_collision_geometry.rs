use alloc::vec::Vec;
use funnel_web::constants::seconds_to_ticks;
use crate::postprocess::Action;
use crate::PostprocessError;
use crate::definitions::tag::model_collision_geometry::ModelCollisionGeometry;

const NODE_CATEGORY: &[&[&str]] = &[
    &["pelvis", "spine"],
    &["spine1"],
    &["head", "neck"],
    &["l clavicle", "l upperarm"],
    &["l forearm", "l hand"],
    &["l thigh"], // listed twice?
    &["l calf", "l foot", "l horselink", "l thigh"],
    &["r clavicle", "r upperarm"],
    &["r forearm", "r hand"],
    &["r thigh"], // listed twice?
    &["r calf", "r horselink", "r foot", "r thigh", "tail"]
];

pub fn postprocess_model_collision_geometry(model_collision_geometry: &mut ModelCollisionGeometry, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    if model_collision_geometry.recharge_time != 0.0 {
        model_collision_geometry.shield_recharge_rate = 1.0 / seconds_to_ticks(model_collision_geometry.recharge_time);
    }

    let mut node_indices: Vec<i16> = Vec::with_capacity(model_collision_geometry.nodes.len());

    for (first_node_index, first_node) in model_collision_geometry.nodes.iter().enumerate() {
        let mut infinite_loop_detector = 0;

        let mut node = first_node;
        let mut node_index = first_node_index;

        loop {
            if let Some(i) = NODE_CATEGORY.iter().position(|i| i.contains(&node.name.as_str())) {
                node_indices.push(i as i16);
                break;
            }

            let Some(next_parent) = node.parent_node.index() else {
                node_indices.push(-1);
                break;
            };

            if let Some(i) = node_indices.get(next_parent) {
                node_indices.push(*i);
                break;
            }

            node = match model_collision_geometry.nodes.get(next_parent) {
                Some(n) => n,
                None => fail_postprocess!("Node #{node_index} has an out-of-bounds parent index {next_parent}")
            };

            node_index = next_parent;

            infinite_loop_detector += 1;

            assert_postprocess!(
                infinite_loop_detector <= model_collision_geometry.nodes.len().saturating_add(1),
                "Infinite node loop detected (first node in loop is #{first_node_index})"
            );
        }
    }

    for (index, &category) in node_indices.iter().enumerate() {
        model_collision_geometry.nodes[index].name_thing = category;
    }

    Ok(())
}
