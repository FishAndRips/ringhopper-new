use funnel_web::constants::seconds_to_ticks;
use funnel_web::vector::Angle;
use crate::definitions::tag::unit::Unit;
use crate::postprocess::{apply_default, apply_minimum_value_clamp, Action};
use crate::{PostprocessError, PostprocessState, TagPath};
use crate::postprocess::globals::assert_globals_grenade_type;
use crate::postprocess::object::assert_weapon_reference_not_readied;

pub fn postprocess_unit(unit: &mut Unit, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        unit.soft_ping_interrupt_ticks = seconds_to_ticks(unit.soft_ping_interrupt_time).clamp(0.0, i16::MAX as f32) as i16;
        unit.hard_ping_interrupt_ticks = seconds_to_ticks(unit.hard_ping_interrupt_time).clamp(0.0, i16::MAX as f32) as i16;
        assert_globals_grenade_type(unit.grenade_type, state)?;
    }

    apply_minimum_value_clamp(&mut unit.camera_field_of_view.0, 0.0, action);
    apply_default(&mut unit.camera_field_of_view, Angle::DEFAULT_HORIZONTAL_FOV, action);

    for (index, unit_weapon) in unit.weapons.iter().enumerate() {
        assert_weapon_reference_not_readied(format_args!("#{index}"), &unit_weapon.weapon, state)?;
    }

    Ok(())
}
