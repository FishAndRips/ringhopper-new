use crate::definitions::tag::sound::{Sound, SoundChannelCount, SoundClass, SoundFormat, SoundPermutation, SoundSampleRate};
use crate::postprocess::{apply_clamp, apply_default, apply_default_le_zero, apply_minimum_value_clamp, Action};
use crate::{Bounds, PostprocessError, PostprocessState, PostprocessWarningType, TagPath};
use funnel_web::constants::{PCM_SAMPLE_SIZE_BYTES, TICK_RATE, TICK_RATE_INVERSE, XBOX_ADPCM_BLOCK_SIZE_BYTES};
use funnel_web::float::FloatOps;
use funnel_web::vector::Angle;

pub fn postprocess_sound(sound: &mut Sound, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    postprocess_pitch_range_parameters(sound, action)?;

    if action.postprocess() || action.unpostprocess() {
        verify_sound_tag_format(sound)?;
        byteswap_pcm(sound)?;
    }

    sound.maximum_bend_rate = process_maximum_bend(sound.maximum_bend_rate, action);
    default_cone_angle_parameters(sound, action, tag_path, state);
    default_gain_modifiers(sound, action);
    default_min_max_distance(sound, action);

    if action.postprocess() {
        postprocess_pitch_ranges(sound, tag_path, state)?;

        sound.scripted_sound_index = u32::MAX;
        sound.scripted_sound_remaining_time = u32::MAX;
    }

    Ok(())
}

fn process_maximum_bend(bend: f32, action: Action) -> f32 {
    if action.postprocess() {
        if bend <= 1.0 {
            return 0.0
        }
        bend.fw_powf(TICK_RATE_INVERSE)
    }
    else if action.unpostprocess() {
        bend.fw_powf(TICK_RATE)
    }
    else {
        bend
    }
}

fn postprocess_pitch_range_parameters(sound: &mut Sound, action: Action) -> Result<(), PostprocessError> {
    let is_split = sound.flags.split_long_sound_into_permutations;

    for (pitch_range_index, pitch_range) in sound.pitch_ranges.iter_mut().enumerate() {
        let mut actual_permutation_count = pitch_range.actual_permutation_count as usize;

        if action.postprocess() {
            assert_postprocess!(
                actual_permutation_count > 0,
                "Pitch range #{pitch_range_index} has an actual_permutation_count of 0 but there are permutations."
            );
            assert_postprocess!(
                actual_permutation_count <= pitch_range.permutations.len(),
                "Pitch range #{pitch_range_index} has an actual_permutation_count greater than the number of permutations."
            );
        }

        // Override what we actually got (in case we're extracting a map that's broken)
        if !action.postprocess() {
            actual_permutation_count = actual_permutation_count.min(pitch_range.permutations.len());
        }

        if action.unpostprocess() {
            if pitch_range.bend_bounds.from == 0.0 && pitch_range.bend_bounds.to == pitch_range.natural_pitch {
                pitch_range.bend_bounds.to = 0.0;
            }
        }

        apply_minimum_value_clamp(&mut pitch_range.natural_pitch, 0.0, action);
        apply_minimum_value_clamp(&mut pitch_range.bend_bounds.from, 0.0, action);
        apply_minimum_value_clamp(&mut pitch_range.bend_bounds.to, 0.0, action);
        apply_default(&mut pitch_range.natural_pitch, 1.0, action);

        let (actual_permutations, subpermutations) = pitch_range.permutations.split_at_mut(actual_permutation_count);

        if action.postprocess() {
            assert_postprocess!(
                subpermutations.is_empty() || !is_split,
                "Pitch range #{pitch_range_index} has an actual_permutation_count less than the number of permutations when split permutations is not enabled for the tag."
            )
        }

        for permutation in actual_permutations.iter_mut() {
            apply_clamp(&mut permutation.skip_fraction, 0.0, 1.0, action);
            apply_clamp(&mut permutation.gain, 0.0, 1.0, action);
            apply_default(&mut permutation.gain, 1.0, action);
        }

        for not_actual_permutation in subpermutations.iter_mut() {
            if action.postprocess() {
                not_actual_permutation.skip_fraction = 0.0; // due to broken tools
            }
            if action.unpostprocess() {
                not_actual_permutation.skip_fraction = 0.0; // due to broken tools
                not_actual_permutation.gain = 0.0;
            }
        }

        if action.postprocess() {
            postprocess_split_permutations(is_split, pitch_range_index, actual_permutations, subpermutations)?;
        }
    }

    Ok(())
}

fn default_cone_angle_parameters(sound: &mut Sound, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) {
    apply_clamp(&mut sound.inner_cone_angle, Angle::_0_DEG, Angle::_360_DEG, action);
    apply_clamp(&mut sound.outer_cone_angle, Angle::_0_DEG, Angle::_360_DEG, action);
    apply_clamp(&mut sound.outer_cone_gain, 0.0, 1.0, action);

    if action.default() && sound.outer_cone_angle == Angle::_0_DEG {
        if action.postprocess() {
            if sound.inner_cone_angle != Angle::_0_DEG && sound.inner_cone_angle != Angle::_360_DEG {
                state.warn(
                    tag_path,
                    format_args!("Outer cone angle is 0 degrees; inner cone angle will be set to 360 degrees."),
                    PostprocessWarningType::IgnoredUserData
                )
            }
            if sound.outer_cone_gain != 0.0 && sound.outer_cone_gain != 1.0 {
                state.warn(
                    tag_path,
                    format_args!("Outer cone angle is 0 degrees; outer cone gain will be overridden to 1."),
                    PostprocessWarningType::IgnoredUserData
                )
            }
        }
        sound.inner_cone_angle = Angle::_360_DEG;
        sound.outer_cone_angle = Angle::_360_DEG;
        sound.outer_cone_gain = 1.0;
    } else if action.undefault() && sound.inner_cone_angle == Angle::_360_DEG && sound.outer_cone_angle == Angle::_360_DEG && sound.outer_cone_gain == 1.0 {
        sound.inner_cone_angle = Angle::_0_DEG;
        sound.outer_cone_angle = Angle::_0_DEG;
        sound.outer_cone_gain = 0.0;
    }
}

fn postprocess_split_permutations(is_split: bool, pitch_range_index: usize, actual_permutations: &mut [SoundPermutation], subpermutations: &mut [SoundPermutation]) -> Result<(), PostprocessError> {
    let actual_permutation_count = actual_permutations.len();

    let mut expected_subpermutation_index = 0;

    for (permutation_index, permutation) in actual_permutations.iter_mut().enumerate() {
        match permutation.next_permutation_index.index() {
            Some(mut raw_subpermutation_index) => {
                assert_postprocess!(
                    raw_subpermutation_index > 0,
                    "Permutation #{permutation_index} of pitch range #{pitch_range_index} points to actual permutation #0 which is invalid."
                );
                assert_postprocess!(
                    is_split,
                    "Permutation #{permutation_index} of pitch range #{pitch_range_index} appears to be split when split permutations are disabled."
                );

                let mut depth = 0usize;
                loop {
                    let Some(subpermutation_index) = raw_subpermutation_index.checked_sub(actual_permutation_count) else {
                        fail_postprocess!("Permutation #{permutation_index} of pitch range #{pitch_range_index} is incorrectly split (its next subpermutation points to another actual permutation #{raw_subpermutation_index}).");
                    };

                    // ensure that all subpermutations are sequential (also prevents infinite loops)
                    //
                    // allowed:
                    //
                    //   P1, P2, P3, S1a, S1b
                    //   P1, P2, P3, S1a, S1b, S2a, S2b, S3a, S3b
                    //   P1, P2, P3, S1a, S2a, S2b, S3a, S3b
                    //   P1, P2, P3, S2a, S2b, S3a, S3b
                    //   P1, P2, P3
                    //
                    // not allowed:
                    //
                    //   P1, P2, P3, S1b, S1a                     <- subpermutations not in order
                    //   P1, P2, P3, S1a, S2a, S3a, S1b           <- S1a and S1b are not adjacent
                    //   P1, P2, P3, S3a, S3b, S2a, S2b, S1a, S1b <- the first subpermutation does
                    //                                               not immediately follow the last
                    //                                               actual permutation
                    //   etc.
                    //
                    // where:
                    //   # = permutation #
                    //   P = actual permutation
                    //   S = subpermutation
                    //   a = first subpermutation
                    //   b = second subpermutation

                    assert_postprocess!(
                        subpermutation_index == expected_subpermutation_index,
                        "Pitch range #{pitch_range_index} has non-linear or out-of-order permutation-subpermutation ordering."
                    );

                    expected_subpermutation_index += 1;

                    let Some(subpermutation) = subpermutations.get_mut(subpermutation_index) else {
                        fail_postprocess!("Permutation #{permutation_index} of pitch range #{pitch_range_index} was cut off (subpermutation #{depth} is not present in the tag).");
                    };

                    // copy over the gain
                    subpermutation.gain = permutation.gain;

                    let Some(next_subpermutation_index) = subpermutation.next_permutation_index.index() else {
                        break;
                    };

                    raw_subpermutation_index = next_subpermutation_index;
                    depth += 1;
                }
            },
            None => continue
        }
    }

    // Ensure that all subpermutations were unaccounted for.
    let unreferenced_permutations = subpermutations.len() - expected_subpermutation_index;
    assert_postprocess!(unreferenced_permutations == 0, "Found {unreferenced_permutations} unreferenced sound permutation(s). This is invalid.");

    Ok(())
}

fn verify_sound_tag_format(sound: &mut Sound) -> Result<(), PostprocessError> {
    let channel_count = match sound.channel_count {
        SoundChannelCount::Stereo => 2usize,
        SoundChannelCount::Mono => 1usize
    };

    let sound_format = sound.format;

    for SoundPermutationIteration { permutation_index, pitch_range_index, permutation } in iterate_permutations(sound) {
        assert_postprocess!(
            permutation.format == sound_format,
            "Sound tag format is set to {sound_format} but contains {permutation_fmt} permutation(s)",
            permutation_fmt = permutation.format
        );

        let sample_len = permutation.samples.len();

        let expected_buffer_length;
        let expected_sample_data_divisiblity;

        match permutation.format {
            SoundFormat::PCM => {
                expected_buffer_length = sample_len;
                expected_sample_data_divisiblity = PCM_SAMPLE_SIZE_BYTES * channel_count;
            }

            SoundFormat::ImaADPCM | SoundFormat::XboxADPCM => {
                expected_buffer_length = 0;
                expected_sample_data_divisiblity = XBOX_ADPCM_BLOCK_SIZE_BYTES * channel_count;
            },

            SoundFormat::OggVorbis => {
                // TODO: Check Ogg header
                return Ok(())
            }
        }

        if expected_sample_data_divisiblity > 1 {
            assert_postprocess!(
                sample_len.is_multiple_of(expected_sample_data_divisiblity),
                "Permutation #{permutation_index} of pitch range #{pitch_range_index} has an invalid sound data length (not divisible by {expected_sample_data_divisiblity})."
            );
        }

        assert_postprocess!(
            (permutation.buffer_size as usize) == expected_buffer_length,
            "Permutation #{permutation_index} of pitch range #{pitch_range_index} has an incorrect buffer size (expected {expected_buffer_length}, got {actual_buffer_length}).",
            actual_buffer_length = permutation.buffer_size
        );
    }

    Ok(())
}

fn postprocess_pitch_ranges(sound: &mut Sound, tag_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    let sample_rate = match sound.sample_rate {
        SoundSampleRate::_22050Hz => 22050.0,
        SoundSampleRate::_44100Hz => 44100.0,
    };
    let channel_modifier = match sound.channel_count {
        SoundChannelCount::Mono => 1.0,
        SoundChannelCount::Stereo => 0.5,
    };

    let sample_rate_modifier = sample_rate
        * sound.random_pitch_bounds.from.min(sound.random_pitch_bounds.to)
        * sound.zero_pitch_modifier.min(sound.one_pitch_modifier);

    let mut longest_permutation_length = sound.longest_permutation_length;

    for (pitch_range_index, pitch_range) in &mut sound.pitch_ranges.iter_mut().enumerate() {
        let old_bend_bounds = pitch_range.bend_bounds;

        pitch_range.bend_bounds.from = pitch_range.bend_bounds.from.min(pitch_range.natural_pitch);
        pitch_range.bend_bounds.to = pitch_range.bend_bounds.to.max(pitch_range.natural_pitch);
        pitch_range.playback_rate = 1.0 / pitch_range.natural_pitch;

        let from_changed = old_bend_bounds.from != 0.0 && pitch_range.bend_bounds.from != old_bend_bounds.from;
        let to_changed = old_bend_bounds.to != 0.0 && pitch_range.bend_bounds.to != old_bend_bounds.to;

        if from_changed || to_changed {
            state.warn(
                tag_path,
                format_args!(
                    "Pitch range #{pitch_range_index}'s natural pitch {natural_pitch} was \
                         outside of bend bounds ({old_bend_bounds_from} to {old_bend_bounds_to}). \
                         The bend bounds have been clamped. ({new_bend_bounds_from} to {new_bend_bounds_to})",
                    natural_pitch = pitch_range.natural_pitch,
                    old_bend_bounds_from = old_bend_bounds.from,
                    old_bend_bounds_to = old_bend_bounds.to,
                    new_bend_bounds_from = pitch_range.bend_bounds.from,
                    new_bend_bounds_to = pitch_range.bend_bounds.to,
                ),
                PostprocessWarningType::IgnoredUserData
            );
        }

        for permutation in &pitch_range.permutations {
            longest_permutation_length = longest_permutation_length.max(
                calculate_permutation_length(permutation, pitch_range.natural_pitch, channel_modifier, sample_rate_modifier)
            );
        }
    }

    Ok(())
}

fn byteswap_pcm(sound: &mut Sound) -> Result<(), PostprocessError> {
    for permutation in iterate_permutations(sound) {
        if permutation.permutation.format != SoundFormat::PCM {
            continue
        }

        for i in permutation.permutation.samples.chunks_exact_mut(2) {
            let word: &mut [u8; 2] = i.try_into().unwrap();
            *word = u16::from_be_bytes(*word).to_le_bytes(); // swaps big to little (or little to big)
        }
    }

    Ok(())
}

struct SoundPermutationIteration<'a> {
    pitch_range_index: usize,
    permutation_index: usize,
    permutation: &'a mut SoundPermutation
}
fn iterate_permutations<'a>(sound: &'a mut Sound) -> impl Iterator<Item=SoundPermutationIteration<'a>> {
    sound.pitch_ranges.iter_mut().enumerate().map(|(pitch_range_index, pitch_range)| {
        pitch_range.permutations.iter_mut().enumerate().map(move |(permutation_index, permutation)| {
            SoundPermutationIteration {
                pitch_range_index, permutation_index, permutation
            }
        })
    }).flatten()
}


fn calculate_permutation_length(permutation: &SoundPermutation, natural_pitch: f32, channel_modifier: f32, sample_rate_modifier: f32) -> u32 {
    let format_modifier;
    let samples_size;

    match permutation.format {
        SoundFormat::PCM => {
            samples_size = permutation.buffer_size as f32;
            format_modifier = 0.5;
        }
        SoundFormat::OggVorbis => {
            // looks like a bug or some horrible hack (should be same as PCM), but tool.exe does it.
            //
            // this makes ogg vorbis sounds play for longer as a result
            samples_size = permutation.buffer_size as f32 / 4.0;
            format_modifier = 2.2;
        }
        SoundFormat::ImaADPCM | SoundFormat::XboxADPCM => {
            samples_size = permutation.samples.len() as f32;
            format_modifier = 2.0;
        }
    }

    let samples = samples_size * 1000.0 * channel_modifier * format_modifier * natural_pitch / sample_rate_modifier;
    samples as u32
}



fn default_min_max_distance_sounds_for_class(sound_class: SoundClass) -> Bounds<f32> {
    match sound_class {
        SoundClass::DeviceMachinery
        | SoundClass::DeviceForceField
        | SoundClass::AmbientMachinery
        | SoundClass::AmbientNature
        | SoundClass::DeviceDoor
        | SoundClass::Music
        | SoundClass::DeviceNature => Bounds { from: 0.9, to: 5.0 },

        SoundClass::WeaponEmpty
        | SoundClass::WeaponIdle
        | SoundClass::WeaponReady
        | SoundClass::WeaponReload
        | SoundClass::WeaponCharge
        | SoundClass::WeaponOverheat => Bounds { from: 1.0, to: 9.0 },

        SoundClass::ScriptedDialogOther
        | SoundClass::ScriptedDialogPlayer
        | SoundClass::GameEvent
        | SoundClass::UnitDialog
        | SoundClass::ScriptedDialogForceUnspatialized => Bounds { from: 3.0, to: 20.0 },

        SoundClass::FirstPersonDamage
        | SoundClass::ObjectImpacts
        | SoundClass::AmbientComputers
        | SoundClass::ParticleImpacts
        | SoundClass::DeviceComputers
        | SoundClass::SlowParticleImpacts => Bounds { from: 0.5, to: 3.0 },

        SoundClass::VehicleEngine
        | SoundClass::ProjectileImpact
        | SoundClass::VehicleCollision => Bounds { from: 1.4, to: 8.0 },

        SoundClass::WeaponFire => Bounds { from: 4.0, to: 70.0 },
        SoundClass::ScriptedEffect => Bounds { from: 2.0, to: 5.0 },
        SoundClass::ProjectileDetonation => Bounds { from: 8.0, to: 120.0 },
        SoundClass::UnitFootsteps => Bounds { from: 0.9, to: 10.0 }
    }
}

fn default_min_max_distance(sound: &mut Sound, action: Action) {
    let defaults = default_min_max_distance_sounds_for_class(sound.sound_class);
    apply_default_le_zero(&mut sound.distance_bounds.from, defaults.from, action);
    apply_default_le_zero(&mut sound.distance_bounds.to, defaults.to, action);
    // note: we can end up with distance from being greater than distance to
}

fn default_zero_gain_modifier_for_class(sound_class: SoundClass) -> f32 {
    match sound_class {
        SoundClass::ObjectImpacts
        | SoundClass::ParticleImpacts
        | SoundClass::SlowParticleImpacts
        | SoundClass::UnitDialog
        | SoundClass::Music
        | SoundClass::AmbientNature
        | SoundClass::AmbientMachinery
        | SoundClass::AmbientComputers
        | SoundClass::ScriptedDialogPlayer
        | SoundClass::ScriptedDialogOther
        | SoundClass::ScriptedDialogForceUnspatialized
        | SoundClass::ScriptedEffect => 0.0,

        _ => 1.0
    }
}

fn default_gain_modifiers(sound: &mut Sound, action: Action) {
    let mut zero_one_gains = [sound.zero_gain_modifier, sound.one_gain_modifier];
    apply_default(&mut zero_one_gains, [default_zero_gain_modifier_for_class(sound.sound_class), 1.0], action);
    sound.zero_gain_modifier = zero_one_gains[0];
    sound.one_gain_modifier = zero_one_gains[1];
}
