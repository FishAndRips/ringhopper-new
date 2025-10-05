use crate::definitions::tag::grenade_hud_interface::GrenadeHUDInterface;
use crate::postprocess::hud_interface::{fixup_scale, load_and_verify_bitmap_for_hud_from_dependency};
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, TagPath};

pub fn postprocess_grenade_hud_interface(grenade_hud_interface: &mut GrenadeHUDInterface, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    fixup_scale(&mut grenade_hud_interface.background.position, action);
    fixup_scale(&mut grenade_hud_interface.total_grenades_background.position, action);
    fixup_scale(&mut grenade_hud_interface.total_grenades_numbers.properties.position, action);

    // FIXME: This should do what unit_hud_interface does as it doesn't have a way to handle things that shouldn't render (besides null bitmaps)...
    super::weapon_hud_interface::postprocess_overlays(
        format_args!("Grenade overlays"),
        &mut grenade_hud_interface.total_grenades_overlays.overlay_data,
        action,
        tag_path,
        state
    )?;

    if action.postprocess() {
        load_and_verify_bitmap_for_hud_from_dependency(
            &grenade_hud_interface.background.interface_bitmap,
            &format_args!("background interface bitmap"),
            state
        )?;
        load_and_verify_bitmap_for_hud_from_dependency(
            &grenade_hud_interface.total_grenades_background.interface_bitmap,
            &format_args!("total grenades background interface bitmap"),
            state
        )?;
    }

    Ok(())
}
