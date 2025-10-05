use funnel_web::float::FloatOps;
use funnel_web::constants::{AIR_DENSITY, METERS_PER_WORLD_UNIT, WATER_DENSITY};
use crate::definitions::tag::point_physics::PointPhysics;
use crate::postprocess::Action;
use crate::PostprocessError;

const FRICTION_SCALE: f32 = 10000.0;

pub fn postprocess_point_physics(point_physics: &mut PointPhysics, action: Action) -> Result<(), PostprocessError> {
    if action.postprocess() {
        point_physics.air_friction *= FRICTION_SCALE;
        point_physics.water_friction *= FRICTION_SCALE;

        let cubic_meters_per_cubic_world_unit = METERS_PER_WORLD_UNIT * METERS_PER_WORLD_UNIT * METERS_PER_WORLD_UNIT;

        // not doing it this way leads to death (i.e. precision errors)
        point_physics.mass_scale = 0.001
            * point_physics.density
            * ((4.0 / 3.0) * f32::FW_PI)
            * (cubic_meters_per_cubic_world_unit * 100.0 * 100.0 * 100.0);

        point_physics.water_gravity_scale = 1.0 - (2.0 * point_physics.density / (point_physics.density + WATER_DENSITY));
        point_physics.air_gravity_scale = 1.0 - (2.0 * point_physics.density / (point_physics.density + AIR_DENSITY));

        point_physics.water_gravity_scale = 1.0 - (2.0 * point_physics.density / (point_physics.density + WATER_DENSITY));
        point_physics.air_gravity_scale = 1.0 - (2.0 * point_physics.density / (point_physics.density + AIR_DENSITY));
    }
    else if action.unpostprocess() {
        point_physics.air_friction /= FRICTION_SCALE;
        point_physics.water_friction /= FRICTION_SCALE;
    }

    Ok(())
}
