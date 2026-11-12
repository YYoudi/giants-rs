//! Debug-reference state23 damage callback. The caller owns typed actor
//! references and executes services in order; a computed reaction is not a
//! completed projectile hit, rendered death, or mission-progress update.
//! Prototype PS2 search key `ripper_shot` is an unknown attribution to PC
//! debug-1.1 0x004dafd0. See docs/SYMBOL_NAVIGATION.md; old aliases remain valid.
use crate::audio_parameters::Random;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target<I> {
    pub id: I,
    pub state: u32,
    pub flags: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct State<I> {
    pub id: I,
    pub class: i32,
    pub limit: f32,
    pub damage: f32,
    pub mode: u32,
    pub action: u32,
    pub flags: u32,
    pub personality: u32,
    pub control: u32,
    pub timer: f32,
    pub position: [f32; 3],
    pub parent: Option<I>,
    pub owner: Option<(I, i32)>,
    pub retaliation: Option<I>,
    pub target: Option<Target<I>>,
    pub group: Option<I>,
    pub group_index: i32,
    pub effect: Option<I>,
}
#[derive(Clone, Copy, Debug)]
pub struct Source<I> {
    pub id: I,
    pub state: u32,
    pub owner: Option<I>,
}
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub player_is_state1: bool,
    pub drop_percent: i32,
    pub death_timer: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Service<I> {
    StopAttached {
        actor: I,
        sound: u32,
    },
    Blood {
        actor: I,
    },
    PlayAttached {
        actor: I,
        sound: u32,
    },
    PlayPosition {
        position: [f32; 3],
        sound: u32,
    },
    Effect {
        name: &'static str,
        position: [f32; 3],
    },
    /// Construct class785, transition to state37, set position, flagsD8 bit30,
    /// control bit0, payload owner/1.0/30.0/tag36, then class-specific FX,
    /// configuration and control bit1/2. This needs a real constructor backend.
    Drop {
        actor: I,
        position: [f32; 3],
        large: bool,
        discarded_angle: f32,
    },
    DecrementTarget {
        target: I,
    },
    DestroyActor {
        actor: I,
    },
    ReorderGroup {
        group: I,
        index: i32,
        except: I,
    },
    GroupMemberDied {
        group: I,
        decrement: bool,
    },
    RemoveEffect {
        effect: I,
    },
    PropagateSource {
        actor: I,
        source: I,
        owner: I,
    },
}

/// Services may consume the same RNG (particles, effects, constructors, sound).
/// They must report failure when unavailable, never acknowledge a no-op as
/// rendering/creation. Earlier service effects are not rolled back on failure.
pub trait Services<I> {
    fn execute(
        &mut self,
        actor: &State<I>,
        service: Service<I>,
        random: &mut Random,
    ) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    AlreadyDead,
    Hurt,
    Died,
}
fn choice(random: &mut Random) -> bool {
    ((random.next_15() * 100) >> 15) >= 50
}

/// Callback only: global suppression, hierarchy redirection, source cleanup,
/// kill credit, mission accounting and notice dispatch belong to the wrapper.
/// Scalar inputs are bounded to finite values. The wide sum selects the death
/// branch before the stored f32 damage is rounded, as in the native callback.
pub fn apply<I: Copy + Eq>(
    actor: &mut State<I>,
    amount: f32,
    source: Option<Source<I>>,
    context: Context,
    random: &mut Random,
    services: &mut impl Services<I>,
) -> Result<Outcome, String> {
    if !matches!(actor.class, 73 | 230)
        || ![
            actor.limit,
            actor.damage,
            actor.timer,
            amount,
            context.death_timer,
        ]
        .into_iter()
        .chain(actor.position)
        .all(f32::is_finite)
        || actor.limit <= 0.
    {
        return Err("damage requires finite debug state23 class73/230 fields".into());
    }
    if actor.damage >= actor.limit {
        return Ok(Outcome::AlreadyDead);
    }
    let sum = f64::from(actor.damage) + f64::from(amount);
    if !(sum as f32).is_finite() {
        return Err("damage sum exceeds the finite callback profile".into());
    }
    // Validate the group needed by action100 before emitting any services.
    if actor.action == 100 && actor.group.is_none() && sum >= f64::from(actor.limit) {
        return Err("lethal action100 requires a typed group reference".into());
    }
    if actor.action == 1 {
        if let Some(source) = source {
            actor.retaliation = source.owner;
        }
        actor.action = 0;
        actor.mode = if actor.class == 230 { 64 } else { 33 };
    }
    for sound in [1256, 1258, 1235, 1271, 1272] {
        services.execute(
            actor,
            Service::StopAttached {
                actor: actor.id,
                sound,
            },
            random,
        )?;
    }
    actor.parent = None;
    if actor
        .target
        .is_some_and(|t| t.flags & 0x8000 != 0 || !matches!(t.state, 47 | 49))
    {
        actor.target = None;
    }
    actor.damage = sum as f32;
    if sum < f64::from(actor.limit) {
        services.execute(actor, Service::Blood { actor: actor.id }, random)?;
        if actor.action != 0 {
            if actor.mode == 103 {
                actor.mode = 108;
            }
            return Ok(Outcome::Hurt);
        }
        if let Some(Source {
            id,
            state: 77,
            owner: Some(owner),
        }) = source
        {
            services.execute(
                actor,
                Service::PropagateSource {
                    actor: actor.id,
                    source: id,
                    owner,
                },
                random,
            )?;
        }
        let sound = (if actor.class == 230 { 1244 } else { 1265 }) + u32::from(choice(random));
        services.execute(
            actor,
            Service::PlayAttached {
                actor: actor.id,
                sound,
            },
            random,
        )?;
        return Ok(Outcome::Hurt);
    }
    if !context.player_is_state1 && ((random.next_15() * 100) >> 15) as i32 <= context.drop_percent
    {
        let angle =
            ((f64::from(random.next_15()) * f64::from(f32::from_bits(0x40c8f5c3))) / 32768.) as f32;
        services.execute(
            actor,
            Service::Drop {
                actor: actor.id,
                position: actor.position,
                large: actor.class == 230,
                discarded_angle: angle,
            },
            random,
        )?;
    }
    if let Some(target) = actor.target {
        services.execute(
            actor,
            Service::DecrementTarget { target: target.id },
            random,
        )?;
    }
    services.execute(
        actor,
        Service::Effect {
            name: if actor.class == 230 {
                "fx_ripper_death_large"
            } else {
                "fx_ripper_death_medium"
            },
            position: actor.position,
        },
        random,
    )?;
    let sound = (if actor.class == 230 { 1236 } else { 1259 }) + u32::from(choice(random));
    services.execute(
        actor,
        Service::PlayPosition {
            position: actor.position,
            sound,
        },
        random,
    )?;
    let sound = (if actor.class == 230 { 1543 } else { 1547 }) + u32::from(choice(random));
    services.execute(
        actor,
        Service::PlayPosition {
            position: actor.position,
            sound,
        },
        random,
    )?;
    if let Some((owner, 219)) = actor.owner {
        services.execute(actor, Service::DestroyActor { actor: owner }, random)?;
    }
    actor.mode = 94;
    actor.flags |= 0x4000_0010;
    actor.personality |= 0x0e00;
    actor.timer = context.death_timer;
    if actor.action == 100 {
        let group = actor.group.expect("validated typed group");
        if actor.group_index != -1 {
            services.execute(
                actor,
                Service::ReorderGroup {
                    group,
                    index: actor.group_index,
                    except: actor.id,
                },
                random,
            )?;
            actor.group_index = -1;
        }
        services.execute(
            actor,
            Service::GroupMemberDied {
                group,
                decrement: actor.control & 4 == 0,
            },
            random,
        )?;
    }
    if let Some(effect) = actor.effect {
        services.execute(actor, Service::RemoveEffect { effect }, random)?;
        actor.effect = None;
    }
    Ok(Outcome::Died)
}
