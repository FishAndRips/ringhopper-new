use funnel_web::constants::seconds_to_ticks;
use funnel_web::float::FloatOps;
use funnel_web::id::Index;
use crate::definitions::tag::biped::Biped;
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, PostprocessWarningType, TagPath};
use crate::definitions::tag::model_animations::ModelAnimations;

pub fn postprocess_biped(biped: &mut Biped, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let Some(model_path) = biped.unit.object.model.get() else {
        fail_postprocess!("Biped tag is missing a model tag")
    };

    let Some(animation_path) = biped.unit.object.animation_graph.get() else {
        fail_postprocess!("Biped tag is missing an animation tag")
    };

    let model = state
        .read_model(model_path)
        .expect("failed to get model");

    let animation = state
        .read_tag_group::<ModelAnimations>(animation_path)
        .expect("not model animations");

    let find_node = |node: &str| -> Index {
        let index = model
            .nodes()
            .iter()
            .position(|i| i.name == node)
            .and_then(Index::from_usize)
            .unwrap_or_default();

        if index.is_null() {
            state.warn(tag_path, format_args!("Biped is missing a '{node}' node on its model."), PostprocessWarningType::BrokenReference);
        }

        index
    };

    biped.head_model_node_index = find_node("bip01 head");
    biped.pelvis_model_node_index = find_node("bip01 pelvis");

    biped.cosine_maximum_slope_angle = biped.maximum_slope_angle.radians().fw_cos();
    biped.cosine_stationary_turning_threshold = biped.stationary_turning_threshold.radians().fw_cos();

    biped.negative_sine_downhill_cutoff_angle = (-biped.downhill_cutoff_angle.radians()).fw_sin();
    biped.negative_sine_downhill_falloff_angle = (-biped.downhill_falloff_angle.radians()).fw_sin();

    biped.sine_uphill_cutoff_angle = biped.uphill_cutoff_angle.radians().fw_sin();
    biped.sine_uphill_falloff_angle = biped.uphill_falloff_angle.radians().fw_sin();

    if biped.crouch_transition_time != 0.0 {
        biped.crouch_camera_velocity = 1.0 / seconds_to_ticks(biped.crouch_transition_time);
    }
    else {
        biped.crouch_camera_velocity = 1.0;
    }

    if biped.flags.uses_limp_body_physics {
        for (node_index, node) in model.nodes().iter().enumerate().skip(1) {
            assert_postprocess!(
                node.node_distance_from_parent.fw_is_close_to(node.default_translation.magnitude()),
                "Limp body physics error: Model node #{node_index}'s distance from parent is not close to its translation's magnitude"
            );
        }
        for (node_index, node) in animation.nodes.iter().enumerate().skip(1) {
            assert_postprocess!(
                node.node_joint_flags.no_movement || !node.base_vector.magnitude().fw_is_close_to_zero(),
                "Limp body physics error: Animation node #{node_index} has a zero or near zero magnitude base vector"
            );
        }
    }

    Ok(())
}
