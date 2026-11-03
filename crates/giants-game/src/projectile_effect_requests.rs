//! Effect selection at the measured basic-constructor/state77 boundaries.
//! This is not projectile simulation, effect activation or audio admission.
use giants_formats::weapons::Projectile;

pub const BASIC_CONSTRUCTOR_STATE: u8 = 77;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
    AttachedToProjectile,
    Position([f32; 3]),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Request<'a> {
    pub name: &'a str,
    pub route: Route,
}
fn selected(definition: &Projectile, slot: usize, route: Route) -> Option<Request<'_>> {
    definition.effects[slot]
        .as_deref()
        .filter(|name| !name.is_empty())
        .map(|name| Request { name, route })
}
pub fn constructor(definition: &Projectile) -> Option<Request<'_>> {
    selected(definition, 0, Route::AttachedToProjectile)
}
/// Preserve numeric message IDs until the caller's collision/expiry producer
/// is established. Flags are the mutable definition copy, not assumed immutable.
/// Only the first named-effect request is projected; damage/visual services and
/// object retirement are separate. The supported input domain is finite points.
pub fn message(
    definition: &Projectile,
    effective_flags: u32,
    code: u32,
    point: [f32; 3],
) -> Result<Option<Request<'_>>, &'static str> {
    if !point.into_iter().all(f32::is_finite) {
        return Err("nonfinite projectile effect point");
    }
    let slot = match code {
        1 if effective_flags & 0x40 == 0 => return Ok(None),
        1 | 2 => 1,
        3 => {
            if point[2] > 0. {
                1
            } else {
                2
            }
        }
        _ => return Err("projectile message outside checked effect-selection profile"),
    };
    Ok(selected(definition, slot, Route::Position(point)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn message_gate_and_zero_boundary_keep_source_slots_distinct() {
        let mut definition = Projectile {
            id: 3,
            class: 123,
            flags: 0,
            interval: 1.,
            aim_response: 0.,
            launch_speed: 1.,
            lifetime: 1.,
            mass: 1.,
            gravity_mass: 0.,
            range: 1.,
            direct_damage: 0.,
            effects: [
                Some("flight".into()),
                Some("contact".into()),
                Some("below".into()),
                None,
                None,
                None,
            ],
        };
        assert_eq!(
            constructor(&definition).unwrap().route,
            Route::AttachedToProjectile
        );
        assert!(message(&definition, 0, 1, [1.; 3]).unwrap().is_none());
        assert_eq!(
            message(&definition, 0x40, 1, [1.; 3])
                .unwrap()
                .unwrap()
                .name,
            "contact"
        );
        assert_eq!(
            message(&definition, 0, 2, [0., 0., -2.])
                .unwrap()
                .unwrap()
                .name,
            "contact"
        );
        for z in [-1., -0., 0.] {
            assert_eq!(
                message(&definition, 0, 3, [0., 0., z])
                    .unwrap()
                    .unwrap()
                    .name,
                "below"
            );
        }
        assert_eq!(
            message(&definition, 0, 3, [0., 0., 1.])
                .unwrap()
                .unwrap()
                .name,
            "contact"
        );
        definition.effects[1] = None;
        assert!(message(&definition, 0, 2, [0.; 3]).unwrap().is_none());
        assert!(message(&definition, 0, 4, [0.; 3]).is_err());
        assert!(message(&definition, 0, 3, [0., 0., f32::NAN]).is_err());
    }
}
