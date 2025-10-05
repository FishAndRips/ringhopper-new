use crate::definitions::tag::string_list::StringList;
use crate::definitions::tag::unicode_string_list::UnicodeStringList;
use crate::postprocess::Action;
use crate::PostprocessError;

pub fn postprocess_string_list(string_list: &mut StringList, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    for (index, string) in string_list.strings.iter().enumerate() {
        assert_postprocess!(string.string.ends_with(&[0]), "String #{index} is not null-terminated.");

        let bytes = &string.string[0 .. string.string.len() - 1];
        assert_postprocess!(bytes.is_ascii(), "String #{index} is non-ASCII.");

        check_if_string_is_valid(index, bytes.iter().map(|i| *i as char))?
    }

    Ok(())
}

pub fn postprocess_unicode_string_list(unicode_string_list: &mut UnicodeStringList, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    for (index, string) in unicode_string_list.strings.iter().enumerate() {
        check_if_string_is_valid(index, string.string.chars())?
    }

    Ok(())
}

fn check_if_string_is_valid<I: Iterator<Item = char>>(index: usize, iterator: I) -> Result<(), PostprocessError> {
    let mut last_char = None;
    for c in iterator {
        if c == '\r' || c == '\n' {
            assert_postprocess!(
                c != '\n' || last_char == Some('\r'),
                "String #{index} has LF endings (only CRLF is allowed)."
            );
            assert_postprocess!(
                c == '\n' || last_char != Some('\r'),
                "String #{index} has CR endings (only CRLF is allowed)."
            );
        }
        else {
            assert_postprocess!(
                !c.is_ascii_control(),
                "String #{index} has banned control characters."
            );
        }
        last_char = Some(c);
    }
    assert_postprocess!(
        last_char != Some('\r'),
        "String #{index} has CR endings (only CRLF is allowed)."
    );
    Ok(())
}
