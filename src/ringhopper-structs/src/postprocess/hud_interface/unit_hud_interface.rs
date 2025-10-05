use crate::definitions::tag::bitmap::{Bitmap, BitmapType};
use crate::definitions::tag::hud_interface_types::{HUDInterfaceMeterElement, HUDInterfaceStaticElement};
use crate::definitions::tag::unit_hud_interface::UnitHUDInterface;
use crate::postprocess::hud_interface::{fixup_scale, postprocess_multitexture_overlay_reflexive};
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, PostprocessWarningType, TagPath};
use funnel_web::id::Index;

pub fn postprocess_unit_hud_interface(unit_hud_interface: &mut UnitHUDInterface, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    postprocess_unit_hud_interface_static_element(&format_args!("HUD background"), &mut unit_hud_interface.hud_background, action, tag_path, state)?;

    postprocess_unit_hud_interface_static_element(&format_args!("Health panel background"), &mut unit_hud_interface.health_panel_background, action, tag_path, state)?;
    postprocess_unit_hud_interface_meter_element(&format_args!("Health meter"), &mut unit_hud_interface.health_panel_meter.hudinterface_meter_element, action, tag_path,state)?;
    postprocess_unit_hud_interface_static_element(&format_args!("Motion sensor background"), &mut unit_hud_interface.motion_sensor_background, action, tag_path, state)?;
    postprocess_unit_hud_interface_static_element(&format_args!("Shield panel background"), &mut unit_hud_interface.shield_panel_background, action, tag_path, state)?;

    postprocess_unit_hud_interface_meter_element(&format_args!("Shield meter"), &mut unit_hud_interface.shield_panel_meter.hudinterface_meter_element, action, tag_path, state)?;
    postprocess_unit_hud_interface_static_element(&format_args!("Motion sensor foreground"), &mut unit_hud_interface.motion_sensor_foreground, action, tag_path, state)?;
    fixup_scale(&mut unit_hud_interface.motion_sensor_center, action);

    for (meter_index, meter) in unit_hud_interface.auxiliary_elements.meters.iter_mut().enumerate() {
        postprocess_unit_hud_interface_meter_element(
            &format_args!("Auxiliary meter #{meter_index} meter"),
            &mut meter.meter,
            action,
            tag_path,
            state
        )?;
        postprocess_unit_hud_interface_static_element(
            &format_args!("Auxiliary meter #{meter_index} background"),
            &mut meter.background,
            action,
            tag_path,
            state
        )?;
    }

    for (overlay_index, overlay) in unit_hud_interface.auxiliary_elements.overlays.iter_mut().enumerate() {
        postprocess_unit_hud_interface_static_element(
            &format_args!("Auxiliary overlay #{overlay_index}"),
            &mut overlay.properties,
            action,
            tag_path,
            state
        )?;
    }

    Ok(())
}

fn postprocess_unit_hud_interface_static_element(name: &core::fmt::Arguments, element: &mut HUDInterfaceStaticElement, action: Action, tag_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    fixup_scale(&mut element.position, action);

    postprocess_multitexture_overlay_reflexive(&mut element.multitexture_overlays, name, action, state)?;

    if action.postprocess() {
        if let Some(bitmap) = element.interface_bitmap.get() {
            verify_sequence_index_for_bitmap_reference(name, bitmap, element.sequence_index, state)?;
        }
        else {
            state.warn(
                tag_path,
                format_args!("{name}'s bitmap is unset; it will not render anything"),
                PostprocessWarningType::UnusedData
            );
        }
    }

    Ok(())
}

fn postprocess_unit_hud_interface_meter_element(name: &core::fmt::Arguments, element: &mut HUDInterfaceMeterElement, action: Action, tag_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    fixup_scale(&mut element.position, action);

    if action.postprocess() {
        // TODO: This needs to be processed for weapon_hud_interface too!!!!
        element.min_alpha = element.min_alpha.clamp(0.0, 1.0);

        if let Some(bitmap) = element.meter_bitmap.get() {
            verify_sequence_index_for_bitmap_reference(name, bitmap, element.sequence_index, state)?;
        }
        else {
            state.warn(
                tag_path,
                format_args!("{name}'s bitmap is unset; it will not render anything"),
                PostprocessWarningType::UnusedData
            );
        }
    }

    Ok(())
}

fn verify_sequence_index_for_bitmap_reference(name: &core::fmt::Arguments, bitmap: &TagPath, sequence_index: Index, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    let Some(sequence_index) = sequence_index.index() else {
        fail_postprocess!("{name}'s sequence index is null (this is invalid)");
    };

    let bitmap = state.read_tag_group::<Bitmap>(bitmap).expect("not a bitmap");
    let Some(sequence) = bitmap.bitmap_group_sequence.get(sequence_index) else {
        fail_postprocess!("{name}'s sequence index is out-of-bounds ({sequence_index} >= {})", bitmap.bitmap_group_sequence.len());
    };

    let sequence_frame_count = match bitmap._type {
        BitmapType::InterfaceBitmaps | BitmapType::_2dTextures => sequence.bitmap_count as usize,
        BitmapType::Sprites => sequence.sprites.len(),
        _ => fail_postprocess!("{name}'s bitmap is not a 2D texture, interface bitmap, or sprite sheet")
    };

    assert_postprocess!(
        sequence_frame_count > 0,
        "{name}'s sequence {sequence_index} has no bitmaps/sprites"
    );

    Ok(())
}
