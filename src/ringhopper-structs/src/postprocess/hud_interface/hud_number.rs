use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, TagPath};
use crate::definitions::tag::bitmap::{Bitmap, BitmapType};
use crate::definitions::tag::hud_number::HUDNumber;

pub fn postprocess_hud_number(hud_number: &mut HUDNumber, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let Some(hud_number_bitmap_path) = hud_number.digits_bitmap.get() else {
        fail_postprocess!("HUD numbers is missing a digits bitmap. This is invalid.")
    };

    let hud_number_bitmap = state.read_tag_group::<Bitmap>(hud_number_bitmap_path).expect("numbers not a bitmap");
    let sequence = &hud_number_bitmap.bitmap_group_sequence[0];
    let sequence_frame_count = match hud_number_bitmap._type {
        BitmapType::Sprites => sequence.sprites.len(),
        BitmapType::_2dTextures | BitmapType::InterfaceBitmaps => sequence.bitmap_count as usize,
        _ => fail_postprocess!("HUD number bitmap is not a 2D texture, interface bitmap, or sprite sheet")
    };
    
    assert_postprocess!(
        sequence_frame_count >= 15,
        "Sequence 0 of the digits bitmap does not have at least 15 frames."
    );

    Ok(())
}
