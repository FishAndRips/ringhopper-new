use crate::definitions::tag::item_collection::ItemCollection;
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, PostprocessWarningType, TagPath};
use crate::postprocess::object::assert_weapon_reference_not_readied;

pub fn postprocess_item_collection(item_collection: &mut ItemCollection, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let mut total = 0u32;

    for (item_index, item) in item_collection.permutations.iter().enumerate() {
        assert_weapon_reference_not_readied(
            format_args!("item #{item_index}"),
            &item.item,
            state
        )?;

        let weight = item.weight;
        assert_postprocess!(weight >= 0.0, "Item #{item_index} has a negative weight of {weight}.");

        let integer_weight = (weight as u32).min(i32::MAX as u32);
        if weight != integer_weight as f32 {
            if integer_weight == 0 {
                state.warn(
                    tag_path,
                    format_args!("Item #{item_index} has a non-integer weight {weight}; this will be truncated to {integer_weight} (and thus will not spawn)"),
                    PostprocessWarningType::ItemCollectionWeights
                )
            }
            else {
                state.warn(
                    tag_path,
                    format_args!("Item #{item_index} has a non-integer weight {weight}; this will be truncated to {integer_weight}"),
                    PostprocessWarningType::ItemCollectionWeights
                )
            }
        }
        else if integer_weight == 0 {
            state.warn(
                tag_path,
                format_args!("Item #{item_index} has a weight of 0 (and thus will not spawn)"),
                PostprocessWarningType::ItemCollectionWeights
            )
        }

        total = total.saturating_add(integer_weight);
    }

    assert_postprocess!(
        total <= (u16::MAX as u32),
        "Item collection has a total weight of {total}."
    );

    Ok(())
}
