use funnel_web::float::FloatOps;
use funnel_web::vector::Angle;
use crate::definitions::tag::lens_flare::LensFlare;
use crate::postprocess::{apply_default, Action};
use crate::PostprocessError;

pub fn postprocess_lens_flare(lens_flare: &mut LensFlare, action: Action) -> Result<(), PostprocessError> {
    // A lot of tags and cache files will have this set to 360 radians due to a bug with tool.exe
    //
    // We can set it to 0 on default AND undefault safely, as it will either already be defaulted
    // (in the case of undefault) or default to the correct value (in the case of default)
    if (action.default() || action.undefault()) && lens_flare.rotation_function_scale.radians() == 360.0 {
        lens_flare.rotation_function_scale = Angle::_0_DEG;
    }

    apply_default(&mut lens_flare.rotation_function_scale, Angle::_360_DEG, action);
    apply_default(&mut lens_flare.horizontal_scale, 1.0, action);
    apply_default(&mut lens_flare.vertical_scale, 1.0, action);

    if action.postprocess() {
        lens_flare.cos_cutoff_angle = lens_flare.cutoff_angle.radians().fw_cos();
        lens_flare.cos_falloff_angle = lens_flare.falloff_angle.radians().fw_cos();
    }

    Ok(())
}
