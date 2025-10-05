use crate::definitions::tag::decal::{Decal, DecalLayer};
use crate::postprocess::{apply_default, Action};
use crate::{Bounds, PostprocessError, PostprocessState, TagPath};
use funnel_web::color::ColorRGB;
use crate::definitions::tag::bitmap::{Bitmap, BitmapType};

pub fn postprocess_decal(decal: &mut Decal, action: Action, _tag_path: &TagPath, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    decal.animation.animation_speed = decal.animation.animation_speed.clamp(1, 120);

    apply_default(&mut decal.appearance.intensity, Bounds { from: 1.0, to: 1.0 }, action);
    apply_default(&mut decal.appearance.color, Bounds { from: ColorRGB::WHITE, to: ColorRGB::WHITE }, action);

    if decal.appearance.intensity.to > 0.0 {
        apply_default(&mut decal.appearance.intensity.from, 1.0 / 32.0, action);
    }

    // There is no good way to really undefault intensity's upper bound, so we gate it behind
    // action.default() here.
    //
    // Setting it to 0 or some other fan-fictioned value just to undefault it will undoubtedly lead
    // to regret.
    if action.default() {
        decal.appearance.intensity.to = decal.appearance.intensity.to.max(decal.appearance.intensity.from);
    }

    if action.postprocess() {
        // These fields are editable in loose tags with the official tools, but their values are
        // overridden on build like here.
        decal.properties.flags.water_effect = decal.properties.layer == DecalLayer::Water;

        if decal.properties.flags.sprite_scale_bug_fix && let Some(path) = decal.shader.map.get() {
            let bitmap = state.read_tag_group::<Bitmap>(path)
                .expect("map needs to be a bitmap");

            decal.maximum_sprite_extent = find_decal_sprite_extent_for_bitmap(bitmap);
        }
        else {
            // tool.exe has a bug where it hardcodes the extent to 16.
            //
            // This is because this is intended to be a fallback, and this function was accidentally
            // run before the bitmap is actually loaded.
            //
            // As such, we end up with decals (that use sprites) being the wrong size. This is why
            // blood splats, for example, are cartoonishly large in Halo: CE, and it is why the
            // plasma burn (from plasma projectiles hitting the world) is so weirdly proportioned.
            //
            // Obviously this changes the game's appearance drastically, so we're keeping the fix
            // behind a flag.
            decal.maximum_sprite_extent = 16.0;
        }
    }

    Ok(())
}

fn find_decal_sprite_extent_for_bitmap(bitmap: &Bitmap) -> f32 {
    if bitmap._type != BitmapType::Sprites {
        return 0.0
    }

    bitmap.bitmap_group_sequence.iter()
        .map(|i| i.sprites.iter()).flatten()
        .filter_map(|sprite| {
            let Some(s) = sprite.bitmap_index.index() else {
                return None
            };

            let bitmap_data = &bitmap.bitmap_data[s];
            let reg_x = sprite.registration_point.x * bitmap_data.width as f32;
            let reg_y = sprite.registration_point.y * bitmap_data.height as f32;
            let width = (sprite.right - sprite.left - sprite.registration_point.x) * (bitmap_data.width as f32);
            let height = (sprite.bottom - sprite.top - sprite.registration_point.y) * (bitmap_data.height as f32);
            Some([reg_x, reg_y, width, height])
        })
        .map(|v| v.into_iter()).flatten()
        .reduce(f32::max)
        .unwrap_or(0.0)
}
