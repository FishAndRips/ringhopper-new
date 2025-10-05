use funnel_web::constants::TICK_RATE;
use crate::definitions::tag::device::Device;
use crate::postprocess::Action;
use crate::PostprocessError;

pub fn postprocess_device(device: &mut Device, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let set_inverse = |from: f32, to: &mut f32| {
        if from != 0.0 {
            *to = 1.0 / (TICK_RATE * from);
        }
    };

    set_inverse(device.power_transition_time, &mut device.inverse_power_transition_time);
    set_inverse(device.power_acceleration_time, &mut device.inverse_power_acceleration_time);
    set_inverse(device.position_transition_time, &mut device.inverse_position_transition_time);
    set_inverse(device.position_acceleration_time, &mut device.inverse_position_acceleration_time);
    set_inverse(device.depowered_position_transition_time, &mut device.inverse_depowered_position_transition_time);
    set_inverse(device.depowered_position_acceleration_time, &mut device.inverse_depowered_position_acceleration_time);

    Ok(())
}