pub mod unit;
pub mod device_machine;
pub mod device;
pub mod weapon;
pub mod equipment;
pub mod vehicle;
pub mod biped;
pub mod projectile;

use crate::definitions::tag::model_animations::ModelAnimations;
use crate::definitions::tag::model_collision_geometry::ModelCollisionGeometry;
use crate::definitions::tag::object::{Object, ObjectChangeColors, ObjectFunction};
use crate::definitions::tag::weapon::Weapon;
use crate::definitions::tag::object_type_of_tag_group;
use crate::postprocess::{apply_default, apply_minimum_value_clamp, Action};
use crate::{Bounds, ModelFns, PostprocessError, PostprocessState, PostprocessWarningType, TagPath, TagReference};
use alloc::borrow::Cow;
use alloc::format;
use funnel_web::nudge::fix_decimal_rounding;
use crate::definitions::tag::enums::FunctionScaleBy;

pub fn postprocess_object(object: &mut Object, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for function in &mut object.functions {
        apply_default(&mut function.period, 1.0, action);

        // This one's default is special: if from = to, then default to 0 to 1
        const DEFAULT_BOUNDS: Bounds<f32> = Bounds {
            from: 0.0,
            to: 1.0
        };
        const ZERO_BOUNDS: Bounds<f32> = Bounds {
            from: 0.0,
            to: 0.0
        };
        if action.default() {
            if function.bounds.from == function.bounds.to {
                function.bounds = DEFAULT_BOUNDS
            }
        }
        else if action.undefault() {
            if function.bounds == DEFAULT_BOUNDS {
                function.bounds = ZERO_BOUNDS
            }
        }
    }

    apply_minimum_value_clamp(&mut object.bounding_radius, 0.0, action);
    apply_minimum_value_clamp(&mut object.render_bounding_radius, object.bounding_radius, action);

    if action.postprocess() {
        let group = tag_path.group();

        object._type = object_type_of_tag_group(group).ok_or_else(|| PostprocessError::GenericError {
            explanation: Cow::Owned(format!("Can't postprocess_object on a {group}."))
        })?;

        if let Some(model_path) = object.model.get() {
            let model = state.read_model(model_path)
                .expect("could not read model");

            if let Some(animation) = object.animation_graph.get() {
                check_model_and_animation_checksums(model_path, model, animation, state)?;
            }

            if let Some(collision) = object.collision_model.get() {
                check_model_and_collision(model_path, model, collision, state)?;
            }

        }
        else if object.animation_graph.is_set() {
            fail_postprocess!("Object references an animation tag without a model.");
        }
        else if object.collision_model.is_set() {
            fail_postprocess!("Object references a collision tag without a model.");
        }

        for function in &mut object.functions {
            postprocess_object_function(function);
        }
    }

    for (change_colors_index, change_colors) in object.change_colors.iter_mut().enumerate() {
        if action.postprocess() && change_colors.scale_by != FunctionScaleBy::None {
            object.runtime_flags.functions_control_color_scale = true;
        }
        handle_change_colors(change_colors_index, change_colors, action, tag_path, state)?;
    }

    Ok(())
}

// Assumes all defaults were set
fn postprocess_object_function(function: &mut ObjectFunction) {
    function.inverse_bounds = 1.0 / (function.bounds.to - function.bounds.from);
    function.inverse_period = 1.0 / function.period;

    if function.step_count > 1 {
        function.inverse_step = 1.0 / (function.step_count - 1) as f32;
    }
    if function.sawtooth_count > 1 {
        function.inverse_sawtooth = 1.0 / (function.sawtooth_count - 1) as f32;
    }
}

fn check_model_and_animation_checksums(model_path: &TagPath, model: &dyn ModelFns, animation_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    let animation = state.read_tag_group::<ModelAnimations>(animation_path).expect("not an animation tag");

    let node_list_checksum = model.node_list_checksum();

    // All the other animations are checked in postprocess_animations
    if let Some(animation) = animation.animations.first() {
        let checksum_bypassed = animation.node_list_checksum == 0 || node_list_checksum == 0;

        if animation.node_count as usize != model.nodes().len() || (animation.node_list_checksum != node_list_checksum && !checksum_bypassed) {
            let mut failure_message = format!("Model {model_path} has a different node list from animation {animation_path} and is not compatible.");
            if checksum_bypassed {
                failure_message += " (this cannot be bypassed by setting the node list checksum to 0 because the size of the node list is different!)"
            }
            fail_postprocess!("{failure_message}")
        }
    }

    Ok(())
}

pub(super) fn check_model_and_collision(model_path: &TagPath, model: &dyn ModelFns, collision_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    let collision = state.read_tag_group::<ModelCollisionGeometry>(collision_path)
        .expect("not a collision model");

    let model_regions = model.regions().as_slice();
    let collision_regions = collision.regions.as_slice();
    
    assert_postprocess!(
        model_regions.len() == collision_regions.len(),
        "Model {model_path} has a different region count from collision model {collision_path} and is not compatible."
    );

    // Check to make sure all permutation counts are the same
    //
    // ...unless all collision permutations have only 1 for some reason, according to tool.exe (TODO: find out why this is)
    let all_collision_regions_have_one_permutation = collision_regions
        .iter()
        .all(|i| i.permutations.len() == 1);

    if !all_collision_regions_have_one_permutation {
        for (index, (model_region, collision_region)) in model_regions.iter().zip(collision_regions.iter()).enumerate() {
            let model_permutation_count = model_region.permutations.len();
            let collision_permutation_count = collision_region.permutations.len();

            assert_postprocess!(
                model_permutation_count == collision_permutation_count,
                "Region {index} of model {model_path} has a different permutation count from collision model {collision_path} (and permutation count on collision != 1) and is not compatible."
            );
        }
    }

    Ok(())
}

fn handle_change_colors(change_colors_index: usize, change_colors: &mut ObjectChangeColors, action: Action, tag_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    // No permutations
    if change_colors.permutations.is_empty() {
        return Ok(());
    }

    // If *all* weights are 0, we can default them to 1.
    if action.default() {
        if change_colors.permutations.iter().all(|i| i.weight == 0.0) {
            change_colors.permutations.iter_mut().for_each(|i| i.weight = 1.0);
        }
    }

    // Then we can postprocess them
    if action.postprocess() {
        postprocess_change_colors(change_colors_index, change_colors, tag_path, state)?;
    }

    // Otherwise, un-postprocess them
    if action.unpostprocess() {
        unpostprocess_change_colors(change_colors);
    }

    // Then nudge the result
    if action.nudge() {
        nudge_change_colors(change_colors);
    }

    // All of them are the same; undefault to 0.0
    if action.undefault() {
        let first_permutation = change_colors.permutations[0].weight;
        if change_colors.permutations.iter().all(|i| i.weight == first_permutation) {
            change_colors.permutations.iter_mut().for_each(|i| i.weight = 0.0);
        }
    }

    Ok(())
}

fn postprocess_change_colors(change_color_index: usize, change_colors: &mut ObjectChangeColors, tag_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    let permutation_count = change_colors.permutations.len();
    match permutation_count {
        0 => unreachable!("postprocess_change_colors with 0 permutations (should have been caught earlier)"),
        1 => {
            // skip doing everything below, as this will anyway be the result
            change_colors.permutations[0].weight = 1.0;
            return Ok(())
        },
        _ => ()
    };

    // Prevent some possible weirdness
    for (permutation_index, permutation) in change_colors.permutations.iter().enumerate() {
        if permutation.weight < 0.0 {
            fail_postprocess!("Change color permutation #{permutation_index} of change color #{change_color_index} has \
                               a weight less than 0.0 ({})", permutation.weight)
        }

        // If there are any zero-weight permutations, we would have defaulted them to 1.0 if they were all 0.0.
        // That means any remaining zero-weight permutations are actually unused, essentially.
        else if permutation.weight == 0.0 {
            state.warn(
                tag_path,
                format_args!("Change color permutation #{permutation_index} of change color #{change_color_index} \
                                   has a weight of 0, but all other permutations' weights are set. As such, it will not \
                                   get defaulted and thus have zero weight (and thus won't get selected)."),
                PostprocessWarningType::UnusedData
            )
        }
    }

    // Add up the weights and also increment each individual weight so we can divide later
    let mut total_weight = 0.0;
    for permutation in &mut change_colors.permutations {
        let permutation_weight = permutation.weight;
        permutation.weight += total_weight;
        total_weight += permutation_weight;
    }

    // Now divide
    for permutation in &mut change_colors.permutations {
        permutation.weight /= total_weight;
    }

    Ok(())
}

fn unpostprocess_change_colors(change_colors: &mut ObjectChangeColors) {
    let permutation_count = change_colors.permutations.len();
    match permutation_count {
        0 => unreachable!("unpostprocess_change_colors with 0 permutations (should have been caught earlier)"),
        1 => {
            // Zero out the weight (it will be defaulted to 1.0 on build later)
            change_colors.permutations[0].weight = 0.0;
            return
        }
        _ => ()
    }

    let mut highest_weight: f32 = 0.0;
    let mut last_normalized_weight: f32 = 0.0;
    
    for (index, permutation) in change_colors.permutations.iter_mut().enumerate() {
        // If it's lower than last_normalized_weight, it can't be drawn and will have a weight of 0.0.
        // If it's higher than 1.0, it might as well be 1.0 since the game won't generate a higher random number.
        //
        // This can only happen if the map was tampered with, but since we're extracting, we aren't too picky on our tags,
        // and this is simply how it would have worked in-game in this state.
        permutation.weight = permutation.weight.clamp(last_normalized_weight, 1.0);

        let permutation_weight = permutation.weight - last_normalized_weight;
        last_normalized_weight = permutation.weight;
        permutation.weight = permutation_weight;
        highest_weight = highest_weight.max(permutation_weight);

        if last_normalized_weight == 1.0 {
            // Anything after this cannot be drawn.
            for remaining_permutations in change_colors.permutations
                .iter_mut()
                .skip(index + 1) {
                remaining_permutations.weight = 0.0;
            }
            break;
        }
    }

    if highest_weight <= 0.0 {
        // wellp, the tags are fucked now
        return
    }

    // Meme up the weights
    for weight in &mut change_colors.permutations {
        weight.weight /= highest_weight;
    }
}

fn nudge_change_colors(change_colors: &mut ObjectChangeColors) {
    // Fix editor rounding
    for permutation in &mut change_colors.permutations {
        permutation.weight = fix_decimal_rounding(permutation.weight);
    }
}

pub fn weapon_reference_must_be_readied<const I: usize>(weapon: &TagReference<I>, state: &mut dyn PostprocessState) -> Option<bool> {
    let path = weapon.get()?;
    let weapon = state.read_tag_group::<Weapon>(path)?;

    Some(weapon.weapon_flags.must_be_readied)
}

// TODO: When finished, make sure all things that can reference weapon tags call this (if they aren't flag/oddball references)
pub fn assert_weapon_reference_not_readied<const I: usize>(args: core::fmt::Arguments, weapon: &TagReference<I>, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    assert_postprocess!(
        weapon_reference_must_be_readied(weapon, state) != Some(true),
        "Weapon {args} ({path}) is set to 'must be readied' which is not valid.",
        path=weapon.get().expect("we just checked this tag...")
    );
    Ok(())
}
