macro_rules! fail_postprocess {
    ($message:literal, $($params:tt)*) => {
        return Err(crate::postprocess::PostprocessError::InvalidTagDataError {
            explanation: alloc::borrow::Cow::Owned(alloc::format!($message, $($params)*))
        })
    };

    ($message:literal) => {
        return Err(crate::postprocess::PostprocessError::InvalidTagDataError {
            explanation: alloc::borrow::Cow::Owned(alloc::format!($message))
        })
    };
}

macro_rules! assert_postprocess {
    ($what:expr, $message:literal, $($params:tt)*) => {
        if (($what) == false) {
            fail_postprocess!($message, $($params)*)
        }
    };

    ($what:expr, $message:literal) => {
        if (($what) == false) {
            fail_postprocess!($message)
        }
    };
}

mod damage_effect;
mod decal;
mod object;
mod shader;
mod contrail;
mod particle;
mod bitmap;
mod effect;
mod string_list;
mod virtual_keyboard;
mod physics;
mod hud_message_text;
mod point_physics;
mod fog;
mod font;
mod sky;
mod sound_environment;
mod ui_widget_definition;
mod meter;
mod globals;
mod actor;
mod item_collection;
mod detail_object_collection;
mod antenna;
mod model_collision_geometry;
mod lens_flare;
mod lightning;
mod light_volume;
mod light;
mod hud_interface;
mod scenario_structure_bsp;
mod sound_looping;
mod sound;
mod model_animations;
mod model;
mod scenario;

use funnel_web::float::FloatOps;
use crate::definitions::tag::TagGroup;
use crate::{EditableTag, ModelFns, Parameters, SimpleWriteableData, TagPath};
use alloc::borrow::Cow;
use core::fmt::{Arguments, Display, Formatter};
use alloc::boxed::Box;
use funnel_web::constants::{reverse_seconds_to_ticks, seconds_to_ticks};
use crate::definitions::engine::Engine;
use crate::definitions::tag::globals::Globals;
use crate::definitions::tag::scenario::{Scenario, ScenarioType};

#[derive(Debug)]
pub enum PostprocessError {
    GenericError {
        explanation: Cow<'static, str>
    },
    InvalidTagDataError {
        explanation: Cow<'static, str>
    }
}

impl Display for PostprocessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GenericError { explanation } => f.write_fmt(format_args!("{explanation}")),
            Self::InvalidTagDataError { explanation } => f.write_fmt(format_args!("Invalid tag data! {explanation}"))
        }
    }
}

pub trait PostprocessState {
    fn scenario_name(&self) -> &str;
    fn scenario_tag(&self) -> &Scenario;
    fn globals_tag(&self) -> &Globals;
    fn tag_path(&self) -> &str;
    fn jason_jones_singleplayer(&self) -> bool;
    fn engine(&self) -> &'static Engine;
    fn scenario_type(&self) -> ScenarioType;
    fn try_read_tag(&self, tag_path: &TagPath) -> Option<&dyn EditableTag>;

    fn read_tag(&self, tag_path: &TagPath) -> &dyn EditableTag {
        self.try_read_tag(tag_path).expect("tag failed to read")
    }

    /// Update a tag that has already been postprocessed.
    ///
    /// This is used for tags updating other tags in cache files.
    fn update_postprocessed_tag(&mut self, tag_path: &TagPath, tag: Box<dyn EditableTag>) -> Result<(), &'static str>;

    /// Log a warning
    fn warn(&self, tag: &TagPath, what: Arguments, warning_type: PostprocessWarningType);
}

impl dyn PostprocessState + '_ {
    fn read_tag_group<T: EditableTag>(&self, tag_path: &TagPath) -> Option<&T> {
        self.read_tag(tag_path).downcast_ref()
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PostprocessWarningType {
    /// Unused data.
    ///
    /// Anything that causes these warnings is harmless, but you could save some tag space by fixing
    /// them.
    UnusedData,

    /// A reference is broken, but it is not one that is critical.
    ///
    /// This will result in nothing happening (which may not be what you want).
    BrokenReference,

    /// Object instances in the scenario tag are outside of the scenario's BSP(s).
    ///
    /// These objects won't spawn. This is probably not what you want, but it's harmless to leave
    /// them in.
    ///
    /// Note, however, that removing these objects will affect multiplayer RNG in the case of random
    /// permutations. As such, you may not want to fix this in maps that you expect to work with
    /// unmodified versions of those maps in network play.
    MisplacedObjects,

    /// User-specified tag data was ignored and overwritten.
    ///
    /// Read the warning to see why this happened. If you are OK with the results, you can ignore
    /// this warning or update the data to match the new data.
    IgnoredUserData,

    /// Item collection weights will not work the way you expect.
    ///
    /// It is recommended to fix this to ensure the item collection works as intended.
    ItemCollectionWeights,

    /// Something that is temporarily allowed but won't be allowed in the future.
    ///
    /// This is similar to unused data, but you may want to fix it now to avoid an error later.
    Deprecated,

    /// No usable player spawns for a map.
    NoUsablePlayerSpawns,

    /// Insufficient player spawns for a map.
    InsufficientPlayerSpawnCount,

    /// No spawns for a map for a given gametype.
    MissingGametypePlayerSpawns,

    /// An object has a spawn point outside of the BSP but its bounding offset puts it inside of the
    /// BSP. This means it will spawn, but may will have some lighting weirdness.
    BoundingOffsetInsideBSP,

    /// A child scenario's BSP does not match the main scenario, and this may result in issues.
    MismatchedChildScenarioBSP,

    /// A trigger volume that looks like a BSP trigger volume but is incorrectly named.
    InvalidBSPTriggerVolumeName,
}

struct NullPostprocessTagProvider;
impl PostprocessState for NullPostprocessTagProvider {
    fn scenario_name(&self) -> &str {
        unimplemented!("NullPostprocessTagProvider::scenario_name")
    }
    fn scenario_tag(&self) -> &Scenario {
        unimplemented!("NullPostprocessTagProvider::scenario_tag")
    }
    fn globals_tag(&self) -> &Globals {
        unimplemented!("NullPostprocessTagProvider::globals_tag")
    }
    fn tag_path(&self) -> &str {
        unimplemented!("NullPostprocessTagProvider::tag_path")
    }
    fn jason_jones_singleplayer(&self) -> bool {
        unimplemented!("NullPostprocessTagProvider::jason_jones_singleplayer")
    }
    fn engine(&self) -> &'static Engine {
        unimplemented!("NullPostprocessTagProvider::engine")
    }
    fn scenario_type(&self) -> ScenarioType {
        unimplemented!("NullPostprocessTagProvider::scenario_type")
    }
    fn try_read_tag(&self, _: &TagPath) -> Option<&dyn EditableTag> {
        unimplemented!("NullPostprocessTagProvider::try_read_tag")
    }
    fn update_postprocessed_tag(&mut self, _: &TagPath, _: Box<dyn EditableTag>) -> Result<(), &'static str> {
        unimplemented!("NullPostprocessTagProvider::update_postprocessed_tag")
    }

    fn warn(&self, _: &TagPath, _: Arguments, _: PostprocessWarningType) {
        unimplemented!("NullPostprocessTagProvider::warn")
    }
}

/// Apply defaults and other mathematical operations.
///
/// Used on tags that are going to be put in cache files. Should NOT be saved to loose tags.
#[inline]
pub fn postprocess_tag(tag: &mut dyn EditableTag, state: &mut dyn PostprocessState, tag_path: &TagPath) -> Result<(), PostprocessError> {
    apply_postprocess_function_for_tag(tag, Action::Postprocess, state, tag_path)
}

/// Un-apply defaults and other mathematical operations.
///
/// Used on tags that are being extracted from cache files.
#[inline]
pub fn unpostprocess_tag(tag: &mut dyn EditableTag, state: &mut dyn PostprocessState, tag_path: &TagPath) -> Result<(), PostprocessError> {
    apply_postprocess_function_for_tag(tag, Action::Unpostprocess, state, tag_path)
}

/// Apply defaults.
///
/// Safe to apply to loose tags, even repeatedly.
///
/// Note: Does not handle cross-tag postprocessing.
#[inline]
pub fn default_tag(tag: &mut dyn EditableTag, tag_path: &TagPath) -> Result<(), PostprocessError> {
    apply_postprocess_function_for_tag(tag, Action::Default, &mut NullPostprocessTagProvider, tag_path)
}

/// Un-apply defaults.
///
/// Safe to apply to loose tags, even repeatedly.
///
/// Note: Does not handle cross-tag postprocessing.
#[inline]
pub fn undefault_tag(tag: &mut dyn EditableTag, tag_path: &TagPath) -> Result<(), PostprocessError> {
    apply_postprocess_function_for_tag(tag, Action::Undefault, &mut NullPostprocessTagProvider, tag_path)
}

/// Nudge tags, fixing rounding errors from (un)postprocessing.
///
/// Safe to apply to loose tags, even repeatedly.
#[inline]
pub fn nudge_tag(tag: &mut dyn EditableTag, tag_path: &TagPath) -> Result<(), PostprocessError> {
    apply_postprocess_function_for_tag(tag, Action::Nudge, &mut NullPostprocessTagProvider, tag_path)
}


#[derive(Copy, Clone, Debug)]
enum Action {
    Postprocess,
    Default,
    Nudge,

    Unpostprocess,
    Undefault
}
impl Action {
    pub fn undefault(self) -> bool {
        matches!(self, Action::Undefault | Action::Unpostprocess)
    }
    pub fn default(self) -> bool {
        matches!(self, Action::Default | Action::Postprocess)
    }
    pub fn unpostprocess(self) -> bool {
        matches!(self, Action::Unpostprocess)
    }
    pub fn postprocess(self) -> bool {
        matches!(self, Action::Postprocess)
    }
    pub fn nudge(self) -> bool { matches!(self, Action::Unpostprocess | Action::Nudge) }
}

fn apply_postprocess_function_for_tag(
    tag: &mut dyn EditableTag,
    action: Action,
    state: &mut dyn PostprocessState,
    tag_path: &TagPath,
) -> Result<(), PostprocessError> {
    macro_rules! apply_postprocess {
        ($func:expr) => {
            ($func)(tag.downcast_mut().expect("failed to downcast on postprocess - the EditableTag instance tells lies!!!"), action)
        };
    }
    macro_rules! apply_postprocess_with_state {
        ($func:expr) => {
            ($func)(tag.downcast_mut().expect("failed to downcast on postprocess - the EditableTag instance tells lies!!!"), action, tag_path, state)
        };
    }

    match tag.tag_group() {
        TagGroup::None => unreachable!(),

        TagGroup::Actor => apply_postprocess!(actor::postprocess_actor)?,
        TagGroup::ActorVariant => apply_postprocess_with_state!(actor::postprocess_actor_variant)?,
        TagGroup::Antenna => apply_postprocess!(antenna::postprocess_antenna)?,
        TagGroup::Biped => apply_postprocess_with_state!(object::biped::postprocess_biped)?,
        TagGroup::Bitmap => apply_postprocess_with_state!(bitmap::postprocess_bitmap)?,
        TagGroup::CameraTrack => { /* nothing */ },
        TagGroup::ColorTable => { /* nothing */ },
        TagGroup::ContinuousDamageEffect => apply_postprocess!(damage_effect::postprocess_continuous_damage_effect)?,
        TagGroup::Contrail => apply_postprocess!(contrail::postprocess_contrail)?,
        TagGroup::DamageEffect => apply_postprocess_with_state!(damage_effect::postprocess_damage_effect)?,
        TagGroup::Decal => apply_postprocess_with_state!(decal::postprocess_decal)?,
        TagGroup::DetailObjectCollection => apply_postprocess_with_state!(detail_object_collection::postprocess_detail_object_collection)?,
        TagGroup::Device => apply_postprocess!(object::device::postprocess_device)?,
        TagGroup::DeviceControl => { /* nothing */ },
        TagGroup::DeviceLightFixture => { /* nothing */ },
        TagGroup::DeviceMachine => apply_postprocess!(object::device_machine::postprocess_device_machine)?,
        TagGroup::Dialogue => { /* nothing */ },
        TagGroup::Effect => apply_postprocess_with_state!(effect::postprocess_effect)?,
        TagGroup::Equipment => apply_postprocess_with_state!(object::equipment::postprocess_equipment)?,
        TagGroup::Flag => { /* nothing */ },
        TagGroup::Fog => apply_postprocess!(fog::postprocess_fog)?,
        TagGroup::Font => apply_postprocess!(font::postprocess_font)?,
        TagGroup::Garbage => { /* nothing */ },
        TagGroup::GBXModel => apply_postprocess_with_state!(model::postprocess_gbxmodel)?,
        TagGroup::Globals => apply_postprocess_with_state!(globals::postprocess_globals)?,
        TagGroup::Glow => { /* handled by tag parser */ },
        TagGroup::GrenadeHUDInterface => apply_postprocess_with_state!(hud_interface::grenade_hud_interface::postprocess_grenade_hud_interface)?,
        TagGroup::HUDGlobals => apply_postprocess_with_state!(hud_interface::hud_globals::postprocess_hud_globals)?,
        TagGroup::HUDMessageText => apply_postprocess!(hud_message_text::postprocess_hud_message_text)?,
        TagGroup::HUDNumber => apply_postprocess_with_state!(hud_interface::hud_number::postprocess_hud_number)?,
        TagGroup::InputDeviceDefaults => { /* nothing */ },
        TagGroup::Item => { /* nothing */ },
        TagGroup::ItemCollection => apply_postprocess_with_state!(item_collection::postprocess_item_collection)?,
        TagGroup::LensFlare => apply_postprocess!(lens_flare::postprocess_lens_flare)?,
        TagGroup::Light => apply_postprocess!(light::postprocess_light)?,
        TagGroup::LightVolume => apply_postprocess!(light_volume::postprocess_light_volume)?,
        TagGroup::Lightning => apply_postprocess_with_state!(lightning::postprocess_lightning)?,
        TagGroup::MaterialEffects => { /* nothing */ },
        TagGroup::Meter => apply_postprocess_with_state!(meter::postprocess_meter)?,
        TagGroup::Model => apply_postprocess_with_state!(model::postprocess_model)?,
        TagGroup::ModelAnimations => apply_postprocess_with_state!(model_animations::postprocess_model_animations)?,
        TagGroup::ModelCollisionGeometry => apply_postprocess!(model_collision_geometry::postprocess_model_collision_geometry)?,
        TagGroup::MultiplayerScenarioDescription => { /* nothing */ },
        TagGroup::Object => apply_postprocess_with_state!(object::postprocess_object)?,
        TagGroup::Particle => apply_postprocess_with_state!(particle::postprocess_particle)?,
        TagGroup::ParticleSystem => apply_postprocess!(particle::postprocess_particle_system)?,
        TagGroup::Physics => apply_postprocess!(physics::postprocess_physics)?,
        TagGroup::Placeholder => { /* nothing */ },
        TagGroup::PointPhysics => apply_postprocess!(point_physics::postprocess_point_physics)?,
        TagGroup::PreferencesNetworkGame => { /* nothing */ },
        TagGroup::Projectile => apply_postprocess!(object::projectile::postprocess_projectile)?,
        TagGroup::Scenario => apply_postprocess_with_state!(scenario::postprocess_scenario)?,
        TagGroup::ScenarioStructureBSP => apply_postprocess_with_state!(scenario_structure_bsp::postprocess_scenario_structure_bsp)?,
        TagGroup::Scenery => { /* nothing */ },
        TagGroup::Shader => apply_postprocess_with_state!(shader::postprocess_shader)?,
        TagGroup::ShaderEnvironment => apply_postprocess_with_state!(shader::postprocess_shader_environment)?,
        TagGroup::ShaderModel => apply_postprocess_with_state!(shader::postprocess_shader_model)?,
        TagGroup::ShaderTransparentChicago => apply_postprocess_with_state!(shader::postprocess_shader_transparent_chicago)?,
        TagGroup::ShaderTransparentChicagoExtended => apply_postprocess_with_state!(shader::postprocess_shader_transparent_chicago_extended)?,
        TagGroup::ShaderTransparentGeneric => apply_postprocess_with_state!(shader::postprocess_shader_transparent_generic)?,
        TagGroup::ShaderTransparentGlass => apply_postprocess!(shader::postprocess_shader_transparent_glass)?,
        TagGroup::ShaderTransparentMeter => { /* nothing */ },
        TagGroup::ShaderTransparentPlasma => apply_postprocess!(shader::postprocess_shader_transparent_plasma)?,
        TagGroup::ShaderTransparentWater => apply_postprocess!(shader::postprocess_shader_transparent_water)?,
        TagGroup::Sky => apply_postprocess!(sky::postprocess_sky)?,
        TagGroup::Sound => apply_postprocess_with_state!(sound::postprocess_sound)?,
        TagGroup::SoundEnvironment => apply_postprocess!(sound_environment::postprocess_sound_environment)?,
        TagGroup::SoundLooping => apply_postprocess_with_state!(sound_looping::postprocess_sound_looping)?,
        TagGroup::SoundScenery => { /* nothing */ },
        TagGroup::StringList => apply_postprocess!(string_list::postprocess_string_list)?,
        TagGroup::TagCollection => { /* nothing */ },
        TagGroup::UIWidgetCollection => { /* nothing */ },
        TagGroup::UIWidgetDefinition => apply_postprocess_with_state!(ui_widget_definition::postprocess_ui_widget_definition)?,
        TagGroup::UnicodeStringList => apply_postprocess!(string_list::postprocess_unicode_string_list)?,
        TagGroup::Unit => apply_postprocess_with_state!(object::unit::postprocess_unit)?,
        TagGroup::UnitHUDInterface => apply_postprocess_with_state!(hud_interface::unit_hud_interface::postprocess_unit_hud_interface)?,
        TagGroup::Vehicle => apply_postprocess!(object::vehicle::postprocess_vehicle)?,
        TagGroup::VirtualKeyboard => apply_postprocess!(virtual_keyboard::postprocess_virtual_keyboard)?,
        TagGroup::Weapon => apply_postprocess_with_state!(object::weapon::postprocess_weapon)?,
        TagGroup::WeaponHUDInterface => apply_postprocess_with_state!(hud_interface::weapon_hud_interface::postprocess_weapon_hud_interface)?,
        TagGroup::WeatherParticleSystem => apply_postprocess_with_state!(particle::postprocess_weather_particle_system)?,
        TagGroup::Wind => { /* nothing */ },
    };

    if let Some(s) = tag.get_super_mut() {
        apply_postprocess_function_for_tag(s, action, state, tag_path)?;
    }

    Ok(())
}

#[inline]
fn apply_default<T: Default + PartialEq>(what: &mut T, to_default: T, action: Action) {
    let zero = T::default();
    if zero == to_default {
        return
    }

    if action.default() {
        if *what == zero {
            *what = to_default
        }
    }
    else if action.undefault() {
        if *what == to_default {
            *what = zero
        }
    }
}

fn apply_seconds_to_ticks(what: &mut f32, action: Action) {
    if action.postprocess() {
        *what = seconds_to_ticks(*what)
    }
    else if action.unpostprocess() {
        *what = reverse_seconds_to_ticks(*what)
    }
}

/// Clamps the value to min and max.
#[inline]
fn apply_clamp<T: Default + PartialOrd>(what: &mut T, min: T, max: T, action: Action) {
    assert!(max >= min);
    apply_minimum_value_clamp(what, min, action);
    apply_maximum_value_clamp(what, max, action);
}

/// If the value is less than min, it gets clamped to min.
#[inline]
fn apply_minimum_value_clamp<T: Default + PartialOrd>(what: &mut T, min: T, action: Action) {
    if action.default() {
        if *what < min {
            *what = min;
        }
    }
    else if action.undefault() {
        let default = T::default();
        if min > default && *what == min {
            *what = default;
        }
    }
}

/// If the value is greater than max, it gets clamped to max.
#[inline]
fn apply_maximum_value_clamp<T: Default + PartialOrd>(what: &mut T, max: T, action: Action) {
    if action.default() {
        if *what > max {
            *what = max;
        }
    }
}

#[inline]
fn default_near_zero(what: &mut f32, to_default: f32, action: Action) {
    if action.default() {
        // NOTE: Past tools just checked if this was 0, but this is not actually accurate! For some
        // values, we need to actually check if it is within ±0.001 of 0.
        if what.fw_is_close_to_zero() {
            *what = to_default
        }
    }
    else if action.undefault() {
        if *what == to_default {
            *what = 0.0
        }
    }
}

#[inline]
fn default_near_zero_or_less(what: &mut f32, to_default: f32, action: Action) {
    if action.default() {
        // NOTE: Past tools just checked if this was 0, but this is not actually accurate! For some
        // values, we need to actually check if it is <0.001.
        if what.fw_is_close_to_zero_or_less() {
            *what = to_default
        }
    }
    else if action.undefault() {
        if *what == to_default {
            *what = 0.0
        }
    }
}

#[inline]
fn apply_default_le_zero<T: Default + PartialOrd>(what: &mut T, to_default: T, action: Action) {
    if action.default() {
        if *what <= T::default() {
            *what = to_default
        }
    }
    else if action.undefault() {
        if *what == to_default {
            *what = T::default()
        }
    }
}

#[inline]
fn postprocess_multiply(what: &mut f32, by: f32, action: Action) {
    if action.postprocess() {
        *what *= by;
    }
    else if action.unpostprocess() {
        *what /= by;
    }
}

#[inline]
fn postprocess_byteswap_big_to_little<T: SimpleWriteableData>(data: &mut [u8], name: &Arguments, action: Action) -> Result<(), PostprocessError> {
    // Although this effectively does the same thing, parsing fails on NaN floats.
    if action.postprocess() {
        match T::read_tag_data_simple::<byteorder::BigEndian>(data, Parameters::TAG_FILES) {
            Ok(n) => n.write_tag_data_simple::<byteorder::LittleEndian>(data, Parameters::TAG_FILES),
            Err(e) => fail_postprocess!("Failed to byteswap {name}: {e}")
        }
    }
    else if action.unpostprocess() {
        match T::read_tag_data_simple::<byteorder::LittleEndian>(data, Parameters::TAG_FILES) {
            Ok(n) => n.write_tag_data_simple::<byteorder::BigEndian>(data, Parameters::TAG_FILES),
            Err(e) => fail_postprocess!("Failed to un-byteswap {name}: {e}")
        }
    }
    Ok(())
}

impl dyn PostprocessState + '_ {
    pub(crate) fn read_model(&self, tag_path: &TagPath) -> Option<&dyn ModelFns> {
        self.read_tag(tag_path).downcast_model()
    }
}
