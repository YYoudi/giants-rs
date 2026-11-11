//! Measured neutral-object unlit inspection programs. Texture alpha is an
//! explicit capability; live tint and special object policy remain separate.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub flags: u32,
    pub vertex_rgb: bool,
    pub add_vertex_rgb: bool,
    pub blend: u8,
    pub alpha: f32,
    pub texture_alpha: bool,
}
pub fn prepare(
    m: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> Result<Profile, &'static str> {
    if texture_alpha {
        return Err("unlit texture-alpha program not established");
    }
    prepare_with_texture_alpha(m, false)
}
/// Descriptor capability including texture-alpha programs. Attached objects
/// retain `prepare`; static scenery uses the narrower `static_texture_color`
/// capability matching its existing opaque/straight-alpha RGB shader.
pub fn prepare_with_texture_alpha(
    m: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> Result<Profile, &'static str> {
    prepare_with_object_transparency(m, texture_alpha, 0.)
}
pub fn prepare_with_object_transparency(
    m: &giants_formats::gbs::Material,
    texture_alpha: bool,
    transparency: f32,
) -> Result<Profile, &'static str> {
    if !(0. ..=1.).contains(&transparency) {
        return Err("unlit object transparency outside unit profile");
    }
    let falloff = if transparency != 0. && transparency > m.falloff {
        transparency
    } else {
        m.falloff
    };
    let pass = crate::render_parts::material_pass(falloff, texture_alpha);
    let (flags, constant, _) =
        crate::material_descriptor::inspection_base(crate::material_descriptor::InspectionInput {
            part_flags: m.flags,
            blend: m.blend,
            falloff,
            has_texture: !m.texture.is_empty(),
            texture_alpha,
            fog: true,
            pass_mask: pass.mask(),
        })?;
    let (vertex_rgb, blend) = match flags {
        0x1094 => (true, 0),
        0x41094 => (true, 0),
        0x40014 => (false, 0),
        0x43094 => (true, 0),
        0x3094 => (true, 0),
        0x433b4 => (true, 2),
        0x40134 => (false, 1),
        0x4013c => (false, 1),
        0x431bc => (true, 1),
        0x433bc => (true, 2),
        0x4033c => (false, 2),
        0x40334 => (false, 2),
        0x413bc => (true, 2),
        0x411b4 => (true, 1),
        _ => return Err("unlit descriptor outside checked inspection programs"),
    };
    Ok(Profile {
        flags,
        vertex_rgb,
        add_vertex_rgb: matches!(flags, 0x1094 | 0x41094 | 0x413bc | 0x411b4),
        blend,
        texture_alpha,
        alpha: if blend == 0 {
            1.
        } else {
            (constant >> 24) as f32 / 255.
        },
    })
}

/// Static neutral-object capability with the existing common RGB shader.
/// Texture alpha is retained; no material/object opacity or special blend mode
/// is silently mapped onto that shader's opaque/straight-alpha paths.
pub fn static_texture_color(
    material: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> Result<Profile, &'static str> {
    if material.flags != 0x40000010 || material.falloff != 0. {
        return Err("static texture/color material outside checked flags/falloff");
    }
    let profile = prepare_with_texture_alpha(material, texture_alpha)?;
    if profile.alpha != 1. || profile.blend != if texture_alpha { 2 } else { 0 } {
        return Err("static texture/color material needs another alpha composition");
    }
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn additive_rgb_ghost_keeps_material_floor_and_object_fade() {
        let m = giants_formats::gbs::Material {
            texture: "synthetic".into(),
            bump_texture: String::new(),
            flags: 0x40000011,
            blend: 0.5,
            falloff: 0.4,
            colors: [0; 4],
            power: 0.,
        };
        // Original renderer state application: ADD(texture,diffuse),
        // SRCALPHA/ONE, factor alpha, depth test without depth writes.
        for (transparency, alpha) in [(0., 152), (0.25, 152), (0.5, 127), (1., 0)] {
            let p = prepare_with_object_transparency(&m, false, transparency).unwrap();
            assert_eq!((p.flags, p.blend), (0x411b4, 1));
            assert!(p.vertex_rgb && p.add_vertex_rgb && !p.texture_alpha);
            assert_eq!(p.alpha, alpha as f32 / 255.);
        }
    }
    #[test]
    fn opaque_texture_times_vertex_color_keeps_the_unguarded_descriptor() {
        let m = giants_formats::gbs::Material {
            texture: "synthetic".into(),
            bump_texture: String::new(),
            flags: 0x40000020,
            blend: 1.,
            falloff: 0.,
            colors: [0; 4],
            power: 0.,
        };
        let p = prepare_with_texture_alpha(&m, false).unwrap();
        // Native D draw-state fixture: MODULATE(texture,diffuse), opaque;
        // unlike ADD, this multiplies the supplied mesh RGB.
        assert_eq!(p.flags, 0x3094);
        assert!(p.vertex_rgb && !p.add_vertex_rgb);
        assert_eq!((p.blend, p.alpha, p.texture_alpha), (0, 1., false));
        let guarded =
            prepare_with_texture_alpha(&giants_formats::gbs::Material { blend: 0.5, ..m }, false)
                .unwrap();
        assert_eq!(guarded.flags, 0x43094);
        assert!(guarded.vertex_rgb && !guarded.add_vertex_rgb);
        assert_eq!((guarded.blend, guarded.alpha), (0, 1.));
    }
    #[test]
    fn static_texture_color_retains_opaque_and_sail_programs() {
        for (blend, texture_alpha, program, add) in [
            (0., false, 0x40014, false),
            (1., false, 0x1094, true),
            (0.15, false, 0x41094, true),
            (0., true, 0x4033c, false),
            (0.15, true, 0x413bc, true),
        ] {
            let mut material = giants_formats::gbs::Material {
                texture: "synthetic".into(),
                bump_texture: String::new(),
                flags: 0x40000010,
                falloff: 0.,
                blend,
                colors: [0; 4],
                power: 0.,
            };
            let p = static_texture_color(&material, texture_alpha).unwrap();
            assert_eq!((p.flags, p.add_vertex_rgb, p.alpha), (program, add, 1.));
            assert_eq!(p.blend, if texture_alpha { 2 } else { 0 });
            material.falloff = 0.5;
            assert!(static_texture_color(&material, texture_alpha).is_err());
            material.falloff = 0.;
            material.flags = 0x50000010;
            assert!(static_texture_color(&material, texture_alpha).is_err());
        }
    }
    #[test]
    fn full_source_blend_adds_vertex_color_without_alpha_blending() {
        let m = giants_formats::gbs::Material {
            texture: "synthetic".into(),
            bump_texture: String::new(),
            falloff: 0.,
            blend: 1.,
            flags: 0x40000010,
            colors: [0; 4],
            power: 0.,
        };
        let p = prepare(&m, false).unwrap();
        assert_eq!(p.flags, 0x1094);
        assert!(p.vertex_rgb && p.add_vertex_rgb);
        assert_eq!((p.blend, p.alpha), (0, 1.));
    }
    #[test]
    fn checked_alpha_programs_keep_native_composition_and_scalar_quantization() {
        // Expected programs and factors are native state-control fixtures.
        for (flags, falloff, blend, program, rgb, mode, alpha) in [
            (0x40000001, 0.4, 0., 0x4013c, false, 1, 152),
            (0x40000021, 0.4, 0.5, 0x431bc, true, 1, 152),
            (0x40000020, 0.5, 0.5, 0x433bc, true, 2, 127),
        ] {
            let m = giants_formats::gbs::Material {
                texture: "synthetic".into(),
                bump_texture: String::new(),
                falloff,
                blend,
                flags,
                colors: [0; 4],
                power: 0.,
            };
            let p = prepare_with_texture_alpha(&m, true).unwrap();
            assert_eq!(
                (p.flags, p.vertex_rgb, p.blend, p.texture_alpha),
                (program, rgb, mode, true)
            );
            assert_eq!(p.alpha, (alpha as f32) / 255.);
            assert!(prepare(&m, true).is_err());
        }
    }
}
