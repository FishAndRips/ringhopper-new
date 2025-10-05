use crate::definitions::tag::effect::Effect;
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, TagPath};
use crate::definitions::tag::{object_type_of_tag_group, TagGroup};
use crate::definitions::tag::damage_effect::DamageEffect;
use crate::postprocess::object::assert_weapon_reference_not_readied;

pub fn postprocess_effect(effect: &mut Effect, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    // nothing in here is getting defaulted
    if !action.postprocess() {
        return Ok(())
    }

    let mut must_be_deterministic = false;

    for (event_index, event) in effect.events.iter_mut().enumerate() {
        for (part_index, part) in &mut event.parts.iter_mut().enumerate() {
            let group = part._type.get().map(|i| i.group()).unwrap_or(TagGroup::DamageEffect);

            if object_type_of_tag_group(group).is_some() {
                part.type_identifier = TagGroup::Object;
                must_be_deterministic = true;

                assert_weapon_reference_not_readied(format_args!("event #{event_index} part #{part_index}"), &part._type, state)?;
            }
            else {
                part.type_identifier = group;
                must_be_deterministic |= matches!(part.type_identifier, TagGroup::DamageEffect | TagGroup::Light);
            }
        }
        for particle in &mut event.particles {
            particle.relative_direction_vector = particle.relative_direction.into();
        }
    }

    effect.flags.must_be_deterministic_pc = must_be_deterministic;
    effect.flags.must_be_deterministic_xbox = must_be_deterministic;
    effect.maximum_damage_radius = effect
        .events
        .iter()
        .map(|e| e.parts.iter())
        .flatten()
        .filter_map(|part| {
            if part.type_identifier == TagGroup::DamageEffect && let Some(damage_effect) = part._type.get() {
                let damage_effect = state.read_tag_group::<DamageEffect>(damage_effect)
                    .expect("part was not actually a damage effect");
                return Some(damage_effect.radius.to)
            }
            None
        })
        .reduce(f32::max)
        .unwrap_or(0.0);

    Ok(())
}
