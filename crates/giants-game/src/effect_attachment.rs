//! Bounded source inspection of persistent attached FX. This is neither the
//! named-effect allocator nor the state78/world callback scheduler.
#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub class: i32,
    pub scale: f32,
    /// Retained without assigning a meaning to the first spawn operand.
    pub opaque: u32,
}
impl Recipe {
    pub fn project(p: &giants_formats::effect_program::Program) -> Result<Self, &'static str> {
        if p.records.iter().map(|r| r.opcode).collect::<Vec<_>>() != [0x24, 6, 0x1b, 7, 2] {
            return Err("effect program outside persistent attachment inspection capability");
        }
        let word = |record: usize, at: usize| -> Result<u32, &'static str> {
            let bytes = p.records[record]
                .bytes
                .get(at..at + 4)
                .ok_or("truncated effect operand")?;
            Ok(u32::from_le_bytes(
                bytes.try_into().map_err(|_| "invalid effect word")?,
            ))
        };
        if word(0, 9)? != 0 {
            return Err("nonzero attachment reference operand outside checked profile");
        }
        let scale = f32::from_bits(word(2, 1)?);
        let class = word(0, 5)? as i32;
        if class < 0 || !scale.is_finite() || scale <= 0. {
            return Err("invalid persistent attachment class/scale");
        }
        Ok(Self {
            class,
            scale,
            opaque: word(0, 1)?,
        })
    }
    pub fn facing(
        self,
        p: &giants_formats::class_program::Program,
    ) -> Result<crate::attachment_pose::InitialFacing, &'static str> {
        if p.records.iter().any(|r| {
            !matches!(
                r.opcode,
                1 | 0x13 | 0x14 | 0x17 | 0x16 | 0x1e | 0x1d | 0x22 | 0xff
            ) || r.opcode == 0x13 && r.bytes.get(1) != Some(&1)
                || r.opcode == 0x1e && r.bytes.get(1) != Some(&78)
        }) || !p.records.iter().any(|r| r.opcode == 0x1e)
        {
            return Err("persistent effect class outside checked initial state78 profile");
        }
        let flags = crate::object_lists::DirectFlags::project(self.class, p);
        if flags.personality & 0x84 == 0 {
            return Err("persistent effect requires camera-facing class fields");
        }
        Ok(crate::attachment_pose::InitialFacing {
            personality: flags.personality,
            scale: self.scale,
        })
    }
}
/// Fresh source support: generation zero still contains the zeroed actor cache.
/// The caller supplies the actor generation and an independently resolved anchor;
/// this does not implement later shared-node cache invalidation or scheduling.
pub fn source_creation_matrix(
    generation: u64,
    actor: i32,
    anchor: crate::procedural_aim::Matrix,
) -> crate::procedural_aim::Matrix {
    if generation == 0 && actor >= 0 {
        let mut values = [0.; 16];
        values[15] = 1.;
        crate::procedural_aim::Matrix { flags: 0, values }
    } else {
        anchor
    }
}
/// Raw constructor position, retained independently of later matrix refreshes.
pub fn source_creation_position(
    generation: u64,
    actor: i32,
    anchor: crate::procedural_aim::Matrix,
) -> [f32; 3] {
    let matrix = source_creation_matrix(generation, actor, anchor);
    [matrix.values[3], matrix.values[7], matrix.values[11]]
}

/// Normal attached FX after its own class reference has been evaluated. The
/// effect support scale is an explicit input, separate from the source actor.
pub fn normal_matrix(
    anchor: crate::procedural_aim::Matrix,
    size: f32,
    support_scale: f32,
    child_reference: crate::procedural_aim::Matrix,
) -> Result<crate::procedural_aim::Matrix, &'static str> {
    if !size.is_finite() || size <= 0. || !support_scale.is_finite() || support_scale <= 0. {
        return Err("invalid attached effect scale");
    }
    let ratio = (f64::from(size) / f64::from(support_scale)) as f32;
    let mut scale = crate::procedural_aim::Matrix::identity();
    scale.flags = 2;
    for i in [0, 5, 10] {
        scale.values[i] = ratio;
    }
    anchor.mul(scale)?.mul(child_reference.inverse()?)
}
/// Resolve a one-shot child binding before detaching it. Both reference slots
/// belong to their respective source models; camera-facing draw rotation is
/// deliberately absent from this world pose.
pub fn initial_child_matrix(
    parent: crate::procedural_aim::Matrix,
    parent_reference: crate::procedural_aim::Matrix,
    child_reference: crate::procedural_aim::Matrix,
    child_size: f32,
    parent_size: f32,
) -> Result<crate::procedural_aim::Matrix, &'static str> {
    normal_matrix(
        parent.mul(parent_reference)?,
        child_size,
        parent_size,
        child_reference,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_creation_preserves_cold_cache_and_supplied_request_pose() {
        let mut anchor = crate::procedural_aim::Matrix::identity();
        let bits = [1147799945, 1156592765, 1104648903];
        for (i, b) in [3, 7, 11].into_iter().zip(bits) {
            anchor.values[i] = f32::from_bits(b);
        }
        assert_eq!(super::source_creation_position(0, 3, anchor), [0.; 3]);
        let cold = super::source_creation_matrix(0, 3, anchor);
        assert_eq!(cold.flags, 0);
        assert_eq!(
            cold.values.map(f32::to_bits),
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1065353216]
        );
        for generation in [1, 898898, u64::MAX] {
            let sampled = super::source_creation_matrix(generation, 3, anchor);
            assert_eq!(sampled.flags, anchor.flags);
            assert_eq!(
                sampled.values.map(f32::to_bits),
                anchor.values.map(f32::to_bits)
            );
            assert_eq!(
                super::source_creation_position(generation, 3, anchor).map(f32::to_bits),
                bits
            );
        }
        assert_eq!(
            super::source_creation_position(0, -1, crate::procedural_aim::Matrix::identity()),
            [0.; 3]
        );
        assert_eq!(
            super::source_creation_matrix(0, -1, anchor).values,
            anchor.values
        );
    }
    use super::*;
    fn program(scale: f32) -> giants_formats::effect_program::Program {
        let mut b = vec![0x24];
        for word in [123_u32, 900, 0] {
            b.extend(word.to_le_bytes());
        }
        b.extend([6, 0x1b]);
        b.extend(scale.to_le_bytes());
        b.extend([7, 2]);
        giants_formats::effect_program::parse(&b).unwrap()
    }
    #[test]
    fn bounded_recipe_rejects_unknown_lifecycle_and_invalid_parameters() {
        let p = program(0.75);
        let r = Recipe::project(&p).unwrap();
        assert_eq!((r.class, r.scale, r.opaque), (900, 0.75, 123));
        for scale in [f32::NAN, f32::INFINITY, 0., -1.] {
            assert!(Recipe::project(&program(scale)).is_err());
        }
        let mut unsupported = p.clone();
        unsupported.records[1].opcode = 4;
        assert!(Recipe::project(&unsupported).is_err());
        let mut truncated = p.clone();
        truncated.records[0].bytes.truncate(4);
        assert!(Recipe::project(&truncated).is_err());
        let mut reference = p;
        reference.records[0].bytes[9] = 1;
        assert!(Recipe::project(&reference).is_err());
    }
    #[test]
    fn unrelated_state_or_field_writer_is_not_admitted_as_a_glow() {
        let r = Recipe::project(&program(1.)).unwrap();
        let p = giants_formats::class_program::parse(&[0x16, 0x1e, 78, 255]).unwrap();
        assert_eq!(r.facing(&p).unwrap().personality, 4);
        for bytes in [
            &[0x16, 0x1e, 79, 255][..],
            &[0x16, 0x1e, 78, 0x18, 255],
            &[0x1e, 78, 255],
            &[0x16, 255],
        ] {
            assert!(
                r.facing(&giants_formats::class_program::parse(bytes).unwrap())
                    .is_err()
            );
        }
    }
}
