use crate::definitions::tag::equipment::Equipment;
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, TagPath};
use crate::postprocess::globals::assert_globals_grenade_type;

pub fn postprocess_equipment(equipment: &mut Equipment, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        assert_globals_grenade_type(equipment.grenade_type, state)?;
    }
    Ok(())
}
