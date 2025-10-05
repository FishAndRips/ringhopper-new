use crate::definitions::tag::projectile::Projectile;
use crate::postprocess::{apply_seconds_to_ticks, Action};
use crate::PostprocessError;

pub fn postprocess_projectile(projectile: &mut Projectile, action: Action) -> Result<(), PostprocessError> {
    apply_seconds_to_ticks(&mut projectile.minimum_velocity, action);
    apply_seconds_to_ticks(&mut projectile.initial_velocity, action);
    apply_seconds_to_ticks(&mut projectile.final_velocity, action);
    Ok(())
}