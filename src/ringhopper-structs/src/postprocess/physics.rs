use funnel_web::vector::{Matrix3x3, Vector3D};
use crate::definitions::tag::physics::{Physics, PhysicsInertialMatrix};
use crate::postprocess::Action;
use crate::{PostprocessError, Reflexive};

pub fn postprocess_physics(physics: &mut Physics, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    assert_postprocess!(physics.moment_scale > 0.0, "Invalid (non-positive) moment scale.");

    let mut total_relative_mass: f64 = 0.0;
    let mut density_scale: f64 = 0.0;
    for mass_point in &physics.mass_points {
        total_relative_mass += mass_point.relative_mass as f64;
        if mass_point.relative_density != 0.0 {
            density_scale += (mass_point.relative_mass / mass_point.relative_density) as f64;
        }
    }

    let mass = physics.mass;
    let density = physics.density;
    let mass_inverse = (1.0 / mass) as f64;

    let mut combined_mass_x: f64 = 0.0;
    let mut combined_mass_y: f64 = 0.0;
    let mut combined_mass_z: f64 = 0.0;

    for mass_point in &mut physics.mass_points {
        if total_relative_mass != 0.0 {
            // do some finessing here with casts to make sure we get the right precision, since not
            // doing it exactly will lead to discrepancies with tool.exe
            mass_point.mass = ((mass * mass_point.relative_mass) as f64 / total_relative_mass) as f32;
            mass_point.density = ((density * mass_point.relative_density) as f64 * density_scale / total_relative_mass) as f32;
        }

        combined_mass_x += (mass_point.mass * mass_point.position.x) as f64;
        combined_mass_y += (mass_point.mass * mass_point.position.y) as f64;
        combined_mass_z += (mass_point.mass * mass_point.position.z) as f64;
    }

    if mass != 0.0 {
        physics.center_of_mass.x = (mass_inverse * combined_mass_x) as f32;
        physics.center_of_mass.y = (mass_inverse * combined_mass_y) as f32;
        physics.center_of_mass.z = (mass_inverse * combined_mass_z) as f32;
    }

    // Get inertial matrix stuff and set base moments.
    // 32-bit floats here match tool precision.
    let mut mxx: f32 = 0.0;
    let mut myy: f32 = 0.0;
    let mut mzz: f32 = 0.0;
    let mut mxy: f32 = 0.0;
    let mut myz: f32 = 0.0;
    let mut mzx: f32 = 0.0;

    for mass_point in &physics.mass_points {
        let moment: f64 = (physics.moment_scale * mass_point.mass) as f64; // We checked earlier that moment_scale was non-zero.
        let x: f64 = (mass_point.position.x - physics.center_of_mass.x) as f64;
        let y: f64 = (mass_point.position.y - physics.center_of_mass.y) as f64;
        let z: f64 = (mass_point.position.z - physics.center_of_mass.z) as f64;
        let xx: f64 = x * x;
        let yy: f64 = y * y;
        let zz: f64 = z * z;

        physics.xx_moment += (moment * (yy + zz)) as f32;
        physics.yy_moment += (moment * (zz + xx)) as f32;
        physics.zz_moment += (moment * (xx + yy)) as f32;

        let radius_term: f64 = 0.4 * moment * mass_point.radius as f64 * mass_point.radius as f64;

        // If the base radius is anything below 0 then we account for local mass point radius.
        if physics.radius < 0.0 {
            physics.xx_moment += radius_term as f32;
            physics.yy_moment += radius_term as f32;
            physics.zz_moment += radius_term as f32;
        }

        // We always account for local mass point radius in the inertial matrices. Is this a tool oversight?
        mxx += (radius_term + moment * (yy + zz)) as f32;
        myy += (radius_term + moment * (zz + xx)) as f32;
        mzz += (radius_term + moment * (xx + yy)) as f32;
        mxy += (-moment * x * y) as f32;
        myz += (-moment * y * z) as f32;
        mzx += (-moment * z * x) as f32;
    }

    let inertial_matrix = Matrix3x3 {
        forward: Vector3D {
            x: mxx,
            y: mxy,
            z: mzx
        },
        left: Vector3D {
            x: mxy,
            y: myy,
            z: myz
        },
        up: Vector3D {
            x: mzx,
            y: myz,
            z: mzz
        }
    };

    physics.inertial_matrix_and_inverse = Reflexive::with_vec(alloc::vec![
        PhysicsInertialMatrix { matrix: inertial_matrix },
        PhysicsInertialMatrix { matrix: inertial_matrix.inverted() }
    ]);

    Ok(())
}
