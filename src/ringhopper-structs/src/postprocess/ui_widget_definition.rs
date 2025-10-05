use crate::definitions::tag::ui_widget_definition::UIWidgetDefinition;
use crate::postprocess::Action;
use crate::{PostprocessError, PostprocessState, PostprocessWarningType, TagPath};
use crate::definitions::tag::bitmap::BitmapType;

pub(super) const STATIC_ELEMENT: &[BitmapType] = &[BitmapType::Sprites, BitmapType::_2dTextures, BitmapType::InterfaceBitmaps];

#[inline]
pub fn postprocess_ui_widget_definition(ui_widget_definition: &mut UIWidgetDefinition, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let scenario_tag = state.scenario_tag();
    for (index, handler) in ui_widget_definition.event_handlers.iter().enumerate() {
        let expected_script = &handler.script;
        if !scenario_tag.scripts.iter().any(|i| &i.name == expected_script) {
            state.warn(
                tag_path,
                format_args!("Event handler #{index} is calling script '{expected_script}' which isn't present in the scenario tag; it will be a no-op"),
                PostprocessWarningType::BrokenReference
            )
        }
    }

    Ok(())
}
