use crate::definitions::tag::bitmap::BitmapType;
use crate::definitions::tag::weapon_hud_interface::{WeaponHUDInterface, WeaponHUDInterfaceCrosshairTypeFlags, WeaponHUDInterfaceOverlayTuple};
use crate::postprocess::bitmap::bitmap_or_sprite_count_of_sequence_by_index;
use crate::postprocess::hud_interface::{fixup_scale, postprocess_multitexture_overlay_reflexive};
use crate::postprocess::Action;
use crate::{Parameters, PostprocessError, PostprocessState, PostprocessWarningType, SimpleWriteableData, TagPath, TagReference};
use core::iter::once;
use funnel_web::id::Index;

pub fn postprocess_weapon_hud_interface(weapon_hud_interface: &mut WeaponHUDInterface, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    postprocess_weapon_hud_interface_static_elements(weapon_hud_interface, action, tag_path, state)?;
    postprocess_weapon_hud_interface_meters(weapon_hud_interface, action, tag_path, state)?;
    postprocess_weapon_hud_interface_crosshairs(weapon_hud_interface, action, tag_path, state)?;
    postprocess_weapon_hud_interface_overlays(weapon_hud_interface, action, tag_path, state)?;
    Ok(())
}

fn postprocess_weapon_hud_interface_meters(weapon_hud_interface: &mut WeaponHUDInterface, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for (meter_element_index, meter_element) in weapon_hud_interface.static_elements.iter_mut().enumerate() {
        fixup_scale(&mut meter_element.properties.position, action);
        let name = format_args!("Meter element #{meter_element_index}");

        if action.postprocess() {
            let flags = check_flags_for_bitmap(
                &name,
                &meter_element.properties.interface_bitmap,
                once((None, meter_element.properties.sequence_index)),
                tag_path,
                state
            )?;

            meter_element.flags.disabled = flags.disabled;
        }

        postprocess_multitexture_overlay_reflexive(&mut meter_element.properties.multitexture_overlays, &name, action, state)?;
    }

    Ok(())
}

fn postprocess_weapon_hud_interface_overlays(weapon_hud_interface: &mut WeaponHUDInterface, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for (overlay_element_index, overlay_element) in weapon_hud_interface.overlay_elements.iter_mut().enumerate() {
        overlay_element.flags.disabled = !postprocess_overlays(format_args!("Overlay element #{overlay_element_index}"), &mut overlay_element.overlay_data, action, tag_path, state)?;
    }
    Ok(())
}

fn postprocess_weapon_hud_interface_static_elements(weapon_hud_interface: &mut WeaponHUDInterface, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for (static_element_index, static_element) in weapon_hud_interface.static_elements.iter_mut().enumerate() {
        fixup_scale(&mut static_element.properties.position, action);
        let name = format_args!("Static element #{static_element_index}");

        if action.postprocess() {
            let flags = check_flags_for_bitmap(
                &name,
                &static_element.properties.interface_bitmap,
                once((None, static_element.properties.sequence_index)),
                tag_path,
                state
            )?;

            static_element.flags.disabled = flags.disabled;
        }

        postprocess_multitexture_overlay_reflexive(&mut static_element.properties.multitexture_overlays, &name, action, state)?;
    }

    Ok(())
}

fn postprocess_weapon_hud_interface_crosshairs(weapon_hud_interface: &mut WeaponHUDInterface, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    let mut crosshair_types: u16 = 0u16;
    for (crosshair_element_index, crosshair_element) in weapon_hud_interface.crosshairs.iter_mut().enumerate() {
        crosshair_types |= crosshair_element.crosshair_type as u16;

        for crosshair in &mut crosshair_element.crosshair_overlays {
            fixup_scale(&mut crosshair.position, action);
        }

        if action.postprocess() {
            let flags = check_flags_for_bitmap(
                &format_args!("Crosshair element #{crosshair_element_index}"),
                &crosshair_element.crosshair_bitmap,
                crosshair_element.crosshair_overlays.iter().enumerate().map(|i| (Some(i.0), i.1.sequence_index)),
                tag_path,
                state
            )?;

            crosshair_element.flags.disabled = flags.disabled;

            // we have to do this loop unconditionally since `not a sprite` can be set by the editor
            // since tool.exe does not set this flag
            for overlay in &mut crosshair_element.crosshair_overlays {
                overlay.flags.not_a_sprite = flags.not_a_sprite
            }
        }
    }

    if action.postprocess() {
        weapon_hud_interface.crosshair_types = WeaponHUDInterfaceCrosshairTypeFlags::read_tag_data_simple::<byteorder::NativeEndian>(
            crosshair_types.to_ne_bytes().as_slice(),
            Parameters::TAG_FILES
        ).expect("invalid crosshair types flags?");
    }

    Ok(())
}

pub(super) fn postprocess_overlays(name: core::fmt::Arguments, overlay_tuple: &mut WeaponHUDInterfaceOverlayTuple, action: Action, tag_path: &TagPath, state: &dyn PostprocessState) -> Result<bool, PostprocessError> {
    for i in &mut overlay_tuple.overlays {
        fixup_scale(&mut i.position, action);
    }

    if !action.postprocess() {
        return Ok(true)
    }

    let Some(bitmap_path) = overlay_tuple.bitmap.get() else {
        state.warn(
            tag_path,
            format_args!("{name} has no bitmap referenced; it will not render anything."),
            PostprocessWarningType::UnusedData
        );
        for overlay in &mut overlay_tuple.overlays {
            overlay.flags.disabled = true;
        }
        return Ok(false)
    };

    let loaded_bitmap = super::load_and_verify_bitmap_for_hud_from_path(bitmap_path, &name, state)?;
    if overlay_tuple.overlays.is_empty() {
        state.warn(
            tag_path,
            format_args!("{name} has no overlay elements; it will not render anything."),
            PostprocessWarningType::UnusedData
        );
        return Ok(false)
    }

    if overlay_tuple.overlays.is_empty() {
        state.warn(
            tag_path,
            format_args!("{name} has no elements; it will not render anything"),
            PostprocessWarningType::UnusedData
        );
        return Ok(false)
    }

    for (reflexive_index, overlay) in overlay_tuple.overlays.iter_mut().enumerate() {
        let sequence_index = overlay.sequence_index;
        let Some(sequence_index) = sequence_index.index() else {
            state.warn(
                tag_path,
                format_args!("{name} overlay #{reflexive_index} references an empty sequence; it will not render anything"),
                PostprocessWarningType::UnusedData
            );
            overlay.flags.disabled = true;
            continue;
        };

        let checked = bitmap_or_sprite_count_of_sequence_by_index(loaded_bitmap, sequence_index);

        match checked {
            Some(0) => {
                state.warn(
                    tag_path,
                    format_args!("{name} overlay #{reflexive_index} references an empty sequence for its bitmap ({bitmap_path}); it will not render anything"),
                    PostprocessWarningType::UnusedData
                );
                overlay.flags.disabled = true;
            }
            Some(_) => (),
            None => fail_postprocess!("{name} overlay #{reflexive_index} references a nonexistent sequence for its bitmap ({bitmap_path})")
        }
    }

    Ok(true)
}

struct Flags {
    disabled: bool,
    not_a_sprite: bool
}

fn check_flags_for_bitmap<const I: usize, IT: Iterator<Item = (Option<usize>, Index)>>(name: &core::fmt::Arguments, bitmap_path: &TagReference<I>, sequence_indices: IT, tag_path: &TagPath, state: &dyn PostprocessState) -> Result<Flags, PostprocessError> {
    let mut not_a_sprite = false;

    let Some(bitmap_path) = bitmap_path.get() else {
        state.warn(
            tag_path,
            format_args!("{name} has no bitmap referenced; it will not render anything."),
            PostprocessWarningType::UnusedData
        );
        return Ok(Flags {
            disabled: true,
            not_a_sprite
        })
    };

    let loaded_bitmap = super::load_and_verify_bitmap_for_hud_from_path(bitmap_path, &name, state)?;
    not_a_sprite = loaded_bitmap._type != BitmapType::Sprites;

    let mut empty = true;
    for (reflexive_index, sequence_index) in sequence_indices {
        empty = false;

        let Some(sequence_index) = sequence_index.index() else {
            state.warn(
                tag_path,
                format_args!("{name} references an empty sequence; it will not render anything"),
                PostprocessWarningType::UnusedData
            );
            return Ok(Flags {
                disabled: true,
                not_a_sprite
            })
        };

        let checked = bitmap_or_sprite_count_of_sequence_by_index(loaded_bitmap, sequence_index);

        match reflexive_index {
            Some(n) => {
                match checked {
                    Some(0) => fail_postprocess!("{name} #{n} references an empty sequence for its bitmap ({bitmap_path})"),
                    Some(_) => (),
                    None => fail_postprocess!("{name} #{n} references a nonexistent sequence for its bitmap ({bitmap_path})")
                }
            },
            None => return match checked {
                Some(0) => {
                    state.warn(
                        tag_path,
                        format_args!("{name} references an empty sequence for its bitmap ({bitmap_path}); it will not render anything"),
                        PostprocessWarningType::UnusedData
                    );
                    Ok(Flags {
                        disabled: true,
                        not_a_sprite
                    })
                }
                Some(_) => Ok(Flags {
                    disabled: true,
                    not_a_sprite
                }),
                None => fail_postprocess!("{name} references a nonexistent sequence for its bitmap ({bitmap_path})")
            }
        }
    }

    if empty {
        state.warn(
            tag_path,
            format_args!("{name} has no elements; it will not render anything"),
            PostprocessWarningType::UnusedData
        );
        Ok(Flags {
            disabled: true,
            not_a_sprite
        })
    }
    else {
        Ok(Flags {
            disabled: false,
            not_a_sprite
        })
    }
}
