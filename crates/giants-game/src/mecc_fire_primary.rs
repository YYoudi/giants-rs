//! Normal offline base-pistol fire callback with actual owned services.
//! The enclosing primary must supply its completed physical/first-ANM frame
//! and the current view/cache/time phase. This does not admit mission input.
use crate::{
    actor_fields::Fields,
    audio_parameters::Random,
    resident_states::{Id, Session},
};

pub struct Frame {
    pub fields: Fields,
    pub payload: Vec<u8>,
}
pub struct Input {
    pub source: Id,
    pub frame: Frame,
    pub buttons: u32,
    pub seconds: f64,
    pub authoritative: bool,
    pub input_mode: i32,
    pub fire_disabled: bool,
    /// Explicit unacquired source-header branch; only zero is admitted here.
    pub source_word174: i32,
    /// The local target-scan producer is not substituted by a false result.
    pub local_target_scan: bool,
    pub auxiliary_reset: f32,
    /// Current projectile/view origin. Local mode requires the viewport origin,
    /// distinct from the camera eye used by the terrain-ray prepass.
    pub camera_origin: [f32; 3],
    pub counter: u32,
}
pub struct Output {
    pub shot: Option<crate::mecc_shot::Shot>,
    pub common_tail: bool,
    /// Only the local scan path owns this reticle decision.
    pub target_in_range: Option<bool>,
    pub local_scan: Option<crate::mecc_target_owned::Output>,
}
pub struct LocalServices<'a, 't, H> {
    pub resources: &'a crate::scene_resources::SceneResources,
    pub terrain: &'a crate::terrain_queries::StaticTerrainQuery<'t>,
    pub camera: crate::first_person::Camera,
    pub viewport_origin: [f32; 3],
    pub viewport_forward: [f32; 3],
    pub vertex_epoch: u32,
    pub force_vertices: bool,
    pub headers: &'a mut H,
}
/// Execute the actual owned scan between fire-clock publication and the firing
/// branch. Physical/first-ANM completion and these view/cache snapshots remain
/// the enclosing primary's responsibility; no rendered-frame sample is inferred.
pub fn advance_local<H: crate::mecc_target_owned::Headers>(
    session: &mut Session,
    factory: &crate::mecc_shot::Factory,
    input: Input,
    local: LocalServices<'_, '_, H>,
    random: &mut Random,
    services: &mut impl crate::mecc_shot::Services,
) -> Result<Output, String> {
    if !input.local_target_scan
        || input.camera_origin.map(f32::to_bits) != local.viewport_origin.map(f32::to_bits)
    {
        return Err("local fire requires its scan and matching current viewport origin".into());
    }
    let source = input.source;
    let mut scan = |session: &mut Session| {
        session.scan_mecc_local_target(
            local.resources,
            crate::mecc_target_owned::Input {
                source,
                camera: local.camera,
                viewport_origin: local.viewport_origin,
                viewport_forward: local.viewport_forward,
                definition: factory.definition(),
                vertex_epoch: local.vertex_epoch,
                force_vertices: local.force_vertices,
            },
            local.terrain,
            local.headers,
        )
    };
    advance_inner(session, factory, input, random, services, Some(&mut scan))
}
fn word(body: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(body[at..at + 4].try_into().unwrap())
}
fn put(body: &mut [u8], at: usize, value: u32) {
    body[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
pub(crate) fn check_local_target_profile(session: &Session, source: Id) -> Result<(), String> {
    if session
        .mecc_local_target(source)
        .is_some_and(|state| state.links.iter().any(Option::is_some))
    {
        // Normalized pointer bytes cannot disguise an actual owned target as
        // the already acquired no-target fire profile.
        return Err("fire callback requires its acquired local-target branch".into());
    }
    Ok(())
}
pub fn advance(
    session: &mut Session,
    factory: &crate::mecc_shot::Factory,
    input: Input,
    random: &mut Random,
    services: &mut impl crate::mecc_shot::Services,
) -> Result<Output, String> {
    advance_inner(session, factory, input, random, services, None)
}
type Scan<'a> = dyn FnMut(&mut Session) -> Result<crate::mecc_target_owned::Output, String> + 'a;
fn advance_inner(
    session: &mut Session,
    factory: &crate::mecc_shot::Factory,
    input: Input,
    random: &mut Random,
    services: &mut impl crate::mecc_shot::Services,
    mut scan: Option<&mut Scan<'_>>,
) -> Result<Output, String> {
    session.check_wield_owner(input.source)?;
    if session.mecc_shot_failure(input.source).is_some() {
        return Err("fire callback source quarantined".into());
    }
    if scan.is_none() {
        check_local_target_profile(session, input.source)?;
    }
    let mut fields = input.frame.fields;
    let mut body = input.frame.payload;
    let actor = session.get(input.source).unwrap();
    let original = actor.seed.storage.data().unwrap();
    if !input.authoritative
        || body.len() != 524
        || fields.class != 2
        || fields.action != 0
        || fields.control_flags & 0x01020000 != 0
        || input.buttons & 0x30 != 0
        || input.source_word174 != 0
        || input.local_target_scan != scan.is_some()
        || [0xe0, 0x11c, 0x19c, 0x1f8]
            .into_iter()
            .any(|at| word(&body, at) != 0)
        || body[0x14c..0x194] != original[0x14c..0x194]
        || !input.seconds.is_finite()
        || input.seconds < 0.
        || !input.auxiliary_reset.is_finite()
        || !input.camera_origin.into_iter().all(f32::is_finite)
        || [0x38, 0x3c]
            .into_iter()
            .any(|at| !f32::from_bits(word(&body, at)).is_finite())
        || crate::mecc_ammunition::selected(2, &body)? != 1
    {
        return Err("fire callback requires acquired normal offline pistol frame/providers".into());
    }
    for at in [0x38, 0x3c] {
        let value = f32::from_bits(word(&body, at));
        if value > 0. {
            let next = (f64::from(value) - input.seconds) as f32;
            if !next.is_finite() {
                return Err("fire clock overflow".into());
            }
            put(&mut body, at, next.to_bits());
        }
    }
    fields.response_flags &= !0x20;
    session.commit_mecc_fire_frame(input.source, fields, body)?;
    let result: Result<Output, String> = (|| {
        let target = if let Some(scan) = &mut scan {
            session.publish_mecc_target_range(input.source, false)?;
            Some(scan(session)?)
        } else {
            None
        };
        let mut reset_auxiliary = true;
        let target_in_range = if let Some(target) = &target {
            let active = target
                .target
                .is_some_and(|id| session.get(id).is_some_and(|a| a.seed.storage.state() != 0));
            let within = target.target.is_none()
                || within_range(
                    input.camera_origin,
                    target.point,
                    f32::from_bits(factory.definition().word(0x2c)?),
                )?;
            reset_auxiliary = within;
            let in_range = active && within;
            session.publish_mecc_target_range(input.source, in_range)?;
            Some(in_range)
        } else {
            None
        };
        let (fields, mut body) = snapshot(session, input.source);
        put(&mut body, 0x124, 0);
        if reset_auxiliary {
            put(&mut body, 0x114, input.auxiliary_reset.to_bits());
            put(&mut body, 0x118, 0);
        }
        session.commit_mecc_fire_frame(input.source, fields, body)?;
        let held = session.wield_list()?.into_iter().any(|child| {
            child.flags & 0x8000 == 0 && child.parent == Some(input.source) && child.reference == 1
        });
        if !held {
            return Ok(Output {
                shot: None,
                common_tail: false,
                target_in_range,
                local_scan: target,
            });
        }
        let (mut fields, mut body) = snapshot(session, input.source);
        let mut clock = f32::from_bits(word(&body, 0x38));
        let eligible = input.buttons & 1 != 0
            && input.buttons & 0x1000 == 0
            && input.input_mode == -1
            && !input.fire_disabled
            && fields.control_flags & 0x100000 == 0;
        let mut shot = None;
        if eligible {
            if clock <= 0. {
                fields.control_flags &= !0x100;
            }
            if session.mecc_ammunition(input.source, 1)?.amount <= 0 {
                return Err("normal base-pistol availability outside acquired inventory".into());
            }
            put(&mut body, 0x3c, 0.5_f32.to_bits());
            session.commit_mecc_fire_frame(input.source, fields, body)?;
            shot = factory.attempt(
                session,
                crate::mecc_shot::Input {
                    source: input.source,
                    cooldown: Some(clock),
                    target: target.as_ref().map_or([0.; 3], |v| v.point),
                    tracking_actor: target.as_ref().and_then(|v| v.target),
                    range: crate::mecc_aim::NORMAL_PISTOL_RANGE,
                    camera_origin: input.camera_origin,
                    counter: input.counter,
                },
                random,
                services,
            )?;
            if let Some(shot) = &shot {
                clock = shot.cooldown.ok_or("fire callback lost its actual clock")?;
                session.consume_mecc_base_pistol(input.source)?;
            }
        }
        let (mut fields, mut body) = snapshot(session, input.source);
        if !eligible {
            // The inactive-trigger cleanup resets these even when the earlier
            // out-of-range target path preserved them. A missing held object
            // returns before this cleanup, as in the acquired callback.
            put(&mut body, 0x114, input.auxiliary_reset.to_bits());
            put(&mut body, 0x118, 0);
        }
        let tail = crate::mecc_fire::finish_tail(
            clock,
            input.buttons,
            fields.response_flags,
            fields.control_flags,
        )?;
        fields.response_flags = tail.response_flags;
        fields.control_flags = tail.control_flags;
        put(&mut body, 0x38, tail.cooldown.to_bits());
        session.commit_mecc_fire_frame(input.source, fields, body)?;
        Ok(Output {
            shot,
            common_tail: true,
            target_in_range,
            local_scan: target,
        })
    })();
    if let Err(error) = &result {
        session.fail_mecc_shot(input.source, error);
    }
    result
}
fn within_range(origin: [f32; 3], point: [f32; 3], range: f32) -> Result<bool, String> {
    if !origin
        .into_iter()
        .chain(point)
        .chain([range])
        .all(f32::is_finite)
        || range <= 0.
    {
        return Err("target range requires finite current viewport inputs".into());
    }
    let [x, y, z] = std::array::from_fn::<_, 3, _>(|i| f64::from(point[i]) - f64::from(origin[i]));
    Ok((x * x + y * y) + z * z < f64::from(range) * f64::from(range))
}
fn snapshot(session: &Session, id: Id) -> (Fields, Vec<u8>) {
    let actor = session.get(id).unwrap();
    (
        actor.construction_fields.clone().unwrap(),
        actor.seed.storage.data().unwrap().to_vec(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_range_uses_viewport_distance_and_a_strict_boundary() {
        let origin = [10_000., -200., 30.];
        assert!(within_range(origin, [10_699., -200., 30.], 700.).unwrap());
        assert!(!within_range(origin, [10_700., -200., 30.], 700.).unwrap());
        assert!(!within_range(origin, [10_701., -200., 30.], 700.).unwrap());
        assert!(!within_range([0.; 3], [10_699., -200., 30.], 700.).unwrap());
        assert!(!within_range([f32::MAX; 3], [-f32::MAX; 3], f32::MAX).unwrap());
        assert!(within_range([0.; 3], [1e30, 1e30, 0.], 1e31).unwrap());
        assert!(within_range(origin, [f32::NAN; 3], 700.).is_err());
    }
}
