use crate::definitions::tag::vehicle::Vehicle;
use crate::postprocess::{default_near_zero_or_less, Action};
use crate::PostprocessError;

pub fn postprocess_vehicle(vehicle: &mut Vehicle, action: Action) -> Result<(), PostprocessError> {
    default_near_zero_or_less(&mut vehicle.minimum_flipping_angular_velocity, 0.2, action);
    default_near_zero_or_less(&mut vehicle.maximum_flipping_angular_velocity, 0.75, action);
    Ok(())
}