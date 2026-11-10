//! Acquired resource-driven action prefix. Following flagged/animation policy,
//! natural caller admission and whole-primary scheduling remain separate.
use crate::kabuto_action_select::Scratch;

#[derive(Clone, Copy)]
pub struct Input {
    pub earlier_behavior: u32,
    pub previous_definition: i32,
    pub charge: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    FlaggedPolicy,
    AnimationPolicy,
    CheckSelection,
}
#[derive(Clone, Copy)]
pub enum Scalar {
    Remaining,
    Maximum,
    Meter,
    RangeA,
    RangeB,
    RangeC,
    RangeD,
    RangeE,
    RangeF,
}
pub trait World<K: Copy> {
    fn flags(&self) -> Result<u32, String>;
    fn set_flags(&mut self, value: u32) -> Result<(), String>;
    fn scalar(&self, field: Scalar) -> Result<f32, String>;
    fn set_scalar(&mut self, field: Scalar, value: f32) -> Result<(), String>;
    fn definition_flags(&self, definition: i32) -> Result<u32, String>;
    fn definition_slot(&self, definition: u32) -> Result<u32, String>;
    fn interaction(&self) -> Result<Option<K>, String>;
    /// Required actual status call, with the captured interaction identity. Its
    /// mutations precede the source/related-position reads below.
    fn target_status(
        &mut self,
        target: Option<K>,
        kind: u32,
        scratch: Scratch,
    ) -> Result<bool, String>;
    fn position(&self) -> Result<[f32; 3], String>;
    /// The other relationship is distinct from the captured interaction actor.
    fn related_position(&self) -> Result<[f32; 3], String>;
    fn random_two(&mut self, scratch: Scratch) -> Result<u32, String>;
    /// Set the acquired class/header flag bit, not the retirement flag.
    fn mark_target(&mut self, target: K) -> Result<(), String>;
    fn set_positions(&mut self, first: [f32; 3], second: [f32; 3]) -> Result<(), String>;
    fn reset_counters(&mut self) -> Result<(), String>;
    fn set_target(&mut self, target: Option<K>) -> Result<(), String>;
}
fn scalar<K: Copy>(world: &impl World<K>, field: Scalar) -> Result<f64, String> {
    let value = world.scalar(field)?;
    if value.is_finite() {
        Ok(f64::from(value))
    } else {
        Err("Kabuto action scalar outside acquired finite profile".into())
    }
}
fn put<K: Copy>(world: &mut impl World<K>, field: Scalar, value: f64) -> Result<(), String> {
    let value = value as f32;
    if !value.is_finite() {
        return Err("Kabuto action scalar overflow".into());
    }
    world.set_scalar(field, value)
}
fn radius_squared<K: Copy>(world: &impl World<K>, field: Scalar) -> Result<f64, String> {
    let value = scalar(world, field)?;
    Ok(value * value)
}
fn position(value: [f32; 3]) -> Result<[f32; 3], String> {
    if value.iter().all(|v| v.is_finite()) {
        Ok(value)
    } else {
        Err("Kabuto action position outside acquired finite profile".into())
    }
}

/// Prefix stores and required service effects survive errors. The owner must
/// quarantine replay; no following policy, selection or primary is completed.
pub fn advance<K: Copy>(
    scratch: &mut Scratch,
    input: Input,
    world: &mut impl World<K>,
) -> Result<Next, String> {
    let flags = world.flags()?;
    if flags & 2 != 0 && matches!(input.earlier_behavior, 8 | 9) {
        world.set_flags(flags | 0x300)?;
        return Ok(Next::CheckSelection);
    }
    scratch.retained_time = flags & 8;
    if !((flags & 8 != 0 && flags & 0x10 == 0) || (flags & 2 != 0 && flags & 4 == 0))
        || input.charge <= 0.
        || input.charge.is_nan()
    {
        return Ok(Next::FlaggedPolicy);
    }
    world.set_scalar(Scalar::Remaining, 10.)?;
    if input.previous_definition >= 0
        && world.definition_flags(input.previous_definition)? & 0x14f0 != 0
    {
        return Ok(Next::AnimationPolicy);
    }
    if input.charge < 0.33 {
        return Ok(Next::AnimationPolicy);
    }
    if !input.charge.is_finite() {
        return Err("Kabuto action charge outside acquired finite profile".into());
    }
    let candidate = world.interaction()?;
    let (candidate, distance) = if world.target_status(candidate, 2, *scratch)? {
        let source = position(world.position()?)?;
        let related = position(world.related_position()?)?;
        let d: [f64; 3] = std::array::from_fn(|i| f64::from(source[i]) - f64::from(related[i]));
        (candidate, (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2])
    } else {
        (None, 0.)
    };
    let (definition, range, distance) = if input.charge < 0.66 {
        if scratch.retained_time != 0 {
            (0x38, Scalar::RangeA, distance)
        } else {
            (0x3a, Scalar::RangeC, distance)
        }
    } else if input.charge < 1. {
        if scratch.retained_time != 0 {
            (0x39, Scalar::RangeB, distance)
        } else {
            (0x3b, Scalar::RangeD, distance)
        }
    } else {
        let draw = world.random_two(*scratch)?;
        if draw > 1 {
            return Err("Kabuto bounded draw outside acquired range".into());
        }
        // The native full-resource path reloads its narrowed local, unlike the
        // two preceding branches, which retain the wide squared distance.
        let distance = f64::from(distance as f32);
        let (definition, range) = if world.flags()? & 8 != 0 {
            if candidate.is_some() && distance >= radius_squared(world, Scalar::RangeF)? {
                (0x2a, Scalar::RangeE)
            } else if draw == 0 {
                (0x2b, Scalar::RangeF)
            } else {
                (0x2a, Scalar::RangeE)
            }
        } else {
            (if draw == 0 { 0x28 } else { 0x29 }, Scalar::RangeE)
        };
        (definition, range, distance)
    };
    let selected = if distance < radius_squared(world, range)? {
        candidate
    } else {
        None
    };
    if input.charge >= 1. {
        if let Some(target) = selected {
            world.mark_target(target)?;
        }
        let meter = scalar(world, Scalar::Meter)?;
        put(world, Scalar::Meter, meter + 1.5)?;
    }
    scratch.definition = definition;
    scratch.behavior = 6;
    let source = position(world.position()?)?;
    world.set_positions(source, source)?;
    let maximum = scalar(world, Scalar::Maximum)?;
    put(world, Scalar::Maximum, maximum - f64::from(input.charge))?;
    let meter = scalar(world, Scalar::Meter)?;
    world.reset_counters()?;
    put(world, Scalar::Meter, meter + 1.5)?;
    world.set_target(selected)?;
    scratch.slot = world.definition_slot(definition)?;
    let flags = world.flags()?;
    world.set_flags(flags | 0x100000)?;
    Ok(Next::CheckSelection)
}
