use alloc::vec::Vec;
use combarc::CombArc;
use funnel_web::vector::Vector2DInt;
use crate::definitions::tag::meter::Meter;
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, PostprocessWarningType, TagPath};
use crate::definitions::tag::bitmap::{Bitmap, BitmapDataFormat, BitmapType};

pub fn postprocess_meter(meter: &mut Meter, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    // Of course the encoded stencil won't be used, but we're including the encoder anyway because
    // we're cool.
    encode_stencil_for_meter(meter, tag_path, state)?;

    // fail_postprocess!("Meter tags are unsupported. Please dereference this tag.");
    state.warn(
        tag_path,
        format_args!("Meter tags are unsupported, and support for building maps with them will be removed in a future version."),
        PostprocessWarningType::Deprecated
    );
    Ok(())
}

const ENCODED_ROW_SIZE: usize = 6;
const ENCODED_PIXEL_SIZE: usize = 4;
const MAX_ENCODED_STENCIL_DATA_SIZE: usize = 65536;

// TODO: Verify this is accurate
fn encode_stencil_for_meter(meter: &mut Meter, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    let Some(stencil_bitmaps) = meter.stencil_bitmaps.get() else {
        state.warn(tag_path, format_args!("Missing stencil bitmap for meter tag."), PostprocessWarningType::UnusedData);
        return Ok(())
    };

    let Some(source_bitmaps) = meter.source_bitmap.get() else {
        state.warn(tag_path, format_args!("Missing source bitmap for meter tag."), PostprocessWarningType::UnusedData);
        return Ok(())
    };

    let allowed_meter_bitmaps = &const { [BitmapType::InterfaceBitmaps, BitmapType::_2dTextures] };

    let source_bitmap = state.read_tag_group::<Bitmap>(source_bitmaps).expect("source bitmaps");
    if !allowed_meter_bitmaps.contains(&source_bitmap._type) {
        fail_postprocess!("Source bitmaps must be 2D textures or interface bitmaps.")
    }
    let source_bitmap = &source_bitmap.bitmap_data[0];

    let stencil_bitmaps = state.read_tag_group::<Bitmap>(stencil_bitmaps).expect("stencil bitmaps");
    if !allowed_meter_bitmaps.contains(&stencil_bitmaps._type) {
        fail_postprocess!("Stencil bitmaps must be 2D textures or interface bitmaps.")
    }

    let sequence_index = meter.stencil_sequence_index as usize;
    let Some(stencil_bitmap_sequence) = stencil_bitmaps.bitmap_group_sequence.get(sequence_index) else {
        fail_postprocess!("Sequence index #{sequence_index} is not present in the meter's stencil bitmaps.")
    };

    assert_postprocess!(stencil_bitmap_sequence.bitmap_count >= 2, "Stencil bitmaps sequence must have at least two bitmaps (mask and levels stencil).");

    let first_bitmap_index = stencil_bitmap_sequence.first_bitmap_index.index().expect("bitmap count > 0");

    let mask_bitmap = &stencil_bitmaps.bitmap_data[first_bitmap_index];
    let levels_bitmap = &stencil_bitmaps.bitmap_data[first_bitmap_index + 1];

    assert_postprocess!(mask_bitmap.format == BitmapDataFormat::Y8, "Level bitmaps must be Y8.");
    assert_postprocess!(levels_bitmap.format == BitmapDataFormat::Y8, "Level bitmaps must be Y8.");

    let width = levels_bitmap.width as usize;
    let height = levels_bitmap.height as usize;
    let total_size = width * height;

    assert_postprocess!(width == levels_bitmap.width as usize && height == mask_bitmap.height as usize, "Mask and levels bitmap dimensions must match.");
    assert_postprocess!(width == source_bitmap.width as usize && height == source_bitmap.height as usize, "Stencil and source bitmap dimensions must match.");

    meter.runtime_width = mask_bitmap.width;
    meter.runtime_height = mask_bitmap.height;
    meter.runtime_registration_point = mask_bitmap.registration_point;

    let mask_bitmap_data = &stencil_bitmaps.processed_pixel_data[mask_bitmap.pixel_data_offset as usize .. mask_bitmap.pixel_data_offset as usize + total_size];
    let level_bitmap_data = &stencil_bitmaps.processed_pixel_data[levels_bitmap.pixel_data_offset as usize .. levels_bitmap.pixel_data_offset as usize + total_size];

    let mut data: Vec<u8> = Vec::with_capacity(MAX_ENCODED_STENCIL_DATA_SIZE);

    for y in 0..height {
        let mut encoded_row: Option<(usize, EncodedRow)> = None;

        let finalize_row = |row: &mut Option<(usize, EncodedRow)>, data: &mut Vec<u8>| -> Result<(), PostprocessError> {
            let Some((offset, row_data)) = row.take() else { return Ok(()) };
            data[offset..offset+ENCODED_ROW_SIZE].copy_from_slice(&row_data.encode());

            if data.len() > MAX_ENCODED_STENCIL_DATA_SIZE {
                fail_postprocess!("Maximum encoded stencil size of {MAX_ENCODED_STENCIL_DATA_SIZE} bytes exceeded.");
            }
            Ok(())
        };

        for x in 0..width {
            let offset = width * y + x;
            let mask = mask_bitmap_data[offset];
            let level = level_bitmap_data[offset];

            if mask != 0 {
                if encoded_row.is_none() {
                    encoded_row = Some((data.len(), EncodedRow {
                        origin: Vector2DInt { x: x as i16, y: y as i16 },
                        pixel_count: 0
                    }));
                    data.extend_from_slice(&[0; ENCODED_ROW_SIZE]);
                }

                let (_, row_data) = encoded_row.as_mut().expect("encoded row should now be set");
                let Some(pixel_count) = row_data.pixel_count.checked_add(1) else {
                    fail_postprocess!("Maximum contiguous row size exceeded for a stencil.");
                };
                row_data.pixel_count = pixel_count;

                data.extend_from_slice(&EncodedPixel {
                    mask_pixel: mask,
                    level: (level as u16) << 8
                }.encode());
            }
            else {
                finalize_row(&mut encoded_row, &mut data)?;
            }
        }
        finalize_row(&mut encoded_row, &mut data)?;
    }

    meter.encoded_stencil = CombArc::new(data);

    // Delete the evidence.
    meter.stencil_bitmaps.clear();

    Ok(())
}

#[derive(Copy, Clone, PartialEq, Debug)]
struct EncodedPixel {
    mask_pixel: u8,
    level: u16
}
impl EncodedPixel {
    fn encode(self) -> [u8; ENCODED_PIXEL_SIZE] {
        let [level_low, level_high] = self.level.to_le_bytes();
        [self.mask_pixel, /* padding */ 0, level_low, level_high]
    }
}

struct EncodedRow {
    origin: Vector2DInt,
    pixel_count: i16
}
impl EncodedRow {
    fn encode(self) -> [u8; ENCODED_ROW_SIZE] {
        let x = self.origin.x.to_le_bytes();
        let y = self.origin.y.to_le_bytes();
        let pixel_count = self.pixel_count.to_le_bytes();
        [x[0], x[1], y[0], y[1], pixel_count[0], pixel_count[1]]
    }
}
