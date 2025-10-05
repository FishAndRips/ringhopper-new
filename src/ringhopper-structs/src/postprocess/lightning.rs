use crate::definitions::tag::lightning::Lightning;
use crate::postprocess::shader::postprocess_shader_effect;
use crate::postprocess::{apply_clamp, apply_minimum_value_clamp, Action};
use crate::{PostprocessError, PostprocessState, PostprocessWarningType, TagPath};

pub fn postprocess_lightning(lightning: &mut Lightning, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    apply_minimum_value_clamp(&mut lightning.count, 1, action);

    for marker in &mut lightning.markers {
        apply_clamp(&mut marker.octaves_to_next_marker, 0, 8, action);
    }

    for (index, shader) in &mut lightning.shader.iter_mut().enumerate() {
        postprocess_shader_effect(shader, action);

        if action.postprocess() && shader.secondary_map.bitmap.is_set() {
            // TODO: This reuses the struct from shader_effect, but the secondary map reference is
            //       officially unsupported by the game, although some versions of the game may
            //       actually render it.
            state.warn(
                tag_path,
                format_args!("Lightning shader #{index} references a secondary map, but secondary maps are unsupported for lightning tags."),
                PostprocessWarningType::UnusedData
            );
            shader.secondary_map.bitmap.clear();
        }
    }

    Ok(())
}
