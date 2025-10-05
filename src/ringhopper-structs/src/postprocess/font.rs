use crate::definitions::tag::font::Font;
use crate::postprocess::Action;
use crate::{PostprocessError, Reflexive};
use alloc::vec;
use funnel_web::id::Index;

pub fn postprocess_font(font: &mut Font, action: Action) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    font.character_tables = Reflexive::with_vec(vec![Default::default(); 256]);

    for (index, c) in font.characters.iter().enumerate() {
        let Some(index) = Index::from_usize(index) else {
            fail_postprocess!("Can't have a font tag with more than 65535 characters");
        };

        let high = &mut font.character_tables[(c.character >> 8) as usize];
        if high.character_table.is_empty() {
            high.character_table = Reflexive::with_vec(vec![Default::default(); 256]);
        }

        let low = &mut high.character_table[(c.character & 0xFF) as usize];
        low.character_index = index.into();
    }

    font.leading_width = ((font.ascending_height as i32 + font.descending_height as i32) / 5) as i16;

    Ok(())
}
