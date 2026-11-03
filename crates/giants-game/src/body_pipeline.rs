//! Common normal preparation and part dispatch. Context is an explicit native
//! input; no class, graphics preset or world-lighting policy is inferred here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Normals {
    pub prepare: bool,
    pub context: bool,
}
impl Normals {
    pub fn select(
        suppressed: bool,
        has_definitions: bool,
        context_suppressed: bool,
        has_context: bool,
    ) -> Self {
        let prepare = !suppressed && has_definitions;
        Self {
            prepare,
            context: prepare && !context_suppressed && has_context,
        }
    }
    /// Flag word as read after the preceding preparation callback. That
    /// callback's effects are separate from this routing decision.
    pub fn contextual_preparation(self, mesh_flags: u32) -> bool {
        self.context && mesh_flags & 0x20000000 != 0
    }
    pub fn part(self, current_part_flags: u32) -> Part {
        if !self.prepare {
            Part::WithoutNormals
        } else if self.context && current_part_flags & 0x20000000 != 0 {
            Part::ContextNormals
        } else {
            Part::OrdinaryNormals
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    WithoutNormals,
    OrdinaryNormals,
    ContextNormals,
}
/// The native normal-list preparation can request normalization for the entire
/// model. This is separate from each part's subsequent rendering route.
pub fn prepare_normals(
    mesh: &giants_formats::gbs::Mesh,
    positions: &[[f32; 3]],
    normalize: bool,
) -> Result<Vec<[f32; 3]>, Box<dyn std::error::Error>> {
    let mut normals = mesh.render_normals_with_positions(positions)?;
    if normalize {
        for normal in &mut normals {
            *normal = normalize_prepared(*normal)?;
        }
    }
    Ok(normals)
}

/// Measured normal-preparation profile: float stores, 53-bit scalar arithmetic
/// and the original quantized square-root rule. The table is calculated from
/// mathematics, never imported from a game binary or runtime dump.
pub fn normalize_prepared(normal: [f32; 3]) -> Result<[f32; 3], &'static str> {
    if !normal.iter().all(|v| v.is_finite()) {
        return Err("non-finite prepared normal");
    }
    let [x, y, z] = normal.map(f64::from);
    let squared = ((z * z + x * x) + y * y) as f32;
    if !squared.is_finite() {
        return Err("prepared normal squared length outside finite profile");
    }
    let inverse = inverse_prepared_length(squared)?;
    let result = normal.map(|v| (f64::from(v) * inverse) as f32);
    if !result.iter().all(|v| v.is_finite()) {
        return Err("prepared normal result outside finite profile");
    }
    Ok(result)
}

pub(crate) fn inverse_prepared_length(squared: f32) -> Result<f64, &'static str> {
    if !squared.is_finite() || squared < 0. {
        return Err("prepared squared length outside finite profile");
    }
    let bits = squared.to_bits();
    Ok(if bits == 0 {
        0.
    } else {
        let index = (bits >> 8) & 0xffff;
        let representative = ((index & 0x7fff) << 8)
            | if index & 0x8000 != 0 {
                0x3f800000
            } else {
                0x40000000
            };
        let mantissa = f32::from_bits(representative).sqrt().to_bits() & 0x7fffff;
        let exponent = (bits.wrapping_add(0xc0800000) >> 1).wrapping_add(0x3f800000) & 0x7f800000;
        let length = f32::from_bits(mantissa | exponent);
        if length == 0. {
            0.
        } else {
            1. / f64::from(length)
        }
    })
}

/// Native direction-mode context color, packed as 00RRGGBB. The caller supplies
/// the selected direction, normal and UV fields; light selection is separate.
pub fn context_color(
    direction: [f32; 3],
    normal: [f32; 3],
    du: [f32; 3],
    dv: [f32; 3],
) -> Result<u32, &'static str> {
    if !direction
        .iter()
        .chain(normal.iter())
        .chain(du.iter())
        .chain(dv.iter())
        .all(|v| v.is_finite())
    {
        return Err("non-finite context color input");
    }
    let direction = direction.map(f64::from);
    let dot = |v: [f32; 3]| {
        let v = v.map(f64::from);
        (v[2] * direction[2] + v[1] * direction[1]) + v[0] * direction[0]
    };
    let u = f64::from(dot(du) as f32);
    let v = f64::from(dot(dv) as f32);
    let n = dot(normal);
    let inverse = inverse_prepared_length(((n * n + v * v) + u * u) as f32)?;
    let encode = |component: f64| {
        let byte = ((component + 1.) * 127.5).trunc();
        if !(0. ..=255.).contains(&byte) {
            Err("context color outside byte profile")
        } else {
            Ok(byte as u32)
        }
    };
    let r = encode(f64::from((u * inverse) as f32))?;
    let b = encode(f64::from((v * inverse) as f32))?;
    let g = encode(n * inverse)?;
    Ok((r << 16) | (g << 8) | b)
}

/// Cached RGBA words to the context GPU factor, including native integer
/// rounding and the byte-order change from cached color to 00RRGGBB.
pub fn context_factor_words(material: u32, light: u32) -> u32 {
    let m = material.to_le_bytes();
    let l = light.to_le_bytes();
    let c = std::array::from_fn::<_, 4, _>(|i| (u32::from(m[i]) * u32::from(l[i]) + 255) >> 8);
    (c[3] << 24) | (c[0] << 16) | (c[1] << 8) | c[2]
}

/// Directional context input in object coordinates. The D draw traversal negates
/// the primary direction, inverts the source object matrix, then transforms with
/// translation masked off. No normalization or live light selection is added.
pub fn local_context_direction(
    matrix: [f32; 16],
    flags: u32,
    light_direction: [f32; 3],
) -> Result<[f32; 3], &'static str> {
    if light_direction.iter().any(|v| !v.is_finite()) {
        return Err("non-finite context light direction");
    }
    let inverse = crate::animation::inverse_affine(matrix, flags)
        .map_err(|_| "context direction requires invertible affine matrix")?;
    let direction = light_direction.map(|v| -v);
    let result = if flags & 11 == 0 {
        direction
    } else {
        std::array::from_fn(|r| {
            let m = &inverse[r * 4..];
            let [x, y, z] = direction.map(f64::from);
            ((x * f64::from(m[0]) + y * f64::from(m[1])) + z * f64::from(m[2])) as f32
        })
    };
    if result.iter().any(|v| !v.is_finite()) {
        return Err("non-finite local context direction");
    }
    Ok(result)
}
impl Part {
    pub fn code(self) -> u32 {
        match self {
            Self::WithoutNormals => 0,
            Self::OrdinaryNormals => 1,
            Self::ContextNormals => 2,
        }
    }
}

/// Existing common preview renderer capabilities. Route selection is checked
/// separately; accepting a source part does not certify its pixels or LOD.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Surface {
    Rgb,
    Lit,
    AdditiveLit { factor: f32 },
    AdditiveLitDiffuse { factor: f32 },
    Cube { factor: f32 },
    AdditiveCube { factor: f32 },
    AlphaCube { factor: f32 },
    Context,
}
impl Surface {
    /// Factor alpha and whether texture RGB bypasses the lit diffuse color.
    pub fn lit_additive(self) -> Option<(f32, bool)> {
        match self {
            Self::AdditiveLit { factor } => Some((factor, true)),
            Self::AdditiveLitDiffuse { factor } => Some((factor, false)),
            _ => None,
        }
    }
    pub fn cube_profile(self) -> Option<(f32, u8)> {
        match self {
            Self::Cube { factor } => Some((factor, 0)),
            Self::AdditiveCube { factor } => Some((factor, 1)),
            Self::AlphaCube { factor } => Some((factor, 2)),
            _ => None,
        }
    }
}
pub fn preview_surface(
    normals: Normals,
    material: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> Result<Surface, &'static str> {
    if !(0. ..=1.).contains(&material.falloff) {
        return Err("material falloff outside measured unit interval");
    }
    match normals.part(material.flags) {
        Part::ContextNormals => {
            if material.flags != 0xe0000020
                || material.falloff != 0.
                || material.blend != 1.
                || texture_alpha
                || material.bump_texture.is_empty()
            {
                return Err("context material outside opaque directional profile");
            }
            Ok(Surface::Context)
        }
        Part::WithoutNormals => {
            if additive_rgb(material, texture_alpha).is_some() {
                return Ok(Surface::Rgb);
            }
            if untextured_diffuse(material) && !texture_alpha {
                return Ok(Surface::Rgb);
            }
            // The RGB body shader already samples an opaque texture and uses
            // unit vertex color when the source has no RGB flag. Reuse only
            // the independently checked texture-only neutral-object program.
            if crate::unlit_material::static_texture_color(material, texture_alpha)
                .is_ok_and(|p| p.flags == 0x40014 && !p.vertex_rgb && p.blend == 0 && p.alpha == 1.)
            {
                return Ok(Surface::Rgb);
            }
            if !matches!(
                material.flags,
                0x40000000 | 0x40000001 | 0x40000020 | 0xc0000000 | 0xc0000020
            ) {
                return Err("unlit material outside common RGB capability");
            }
            Ok(Surface::Rgb)
        }
        Part::OrdinaryNormals => {
            if let Some(factor) = additive_lit_factor(material, texture_alpha) {
                return Ok(Surface::AdditiveLit { factor });
            }
            if let Some(factor) = additive_lit_diffuse_factor(material, texture_alpha) {
                return Ok(Surface::AdditiveLitDiffuse { factor });
            }
            if lit_null_base(material) && !texture_alpha {
                return Ok(Surface::Lit);
            }
            if untextured_diffuse(material) {
                if texture_alpha {
                    return Err("untextured normal part cannot have texture alpha");
                }
                return Ok(Surface::Lit);
            }
            let pass = crate::render_parts::material_pass(material.falloff, texture_alpha);
            // The existing measured rigid gun program uses this opaque,
            // texture-only normal profile with the same shared lit shader.
            if matches!(material.flags, 0xc0000010 | 0xc0000001)
                && (texture_alpha || material.falloff != 0.)
            {
                return Err("textured normal material outside measured opaque profile");
            }
            if material.flags & 0x10000000 != 0 {
                if !matches!(
                    material.flags,
                    0x50000000
                        | 0x50000010
                        | 0x50000011
                        | 0x50000020
                        | 0xd0000001
                        | 0xd0000010
                        | 0xd0000020
                ) || texture_alpha
                {
                    return Err("auxiliary material outside opaque normal capability");
                }
                // Same explicit inspection inputs as the common rigid path.
                // Live object/settings projection remains separately unresolved.
                let (base_flags, constant, constant_guard) =
                    crate::material_descriptor::inspection_base(
                        crate::material_descriptor::InspectionInput {
                            part_flags: material.flags,
                            blend: material.blend,
                            falloff: material.falloff,
                            has_texture: !material.texture.is_empty(),
                            texture_alpha,
                            fog: true,
                            pass_mask: pass.mask(),
                        },
                    )?;
                let descriptor =
                    crate::material_descriptor::lit(crate::material_descriptor::LitInput {
                        part_flags: material.flags,
                        base_flags,
                        constant,
                        constant_guard,
                        mode: 0x5000,
                        toggle: false,
                        force_auxiliary: false,
                    });
                let (factor, blend) = descriptor.cube_blend_profile()?;
                return Ok(match blend {
                    1 => Surface::AdditiveCube { factor },
                    2 => Surface::AlphaCube { factor },
                    _ => Surface::Cube { factor },
                });
            }
            if !matches!(
                material.flags,
                0x40000000
                    | 0x40000001
                    | 0x40000010
                    | 0x40000020
                    | 0xc0000000
                    | 0xc0000001
                    | 0xc0000010
                    | 0xc0000020
            ) || pass == crate::render_parts::Pass::ObjectOrMaterial
            {
                return Err("lit material outside common normal capability");
            }
            Ok(Surface::Lit)
        }
    }
}

/// Measured texture-only additive RGB descriptor, without a source alpha image.
pub fn additive_rgb(
    material: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> Option<crate::unlit_material::Profile> {
    if material.flags != 0x40000011 || !material.bump_texture.is_empty() || texture_alpha {
        return None;
    }
    let profile = crate::unlit_material::prepare(material, texture_alpha).ok()?;
    (profile.flags == 0x40134
        && !profile.vertex_rgb
        && !profile.add_vertex_rgb
        && profile.blend == 1)
        .then_some(profile)
}

/// Checked additive, texture-factor-white normal descriptor. Lighting policy
/// and secondary color remain explicit renderer inputs, not source flags.
pub fn additive_lit_factor(
    material: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> Option<f32> {
    let d = additive_lit_descriptor(material, texture_alpha)?;
    (d.flags == 0x63137 && !d.auxiliary && d.constant & 0xffffff == 0xffffff)
        .then_some((d.constant >> 24) as f32 / 255.)
}
/// Checked D lit additive program: texture times lit diffuse, factor alpha.
pub fn additive_lit_diffuse_factor(
    material: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> Option<f32> {
    let d = additive_lit_descriptor(material, texture_alpha)?;
    (d.flags == 0x451b7 && !d.auxiliary).then_some((d.constant >> 24) as f32 / 255.)
}
fn additive_lit_descriptor(
    material: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> Option<crate::material_descriptor::Descriptor> {
    if material.flags != 0x40000001
        || material.texture.is_empty()
        || !material.bump_texture.is_empty()
        || texture_alpha
    {
        return None;
    }
    let pass = crate::render_parts::material_pass(material.falloff, texture_alpha);
    let (base_flags, constant, constant_guard) =
        crate::material_descriptor::inspection_base(crate::material_descriptor::InspectionInput {
            part_flags: material.flags,
            blend: material.blend,
            falloff: material.falloff,
            has_texture: true,
            texture_alpha,
            fog: true,
            pass_mask: pass.mask(),
        })
        .ok()?;
    let d = crate::material_descriptor::lit(crate::material_descriptor::LitInput {
        part_flags: material.flags,
        base_flags,
        constant,
        constant_guard,
        mode: 0x5000,
        toggle: false,
        force_auxiliary: false,
    });
    Some(d)
}

/// Source-measured opaque diffuse-only profile, with or without normals. The debug null-texture
/// program selects diffuse without texture or specular contribution; other profiles remain needs.
pub fn untextured_diffuse(material: &giants_formats::gbs::Material) -> bool {
    material.texture.is_empty()
        && material.bump_texture.is_empty()
        && material.flags == 0x22
        && material.falloff == 0.
        && material.blend == 1.
}

/// Null base with a lit auxiliary descriptor: the measured renderer selects
/// primary diffuse directly and disables later stages. The auxiliary pointer
/// alone does not imply a sampled reflection. Live lighting policy is separate.
pub fn lit_null_base(material: &giants_formats::gbs::Material) -> bool {
    if !material.texture.is_empty()
        || !material.bump_texture.is_empty()
        || material.flags != 0x10000000
        || material.falloff != 0.
    {
        return false;
    }
    let Ok((base_flags, constant, constant_guard)) =
        crate::material_descriptor::inspection_base(crate::material_descriptor::InspectionInput {
            part_flags: material.flags,
            blend: material.blend,
            falloff: material.falloff,
            has_texture: false,
            texture_alpha: false,
            fog: true,
            pass_mask: 1,
        })
    else {
        return false;
    };
    crate::material_descriptor::lit(crate::material_descriptor::LitInput {
        part_flags: material.flags,
        base_flags,
        constant,
        constant_guard,
        mode: 0x5000,
        toggle: false,
        force_auxiliary: false,
    })
    .flags
        == 0x47093
}

#[cfg(test)]
mod tests {
    #[test]
    fn remaining_weapon_profiles_keep_native_cube_opaque_and_lit_additive_routes() {
        let normals = super::Normals::select(false, true, false, true);
        let mut m = giants_formats::gbs::Material {
            texture: "synthetic".into(),
            bump_texture: String::new(),
            flags: 0xd0000010,
            blend: 0.1,
            falloff: 0.,
            colors: [0; 4],
            power: 0.,
        };
        // D source draw fixture:470b7/19000000 when requesting all passes;
        // the material-only opaque packet is47097 with the same factor.
        assert_eq!(
            super::preview_surface(normals, &m, false),
            Ok(super::Surface::Cube { factor: 25. / 255. })
        );
        m.flags = 0xc0000001;
        m.blend = 0.75;
        assert_eq!(
            super::preview_surface(normals, &m, false),
            Ok(super::Surface::Lit)
        );
        assert!(super::preview_surface(normals, &m, true).is_err());
        m.falloff = 0.4;
        assert!(super::preview_surface(normals, &m, false).is_err());
        m.flags = 0x40000001;
        let s = super::preview_surface(normals, &m, false).unwrap();
        // D source451b7/98000000: preserve lit diffuse and constant alpha.
        assert_eq!(
            s,
            super::Surface::AdditiveLitDiffuse {
                factor: 152. / 255.
            }
        );
        assert_eq!(s.lit_additive(), Some((152. / 255., false)));
        assert_eq!(super::additive_lit_factor(&m, false), None);
        m.blend = 0.;
        assert_eq!(
            super::preview_surface(normals, &m, false)
                .unwrap()
                .lit_additive(),
            Some((152. / 255., true))
        );
    }
    use super::*;
    #[test]
    fn additive_source_profiles_keep_native_quantization_and_distinct_normal_routes() {
        let mut m = giants_formats::gbs::Material {
            texture: "synthetic".into(),
            bump_texture: String::new(),
            flags: 0x40000011,
            falloff: 0.3,
            blend: 0.,
            colors: [0; 4],
            power: 0.,
        };
        let rgb = Normals::select(false, false, false, true);
        let lit = Normals::select(false, true, false, true);
        assert_eq!(additive_rgb(&m, false).unwrap().alpha, 178. / 255.);
        assert_eq!(preview_surface(rgb, &m, false), Ok(Surface::Rgb));
        assert!(additive_rgb(&m, true).is_none());
        m.flags = 0x40000001;
        m.falloff = 0.2;
        assert_eq!(additive_lit_factor(&m, false), Some(203. / 255.));
        assert_eq!(
            preview_surface(lit, &m, false),
            Ok(Surface::AdditiveLit {
                factor: 203. / 255.
            })
        );
        assert!(additive_lit_factor(&m, true).is_none());
        m.blend = 0.5;
        assert!(additive_lit_factor(&m, false).is_none());
        m.blend = 0.;
        m.bump_texture = "unsupported".into();
        assert!(additive_lit_factor(&m, false).is_none());
    }
    #[test]
    fn lit_null_base_requires_normals_and_the_checked_descriptor() {
        let mut m = giants_formats::gbs::Material {
            texture: String::new(),
            bump_texture: String::new(),
            flags: 0x10000000,
            falloff: 0.,
            blend: 0.5,
            colors: [0; 4],
            power: 20.,
        };
        let normal = Normals::select(false, true, false, true);
        assert!(lit_null_base(&m));
        assert_eq!(preview_surface(normal, &m, false), Ok(Surface::Lit));
        assert!(preview_surface(Normals::select(false, false, false, true), &m, false).is_err());
        assert!(preview_surface(normal, &m, true).is_err());
        for blend in [0., f32::NAN, -1., 2.] {
            m.blend = blend;
            assert!(!lit_null_base(&m));
        }
        m.blend = 1.;
        assert!(lit_null_base(&m));
        m.falloff = 0.3;
        assert!(!lit_null_base(&m));
        m.falloff = 0.;
        m.bump_texture = "synthetic".into();
        assert!(!lit_null_base(&m));
        m.bump_texture.clear();
        m.texture = "synthetic".into();
        assert!(!lit_null_base(&m));
    }
    #[test]
    fn unlit_body_reuses_only_the_checked_opaque_texture_program() {
        let mut material = giants_formats::gbs::Material {
            flags: 0x40000010,
            falloff: 0.,
            blend: 0.,
            texture: "synthetic".into(),
            bump_texture: String::new(),
            colors: [0; 4],
            power: 0.,
        };
        let normals = Normals::select(false, false, false, true);
        assert_eq!(preview_surface(normals, &material, false), Ok(Surface::Rgb));
        assert!(preview_surface(normals, &material, true).is_err());
        material.blend = 0.15;
        assert!(preview_surface(normals, &material, false).is_err());
        material.blend = 0.;
        material.falloff = 0.5;
        assert!(preview_surface(normals, &material, false).is_err());
    }
    #[test]
    fn extractor_reflection_reuses_the_measured_additive_descriptor() {
        let mut material = giants_formats::gbs::Material {
            flags: 0xd0000001,
            falloff: 0.3,
            blend: 0.2,
            texture: "synthetic".into(),
            bump_texture: String::new(),
            colors: [0; 4],
            power: 25.,
        };
        let normals = Normals::select(false, true, true, false);
        assert_eq!(
            preview_surface(normals, &material, false),
            Ok(Surface::AdditiveCube {
                factor: 178. / 255.
            })
        );
        assert!(preview_surface(normals, &material, true).is_err());
        material.flags = 0xd0000003;
        assert!(preview_surface(normals, &material, false).is_err());
    }
    #[test]
    fn null_texture_normal_requires_the_measured_material_profile() {
        let mut material = giants_formats::gbs::Material {
            flags: 0x22,
            falloff: 0.,
            blend: 1.,
            texture: String::new(),
            bump_texture: String::new(),
            colors: [0; 4],
            power: 25.,
        };
        let normals = Normals::select(false, true, false, false);
        assert_eq!(preview_surface(normals, &material, false), Ok(Surface::Lit));
        assert_eq!(
            preview_surface(
                Normals::select(false, false, false, false),
                &material,
                false
            ),
            Ok(Surface::Rgb)
        );
        assert!(preview_surface(normals, &material, true).is_err());
        material.falloff = 0.5;
        assert!(!untextured_diffuse(&material));
        material.falloff = 0.;
        material.blend = 0.;
        assert!(!untextured_diffuse(&material));
        material.blend = 1.;
        material.texture = "missing".into();
        assert!(!untextured_diffuse(&material));
    }
    #[test]
    fn measured_textured_normal_program_requires_normals_and_opaque_input() {
        let mut material = giants_formats::gbs::Material {
            flags: 0xc0000010,
            falloff: 0.,
            blend: 0.7,
            texture: "synthetic".into(),
            bump_texture: String::new(),
            colors: [0; 4],
            power: 0.,
        };
        let normals = Normals::select(false, true, false, true);
        assert_eq!(preview_surface(normals, &material, false), Ok(Surface::Lit));
        assert!(
            preview_surface(Normals::select(true, true, false, true), &material, false).is_err()
        );
        assert!(preview_surface(normals, &material, true).is_err());
        material.falloff = 0.5;
        assert!(preview_surface(normals, &material, false).is_err());
    }
    #[test]
    fn transparent_cube_retains_quantized_opacity_and_straight_blend() {
        let material = giants_formats::gbs::Material {
            flags: 0x50000000,
            falloff: 0.3,
            blend: 0.2,
            texture: "surface".into(),
            bump_texture: String::new(),
            colors: [0; 4],
            power: 0.,
        };
        let surface =
            preview_surface(Normals::select(false, true, false, true), &material, false).unwrap();
        assert_eq!(surface.cube_profile(), Some((178. / 255., 2)));
        assert!(
            preview_surface(Normals::select(false, true, false, true), &material, true).is_err()
        );
    }
    #[test]
    fn context_color_places_the_normal_on_green_and_rejects_nonfinite_inputs() {
        let n = [0., 0., 1.];
        let u = [1., 0., 0.];
        let v = [0., 1., 0.];
        assert_eq!(context_color([1., 0., 0.], n, u, v).unwrap(), 0xff7f7f);
        assert_eq!(context_color([0., 0., 1.], n, u, v).unwrap(), 0x7fff7f);
        assert_eq!(context_color([0.; 3], n, u, v).unwrap(), 0x7f7f7f);
        assert!(context_color([f32::NAN; 3], n, u, v).is_err());
    }
    #[test]
    fn prepared_normal_profile_preserves_zero_and_rejects_unbounded_values() {
        assert_eq!(normalize_prepared([0.; 3]).unwrap(), [0.; 3]);
        assert_eq!(normalize_prepared([3., 4., 0.]).unwrap(), [0.6, 0.8, 0.]);
        assert!(normalize_prepared([f32::NAN, 0., 0.]).is_err());
        assert!(normalize_prepared([f32::INFINITY, 0., 0.]).is_err());
        assert!(normalize_prepared([f32::MAX, 0., 0.]).is_err());
    }
    #[test]
    fn missing_normal_definitions_prevent_both_normal_paths() {
        let profile = Normals::select(false, false, false, true);
        assert!(!profile.prepare);
        assert!(!profile.contextual_preparation(u32::MAX));
        assert_eq!(profile.part(u32::MAX), Part::WithoutNormals);
    }
    #[test]
    fn suppressing_context_retains_ordinary_normals() {
        let profile = Normals::select(false, true, true, true);
        assert!(profile.prepare);
        assert_eq!(profile.part(u32::MAX), Part::OrdinaryNormals);
    }
    #[test]
    fn mesh_and_part_flags_are_independent_decisions() {
        let profile = Normals::select(false, true, false, true);
        assert!(!profile.contextual_preparation(0));
        assert_eq!(profile.part(0xe0000020), Part::ContextNormals);
        assert!(profile.contextual_preparation(0x20000000));
        assert_eq!(profile.part(0xc0000020), Part::OrdinaryNormals);
    }
}
