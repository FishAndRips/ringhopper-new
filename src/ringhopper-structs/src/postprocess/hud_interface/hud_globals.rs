use funnel_web::id::Index;
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, TagPath};
use crate::definitions::tag::bitmap::{Bitmap, BitmapType};
use crate::definitions::tag::hud_globals::{HUDGlobals, HUDGlobalsWaypointArrow};
use crate::postprocess::hud_interface::{fixup_scale, load_and_verify_bitmap_for_hud_from_dependency};

pub fn postprocess_hud_globals(hud_globals: &mut HUDGlobals, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    fixup_scale(&mut hud_globals.messaging_parameters.position, action);

    if !action.postprocess() {
        return Ok(())
    }

    if let Some(arrow_bitmap) = load_and_verify_bitmap_for_hud_from_dependency(&hud_globals.waypoint_parameters.arrow_bitmap, &format_args!("Waypoint parameters"), state)? {
        verify_arrow_bitmap(arrow_bitmap, &hud_globals.waypoint_parameters.waypoint_arrows)?;
    }

    Ok(())
}

fn verify_arrow_bitmap(bitmap: &Bitmap, arrows: &[HUDGlobalsWaypointArrow]) -> Result<(), PostprocessError> {
    for (arrow_index, arrow) in arrows.iter().enumerate() {
        let check_sequence_index = |i: Index, name: &str| {
            let Some(i) = i.index().and_then(|i| bitmap.bitmap_group_sequence.get(i)) else {
                fail_postprocess!("Arrow {name} of arrow #{arrow_index} does not point to a valid bitmap group sequence.")
            };

            if bitmap._type == BitmapType::Sprites {
                assert_postprocess!(!i.sprites.is_empty(), "Empty sprite sequence for {name} of arrow #{arrow_index}.")
            }
            else {
                assert_postprocess!(i.bitmap_count > 0, "Empty bitmap sequence for {name} of arrow #{arrow_index}.")
            }

            Ok(())
        };

        check_sequence_index(arrow.occluded_sequence_index, "occluded sequence index")?;
        check_sequence_index(arrow.on_screen_sequence_index, "on-screen sequence index")?;
        check_sequence_index(arrow.off_screen_sequence_index, "off-screen sequence index")?;
    }

    Ok(())
}
