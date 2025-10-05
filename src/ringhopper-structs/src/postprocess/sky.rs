use crate::definitions::tag::sky::Sky;
use crate::postprocess::{apply_default, Action};
use crate::PostprocessError;

pub fn postprocess_sky(sky: &mut Sky, action: Action) -> Result<(), PostprocessError> {
    apply_default(&mut sky.indoor_fog.maximum_density, 1.0, action);
    apply_default(&mut sky.outdoor_fog.maximum_density, 1.0, action);
    Ok(())
}
