use crate::definitions::tag::enums::{GrenadeType, MaterialType};
use crate::definitions::tag::globals::{Globals, GlobalsWeaponType};
use crate::definitions::tag::scenario::ScenarioType;
use crate::postprocess::object::{assert_weapon_reference_not_readied, weapon_reference_must_be_readied};
use crate::postprocess::{apply_default_le_zero, Action};
use crate::{EditableEnumTagField, PostprocessError, PostprocessState, TagPath};
use funnel_web::constants::GRAVITY_WORLD_UNITS_PER_TICK_SQUARED;
use funnel_web::float::FloatOps;

pub fn postprocess_globals(globals: &mut Globals, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        let engine = state.engine();
        let scenario_type = state.scenario_type();

        let range = engine.grenade_limits.get_limit_for_scenario(scenario_type);
        let grenade_count = globals.grenades.len();

        if u8::try_from(grenade_count)
            .ok()
            .and_then(|i| range.contains(&i).then_some(())).is_none() {
            fail_postprocess!(
                "Globals tag contains {grenade_count} grenade(s), but {scenario_type} maps on engine '{engine}' must have between {from} and {to} grenades",
                engine = engine.name,
                from = range.start(),
                to = range.end()
            )
        }

        for falling_damage in &mut globals.falling_damage {
            let calcumulate = |value: f32| (2.0f32 * GRAVITY_WORLD_UNITS_PER_TICK_SQUARED * value).fw_sqrt();
            falling_damage.maximum_falling_velocity = calcumulate(falling_damage.maximum_falling_distance);
            falling_damage.harmful_falling_velocity.from = calcumulate(falling_damage.harmful_falling_distance.from);
            falling_damage.harmful_falling_velocity.to = calcumulate(falling_damage.harmful_falling_distance.to);
        }

        if scenario_type != ScenarioType::Multiplayer {
            if scenario_type == ScenarioType::UserInterface {
                for player_information in &mut globals.player_information {
                    player_information.unit.clear();
                }
                globals.materials.clear();
                globals.falling_damage.clear();
            }
            globals.multiplayer_information.clear();
            globals.cheat_powerups.clear();
            globals.weapon_list.clear();
        }

        if scenario_type == ScenarioType::Multiplayer {
            assert_postprocess!(
                globals.multiplayer_information.len() == 1,
                "Need exactly 1 multiplayer information entry in multiplayer maps"
            );

            assert_postprocess!(
                globals.weapon_list.len() >= engine.minimum_weapons,
                "Need at least {} weapons in multiplayer maps", engine.minimum_weapons
            );

            for (index, weapon_type) in globals.weapon_list.iter().enumerate() {
                let must_be_readied = (index == (GlobalsWeaponType::Ball as usize)) || (index == (GlobalsWeaponType::Flag as usize));

                if must_be_readied {
                    match weapon_reference_must_be_readied(&weapon_type.weapon, state) {
                        Some(true) => {},
                        Some(false) => fail_postprocess!(
                            "Weapon #{index} ({}) is NOT set to 'must be readied' which is not valid.",
                            weapon_type.weapon.get().expect("we just checked this tag...")
                        ),
                        None => fail_postprocess!("Weapon #{index} needs a weapon reference with ('must be readied') set.")
                    }
                }
                else {
                    assert_weapon_reference_not_readied(format_args!("#{index}"), &weapon_type.weapon, state)?;
                }
            }
        }

        if scenario_type != ScenarioType::UserInterface {
            let material_count = MaterialType::Dirt.values().len();
            assert_postprocess!(globals.falling_damage.len() == 1, "Need exactly 1 falling damage entry in non-UI maps");
            assert_postprocess!(globals.materials.len() == material_count, "Need exactly {material_count} materials in non-UI maps");
        }
    }

    for player_control in &mut globals.player_control {
        apply_default_le_zero(&mut player_control.minimum_weapon_swap_ticks, 5, action);
    }

    Ok(())
}

pub fn assert_globals_grenade_type(grenade_type: GrenadeType, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    let grenade_count = state.globals_tag().grenades.len();
    let grenade_type_usize = grenade_type as usize;

    assert_postprocess!(
        (grenade_type as usize) < grenade_count,
        "Grenade type {grenade_type} (#{grenade_type_usize}) was specified, but there are only {grenade_count} grenade(s) defined in the globals tag."
    );

    Ok(())
}
