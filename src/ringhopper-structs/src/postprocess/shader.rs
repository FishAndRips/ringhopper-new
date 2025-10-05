use crate::definitions::tag::bitmap::{Bitmap, BitmapType};
use crate::definitions::tag::shader::{Shader, ShaderTextureScrollingAnimation, ShaderTypePC, ShaderTypeXbox};
use crate::definitions::tag::shader_effect::ShaderEffect;
use crate::definitions::tag::shader_environment::ShaderEnvironment;
use crate::definitions::tag::shader_model::ShaderModel;
use crate::definitions::tag::shader_transparent_chicago::{ShaderTransparentChicago, ShaderTransparentChicagoMap};
use crate::definitions::tag::shader_transparent_chicago_extended::ShaderTransparentChicagoExtended;
use crate::definitions::tag::shader_transparent_generic::{ShaderTransparentGeneric, ShaderTransparentMapParameters};
use crate::definitions::tag::shader_transparent_glass::ShaderTransparentGlass;
use crate::definitions::tag::shader_transparent_plasma::ShaderTransparentPlasma;
use crate::definitions::tag::shader_transparent_water::ShaderTransparentWater;
use crate::definitions::tag::{shader_type_of_tag_group, TagGroup};
use crate::postprocess::{apply_clamp, apply_default, apply_default_le_zero, Action};
use crate::{PostprocessError, PostprocessState, TagPath};
use alloc::borrow::Cow;
use funnel_web::color::ColorRGB;
use funnel_web::vector::Vector2D;
use crate::postprocess::bitmap::{assert_bitmap_types_from_dependency};

const DIFFUSE_TEXTURE: &[BitmapType] = &[BitmapType::_2dTextures];
const CUBEMAP_TEXTURE: &[BitmapType] = &[BitmapType::CubeMaps];

pub fn postprocess_shader_environment(shader: &mut ShaderEnvironment, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    macro_rules! default1 {
        ($what: expr) => {apply_default(&mut $what, 1.0, action)};
    }

    default1!(shader.diffuse.primary_detail_map_scale);
    default1!(shader.diffuse.secondary_detail_map_scale);
    default1!(shader.diffuse.micro_detail_map_scale);

    default1!(shader.texture_scrolling_animation.u_animation_scale);
    default1!(shader.texture_scrolling_animation.u_animation_period);
    default1!(shader.texture_scrolling_animation.v_animation_scale);
    default1!(shader.texture_scrolling_animation.v_animation_period);

    default1!(shader.self_illumination.primary_animation_period);
    default1!(shader.self_illumination.secondary_animation_period);
    default1!(shader.self_illumination.plasma_animation_period);

    default1!(shader.self_illumination.map_scale);
    default1!(shader.bump.bump_map_scale);

    apply_default(&mut shader.diffuse.material_color, ColorRGB::WHITE, action);

    apply_clamp(&mut shader.reflection.parallel_brightness, 0.0, 1.0, action);
    apply_clamp(&mut shader.reflection.perpendicular_brightness, 0.0, 1.0, action);

    if action.postprocess() {
        let bump_base = (shader.bump.bump_map.get(), shader.diffuse.base_map.get());
        let bump_scale = if shader.diffuse.flags.rescale_bump_map && let (Some(bump), Some(base)) = bump_base {
            calculate_shader_environment_bump_scale(bump, base, state)?
        }
        else {
            Vector2D::from_scalar(1.0)
        };

        assert_bitmap_types_from_dependency(&shader.diffuse.base_map, DIFFUSE_TEXTURE, &format_args!("Base map"), state)?;
        assert_bitmap_types_from_dependency(&shader.diffuse.micro_detail_map, DIFFUSE_TEXTURE, &format_args!("Micro detail map"), state)?;
        assert_bitmap_types_from_dependency(&shader.diffuse.primary_detail_map, DIFFUSE_TEXTURE, &format_args!("Primary detail map"), state)?;
        assert_bitmap_types_from_dependency(&shader.diffuse.secondary_detail_map, DIFFUSE_TEXTURE, &format_args!("Secondary detail map"), state)?;
        assert_bitmap_types_from_dependency(&shader.bump.bump_map, DIFFUSE_TEXTURE, &format_args!("Bump map"), state)?;
        assert_bitmap_types_from_dependency(&shader.reflection.reflection_cube_map, CUBEMAP_TEXTURE, &format_args!("Cube map"), state)?;

        shader.bump.bump_map_scale_xy = bump_scale * shader.bump.bump_map_scale;
    }

    Ok(())
}

fn calculate_shader_environment_bump_scale(bump: &TagPath, base: &TagPath, state: &mut dyn PostprocessState) -> Result<Vector2D, PostprocessError> {
    let Some(bump) = state.read_tag_group::<Bitmap>(bump)
        .expect("bump map bitmap not checked")
        .bitmap_data
        .get(0)
        else {
            fail_postprocess!("Bump map has no bitmaps.")
        };

    let Some(base) = state.read_tag_group::<Bitmap>(base)
        .expect("base map bitmap not checked")
        .bitmap_data
        .get(0)
        else {
            fail_postprocess!("Base map has no bitmaps.")
        };

    assert!(bump.width > 0, "bump/base dimensions are 0");
    assert!(base.width > 0, "bump/base dimensions are 0");
    assert!(bump.height > 0, "bump/base dimensions are 0");
    assert!(base.height > 0, "bump/base dimensions are 0");

    Ok(Vector2D {
        x: bump.width as f32 / base.width as f32,
        y: bump.height as f32 / base.height as f32,
    })
}

pub fn postprocess_shader_model(shader: &mut ShaderModel, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    macro_rules! default1 {
        ($what: expr) => {apply_default(&mut $what, 1.0, action)};
    }

    default1!(shader.maps.detail_map_scale);
    default1!(shader.maps.detail_map_v_scale);
    default1!(shader.self_illumination.animation_period);

    postprocess_shader_base_map_scale(&mut shader.maps.map_scale, action)?;
    postprocess_shader_texture_scrolling_animation(&mut shader.animation, action)?;

    // Cannot be undefaulted
    if action.default() {
        if shader.reflection.cutoff_distance <= shader.reflection.falloff_distance.max(0.0) {
            shader.reflection.cutoff_distance = 0.0;
            shader.reflection.falloff_distance = 0.0;
        }
    }

    // Not used, but still defaulted
    default1!(shader.reflection.bump_map_scale);

    if action.postprocess() {
        assert_bitmap_types_from_dependency(&shader.reflection.bump_map, DIFFUSE_TEXTURE, &format_args!("Bump map"), state)?;
        assert_bitmap_types_from_dependency(&shader.reflection.cube_map, CUBEMAP_TEXTURE, &format_args!("Cube map"), state)?;
        assert_bitmap_types_from_dependency(&shader.maps.base_map, DIFFUSE_TEXTURE, &format_args!("Base map"), state)?;
        assert_bitmap_types_from_dependency(&shader.maps.multipurpose_map, DIFFUSE_TEXTURE, &format_args!("Multipurpose map"), state)?;
        assert_bitmap_types_from_dependency(&shader.maps.detail_map, DIFFUSE_TEXTURE, &format_args!("Detail map"), state)?;
    }

    Ok(())
}

pub fn postprocess_shader_transparent_chicago(shader: &mut ShaderTransparentChicago, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    postprocess_shader_transparent_chicago_maps(&format_args!("Chicago maps"), &mut shader.maps, action, state)
}

pub fn postprocess_shader_transparent_chicago_extended(shader: &mut ShaderTransparentChicagoExtended, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    postprocess_shader_transparent_chicago_maps(&format_args!("2 stage maps"), &mut shader._2_stage_maps, action, state)?;
    postprocess_shader_transparent_chicago_maps(&format_args!("4 stage maps"), &mut shader._4_stage_maps, action, state)?;
    Ok(())
}

fn postprocess_shader_transparent_chicago_maps(name: &core::fmt::Arguments, maps: &mut [ShaderTransparentChicagoMap], action: Action, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for (map_index, map) in maps.iter_mut().enumerate() {
        postprocess_shader_transparent_map(&format_args!("{name} map #{map_index}"), &mut map.parameters, &mut map.animation, action, state)?;
    }

    Ok(())
}

fn postprocess_shader_transparent_map(name: &core::fmt::Arguments, parameters: &mut ShaderTransparentMapParameters, animation: &mut ShaderTextureScrollingAnimation, action: Action, state: &dyn PostprocessState) -> Result<(), PostprocessError> {
    if action.postprocess() {
        assert_bitmap_types_from_dependency(&parameters.map, &[BitmapType::_2dTextures, BitmapType::CubeMaps], name, state)?;
    }

    postprocess_shader_base_map_scale(&mut parameters.map_scale, action)?;
    postprocess_shader_texture_scrolling_animation(animation, action)?;

    Ok(())
}

fn postprocess_shader_base_map_scale(scale: &mut Vector2D, action: Action) -> Result<(), PostprocessError> {
    apply_default(scale, Vector2D::from_scalar(1.0), action);
    apply_default(&mut scale.x, scale.y, action);
    apply_default(&mut scale.y, scale.x, action);
    Ok(())
}

fn postprocess_shader_texture_scrolling_animation(animation: &mut ShaderTextureScrollingAnimation, action: Action) -> Result<(), PostprocessError> {
    macro_rules! default1 {
        ($what: expr) => {apply_default(&mut $what, 1.0, action)};
    }

    default1!(animation.u_animation_scale);
    default1!(animation.v_animation_scale);
    default1!(animation.u_animation_period);
    default1!(animation.v_animation_period);
    apply_default(&mut animation.rotation_animation_scale, 360.0, action);
    default1!(animation.rotation_animation_period);

    Ok(())
}

pub fn postprocess_shader_transparent_generic(shader: &mut ShaderTransparentGeneric, action: Action, _tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    for (index, map) in shader.maps.iter_mut().enumerate() {
        postprocess_shader_transparent_map(&format_args!("Generic map #{index}"), &mut map.parameters, &mut map.animation, action, state)?;
    }

    for i in &mut shader.stages {
        apply_default(&mut i.color0_animation_period, 1.0, action);
        // the other ones are not defaulted?
    }

    Ok(())
}

pub fn postprocess_shader_transparent_glass(shader: &mut ShaderTransparentGlass, action: Action) -> Result<(), PostprocessError> {
    macro_rules! default1 {
        ($what: expr) => {apply_default(&mut $what, 1.0, action)};
    }

    default1!(shader.background_tint.background_tint_map_scale);
    default1!(shader.diffuse.diffuse_detail_map_scale);
    default1!(shader.diffuse.diffuse_map_scale);
    default1!(shader.specular.specular_detail_map_scale);
    default1!(shader.specular.specular_map_scale);
    default1!(shader.reflection.bump_map_scale);

    apply_default(&mut shader.background_tint.background_tint_color, ColorRGB::WHITE, action);

    // TODO: Check bitmap types

    Ok(())
}

pub fn postprocess_shader_transparent_plasma(shader: &mut ShaderTransparentPlasma, action: Action) -> Result<(), PostprocessError> {
    macro_rules! default1 {
        ($what: expr) => {apply_default(&mut $what, 1.0, action)};
    }

    default1!(shader.primary_noise_map.animation_period);
    default1!(shader.secondary_noise_map.animation_period);

    apply_default_le_zero(&mut shader.intensity.intensity_exponent, 1.0, action);
    apply_default_le_zero(&mut shader.offset.offset_exponent, 1.0, action);

    // TODO: Check bitmap types

    Ok(())
}

pub fn postprocess_shader_transparent_water(shader: &mut ShaderTransparentWater, action: Action) -> Result<(), PostprocessError> {
    apply_default(&mut shader.ripples.scale, 1.0, action);
    apply_default(&mut shader.properties.reflection_map_properties.parallel_brightness, 1.0, action);
    apply_default(&mut shader.ripples.mipmap_levels, 1, action);

    for i in &mut shader.ripples.ripples {
        apply_default(&mut i.map_repeats, 1, action);
    }

    // TODO: Check bitmap types

    Ok(())
}

pub fn postprocess_shader(shader: &mut Shader, action: Action, tag_path: &TagPath, state: &mut dyn PostprocessState) -> Result<(), PostprocessError> {
    if !action.postprocess() {
        return Ok(())
    }

    let engine = state.engine();

    // We need to note whether or not shader_transparent_chicago_extended is supported on the target
    // engine, as it determines the index order.
    //
    // Basically, rather than putting shader_transparent_chicago_extended at the end of the list of
    // shaders, Gearbox put it immediately after chicago, offsetting everything after it. Since we
    // want to support both Xbox and PC, we have to be careful here.
    let supports_shader_transparent_chicago_extended = TagGroup::ShaderTransparentChicagoExtended.supports_engine(engine);
    
    let primary_group = tag_path.group();
    let Some((xbox, pc)) = shader_type_of_tag_group(primary_group) else {
        return Err(PostprocessError::GenericError {
            explanation: Cow::Owned(alloc::format!("Cannot postprocess_shader on a {primary_group}."))
        })
    };

    shader._type = if supports_shader_transparent_chicago_extended {
        pc as u16
    }
    else {
        xbox.ok_or_else(|| PostprocessError::GenericError {
            explanation: Cow::Owned(alloc::format!("Shader {primary_group} is not supported on the target engine and cannot be postprocessed."))
        })? as u16
    };

    Ok(())
}

pub fn postprocess_shader_effect(shader: &mut ShaderEffect, action: Action) {
    // Not an actual shader tag group, but it is included in-line in some tags
    const _: () = {
        assert!(ShaderTypeXbox::Effect as u16 == ShaderTypePC::Effect as u16, "ShaderTypeXbox::Effect and ShaderTypePC::Effect are not the same numerically");
    };

    if action.postprocess() {
        shader.shader_properties.shader._type = ShaderTypeXbox::Effect as u16;
    }
}
