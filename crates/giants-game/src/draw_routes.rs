//! Admission into adjacent ordinary/effect object-drawing traversals.
//! The caller supplies registered membership, visibility and camera exclusions.
//! These predicates alone do not enumerate a world or schedule a draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Routes {
    pub ordinary: bool,
    pub effect: bool,
}
pub fn admit(state: u32, active: u32, render_flags: u32) -> Routes {
    Routes {
        ordinary: active & 0x4000_c080 == 0,
        effect: state == 78 && active & 0x8000 == 0 && render_flags & 0x1000 != 0,
    }
}
impl Routes {
    pub fn mask(self) -> u32 {
        u32::from(self.ordinary) | (u32::from(self.effect) << 1)
    }
}
/// Compose the director's ordinary-object filter with the adjacent effect
/// traversal. The effect traversal does not inherit the ordinary range gate.
/// Registered membership, model/opacity and camera-owner gates remain caller
/// prerequisites; this does not enumerate or admit an entire native world.
pub fn with_ordinary_admission(
    input: &crate::cinematic_ordinary::Input,
    active: u32,
    render_flags: u32,
) -> Result<Routes, &'static str> {
    let mut routes = admit(input.state, active, render_flags);
    if routes.ordinary {
        routes.ordinary =
            crate::cinematic_ordinary::admit(input, crate::arithmetic::Precision::Double53)?;
    }
    Ok(routes)
}
/// Registration profile acquired for a vertical-fade free particle's class.
/// The effect recipe and material profile are admitted by the caller separately.
#[derive(Clone, Copy, Debug)]
pub struct VerticalParticle(());
impl VerticalParticle {
    pub fn project(program: &giants_formats::class_program::Program) -> Option<Self> {
        let ops = program.records.iter().map(|r| r.opcode).collect::<Vec<_>>();
        if ops != [1, 0x13, 0x1d, 0x1b, 0x36, 0x22, 0x1e, 0xff]
            || program.records[1].bytes.get(1) != Some(&1)
            || program.records[4].bytes.get(1) != Some(&6)
            || program.records[6].bytes.get(1) != Some(&78)
        {
            return None;
        }
        Some(Self(()))
    }
    pub fn routes(self, created: bool, first_callback_seen: bool, deleted: bool) -> Routes {
        if !created {
            return Routes {
                ordinary: false,
                effect: false,
            };
        }
        let active = if first_callback_seen {
            0x10070
        } else {
            0x40010030
        };
        admit(78, active | if deleted { 0x8000 } else { 0 }, 0x1000)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_range_does_not_suppress_the_adjacent_effect_route() {
        let mut input = crate::cinematic_ordinary::Input {
            class: 71,
            state: 78,
            flick_slot: -1,
            exclusion_words: [0; 3],
            position: [0.; 3],
            eye: [0.; 3],
            range: 1000.,
        };
        assert_eq!(
            with_ordinary_admission(&input, 0x40410030, 0x1000)
                .unwrap()
                .mask(),
            2
        );
        assert_eq!(
            with_ordinary_admission(&input, 0x410070, 0x1000)
                .unwrap()
                .mask(),
            3
        );
        input.eye[0] = 1000.;
        assert_eq!(
            with_ordinary_admission(&input, 0x410070, 0x1000)
                .unwrap()
                .mask(),
            2
        );
        input.eye[0] = 999.;
        assert_eq!(
            with_ordinary_admission(&input, 0x410070, 0x1000)
                .unwrap()
                .mask(),
            3
        );
        input.exclusion_words[0] = 1;
        input.flick_slot = 0;
        assert_eq!(
            with_ordinary_admission(&input, 0x410070, 0x1000)
                .unwrap()
                .mask(),
            2
        );
    }
    #[test]
    fn source_profile_does_not_select_by_class_or_specification_id() {
        let p = giants_formats::class_program::parse(&[
            1, 42, 0, 0, 0, 0x13, 1, 0x1d, 0x1b, 0x36, 6, 0x22, 0x1e, 78, 0xff,
        ])
        .unwrap();
        let profile = VerticalParticle::project(&p).unwrap();
        assert_eq!(profile.routes(false, false, false).mask(), 0);
        assert_eq!(profile.routes(true, false, false).mask(), 2);
        assert_eq!(profile.routes(true, true, false).mask(), 3);
        assert_eq!(profile.routes(true, true, true).mask(), 0);
        let mut other = p.clone();
        other.records[6].bytes[1] = 84;
        assert!(VerticalParticle::project(&other).is_none());
        let mut other = p.clone();
        other.records[1].bytes[1] = 2;
        assert!(VerticalParticle::project(&other).is_none());
    }
    #[test]
    fn construction_suppression_and_deletion_affect_routes_differently() {
        assert_eq!(admit(78, 0x4001_0030, 0x1000).mask(), 2);
        assert_eq!(admit(78, 0x0001_0070, 0x1000).mask(), 3);
        assert_eq!(admit(78, 0x0001_8070, 0x1000).mask(), 0);
        assert_eq!(admit(78, 0x0001_4070, 0x1000).mask(), 2);
    }
    #[test]
    fn state_and_render_flags_do_not_create_ordinary_membership() {
        assert_eq!(admit(0, 0, 0).mask(), 1);
        assert_eq!(admit(78, 0, 0).mask(), 1);
        assert_eq!(admit(18, 0, 0x1000).mask(), 1);
        assert_eq!(admit(78, 0x80, 0x1000).mask(), 2);
    }
}
