use crate::constants::HUD_MESSAGE_TEXT_TYPES;
use crate::definitions::tag::hud_message_text::HUDMessageText;
use crate::postprocess::Action;
use crate::PostprocessError;

pub fn postprocess_hud_message_text(halo_mapping_tools: &mut HUDMessageText, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let message_elements = halo_mapping_tools.message_elements.as_slice();

    for (message_index, message) in halo_mapping_tools.messages.iter().enumerate() {
        let count = message.panel_count as usize;
        if count == 0 {
            continue
        }

        let Some(start_index) = message.start_index_of_message_block
            .index() else {
            fail_postprocess!("Message #{message_index} has a null start panel index but {count} panel(s).");
        };

        let end_index = start_index + count;

        let Some(cursor) = message.start_index_into_text_blob.index() else {
            fail_postprocess!("Message #{message_index} has a null start text index but {count} panel(s).");
        };

        let Some(mut remaining_text_data) = halo_mapping_tools.text_data.get(end_index..) else {
            fail_postprocess!("Message #{message_index} has an out-of-bounds start text index {cursor}.");
        };

        let Some(elements) = message_elements.get(start_index..end_index) else {
            fail_postprocess!("Message #{message_index}'s panel(s) are out-of-bounds ({end_index} > {}).", message_elements.len());
        };

        for (element_index, element) in elements.iter().enumerate() {
            match element._type {
                0 => {
                    let Some((start, end)) = remaining_text_data.split_at_checked(2*element.data as usize) else {
                        fail_postprocess!("Message #{message_index}'s panel(s) are out-of-bounds for the text data.");
                    };

                    assert_postprocess!(!start.is_empty(), "Element {element_index} of message #{message_index} is empty.");
                    assert_postprocess!(start.ends_with(&[0,0]), "Element {element_index} of message #{message_index} is not null-terminated.");

                    for c in char::decode_utf16(
                        start
                            .chunks(2)
                            .map(|i| u16::from_le_bytes([i[0], i[1]]))
                            .take(start.len() / 2 - 1) // exclude the null terminator
                    ) {
                        // FIXME: This will NOT work if building for CJK on Xbox
                        match c {
                            Ok('\n') | Ok('\r') => {
                                fail_postprocess!("Element {element_index} of message #{message_index} contains invalid newlines (use |n instead).");
                            },
                            Ok(n) if n.is_control() => {
                                fail_postprocess!("Element {element_index} of message #{message_index} contains invalid control characters.");
                            },
                            Ok('\x00') => {
                                fail_postprocess!("Element {element_index} of message #{message_index} contains interior null characters.");
                            }
                            Err(_) => {
                                fail_postprocess!("Element {element_index} of message #{message_index} contains invalid UTF-16.");
                            },
                            _ => ()
                        }
                    }

                    remaining_text_data = end;
                },
                1 => {
                    assert_postprocess!(
                        (element.data as usize) < HUD_MESSAGE_TEXT_TYPES.len(),
                        "Element #{element_index} of message #{message_index} has an invalid HUD message text type {}.", element.data
                    )
                },
                n => {
                    fail_postprocess!("Element #{element_index} of message #{message_index} has invalid type {n}.");
                }
            }
        }
    }

    Ok(())
}
