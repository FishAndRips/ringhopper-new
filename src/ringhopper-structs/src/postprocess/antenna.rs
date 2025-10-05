use funnel_web::float::TrigScalarFloatOps;
use funnel_web::vector::Vector3D;
use crate::definitions::tag::antenna::Antenna;
use crate::postprocess::Action;
use crate::PostprocessError;

pub fn postprocess_antenna(antenna: &mut Antenna, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let mut total = 0.0;
    for vertex in &mut antenna.vertices {
        total += vertex.length;

        let sine_pitch = vertex.angles.pitch.tfw_sin();
        let cosine_pitch = vertex.angles.pitch.tfw_cos();
        let sine_yaw = vertex.angles.yaw.tfw_sin();
        let cosine_yaw = vertex.angles.yaw.tfw_cos();

        vertex.offset = Vector3D {
            x: vertex.length * cosine_yaw * sine_pitch,
            y: vertex.length * sine_yaw * sine_pitch,
            z: vertex.length * cosine_pitch * sine_pitch
        };
    }
    antenna.length = total;

    Ok(())
}
