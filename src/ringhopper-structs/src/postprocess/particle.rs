use super::shader::postprocess_shader_effect;
use crate::definitions::tag::bitmap::{Bitmap, BitmapType};
use crate::definitions::tag::particle::Particle;
use crate::definitions::tag::particle_system::ParticleSystem;
use crate::definitions::tag::weather_particle_system::WeatherParticleSystem;
use crate::postprocess::{apply_clamp, apply_default, apply_maximum_value_clamp, apply_default_le_zero, Action};
use crate::{PostprocessError, PostprocessState, TagPath};
use funnel_web::constants::TICK_RATE_INVERSE;

pub fn postprocess_particle(particle: &mut Particle, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    postprocess_shader_effect(&mut particle.shader, action);

    const DEFAULT_FADE_START_SIZE: f32 = 5.0;
    const DEFAULT_FADE_END_SIZE: f32 = 4.0;

    if action.default() && (particle.fade_start_size <= 0.0 || particle.fade_end_size <= 0.0) {
        particle.fade_start_size = DEFAULT_FADE_START_SIZE;
        particle.fade_end_size = DEFAULT_FADE_END_SIZE;
    }
    else if action.undefault() && particle.fade_start_size == DEFAULT_FADE_START_SIZE && particle.fade_end_size == DEFAULT_FADE_END_SIZE {
        particle.fade_start_size = 0.0;
        particle.fade_end_size = 0.0;
    }

    apply_clamp(&mut particle.animation_rate.from, 0.0, 90.0, action);
    apply_clamp(&mut particle.animation_rate.to, particle.animation_rate.from, 90.0, action);

    apply_default_le_zero(&mut particle.radius_animation.from, 1.0, action);
    apply_default_le_zero(&mut particle.radius_animation.to, 1.0, action);

    if action.postprocess() || action.unpostprocess() {
        postprocess_particle_cross_tag(particle, action, state)?;
    }

    // Setting this to anything besides 0 leads to the game exploding violently. It's commented out
    // because we do not even handle this value anymore, and tool always sets it to 0.
    //
    // particle.contact_deterioration = 0.0;

    Ok(())
}

fn postprocess_particle_cross_tag(particle: &mut Particle, action: Action, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    let Some(p) = particle.bitmap.get() else {
        fail_postprocess!("Missing sprite bitmap for particle");
    };

    let bitmap: &Bitmap = state.read_tag_group::<Bitmap>(p).expect("sprite bitmap wasn't a bitmap");

    if action.postprocess() {
        super::bitmap::assert_bitmap_type(bitmap, BitmapType::Sprites, &format_args!("particle"), p)?;
        particle.sprite_size = find_sprite_length_for_bitmap_by_path(bitmap, p)?;
    }

    let Some(final_sequence_offset) = particle.first_sequence_index.index()
        .map(|i| i + particle.initial_sequence_count as usize + particle.looping_sequence_count as usize)
        else {
            fail_postprocess!("No first sequence index")
        };

    let final_sequence_count = particle.final_sequence_count as usize;

    let total_expected = final_sequence_offset + final_sequence_count;
    let actual_sequence_count = bitmap.bitmap_group_sequence.len();

    assert_postprocess!(actual_sequence_count >= total_expected, "Expected {total_expected} sequences, but only {actual_sequence_count} were found");

    if particle.final_sequence_count > 0 {
        let frame_rate = if particle.flags.animate_once_per_frame {
            TICK_RATE_INVERSE
        } else {
            particle.animation_rate.to
        };

        let max_sprites = bitmap.bitmap_group_sequence
            .iter()
            .skip(final_sequence_offset)
            .take(final_sequence_count)
            .map(|i| i.sprites.len())
            .max()
            .expect("should have gotten at least one sequence");

        let max_duration = max_sprites as f32 * frame_rate;

        // yay, floating point precision fun... 😔
        if action.postprocess() {
            particle.lifespan.to -= max_duration;
            particle.lifespan.from -= max_duration;
            assert_postprocess!(particle.lifespan.from >= 0.0 && particle.lifespan.to >= 0.0, "Final sequence is longer than the particle's total lifespan.");
        } else if action.unpostprocess() {
            particle.lifespan.to += max_duration;
            particle.lifespan.from += max_duration;
        }
    }

    Ok(())
}

pub fn postprocess_particle_system(system: &mut ParticleSystem, action: Action) -> Result<(), PostprocessError> {
    for particle_type in &mut system.particle_types {
        for state in &mut particle_type.particle_states {
            postprocess_shader_effect(&mut state.shader, action);
            apply_default_le_zero(&mut state.radius_multiplier, 1.0, action);
        }
        for state in &mut particle_type.states {
            apply_default_le_zero(&mut state.radius_multiplier, 1.0, action);
        }
    }

    Ok(())
}


pub fn postprocess_weather_particle_system(system: &mut WeatherParticleSystem, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for (index, particle_type) in system.particle_types.iter_mut().enumerate() {
        if action.postprocess() {
            let Some(p) = particle_type.sprite_bitmap.get() else {
                fail_postprocess!("Missing sprite bitmap for particle type #{index}");
            };

            let bitmap = state.read_tag_group::<Bitmap>(p).expect("sprite bitmap wasn't a bitmap");
            super::bitmap::assert_bitmap_type(bitmap, BitmapType::Sprites, &format_args!("sprite #{index}"), p)?;
            particle_type.sprite_size = find_sprite_length_for_bitmap_by_path(bitmap, p)?;
        }

        postprocess_shader_effect(&mut particle_type.shader, action);

        apply_default_le_zero(&mut particle_type.fade_in_start_distance, 0.0, action);
        apply_maximum_value_clamp(&mut particle_type.fade_in_end_distance, particle_type.fade_in_start_distance, action);
        apply_maximum_value_clamp(&mut particle_type.fade_out_start_distance, particle_type.fade_in_end_distance, action);
        apply_maximum_value_clamp(&mut particle_type.fade_out_end_distance, particle_type.fade_out_start_distance, action);

        apply_default(&mut particle_type.fade_out_end_distance, 1.0, action);

        if action.default()
            && particle_type.fade_in_start_height == 0.0
            && particle_type.fade_in_end_height == 0.0
            && particle_type.fade_out_start_height == 0.0
            && particle_type.fade_out_end_height == 0.0 {
            particle_type.fade_in_start_height = f32::MIN;
            particle_type.fade_in_end_height = f32::MIN;
            particle_type.fade_out_start_height = f32::MAX;
            particle_type.fade_out_end_height = f32::MAX;
        }
        else if action.undefault()
            && particle_type.fade_in_start_height == f32::MIN
            && particle_type.fade_in_end_height == f32::MIN
            && particle_type.fade_out_start_height == f32::MAX
            && particle_type.fade_out_end_height == f32::MAX {
            particle_type.fade_in_start_height = 0.0;
            particle_type.fade_in_end_height = 0.0;
            particle_type.fade_out_start_height = 0.0;
            particle_type.fade_out_end_height = 0.0;
        }
    }

    Ok(())
}

fn find_sprite_length_for_bitmap_by_path(bitmap: &Bitmap, tag: &TagPath) -> Result<f32, PostprocessError> {
    match find_sprite_length_for_bitmap(bitmap) {
        Ok(f) => Ok(f),
        Err(e) => fail_postprocess!("Can't calculate length of sprites for bitmap {tag}: {e}")
    }
}

fn find_sprite_length_for_bitmap(bitmap: &Bitmap) -> Result<f32, &'static str> {
    if bitmap._type != BitmapType::Sprites {
        return Err("not a sprite sheet")
    }

    let mut longest_length = 0.0f32;
    for sequence in &bitmap.bitmap_group_sequence {
        for sprite in &sequence.sprites {
            let bitmap_data = bitmap.bitmap_data
                .get(sprite.bitmap_index.index().expect("no bitmap index set"))
                .expect("out-of-bounds bitmap index on sprite");

            let sprite_width = (sprite.right - sprite.left) * bitmap_data.width as f32;
            let sprite_height = (sprite.bottom - sprite.top) * bitmap_data.height as f32;

            let max_length = sprite_height.max(sprite_width);
            longest_length = longest_length.max(max_length);
        }
    }

    if longest_length == 0.0 {
        return Err("no sprites, or all sprites are zero-length (or less!)")
    }

    Ok(1.0 / longest_length)
}
