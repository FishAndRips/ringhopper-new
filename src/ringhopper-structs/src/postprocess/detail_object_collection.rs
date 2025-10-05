use crate::{PostprocessError, PostprocessState, TagPath};
use crate::definitions::tag::bitmap::{Bitmap, BitmapType};
use crate::definitions::tag::detail_object_collection::DetailObjectCollection;
use crate::postprocess::Action;

pub fn postprocess_detail_object_collection(detail_object_collection: &mut DetailObjectCollection, action: Action, _tag_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    // Let's begin.

    if !action.postprocess() {
        return Ok(())
    }

    let sprite_plate_path = detail_object_collection.sprite_plate.get().expect("no sprite plate");
    let sprite_plate = state.read_tag_group::<Bitmap>(sprite_plate_path).expect("not a bitmap");

    assert_postprocess!(sprite_plate._type == BitmapType::Sprites, "Sprite plate is not a sprites bitmap but instead a {}", sprite_plate._type);

    for (index, detail_type) in detail_object_collection.types.iter_mut().enumerate() {
        let sequence_index = detail_type.sequence_index as usize;
        let Some(sequences) = sprite_plate
            .bitmap_group_sequence
            .get(0..=sequence_index) else {
                fail_postprocess!("Collection type #{index} references an out-of-bounds sequence index {sequence_index} for bitmap {sprite_plate_path}");
            };

        let all_sequences_before = &sequences[..sequence_index];

        // First we need to count all sprites before this one
        let mut sprites = 0usize;
        for i in all_sequences_before {
            sprites = sprites.saturating_add(i.sprites.len());
        }

        // Can we store it in an 8-bit number?
        assert_postprocess!(sprites <= u8::MAX as usize, "Collection type #{index}'s first frame index exceeds 255");

        // Can we store the sprite count in an 8-bit number?
        let sprite_count = sequences[sequence_index].sprites.len();
        assert_postprocess!(sprite_count <= u8::MAX as usize, "Collection type #{index}'s sprite count exceeds 255");

        // Done.
        detail_type.first_sprite_index = sprites as u8;
        detail_type.sprite_count = sprite_count as u8;
    }

    Ok(())
}
