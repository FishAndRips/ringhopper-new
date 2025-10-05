use funnel_web::constants::TICK_RATE_INVERSE;
use funnel_web::float::FloatOps;
use funnel_web::id::Index;
use crate::definitions::tag::weapon::Weapon;
use crate::postprocess::{apply_default, Action};
use crate::{PostprocessError, PostprocessState, TagPath};
use crate::definitions::tag::bitmap::{Bitmap, BitmapType};
use crate::definitions::tag::weapon_hud_interface::{WeaponHUDInterface, WeaponHUDInterfaceCrosshairType};

pub fn postprocess_weapon(weapon: &mut Weapon, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        check_zoom_levels_are_valid_for_weapon(weapon, state)?;

        for mag in &mut weapon.magazines {
            if mag.rounds_reloaded < 0 {
                mag.rounds_reloaded = mag.rounds_loaded_maximum;
            }
        }
    }

    for (trigger_index, trigger) in weapon.triggers.iter_mut().enumerate() {
        apply_default(&mut trigger.projectiles_per_shot, 1, action);

        if action.postprocess() {
            if let Some(magazine_index) = trigger.magazine.0.index() && magazine_index >= weapon.magazines.len() {
                assert_postprocess!(
                    magazine_index == 0,
                    "Trigger #{trigger_index} specifies an out-of-bounds magazine index."
                );
                trigger.magazine.0 = Index::new();
            }

            let set_time_value = |seconds: f32, ticks: &mut f32| {
                if seconds.fw_is_close_to_zero_or_less() {
                    *ticks = 1.0;
                }
                else {
                    *ticks = TICK_RATE_INVERSE / seconds;
                }
            };

            set_time_value(trigger.ejection_port_recovery_time, &mut trigger.ejection_port_recovery_rate);
            set_time_value(trigger.illumination_recovery_time, &mut trigger.illumination_recovery_rate);
            set_time_value(trigger.acceleration_time, &mut trigger.firing_acceleration_rate);
            set_time_value(trigger.deceleration_time, &mut trigger.firing_deceleration_rate);
            set_time_value(trigger.error_acceleration_time, &mut trigger.error_acceleration_rate);
            set_time_value(trigger.error_deceleration_time, &mut trigger.error_deceleration_rate);
        }
    }

    Ok(())

}

fn check_zoom_levels_are_valid_for_weapon(weapon: &Weapon, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    let max_zoom_levels = weapon.zoom_levels as usize;

    // no zoom levels is fine
    if max_zoom_levels == 0 {
        return Ok(())
    }

    // no weapon hud interface is fine
    let Some(weapon_hud_interface_path) = weapon.hud_interface.get() else {
        return Ok(())
    };

    let weapon_hud_interface = state
        .read_tag_group::<WeaponHUDInterface>(weapon_hud_interface_path)
        .expect("not a weapon hud interface");

    // We need to go through each zoom overlay, as bitmap used will be the zoom level-th bitmap or
    // sprite in the sequence, and if we go beyond that, the game may crash!
    //
    // Of course, tool.exe does NOT check this and thus this check will fail on many custom tags.
    //
    // Some custom maps incorrectly put each zoom bitmap on its own sequence
    //
    // (note: tool.exe does not check this)
    for (crosshair_index, crosshair) in weapon_hud_interface.crosshairs.iter().enumerate() {
        if crosshair.crosshair_type != WeaponHUDInterfaceCrosshairType::ZoomOverlay {
            continue
        }
        let Some(bitmap_path) = crosshair.crosshair_bitmap.get() else {
            continue
        };

        let bitmap = state.read_tag_group::<Bitmap>(bitmap_path).expect("not a bitmap");

        for (overlay_index, overlay) in crosshair.crosshair_overlays.iter().enumerate() {
            let Some(crosshair_overlay_sequence_index) = overlay.sequence_index.index() else {
                continue
            };

            let bitmap_sequence = bitmap
                .bitmap_group_sequence
                .get(crosshair_overlay_sequence_index)
                .expect("failed to get crosshair overlay sequence index (was not checked properly)");

            let zoom_levels_in_sequence = if bitmap._type == BitmapType::Sprites {
                bitmap_sequence.sprites.len()
            }
            else {
                bitmap_sequence.bitmap_count as usize
            };

            if zoom_levels_in_sequence < max_zoom_levels {
                let mut hint = "";

                if zoom_levels_in_sequence == 1 && bitmap.bitmap_group_sequence[crosshair_overlay_sequence_index..].len() >= max_zoom_levels {
                    hint = "\n\n\
                            Hint: All zoom levels need to be put on the same sequence! Putting them \
                            on separate sequences leads to out-of-bounds accesses which is undefined \
                            behavior (and can cause crashes).\
                            \n\n\
                            (this hint appeared because the sequence referenced has only one sprite \
                            or bitmap, and there are at least as many remaining sequences as zoom \
                            levels)"
                }

                fail_postprocess!(
                    "Weapon has {max_zoom_levels} zoom level(s), but zoom crosshair #{crosshair_index}, \
                     overlay #{overlay_index} references sequence #{crosshair_overlay_sequence_index} \
                     in {bitmap_path} which has only {zoom_levels_in_sequence} sequence(s).{hint}"
                )
            }
        }
    }

    Ok(())
}
