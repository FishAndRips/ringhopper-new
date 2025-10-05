use crate::definitions::tag::sound::Sound;
use crate::definitions::tag::sound_looping::SoundLooping;
use crate::postprocess::{apply_clamp, apply_default, apply_minimum_value_clamp, Action};
use crate::{Bounds, PostprocessError, PostprocessState, TagPath, TagReference};
use funnel_web::id::TagID;
use funnel_web::vector::Angle;

pub fn postprocess_sound_looping(sound_looping: &mut SoundLooping, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        sound_looping.zero_detail_unused = [1.0, 1.0];
        sound_looping.one_detail_unused = [1.0, 1.0];
        sound_looping.runtime_scripting_sound = TagID::new();
        sound_looping.maximum_distance = calculate_minimum_distance(sound_looping, state);
    }

    for detail in &mut sound_looping.detail_sounds {
        apply_clamp(&mut detail.gain, 0.0, 1.0, action);
        apply_default(&mut detail.gain, 1.0, action);

        apply_minimum_value_clamp(&mut detail.random_period_bounds.from, 0.0, action);
        apply_minimum_value_clamp(&mut detail.random_period_bounds.to, 0.0, action);
        apply_minimum_value_clamp(&mut detail.distance_bounds.from, 0.0, action);
        apply_minimum_value_clamp(&mut detail.distance_bounds.to, 0.0, action);

        apply_default(
            &mut detail.yaw_bounds,
            Bounds { from: -Angle::_180_DEG, to: Angle::_180_DEG },
            action
        );
        apply_default(
            &mut detail.pitch_bounds,
            Bounds { from: -Angle::_90_DEG, to: Angle::_90_DEG },
            action
        );
    }

    for track in &mut sound_looping.tracks {
        apply_clamp(&mut track.gain, 0.0, 1.0, action);
        apply_default(&mut track.gain, 1.0, action);

        apply_minimum_value_clamp(&mut track.fade_in_duration, 0.0, action);
        apply_minimum_value_clamp(&mut track.fade_out_duration, 0.0, action);
    }

    apply_minimum_value_clamp(&mut sound_looping.zero_detail_sound_period, 0.0, action);
    apply_minimum_value_clamp(&mut sound_looping.one_detail_sound_period, 0.0, action);

    let mut zero_one = [sound_looping.zero_detail_sound_period, sound_looping.one_detail_sound_period];
    apply_default(&mut zero_one, [1.0, 1.0], action);
    sound_looping.zero_detail_sound_period = zero_one[0];
    sound_looping.one_detail_sound_period = zero_one[1];

    Ok(())
}

pub fn calculate_minimum_distance(sound_looping: &SoundLooping, state: &dyn PostprocessState) -> f32 {
    sound_looping.tracks
        .iter()
        .map(|t| {
            [
                get_sound_maximum_distance(&t.start, state),
                get_sound_maximum_distance(&t.end, state),
                get_sound_maximum_distance(&t._loop, state),
                get_sound_maximum_distance(&t.alternate_loop, state),
                get_sound_maximum_distance(&t.alternate_end, state),
            ]
        })
        .flatten()
        .chain(sound_looping.detail_sounds
            .iter()
            .map(|i| get_sound_maximum_distance(&i.sound, state))
        )
        .reduce(f32::max)
        .unwrap_or(0.0)
}

fn get_sound_maximum_distance<const I: usize>(sound: &TagReference<I>, state: &dyn PostprocessState) -> f32 {
    let Some(s) = sound.get() else { return 0.0 };
    let sound_tag = state.read_tag_group::<Sound>(s).expect("not a sound");

    // we assume maximum distance has been defaulted already
    sound_tag.distance_bounds.to
}
