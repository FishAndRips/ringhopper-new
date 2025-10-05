use crate::definitions::tag::bitmap::{Bitmap, BitmapDataFormat, BitmapDataType, BitmapType};
use crate::postprocess::Action;
use crate::{EditableEnumTagField, PostprocessError, PostprocessState, PostprocessWarningType, TagPath, TagReference};
use alloc::string::String;
use funnel_web::id::Index;

// TODO: Go through all bitmap references and assert this.
#[inline]
pub fn assert_bitmap_types_from_dependency<'a, const I: usize>(tag_dependency: &TagReference<I>, bitmap_types: &[BitmapType], name: &core::fmt::Arguments, state: &'a dyn PostprocessState) -> Result<Option<&'a Bitmap>, PostprocessError> {
    let Some(path) = tag_dependency.get() else {
        return Ok(None)
    };

    assert_bitmap_types_from_path(path, bitmap_types, name, state).map(Some)
}


#[inline]
pub fn assert_bitmap_types_from_path<'a>(tag_path: &TagPath, bitmap_types: &[BitmapType], name: &core::fmt::Arguments, state: &'a dyn PostprocessState) -> Result<&'a Bitmap, PostprocessError> {
    let bitmap = state.read_tag_group::<Bitmap>(tag_path).expect("not a bitmap");

    assert_bitmap_types(
        bitmap,
        bitmap_types,
        name,
        tag_path
    )?;

    Ok(bitmap)
}

#[inline]
pub fn assert_bitmap_type(bitmap: &Bitmap, bitmap_type: BitmapType, name: &core::fmt::Arguments, path: &TagPath) -> Result<(), PostprocessError> {
    assert_bitmap_types(bitmap, &[bitmap_type], name, path)
}

pub fn assert_bitmap_types(bitmap: &Bitmap, bitmap_types: &[BitmapType], name: &core::fmt::Arguments, path: &TagPath) -> Result<(), PostprocessError> {
    // note: simply checking the bitmap type is not enough; more stringent checks are run when the
    // bitmap is postprocessed to make sure the type is actually what it says it is
    if bitmap_types.contains(&bitmap._type) {
        Ok(())
    }
    else {
        let (first, rest) = bitmap_types.split_at(1);

        let mut all = String::with_capacity(1024);
        all += first[0].get_value();
        for i in rest {
            all += ", ";
            all += &i.get_value();
        }

        fail_postprocess!("Expected type of {name} bitmap \"{path}\" to be {all}, but it was {}", bitmap._type);
    }
}

pub fn postprocess_bitmap(bitmap: &mut Bitmap, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    // Verify it is what it says it is; we can't proceed otherwise
    assert_bitmap_type_is_correct(bitmap, tag_path, state)?;

    if action.postprocess() {
        postprocess_bitmap_initial(bitmap)?;
    }

    if action.default() {
        if bitmap._type == BitmapType::Sprites {
            for i in &mut bitmap.bitmap_group_sequence {
                i.bitmap_count = 0;
                i.first_bitmap_index = Index(0).into();
            }
        }

        // Fix up flags
        for b in &mut bitmap.bitmap_data {
            b.flags.compressed = is_block_compression_format(b.format);
            b.flags.linear = bitmap._type == BitmapType::InterfaceBitmaps;
            b.flags.power_of_two_dimensions = bitmap._type != BitmapType::InterfaceBitmaps;
        }
    }

    Ok(())
}

pub(crate) fn bitmap_or_sprite_count_of_sequence_by_index(bitmap: &Bitmap, sequence_index: usize) -> Option<usize> {
    let sequence = bitmap.bitmap_group_sequence.get(sequence_index)?;
    match bitmap._type {
        BitmapType::Sprites => Some(sequence.sprites.len()),
        _ => Some(sequence.bitmap_count as usize)
    }
}

fn postprocess_bitmap_initial(bitmap: &mut Bitmap) -> Result<(), PostprocessError> {
    let bitmap_type = bitmap._type;

    assert_postprocess!(!bitmap.bitmap_data.is_empty(), "Bitmap tag has no bitmaps.");
    assert_postprocess!(!bitmap.bitmap_group_sequence.is_empty(), "Bitmap tag has no sequences.");

    // Make sure all bitmaps are accounted for and the sequences aren't wrong
    if bitmap_type != BitmapType::Sprites {
        let mut checked = alloc::vec![0u8; bitmap.bitmap_data.len()];
        for (sequence_index, sequence) in bitmap.bitmap_group_sequence.iter().enumerate() {
            let bitmap_count = sequence.bitmap_count as usize;
            if bitmap_count == 0 {
                continue
            }

            let Some(first) = sequence.first_bitmap_index.index() else {
                fail_postprocess!("Sequence #{sequence_index} has bitmap(s) but no first bitmap index.");
            };

            let range = first..(first + bitmap_count);
            for r in range {
                let Some(l) = checked.get_mut(r) else {
                    fail_postprocess!("Sequence #{sequence_index} references bitmap #{r}, but there are only {} bitmap(s).", checked.len());
                };
                *l = l.saturating_add(1);
            }
        }

        if let Some((bitmap_index, &count)) = checked.iter().enumerate().find(|i| *i.1 != 0) {
            if count == 0 {
                fail_postprocess!("Bitmap #{bitmap_index} does not appear in any sequences.");
            }
            else {
                fail_postprocess!("Bitmap #{bitmap_index} appears in {count} sequences (it should only appear in exactly one sequence).");
            }
        }
    }

    // Make sure the depth is correct
    for (i, bitmap_data) in bitmap.bitmap_data.iter().enumerate() {
        assert_postprocess!(bitmap_data.depth > 0, "Bitmap #{i} has an invalid depth of {}", bitmap_data.depth);
        assert_postprocess!(bitmap_data.depth == 1 || bitmap_data._type == BitmapDataType::_3dTexture, "Bitmap #{i} is not marked as 3D textures but has a depth of {}", bitmap_data.depth);
    }

    // Remove empty sequences at the end.
    for i in (0..bitmap.bitmap_group_sequence.len()).rev() {
        let sequence = &bitmap.bitmap_group_sequence[i];

        let frame_count = match bitmap._type {
            BitmapType::Sprites => sequence.sprites.len(),
            _ => sequence.bitmap_count as usize
        };

        if frame_count == 0 {
            bitmap.bitmap_group_sequence.remove(i);
        }
    }

    // Clear the color plate
    bitmap.color_plate.compressed_data = Default::default();

    Ok(())
}

fn assert_bitmap_type_is_correct(bitmap: &mut Bitmap, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    match bitmap._type {
        BitmapType::_2dTextures => {
            assert_all_bitmaps_are_type(bitmap, BitmapDataType::_2dTexture)?;
            assert_no_sprites(bitmap)?;
            assert_all_bitmaps_are_power_of_two(bitmap, tag_path, state)?;
        }
        BitmapType::_3dTextures => {
            assert_all_bitmaps_are_type(bitmap, BitmapDataType::_3dTexture)?;
            assert_no_sprites(bitmap)?;
            assert_all_bitmaps_are_power_of_two(bitmap, tag_path, state)?;
        }
        BitmapType::CubeMaps => {
            assert_all_bitmaps_are_type(bitmap, BitmapDataType::CubeMap)?;
            assert_no_sprites(bitmap)?;
            assert_all_bitmaps_are_power_of_two(bitmap, tag_path, state)?;
        }
        BitmapType::InterfaceBitmaps => {
            assert_all_bitmaps_are_type(bitmap, BitmapDataType::_2dTexture)?;
            assert_no_sprites(bitmap)?;

            assert_postprocess!(!bitmap.bitmap_data.iter().any(|i| i.mipmap_count != 1), "Bitmap is marked as interface bitmaps but contains mipmaps.");

            for (i, data) in bitmap.bitmap_data.iter().enumerate() {
                assert_postprocess!(
                    !is_block_compression_format(data.format),
                    "Bitmap #{i} is block-compressed as {} which is not allowed for interface bitmaps.", data.format
                )
            }
        }
        BitmapType::Sprites => {
            assert_all_bitmaps_are_type(bitmap, BitmapDataType::_2dTexture)?;
            assert_only_sprites(bitmap)?;
            assert_all_bitmaps_are_power_of_two(bitmap, tag_path, state)?;
        }
    }

    Ok(())
}

fn assert_all_bitmaps_are_power_of_two(bitmap: &Bitmap, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for (i, data) in bitmap.bitmap_data.iter().enumerate() {
        if !data.width.is_power_of_two() || !data.height.is_power_of_two() || !data.depth.is_power_of_two() {
            if bitmap._type == BitmapType::_2dTextures {
                state.warn(
                    tag_path,
                    format_args!(
                        "Bitmap #{i} is a non-power-of-two ({}x{}x{}) 2D texture (not an interface bitmap); this will be an error in the future",
                        data.width, data.height, data.depth
                    ),
                    PostprocessWarningType::Deprecated
                );
            }
            else {
                fail_postprocess!("Bitmap #{i} is non-power-of-two and is marked as a {}. Only interface bitmaps can be non-power-of-two.", bitmap._type);
            }
        }
    }
    Ok(())
}

fn is_block_compression_format(format: BitmapDataFormat) -> bool {
    matches!(format, BitmapDataFormat::DXT1 | BitmapDataFormat::DXT3 | BitmapDataFormat::DXT5 | BitmapDataFormat::BC7)
}

fn assert_all_bitmaps_are_type(bitmap: &Bitmap, bitmap_type: BitmapDataType) -> Result<(), PostprocessError> {
    assert_postprocess!(
        !bitmap.bitmap_data.iter().any(|i| i._type != bitmap_type),
        "Bitmap is marked as {} but contains bitmaps that aren't {bitmap_type}.", bitmap._type
    );
    Ok(())
}

fn assert_no_sprites(bitmap: &Bitmap) -> Result<(), PostprocessError> {
    assert_postprocess!(
        !bitmap.bitmap_group_sequence.iter().any(|i| !i.sprites.is_empty()),
        "Bitmap is marked as {} but contains sprites.", bitmap._type
    );
    Ok(())
}

fn assert_only_sprites(bitmap: &Bitmap) -> Result<(), PostprocessError> {
    assert_postprocess!(
        !bitmap.bitmap_group_sequence.iter().any(|i| i.bitmap_count > 0 && i.sprites.is_empty()),
        "Bitmap is marked as {} but contains non-sprite sequences.", bitmap._type
    );
    Ok(())
}
