//! Lit-part descriptor preparation after object/texture policy has supplied
//! the base flags and constant. Graphics API state application is separate.
#[derive(Clone, Copy, Debug)]
pub struct LitInput {
    /// Current flags after source loading and runtime preparation.
    pub part_flags: u32,
    pub base_flags: u32,
    pub constant: u32,
    /// Explicit guard supplied by the preceding preparation stage. Its source
    /// must be established before adapting this kernel to a runtime object.
    pub constant_guard: bool,
    pub mode: u32,
    pub toggle: bool,
    pub force_auxiliary: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub flags: u32,
    pub constant: u32,
    /// The native fragment supplies the current world's auxiliary descriptor.
    /// This does not itself resolve a texture or bind a graphics resource.
    pub auxiliary: bool,
}
/// Neutral-object inspection inputs before the lit descriptor. Object opacity,
/// special render modes and live texture preparation must be resolved separately.
#[derive(Clone, Copy, Debug)]
pub struct InspectionInput {
    pub part_flags: u32,
    pub blend: f32,
    pub falloff: f32,
    pub has_texture: bool,
    pub texture_alpha: bool,
    pub fog: bool,
    pub pass_mask: u8,
}
pub fn inspection_base(input: InspectionInput) -> Result<(u32, u32, bool), &'static str> {
    if !input.falloff.is_finite() || !(0. ..=1.).contains(&input.falloff) {
        return Err("material falloff outside measured unit interval");
    }
    let (mut constant, _) = part_constant(input.blend)?;
    let mut flags = (u32::from(input.has_texture) * 4)
        | (u32::from(input.texture_alpha) * 8)
        | (u32::from(input.fog) * 0x10);
    if constant & 0xff000000 != 0 {
        flags |= 0x80;
        flags |= match input.part_flags & 0xf0 {
            0x10 => 0x1000,
            0x20 => 0x3000,
            _ => 0x2000,
        };
    }
    if input.falloff != 0. || input.texture_alpha {
        flags |= match input.part_flags & 0xf {
            1 => 0x100,
            2 => 0x200,
            _ => 0x300,
        };
        constant = (((1. - f64::from(input.falloff)) * 255.).trunc() as u32) << 24;
        flags |= 0x40000;
    }
    if input.pass_mask & 6 != 0 {
        flags |= 0x20;
    }
    let guard = constant & 0xff000000 != 0xff000000;
    if guard {
        flags |= 0x40000;
    }
    Ok((flags, constant, guard))
}
/// Same neutral render-policy boundary as inspection_base, with the measured
/// per-object transparency override. Part flags, texture and pass are explicit.
pub fn object_base(
    mut input: InspectionInput,
    transparency: f32,
) -> Result<(u32, u32, bool), &'static str> {
    if !(0. ..=1.).contains(&transparency) {
        return Err("object transparency outside unit profile");
    }
    if transparency != 0. && transparency > input.falloff {
        input.falloff = transparency;
    }
    inspection_base(input)
}
impl Descriptor {
    /// Acquired opaque zero-blend normal-material program: texture times white
    /// factor, alpha and subsequent stages disabled. This is distinct from a
    /// cube program whose factor alpha happens to be zero.
    pub fn texture_factor_only(self) -> bool {
        self.auxiliary && self.flags == 0x63017 && self.constant == 0x00ffffff
    }
    /// Explicit native reflection composition: 0 opaque, 1 additive, 2 alpha.
    pub fn cube_blend_profile(self) -> Result<(f32, u8), &'static str> {
        if self.auxiliary && self.flags == 0x473b7 {
            return Ok(((self.constant >> 24) as f32 / 255., 2));
        }
        self.cube_profile()
            .map(|(factor, additive)| (factor, u8::from(additive)))
    }
    /// Checked directional inspection profiles: opaque reflection or additive
    /// reflection with factor alpha. Queue ordering remains a separate policy.
    pub fn cube_profile(self) -> Result<(f32, bool), &'static str> {
        if !self.auxiliary || !matches!(self.flags, 0x47097 | 0x471b7) {
            return Err("descriptor outside measured lit cube profiles");
        }
        Ok(((self.constant >> 24) as f32 / 255., self.flags == 0x471b7))
    }
    /// Measured lit reflection program with factor-alpha mixing. Other flags
    /// must not be interpreted through this shader capability.
    pub fn cube_factor(self) -> Result<f32, &'static str> {
        if !self.auxiliary || self.flags != 0x47097 {
            return Err("descriptor outside measured lit cube blend profile");
        }
        Ok((self.constant >> 24) as f32 / 255.)
    }
}
/// Bounded source input for the independently recorded zero-blend program.
/// Other material/texture combinations retain their existing capability checks.
pub fn opaque_zero_blend_texture_factor(
    material: &giants_formats::gbs::Material,
    texture_alpha: bool,
) -> bool {
    if material.flags != 0x50000000
        || material.blend != 0.
        || material.falloff != 0.
        || material.texture.is_empty()
        || texture_alpha
    {
        return false;
    }
    let Ok((base_flags, constant, constant_guard)) = inspection_base(InspectionInput {
        part_flags: material.flags,
        blend: material.blend,
        falloff: material.falloff,
        has_texture: true,
        texture_alpha,
        fog: true,
        pass_mask: 1,
    }) else {
        return false;
    };
    lit(LitInput {
        part_flags: material.flags,
        base_flags,
        constant,
        constant_guard,
        mode: 0x5000,
        toggle: false,
        force_auxiliary: false,
    })
    .texture_factor_only()
}
/// Source material blend scalar to the native high-byte constant. This bounded
/// profile uses 53-bit multiplication followed by truncation, without clamping.
pub fn part_constant(blend: f32) -> Result<(u32, bool), &'static str> {
    if !blend.is_finite() || !(0. ..=1.).contains(&blend) {
        return Err("material blend outside measured unit interval");
    }
    let constant = ((f64::from(blend) * 255.).trunc() as u32) << 24;
    Ok((constant, constant != 0xff000000))
}
pub fn lit(input: LitInput) -> Descriptor {
    let auxiliary = (!input.toggle && input.part_flags & 0x10000000 != 0) || input.force_auxiliary;
    let mut constant = input.constant;
    let mut flags = if auxiliary {
        let mut flags = (input.base_flags & !0x8000) | 0x7003;
        if input.base_flags & 0xf00 == 0 {
            if !input.constant_guard {
                constant = (constant & 0x00ffffff) | 0xc0000000;
            }
            flags |= 0x40000;
        }
        flags
    } else {
        let mask = if input.part_flags & 0x10000000 == 0 {
            0xffff0fff
        } else {
            0xffff0ff7
        };
        (input.base_flags & mask) | input.mode | 3
    };
    if flags & 0x80 == 0 {
        flags = (flags & !0xc000) | 0x23000;
        constant |= 0x00ffffff;
    }
    Descriptor {
        flags,
        constant,
        auxiliary,
    }
}

#[cfg(test)]
mod object_transparency_tests {
    use super::*;
    #[test]
    fn opaque_zero_blend_is_texture_factor_not_a_zero_alpha_cube_program() {
        let source = giants_formats::gbs::Material {
            flags: 0x50000000,
            falloff: 0.,
            blend: 0.,
            texture: "synthetic".into(),
            bump_texture: String::new(),
            colors: [0; 4],
            power: 0.,
        };
        assert!(opaque_zero_blend_texture_factor(&source, false));
        assert!(!opaque_zero_blend_texture_factor(&source, true));
        let mut cube = source;
        cube.blend = 1.;
        assert!(!opaque_zero_blend_texture_factor(&cube, false));
        assert!(
            !Descriptor {
                flags: 0x47097,
                constant: 0x00ffffff,
                auxiliary: true
            }
            .texture_factor_only()
        );
    }
    #[test]
    fn object_override_retains_stronger_material_transparency_and_quantizes() {
        let input = InspectionInput {
            part_flags: 0x40000000,
            blend: 0.,
            falloff: 0.5,
            has_texture: true,
            texture_alpha: false,
            fog: true,
            pass_mask: 2,
        };
        assert_eq!(
            object_base(input, 0.25).unwrap(),
            inspection_base(input).unwrap()
        );
        let (flags, constant, _) = object_base(input, 0.75).unwrap();
        assert_eq!(flags & 0xf00, 0x300);
        assert_eq!(constant >> 24, 63);
        assert_eq!(object_base(input, 1.).unwrap().1 >> 24, 0);
        assert!(object_base(input, f32::NAN).is_err());
    }
}
