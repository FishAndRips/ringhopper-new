pub mod weapon_hud_interface;
pub mod unit_hud_interface;
pub mod grenade_hud_interface;
pub mod hud_number;
pub mod hud_globals;

use crate::definitions::tag::hud_interface_types::{HUDInterfaceElementPosition, HUDInterfaceMultitextureOverlay};
use crate::postprocess::{apply_clamp, apply_default_le_zero, Action};
use crate::{PostprocessError, PostprocessState, Reflexive, TagPath, TagReference};
use crate::definitions::tag::bitmap::{Bitmap, BitmapType};
use crate::postprocess::bitmap::{assert_bitmap_types_from_dependency, assert_bitmap_types_from_path};
use crate::postprocess::ui_widget_definition::STATIC_ELEMENT;

fn fixup_scale(position: &mut HUDInterfaceElementPosition, action: Action) {
    apply_default_le_zero(&mut position.width_scale, 1.0, action);
    apply_default_le_zero(&mut position.height_scale, 1.0, action);
}

fn load_and_verify_bitmap_for_hud_from_path<'a>(bitmap: &TagPath, name: &core::fmt::Arguments, state: &'a dyn PostprocessState) -> Result<&'a Bitmap, PostprocessError> {
    assert_bitmap_types_from_path(bitmap, STATIC_ELEMENT, name, state)
}

fn load_and_verify_bitmap_for_hud_from_dependency<'a, const I: usize>(bitmap: &TagReference<I>, name: &core::fmt::Arguments, state: &'a dyn PostprocessState) -> Result<Option<&'a Bitmap>, PostprocessError> {
    assert_bitmap_types_from_dependency(bitmap, STATIC_ELEMENT, name, state)
}

fn postprocess_multitexture_overlay_reflexive(multitexture_overlays: &mut Reflexive<HUDInterfaceMultitextureOverlay>, name: &core::fmt::Arguments, action: Action, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    for (overlay_index, overlay) in multitexture_overlays.iter_mut().enumerate() {
        if action.postprocess() {
            assert_bitmap_types_from_dependency(&overlay.primary, &[BitmapType::_2dTextures, BitmapType::InterfaceBitmaps], &format_args!("{name} multitexture overlay #{overlay_index} primary"), state)?;
            assert_bitmap_types_from_dependency(&overlay.secondary, &[BitmapType::_2dTextures, BitmapType::InterfaceBitmaps], &format_args!("{name} multitexture overlay #{overlay_index} secondary"), state)?;
            assert_bitmap_types_from_dependency(&overlay.tertiary, &[BitmapType::_2dTextures, BitmapType::InterfaceBitmaps], &format_args!("{name} multitexture overlay #{overlay_index} tertiary"), state)?;
        }
        
        for effector in &mut overlay.effectors {
            // I doubt the phone number that tool.exe gives you when this happens is in service anymore.
            apply_clamp(&mut effector.out_bounds.from, 0.0, 1.0, action);
            apply_clamp(&mut effector.out_bounds.to, 0.0, 1.0, action);
        }
    }

    Ok(())
}
