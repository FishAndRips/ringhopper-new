use crate::definitions::tag::light_volume::LightVolume;
use crate::postprocess::{apply_default_le_zero, Action};
use crate::PostprocessError;

pub fn postprocess_light_volume(light_volume: &mut LightVolume, action: Action) -> Result<(), PostprocessError> {
    if action.default() && light_volume.perpendicular_brightness_scale == 0.0 && light_volume.parallel_brightness_scale == 0.0 {
        light_volume.parallel_brightness_scale = 1.0;
        light_volume.perpendicular_brightness_scale = 1.0;
    }
    else if action.undefault() && light_volume.perpendicular_brightness_scale == 1.0 && light_volume.parallel_brightness_scale == 1.0 {
        light_volume.parallel_brightness_scale = 0.0;
        light_volume.perpendicular_brightness_scale = 0.0;
    }

    for frame in &mut light_volume.frames {
        apply_default_le_zero(&mut frame.offset_exponent, 1.0, action);
        apply_default_le_zero(&mut frame.radius_exponent, 1.0, action);
        apply_default_le_zero(&mut frame.tint_color_exponent, 1.0, action);
        apply_default_le_zero(&mut frame.brightness_exponent, 1.0, action);
    }

    Ok(())
}
