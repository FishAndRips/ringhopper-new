use funnel_web::constants::TICK_RATE;
use crate::definitions::tag::device_machine::DeviceMachine;
use crate::postprocess::Action;
use crate::PostprocessError;

pub fn postprocess_device_machine(device_machine: &mut DeviceMachine, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }
    device_machine.door_open_time_ticks = (device_machine.door_open_time * TICK_RATE) as u32;
    Ok(())
}