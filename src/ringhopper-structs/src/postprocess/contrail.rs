use funnel_web::constants::TICK_RATE;
use crate::definitions::tag::contrail::{Contrail, ContrailRenderType};
use crate::postprocess::{apply_default_le_zero, Action};
use crate::PostprocessError;

pub fn postprocess_contrail(contrail: &mut Contrail, action: Action) -> Result<(), PostprocessError> {
    super::shader::postprocess_shader_effect(&mut contrail.shader, action);

    match contrail.render_type {
        ContrailRenderType::MediaMapped => {
            contrail.flags.points_start_pinned_to_media = true;
            contrail.flags.points_always_pinned_to_media = true;
            contrail.flags.points_start_pinned_to_ground = false;
            contrail.flags.points_always_pinned_to_ground = false;
        }
        ContrailRenderType::GroundMapped => {
            contrail.flags.points_start_pinned_to_media = false;
            contrail.flags.points_always_pinned_to_media = false;
            contrail.flags.points_start_pinned_to_ground = true;
            contrail.flags.points_always_pinned_to_ground = true;
        }
        ContrailRenderType::DoubleMarkerLinked => {
            contrail.flags.points_start_pinned_to_media = false;
            contrail.flags.points_always_pinned_to_media = false;
            contrail.flags.points_start_pinned_to_ground = false;
            contrail.flags.points_always_pinned_to_ground = false;
        }
        _ => ()
    }

    apply_default_le_zero(&mut contrail.point_generation_rate, TICK_RATE, action);

    Ok(())
}
