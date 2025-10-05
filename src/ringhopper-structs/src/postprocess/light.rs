use crate::definitions::tag::light::Light;
use crate::postprocess::{apply_default, apply_maximum_value_clamp, apply_seconds_to_ticks, Action};
use crate::PostprocessError;
use funnel_web::color::{ColorARGB, ColorRGB};
use funnel_web::float::FloatOps;
use funnel_web::vector::Angle;

pub fn postprocess_light(light: &mut Light, action: Action) -> Result<(), PostprocessError> {
    apply_default(&mut light.shape.radius, 1.0, action);
    apply_default(&mut light.gel.yaw_period, 1.0, action);
    apply_default(&mut light.gel.pitch_period, 1.0, action);
    apply_default(&mut light.gel.roll_period, 1.0, action);

    apply_seconds_to_ticks(&mut light.effect_parameters.duration, action);

    let color_almost_less_than_zero = |color: &ColorARGB| -> bool {
        color.color.r.fw_is_close_to_zero_or_less()
        && color.color.g.fw_is_close_to_zero_or_less()
        && color.color.b.fw_is_close_to_zero_or_less()
    };

    if color_almost_less_than_zero(&light.color.color.from) && color_almost_less_than_zero(&light.color.color.to) {
        light.color.color.from.color = ColorRGB::WHITE;
        light.color.color.to.color = ColorRGB::WHITE;
    }

    light.color.color.from = light.color.color.from.clamped();
    light.color.color.to = light.color.color.to.clamped();

    postprocess_falloff_cutoff_angle(light, action);

    if action.postprocess() {
        light.shape.specular_radius_multiplier = 2.0; // unconditional; this is anyway not editable
    }

    Ok(())
}
fn postprocess_falloff_cutoff_angle(light: &mut Light, action: Action) {
    let max_angle = Angle::_180_DEG;

    if action.default() && light.shape.cutoff_angle.radians().fw_is_close_to_zero_or_less() {
        light.shape.cutoff_angle = max_angle;
        light.shape.falloff_angle = max_angle;
    }
    else if action.undefault() && light.shape.cutoff_angle == max_angle && light.shape.falloff_angle == max_angle {
        light.shape.cutoff_angle = Angle::_0_DEG;
        light.shape.falloff_angle = Angle::_0_DEG;
    }

    apply_maximum_value_clamp(&mut light.shape.cutoff_angle, max_angle, action);
    apply_maximum_value_clamp(&mut light.shape.falloff_angle, max_angle, action);

    if action.default() && light.shape.falloff_angle > light.shape.cutoff_angle {
        light.shape.falloff_angle = Angle::_0_DEG;
    }

    if action.postprocess() {
        if light.shape.falloff_angle < max_angle {
            light.shape.cos_falloff_angle = light.shape.falloff_angle.radians().fw_cos();
            light.shape.cos_cutoff_angle = light.shape.cutoff_angle.radians().fw_cos();
            light.shape.sin_cutoff_angle = light.shape.cutoff_angle.radians().fw_sin();
        }
        else {
            // falloff angle is ≥ 180˚
            //
            // this can only happen if falloff angle and cutoff angle are both equal to 180˚
            // because of the earlier defaulting
            light.shape.cos_falloff_angle = -1.0;
            light.shape.cos_cutoff_angle = -1.0;
            light.shape.sin_cutoff_angle = 0.0;
        }
    }
}
