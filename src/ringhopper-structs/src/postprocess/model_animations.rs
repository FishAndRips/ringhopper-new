use crate::definitions::tag::model_animations::{AnimationFrameInfoType, ModelAnimations, ModelAnimationsFrameInfoDxDy, ModelAnimationsFrameInfoDxDyDyaw, ModelAnimationsFrameInfoDxDyDzDyaw};
use crate::postprocess::{postprocess_byteswap_big_to_little, Action};
use crate::{PostprocessError, PostprocessState, ReflexiveIndex, SimpleWriteableData, TagPath};
use funnel_web::float::FloatOps;
use funnel_web::id::Index;

#[expect(unused_variables)]
pub fn postprocess_model_animations(model_animations: &mut ModelAnimations, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        set_indices(model_animations)?;
        validate_animation_data(model_animations)?;
    }

    if action.postprocess() || action.unpostprocess() {
        byteswap_frame_info(model_animations, action)?;
        todo!("byteswap the frame data");
    }

    Ok(())
}

fn set_indices(model_animations: &mut ModelAnimations) -> Result<(), PostprocessError> {
    // Initialize everything
    let animation_count = model_animations.animations.len();
    for animation in &mut model_animations.animations {
        if model_animations.sound_references.is_empty() {
            animation.sound = ReflexiveIndex::new();
        }
        animation.main_animation_index = Index::new();
    }

    // Set everything.
    for starting_animation_index in 0..animation_count {
        // Is this a sub-animation?
        if model_animations.animations[starting_animation_index].main_animation_index.is_null() {
            continue
        }

        // First, we want to get the total weight.
        let mut total_weight = 0.0f32;
        let mut animation_index = Some(starting_animation_index);
        while let Some(current_index) = animation_index {
            let Some(animation) = model_animations.animations.get_mut(current_index) else {
                fail_postprocess!("Animation #{starting_animation_index} eventually has out-of-bounds next animation index #{current_index}");
            };

            // tool.exe handles this by nulling animation indices (which ends up leaving some animations orphaned potentially)
            // there aren't any stock tag animations that have this
            assert_postprocess!(!animation.main_animation_index.is_null(), "Animation #{current_index} is part of multiple animations.");

            total_weight += if animation.weight.fw_is_close_to_zero() { 1.0 } else { animation.weight.fw_fabs() };
            animation.main_animation_index = Index::from_usize(starting_animation_index).expect("starting_animation_index exceeded an Index somehow");
            animation_index = animation.next_animation.index();
        }

        // Next, we want to update the partial weight.
        let mut animation_index = Some(starting_animation_index);
        let mut current_weight = 0.0;
        while let Some(current_index) = animation_index {
            let animation = &mut model_animations.animations[current_index];

            current_weight += if animation.weight.fw_is_close_to_zero() { 1.0 } else { animation.weight.fw_fabs() };
            animation.relative_weight = current_weight / total_weight;
            animation_index = animation.next_animation.index();
        }
    }

    Ok(())
}

fn validate_animation_data(model_animations: &mut ModelAnimations) -> Result<(), PostprocessError> {
    for (animation_index, animation) in model_animations.animations.iter().enumerate() {
        let is_compressed = animation.flags.compressed_data;

        assert_postprocess!(
            is_compressed == model_animations.flags.compress_all_animations,
            "Animation #{animation_index}'s compression flag does not match the tag's compression setting (animation needs reprocessed)"
        );

        if animation.frame_size as usize != animation.frame_data.len() {
            assert_postprocess!(
                is_compressed && ((animation.frame_size as usize) <= animation.frame_data.len()),
                "Animation #{animation_index}'s frame size does not match the actual frame data length"
            );
        }

        // This is correct, right?
        assert_postprocess!(
            is_compressed && animation.offset_to_compressed_data != animation.frame_size as u32,
            "Animation #{animation_index}'s offset to compressed data is incorrect"
        );
    }

    todo!()
}

fn byteswap_frame_info(model_animations: &mut ModelAnimations, action: Action) -> Result<(), PostprocessError> {
    for (index, animation) in model_animations.animations.iter_mut().enumerate() {
        let frame_info_type = animation.frame_info_type;
        let frame_count = animation.frame_count as usize;

        fn byteswap_frame_info_chunk<T: SimpleWriteableData>(data: &mut [u8], index: usize, frame_count: usize, frame_info_type: AnimationFrameInfoType, action: Action) -> Result<(), PostprocessError> {
            let expected_len = frame_count * T::length();
            assert_postprocess!(expected_len == data.len(), "Frame info length is incorrect for animation #{index} ({frame_info_type}) (expected {expected_len}, was {})", data.len());

            for i in data.chunks_mut(T::length()) {
                postprocess_byteswap_big_to_little::<T>(i, &format_args!("frame info for animation #{index} ({frame_info_type})"), action)?;
            }

            Ok(())
        }

        match frame_info_type {
            AnimationFrameInfoType::None => {
                assert_postprocess!(animation.frame_info.is_empty(), "Frame info type is none, but frame info data was set");
            }
            AnimationFrameInfoType::DxDy => byteswap_frame_info_chunk::<ModelAnimationsFrameInfoDxDy>(animation.frame_info.as_mut_slice(), index, frame_count, frame_info_type, action)?,
            AnimationFrameInfoType::DxDyDyaw => byteswap_frame_info_chunk::<ModelAnimationsFrameInfoDxDyDyaw>(animation.frame_info.as_mut_slice(), index, frame_count, frame_info_type, action)?,
            AnimationFrameInfoType::DxDyDzDyaw => byteswap_frame_info_chunk::<ModelAnimationsFrameInfoDxDyDzDyaw>(animation.frame_info.as_mut_slice(), index, frame_count, frame_info_type, action)?
        }
    }

    Ok(())
}
