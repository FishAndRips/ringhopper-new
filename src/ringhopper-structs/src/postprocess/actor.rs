use funnel_web::constants::seconds_to_ticks;
use funnel_web::float::{FloatOps, TrigScalarFloatOps};
use funnel_web::vector::Angle;
use crate::postprocess::{apply_seconds_to_ticks, Action};
use crate::{PostprocessError, PostprocessState, TagPath};
use crate::definitions::tag::actor::Actor;
use crate::definitions::tag::actor_variant::ActorVariant;
use crate::postprocess::globals::assert_globals_grenade_type;
use crate::postprocess::object::assert_weapon_reference_not_readied;

pub fn postprocess_actor_variant(actor_variant: &mut ActorVariant, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        assert_globals_grenade_type(actor_variant.grenades.grenade_type, state)?;
        assert_weapon_reference_not_readied(format_args!("ranged_combat.weapon"), &actor_variant.ranged_combat.weapon, state)?;
    }

    apply_seconds_to_ticks(&mut actor_variant.grenades.grenade_velocity, action);

    Ok(())
}

pub fn postprocess_actor(actor: &mut Actor, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let set_inverted = |inverted: &mut f32, of: f32| {
        if of > 0.0 {
            *inverted = 1.0 / seconds_to_ticks(of)
        }
    };

    set_inverted(&mut actor.perception.inverse_combat_perception_time, actor.perception.combat_perception_time);
    set_inverted(&mut actor.perception.inverse_guard_perception_time, actor.perception.guard_perception_time);
    set_inverted(&mut actor.perception.inverse_non_combat_perception_time, actor.perception.non_combat_perception_time);

    // FIXME: This is not actually an Euler2D. The type should be fixed later.
    actor.looking.cosine_maximum_aiming_deviation.yaw = Angle(actor.looking.maximum_aiming_deviation.yaw.tfw_cos());
    actor.looking.cosine_maximum_aiming_deviation.pitch = Angle(actor.looking.maximum_aiming_deviation.pitch.tfw_cos());
    actor.looking.cosine_maximum_looking_deviation.yaw = Angle(actor.looking.maximum_looking_deviation.yaw.tfw_cos());
    actor.looking.cosine_maximum_looking_deviation.pitch = Angle(actor.looking.maximum_looking_deviation.pitch.tfw_cos());

    let begin_moving_angle = actor.movement.begin_moving_angle.radians();
    if begin_moving_angle > 0.0 {
        actor.movement.cosine_begin_moving_angle = begin_moving_angle.fw_cos();
    }

    Ok(())
}
