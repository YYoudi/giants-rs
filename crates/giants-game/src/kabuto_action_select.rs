//! Locomotion/idle selection before the held-reference and shared animation
//! continuations. Earlier admission/scratch and all world services are required;
//! a selected request is not yet an animation commit or completed primary.

#[derive(Clone, Copy, Debug)]
pub struct Scratch {
    pub packet_word: u32,
    pub slot: u32,
    pub retained_time: u32,
    pub flags: u32,
    pub clear_flags: u32,
    pub behavior: u32,
    pub definition: u32,
    pub blend_seconds: f32,
    pub reference_speed: f32,
    pub speed_source: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub earlier_behavior: u32,
    pub activity: u32,
    pub wrapped: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    HeldReferences,
    CheckSelection,
    ChooseSelection,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Continuation {
    HeldReferences,
    SkipCommit,
    Commit,
}
/// Resolve the shared gate against the earlier captured primary slot. Skipped
/// selections preserve every scratch word, including arbitrary speed bits.
/// This does not execute the held-reference path or the animation commit.
pub fn resolve(scratch: &mut Scratch, next: Next, earlier_slot: u32) -> Continuation {
    match next {
        Next::HeldReferences => return Continuation::HeldReferences,
        Next::CheckSelection if (scratch.slot as i32) < 0 => return Continuation::SkipCommit,
        _ => {}
    }
    if scratch.slot == earlier_slot {
        return Continuation::SkipCommit;
    }
    scratch.reference_speed = match scratch.speed_source as u32 {
        0 => 200.,
        1 => scratch.reference_speed,
        2 => -40.,
        3 => -90.,
        _ => 0.,
    };
    Continuation::Commit
}
#[derive(Clone, Copy, Debug)]
pub enum Scalar {
    LongitudinalRate,
    LateralRate,
    CurrentSize,
    TargetSize,
    SlowThreshold,
    FastThreshold,
    SpeedA,
    SpeedB,
    SpeedC,
}
#[derive(Clone, Copy, Debug)]
pub enum Sound {
    Start,
    Stop,
}
#[derive(Clone, Copy, Debug)]
pub enum PacketWord {
    RetainedTime,
    Temporary,
}
pub trait World<K: Copy> {
    fn scalar(&self, field: Scalar) -> Result<f32, String>;
    fn buttons(&self) -> Result<u32, String>;
    fn reference(&self) -> Result<Option<K>, String>;
    /// Presence only; no identity/layout or producer of this other relationship
    /// is inferred by this component.
    fn other_reference_present(&self) -> Result<bool, String>;
    fn held(&self) -> Result<[Option<K>; 2], String>;
    fn class(&self, actor: K) -> Result<u32, String>;
    fn definition_slot(&self, key: u32) -> Result<u32, String>;
    /// Actual PC bounded draw, including calls whose return is discarded. The
    /// caller retains RNG/TLS ownership; this is not an audio-DLL stream merge.
    fn random(&mut self, upper: u32, scratch: Scratch) -> Result<u32, String>;
    fn sound(&mut self, kind: Sound, id: u32, scratch: Scratch) -> Result<(), String>;
    fn notifying(&self) -> Result<bool, String>;
    /// This word can alias the retained caller clock or its separate packet
    /// temporary. Completed packet writes and world effects survive errors.
    fn notify(
        &mut self,
        kind: Sound,
        scratch: Scratch,
        alias: PacketWord,
        packet: &mut u32,
    ) -> Result<(), String>;
}

/// Actual world dependencies for the Session adapter. Missing relationships,
/// external definition data, PC RNG ownership or sound/transport cannot default
/// to successful empty providers. The packet argument is canonical in flight;
/// the Group retains the last published caller-scratch snapshot.
pub trait Services {
    fn other_reference_present(
        &self,
        session: &crate::resident_states::Session,
        owner: crate::resident_states::Id,
    ) -> Result<bool, String>;
    fn definition_slot(
        &self,
        session: &crate::resident_states::Session,
        owner: crate::resident_states::Id,
        key: u32,
    ) -> Result<u32, String>;
    fn random(
        &mut self,
        session: &crate::resident_states::Session,
        owner: crate::resident_states::Id,
        upper: u32,
    ) -> Result<u32, String>;
    fn sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        kind: Sound,
        id: u32,
    ) -> Result<(), String>;
    fn network<'a>(
        &'a self,
        session: &crate::resident_states::Session,
        owner: crate::resident_states::Id,
    ) -> Result<crate::kabuto_geometry_mode::NotificationContext<'a>, String>;
    fn notify(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        kind: Sound,
        alias: PacketWord,
        packet: &mut u32,
    ) -> Result<(), String>;
}
fn value<K: Copy>(w: &impl World<K>, field: Scalar) -> Result<f32, String> {
    let value = w.scalar(field)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Kabuto selection scalar outside finite profile".into())
    }
}
fn sound<K: Copy>(
    s: &mut Scratch,
    w: &mut impl World<K>,
    kind: Sound,
    id: u32,
    clock_alias: bool,
) -> Result<(), String> {
    w.sound(kind, id, *s)?;
    if w.notifying()? {
        if clock_alias {
            s.retained_time = id;
            let snapshot = *s;
            w.notify(
                kind,
                snapshot,
                PacketWord::RetainedTime,
                &mut s.retained_time,
            )?;
        } else {
            s.packet_word = id;
            let snapshot = *s;
            w.notify(kind, snapshot, PacketWord::Temporary, &mut s.packet_word)?;
        }
    }
    Ok(())
}
fn definition<K: Copy>(s: &mut Scratch, w: &impl World<K>, key: u32) -> Result<(), String> {
    s.definition = key;
    s.slot = w.definition_slot(key)?;
    Ok(())
}
fn voice<K: Copy>(s: &mut Scratch, w: &mut impl World<K>, clock_alias: bool) -> Result<(), String> {
    let id = if w.random(100, *s)? < 50 {
        0x467
    } else {
        0x468
    };
    sound(s, w, Sound::Start, id, clock_alias)
}
/// Prefix stores and service effects remain on failure. The caller must retain
/// the directed continuation and establish its actual primary admission/order.
pub fn advance<K: Copy>(
    s: &mut Scratch,
    input: Input,
    w: &mut impl World<K>,
) -> Result<Next, String> {
    let old = input.earlier_behavior;
    if !matches!(old, 6 | 7) {
        if input.activity & 2 != 0 {
            s.behavior = 4;
            s.speed_source = 2;
            if w.reference()?.is_none() && w.other_reference_present()? {
                if value(w, Scalar::LateralRate)? > 0. {
                    s.slot = 126;
                    s.flags = 0x31;
                } else {
                    s.slot = 127;
                    s.flags = 0x21;
                }
            } else if value(w, Scalar::LateralRate)? > 0. {
                s.slot = 115;
                s.flags = 0x31;
            } else {
                s.slot = 116;
                s.flags = 0x21;
            }
            return Ok(Next::ChooseSelection);
        }
        if input.activity & 1 != 0 {
            s.behavior = 3;
            s.speed_source = 1;
            s.reference_speed = value(w, Scalar::SpeedA)?;
            if !input.wrapped && old == 3 {
                return Ok(Next::CheckSelection);
            }
            if w.reference()?.is_some() {
                if old != 3 {
                    s.flags = 0xa0;
                    s.clear_flags = 0x20;
                    s.retained_time = 0.25f32.to_bits();
                }
                if value(w, Scalar::LongitudinalRate)?.abs() >= value(w, Scalar::FastThreshold)? {
                    s.slot = 203;
                    s.reference_speed = value(w, Scalar::SpeedC)?;
                    s.flags |= 1;
                    s.blend_seconds = 0.125;
                    return Ok(Next::ChooseSelection);
                }
                definition(s, w, 0x35)?;
                s.flags |= 1;
                s.blend_seconds = 0.125;
                return Ok(Next::CheckSelection);
            }
            if w.other_reference_present()? {
                s.clear_flags = 0x20;
                s.retained_time = 0.25f32.to_bits();
                s.slot = 124;
                s.flags = 0xa1;
                return Ok(Next::ChooseSelection);
            }
            let _ = w.random(100, *s)?;
            if old != 3 {
                s.flags = 0xa0;
                s.clear_flags = 0x20;
                s.retained_time = 0.25f32.to_bits();
            }
            let speed = value(w, Scalar::LongitudinalRate)?.abs();
            if speed >= value(w, Scalar::FastThreshold)? {
                s.reference_speed = value(w, Scalar::SpeedC)?;
                sound(s, w, Sound::Start, 0x6dd, false)?;
                sound(s, w, Sound::Stop, 0x6dc, false)?;
                s.slot = 165;
                s.flags |= 1;
                s.blend_seconds = 0.125;
            } else if speed >= value(w, Scalar::SlowThreshold)? {
                s.reference_speed = value(w, Scalar::SpeedB)?;
                s.slot = 164;
                s.flags |= 1;
                s.blend_seconds = 0.125;
            } else {
                sound(s, w, Sound::Start, 0x6dc, false)?;
                sound(s, w, Sound::Stop, 0x6dd, false)?;
                s.slot = 2;
                s.flags |= 1;
            }
            return Ok(Next::ChooseSelection);
        }
    }
    if old != 3 {
        sound(s, w, Sound::Stop, 0x6dd, true)?;
        sound(s, w, Sound::Stop, 0x6dc, true)?;
    }
    if w.buttons()? & 0x80 != 0 && value(w, Scalar::CurrentSize)? >= 3. {
        if let Some(actor) = w.reference()? {
            if w.class(actor)? != 677
                && let Some(index) = w.held()?.iter().position(Option::is_none)
            {
                s.behavior = 11;
                definition(s, w, 0x2f + index as u32)?;
                return Ok(Next::CheckSelection);
            }
        } else if let Some(index) = w.held()?.iter().position(Option::is_some) {
            s.behavior = 10;
            definition(s, w, 0x31 + index as u32)?;
            return Ok(Next::CheckSelection);
        }
    }
    if value(w, Scalar::CurrentSize)? != value(w, Scalar::TargetSize)? && old != 16 {
        s.behavior = 16;
        match w.random(3, *s)? {
            0 => definition(s, w, 0x36)?,
            1 | 2 => s.slot = 186,
            _ => {}
        }
        voice(s, w, true)?;
    }
    // Reread after the preceding service: a callback can change these buttons.
    let buttons = w.buttons()?;
    if buttons & 0x100 != 0 {
        voice(s, w, false)?;
        definition(s, w, 0x36)?;
        return Ok(Next::CheckSelection);
    }
    if buttons & 0x200 != 0 {
        voice(s, w, false)?;
        s.slot = 186;
        return Ok(Next::ChooseSelection);
    }
    if buttons & 0x400 != 0 {
        voice(s, w, false)?;
        s.slot = 187;
        return Ok(Next::ChooseSelection);
    }
    Ok(Next::HeldReferences)
}
