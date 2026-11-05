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
    pub camera_origin: [f32; 3],
    pub counter: u32,
}
pub struct Output {
    pub shot: Option<crate::mecc_shot::Shot>,
    pub common_tail: bool,
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
    session.check_wield_owner(input.source)?;
    if session.mecc_shot_failure(input.source).is_some() {
        return Err("fire callback source quarantined".into());
    }
    check_local_target_profile(session, input.source)?;
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
        || input.local_target_scan
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
    put(&mut body, 0x124, 0);
    put(&mut body, 0x114, input.auxiliary_reset.to_bits());
    put(&mut body, 0x118, 0);
    session.commit_mecc_fire_frame(input.source, fields, body)?;
    let result: Result<Output, String> = (|| {
        let held = session.wield_list()?.into_iter().any(|child| {
            child.flags & 0x8000 == 0 && child.parent == Some(input.source) && child.reference == 1
        });
        if !held {
            return Ok(Output {
                shot: None,
                common_tail: false,
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
                    target: [0.; 3],
                    tracking_actor: None,
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
        })
    })();
    if let Err(error) = &result {
        session.fail_mecc_shot(input.source, error);
    }
    result
}
fn snapshot(session: &Session, id: Id) -> (Fields, Vec<u8>) {
    let actor = session.get(id).unwrap();
    (
        actor.construction_fields.clone().unwrap(),
        actor.seed.storage.data().unwrap().to_vec(),
    )
}
