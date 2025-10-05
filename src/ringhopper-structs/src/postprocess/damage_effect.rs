use funnel_web::constants::TICK_RATE;
use crate::definitions::tag::continuous_damage_effect::ContinuousDamageEffect;
use crate::definitions::tag::damage_effect::DamageEffect;
use crate::postprocess::{default_near_zero, postprocess_multiply, Action};
use crate::{get_stock_map_info, PostprocessError, PostprocessState, TagPath};
use crate::definitions::tag::scenario::ScenarioType;

macro_rules! default_camera_shake {
    ($camera_shake:expr, $action:expr) => {
        if $action.default() {
            default_near_zero(&mut $camera_shake.camera_shaking.wobble_period, 1.0, $action);
            postprocess_multiply(&mut $camera_shake.camera_shaking.wobble_period, TICK_RATE, $action);
        }
        else if $action.undefault() {
            postprocess_multiply(&mut $camera_shake.camera_shaking.wobble_period, TICK_RATE, $action);
            default_near_zero(&mut $camera_shake.camera_shaking.wobble_period, 1.0, $action);
        }
    };
}

pub fn postprocess_damage_effect(damage_effect: &mut DamageEffect, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    default_camera_shake!(damage_effect, action);
    default_near_zero(&mut damage_effect.screen_flash.maximum_intensity, 1.0, action);

    // Something that could have been changed in Guerilla and it wouldn't have affected multiplayer, but lmao
    if state.jason_jones_singleplayer() {
        if state.scenario_type() == ScenarioType::Singleplayer && state.tag_path() == "weapons\\pistol\\bullet" {
            if action.postprocess() {
                damage_effect.damage.modifiers.elite_energy_shield = 0.8;
            }
            else if action.unpostprocess() {
                damage_effect.damage.modifiers.elite_energy_shield = 1.0;
            }
        }
    }

    if get_stock_map_info(state.scenario_name()).is_some_and(|i| i.scenario_type == ScenarioType::Multiplayer) {
        // Custom Edition removed stun on vehicle weapons probably because the netcode was too powerful
        if matches!(state.tag_path(), "weapons\\banshee\\banshee bolt" | "vehicles\\ghost\\ghost bolt") {
            let is_custom_edition = state.engine().name == "pc-custom";

            if is_custom_edition {
                damage_effect.damage.stun = 0.0;
                damage_effect.damage.maximum_stun = 0.0;
                damage_effect.damage.stun_time = 0.0;
            }
            else {
                damage_effect.damage.stun = 1.0;
                damage_effect.damage.maximum_stun = 1.0;
                damage_effect.damage.stun_time = 0.15;
            }
        }
    }

    Ok(())
}

pub fn postprocess_continuous_damage_effect(damage_effect: &mut ContinuousDamageEffect, action: Action) -> Result<(), PostprocessError> {
    default_camera_shake!(damage_effect, action);
    Ok(())
}
