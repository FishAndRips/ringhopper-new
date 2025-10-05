use crate::definitions::tag::virtual_keyboard::{VirtualKeyboard, VirtualKeyboardKeyboardKey};
use crate::postprocess::Action;
use crate::{EditableEnumTagField, PostprocessError};

pub fn postprocess_virtual_keyboard(virtual_keyboard: &mut VirtualKeyboard, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    // there are 43 keys defined in the enum
    let number_of_keys = VirtualKeyboardKeyboardKey::_0.values().len();
    let key_count = virtual_keyboard.virtual_keys.len();

    assert_postprocess!(virtual_keyboard.virtual_keys.len() == number_of_keys, "Incorrect number of keys in the keyboard (expected {number_of_keys}, got {key_count})");

    // these are supposed to be sorted
    virtual_keyboard.virtual_keys
        .sort_by(|i, j| (i.keyboard_key as u16)
        .cmp(&(j.keyboard_key as u16)));

    // since keyboard_key as usize should now equal the index, we can see if there are duplicates!
    let all_keys_unique = virtual_keyboard.virtual_keys
        .iter()
        .enumerate()
        .all(|(index, key)| key.keyboard_key as usize == index);

    assert_postprocess!(all_keys_unique, "Keyboard has duplicate keys.");

    Ok(())
}
