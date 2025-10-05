use crate::definitions::tag::sound_environment::SoundEnvironment;
use crate::postprocess::{apply_clamp, Action};
use crate::PostprocessError;

#[inline]
pub fn postprocess_sound_environment(sound_environment: &mut SoundEnvironment, action: Action) -> Result<(), PostprocessError> {
    apply_clamp(&mut sound_environment.room_intensity, 0.0, 1.0, action);
    apply_clamp(&mut sound_environment.room_intensity_hf, 0.0, 1.0, action);
    apply_clamp(&mut sound_environment.reflections_intensity, 0.0, 1.0, action);
    apply_clamp(&mut sound_environment.reverb_delay, 0.0, 0.1, action);
    apply_clamp(&mut sound_environment.diffusion, 0.0, 1.0, action);
    apply_clamp(&mut sound_environment.density, 0.0, 1.0, action);
    apply_clamp(&mut sound_environment.reverb_intensity, 0.0, 1.0, action);
    apply_clamp(&mut sound_environment.hf_reference, 20.0, 20000.0, action);

    apply_clamp(&mut sound_environment.room_rolloff, 0.0, 10.0, action);
    apply_clamp(&mut sound_environment.decay_time, 0.0, 20.0, action);
    apply_clamp(&mut sound_environment.decay_hf_ratio, 0.0, 2.0, action);
    apply_clamp(&mut sound_environment.reflections_delay, 0.0, 0.3, action);

    Ok(())
}
