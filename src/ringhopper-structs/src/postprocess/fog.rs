use crate::definitions::tag::fog::Fog;
use crate::postprocess::{apply_default, apply_maximum_value_clamp, Action};
use crate::PostprocessError;

pub fn postprocess_fog(fog: &mut Fog, action: Action) -> Result<(), PostprocessError> {
    apply_maximum_value_clamp(&mut fog.layer_count, 4, action); // also in definitions

    apply_default(&mut fog.wind_period.from, 1.0, action);
    apply_default(&mut fog.wind_period.to, 1.0, action);
    Ok(())
}
