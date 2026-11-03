//! Typed FLK39 requests. This preserves requests, not effect execution/rendering.
use giants_formats::flick::{Command, DependencyMetadata};
use std::ops::Range;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub command_offset: usize,
    pub slot: i32,
    pub effect_offset: u32,
    pub effect: Option<String>,
    pub mode: u32,
    pub actor: i32,
    pub node_offset: u32,
    pub node: Option<String>,
}
impl Request {
    /// Slot bounds are a loader guard, not a claim of native bounds checking.
    pub fn decode(
        data: &[u8],
        command: &Command,
        pool: Range<usize>,
        actors: u32,
        slots: u32,
    ) -> Result<Self, String> {
        if command.opcode != 39
            || command.words.len() != 5
            || pool.end > data.len()
            || pool.start > pool.end
            || actors > 32
            || slots > 4096
        {
            return Err("unsupported cinematic effect request framing".into());
        }
        let w = &command.words;
        let slot = w[0] as i32;
        let actor = w[3] as i32;
        if (slot >= 0 && slot as u32 >= slots) || (actor >= 0 && actor as u32 >= actors) {
            return Err("cinematic effect slot outside declarations".into());
        }
        let label = |at: u32| -> Result<Option<String>, String> {
            if at == 0 {
                return Ok(None);
            }
            let at = at as usize;
            if !pool.contains(&at) {
                return Err("cinematic effect name outside string pool".into());
            }
            let bytes = &data[at..pool.end.min(at + 128)];
            let n = bytes
                .iter()
                .position(|b| *b == 0)
                .ok_or("unterminated cinematic effect name")?;
            if n == 0 || !bytes[..n].is_ascii() {
                return Err("invalid cinematic effect name".into());
            }
            Ok(Some(String::from_utf8(bytes[..n].to_vec()).unwrap()))
        };
        Ok(Self {
            command_offset: command.offset,
            slot,
            effect_offset: w[1],
            effect: label(w[1])?,
            mode: w[2],
            actor,
            node_offset: w[4],
            node: label(w[4])?,
        })
    }
    pub fn words(&self) -> [u32; 5] {
        [
            self.slot as u32,
            self.effect_offset,
            self.mode,
            self.actor as u32,
            self.node_offset,
        ]
    }
    /// Resolve only in the context's active image; never guess another image.
    pub fn target(
        &self,
        resources: &crate::scene_resources::SceneResources,
    ) -> Result<crate::model_registry::StateTarget, String> {
        if self.slot < 0 {
            return Err("negative cinematic effect slot is inactive".into());
        }
        let name = self
            .effect
            .as_deref()
            .ok_or("cinematic effect name is null")?;
        let active = resources
            .classes
            .models
            .effects
            .last()
            .ok_or("no active effect image")?;
        resources.effect_target(&active.path, name)
    }
}
pub fn declarations(data: &[u8], metadata: &DependencyMetadata) -> Result<Vec<Request>, String> {
    let counts = data
        .get(12..20)
        .ok_or("truncated cinematic effect counts")?;
    let actors = u32::from_le_bytes(counts[..4].try_into().unwrap());
    let slots = u32::from_le_bytes(counts[4..].try_into().unwrap());
    let end = metadata
        .commands
        .last()
        .filter(|c| c.opcode == 5)
        .ok_or("effect discovery lacks terminating command")?;
    let pool = end.offset + (end.words.len() + 2) * 4..metadata.animation_region_offset;
    metadata
        .commands
        .iter()
        .filter(|c| c.opcode == 39)
        .map(|c| Request::decode(data, c, pool.clone(), actors, slots))
        .collect()
}

/// Source-bound placement only. A resolved transform is not a live effect
/// allocation, queue entry, update callback or completed cinematic command.
pub struct ProxyPlacement {
    pub slot: usize,
    pub actor: Option<usize>,
    pub node: Option<crate::actor_links::Reference>,
    pub mode: u32,
    pub scale: f32,
    pub matrix: crate::procedural_aim::Matrix,
    pub position: [f32; 3],
}
/// Managed ownership of the measured mode1 / FX02-only queue profile. The
/// constructor's skip pass leaves the entry active; its first normal update
/// consumes the terminator and releases queue membership. No particles exist
/// in this profile. General effect flags, classes and callbacks remain separate.
pub struct TerminalProxy {
    pub placement: ProxyPlacement,
    program: giants_formats::effect_program::Program,
    active: bool,
    cursor: u32,
}
impl TerminalProxy {
    pub fn new(
        placement: ProxyPlacement,
        program: giants_formats::effect_program::Program,
    ) -> Result<Self, &'static str> {
        if placement.mode != 1 {
            return Err("terminal proxy queue requires measured cinematic mode1");
        }
        let initial = finish_terminator(&program, 1)?;
        Ok(Self {
            placement,
            program,
            active: true,
            cursor: initial.cursor_advance,
        })
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn cursor(&self) -> u32 {
        self.cursor
    }
    /// Call at the effect update stage, never during CPU mission preparation.
    pub fn update(&mut self) -> Result<(), &'static str> {
        if self.active {
            let step = finish_terminator(&self.program, 0)?;
            self.cursor += step.cursor_advance;
            self.active = step.result == 0;
        }
        Ok(())
    }
}
impl Request {
    pub fn placement(
        &self,
        actors: &crate::actor_links::Projection,
        seconds: &std::collections::BTreeMap<usize, f32>,
        locals: &std::collections::BTreeMap<usize, Option<crate::procedural_aim::Matrix>>,
    ) -> Result<Option<ProxyPlacement>, String> {
        if self.slot < 0 {
            return Ok(None);
        }
        let actor = usize::try_from(self.actor).ok();
        let (matrix, node) = if let Some(slot) = actor {
            actors.attachment_matrix(
                self.command_offset,
                slot,
                self.node.as_deref(),
                seconds,
                locals,
            )?
        } else {
            (crate::procedural_aim::Matrix::identity(), None)
        };
        Ok(Some(ProxyPlacement {
            slot: self.slot as usize,
            actor,
            node,
            mode: self.mode,
            scale: 1.,
            matrix,
            position: [matrix.values[3], matrix.values[7], matrix.values[11]],
        }))
    }
}
/// CPU preparation with explicit initial clocks and absent local transforms.
/// Future/director commands remain unexecuted. Any unsupported effect profile
/// is an error rather than a skipped request or a cinematic success.
pub fn prepare_terminal_proxies(
    resources: &crate::scene_resources::SceneResources,
    bytes: &[u8],
    director_seconds: f32,
) -> Result<Vec<TerminalProxy>, String> {
    if !director_seconds.is_finite() || director_seconds < 0. {
        return Err("invalid effect preparation clock".into());
    }
    let metadata =
        giants_formats::flick::discover_dependencies(bytes).map_err(|e| e.to_string())?;
    let requests = declarations(bytes, &metadata)?;
    if requests.is_empty() {
        return Ok(Vec::new());
    }
    let active = resources
        .classes
        .models
        .effects
        .last()
        .ok_or("no active effect image")?;
    let actors = crate::actor_links::Factory::new(resources, &active.image).project(bytes)?;
    let clocks = actors
        .actors
        .iter()
        .map(|(slot, actor)| {
            (
                *slot,
                if actor.director_slot.is_some() {
                    director_seconds
                } else {
                    0.
                },
            )
        })
        .collect();
    let locals = actors.actors.keys().map(|slot| (*slot, None)).collect();
    let mut proxies = Vec::new();
    for request in requests {
        if let Some(placement) = request.placement(&actors, &clocks, &locals)? {
            let target = request.target(resources)?;
            let program = resources.bound_effect_program(&target)?;
            proxies.push(TerminalProxy::new(placement, program).map_err(str::to_owned)?);
        }
    }
    Ok(proxies)
}
pub fn load_terminal_proxies(
    root: &std::path::Path,
    sequence: &str,
    resources: &crate::scene_resources::SceneResources,
    director_seconds: f32,
) -> Result<Vec<TerminalProxy>, String> {
    let path = crate::flick::sequence_path(root, sequence).map_err(|e| e.to_string())?;
    let bytes = crate::read_asset(path).map_err(|e| e.to_string())?;
    prepare_terminal_proxies(resources, &bytes, director_seconds)
}

// Opcode34 targets its explicit actor slot. A mutation outside the attachment's
// source-scoped ancestry cannot change that attachment; unknown opcodes still fail.
fn irrelevant_actor_mutation(
    opcode: u32,
    words: &[u32],
    dependencies: &std::collections::BTreeSet<usize>,
) -> bool {
    opcode == 34
        && words
            .first()
            .is_some_and(|slot| !dependencies.contains(&(*slot as usize)))
}

#[cfg(test)]
mod dispatch_guards {
    #[test]
    fn explicit_other_actor_aim_does_not_block_but_parent_aim_does() {
        let dependencies = [0, 3].into_iter().collect();
        assert!(super::irrelevant_actor_mutation(
            34,
            &[1, 6, 5],
            &dependencies
        ));
        assert!(!super::irrelevant_actor_mutation(
            34,
            &[3, 6, 5],
            &dependencies
        ));
        assert!(!super::irrelevant_actor_mutation(
            34,
            &[0, 6, 5],
            &dependencies
        ));
        assert!(!super::irrelevant_actor_mutation(34, &[], &dependencies));
        assert!(!super::irrelevant_actor_mutation(17, &[1], &dependencies));
    }
}

/// Bind source requests without constructing their live queue entries. This is
/// the measured mode1 / FX02 profile, not a general effect interpreter.
pub struct Bindings {
    requests: std::collections::BTreeMap<
        usize,
        (Request, Option<giants_formats::effect_program::Program>),
    >,
    actors: Option<crate::actor_links::Projection>,
}
impl Bindings {
    pub fn load(
        resources: &crate::scene_resources::SceneResources,
        bytes: &[u8],
    ) -> Result<Self, String> {
        let metadata =
            giants_formats::flick::discover_dependencies(bytes).map_err(|e| e.to_string())?;
        let declarations = declarations(bytes, &metadata)?;
        if declarations.is_empty() {
            return Ok(Self {
                requests: Default::default(),
                actors: None,
            });
        }
        let active = resources
            .classes
            .models
            .effects
            .last()
            .ok_or("no active effect image")?;
        let actors = crate::actor_links::Factory::new(resources, &active.image).project(bytes)?;
        let mut requests = std::collections::BTreeMap::new();
        for request in declarations {
            let mut dependencies = std::collections::BTreeSet::new();
            let mut slot = usize::try_from(request.actor).ok();
            while let Some(actor) = slot {
                if !dependencies.insert(actor) {
                    return Err("cyclic effect actor dependencies".into());
                }
                slot = actors
                    .links
                    .iter()
                    .rev()
                    .find(|l| l.offset < request.command_offset && l.child == actor)
                    .map(|l| l.parent);
            }
            if request.slot >= 0
                && metadata.commands.iter().any(|c| {
                    c.offset < request.command_offset
                        && !irrelevant_actor_mutation(c.opcode, &c.words, &dependencies)
                        && !matches!(
                            c.opcode,
                            1 | 2
                                | 3
                                | 4
                                | 6
                                | 7
                                | 9
                                | 11
                                | 12
                                | 16
                                | 25
                                | 31
                                | 37
                                | 39
                                | 40
                                | 41
                                | 42
                                | 44
                                | 45
                                | 46
                                | 52
                        )
                })
            {
                return Err(
                    "effect attachment prefix requires unimplemented actor mutations".into(),
                );
            }
            let program = if request.slot < 0 {
                None
            } else {
                let target = request.target(resources)?;
                let program = resources.bound_effect_program(&target)?;
                if request.mode != 1 || !terminator_only(&program) {
                    return Err("live cinematic effect outside measured mode1/FX02 profile".into());
                }
                Some(program)
            };
            requests.insert(request.command_offset, (request, program));
        }
        Ok(Self {
            requests,
            actors: Some(actors),
        })
    }
    pub fn len(&self) -> usize {
        self.requests.len()
    }
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
    /// Use the actor clocks at dispatch, before the next director clock advance.
    /// Local transforms are explicitly absent in this admitted source prefix.
    pub fn instantiate(
        &self,
        offset: usize,
        seconds: f32,
    ) -> Result<Option<TerminalProxy>, String> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("invalid dispatch clock".into());
        }
        let (request, program) = self
            .requests
            .get(&offset)
            .ok_or("effect dispatch has no bound source request")?;
        let Some(program) = program else {
            return Ok(None);
        };
        let actors = self
            .actors
            .as_ref()
            .ok_or("effect actor projection absent")?;
        let clocks = actors
            .actors
            .iter()
            .map(|(slot, actor)| {
                (
                    *slot,
                    if actor.director_slot.is_some() {
                        seconds
                    } else {
                        0.
                    },
                )
            })
            .collect();
        let locals = actors.actors.keys().map(|slot| (*slot, None)).collect();
        let placement = request
            .placement(actors, &clocks, &locals)?
            .ok_or("active effect placement absent")?;
        TerminalProxy::new(placement, program.clone())
            .map(Some)
            .map_err(str::to_owned)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn string_offsets_cannot_escape_pool_and_slot_guards_do_not_drop_requests() {
        let mut bytes = vec![0u8; 128];
        bytes[64..67].copy_from_slice(b"fx\0");
        let mut c = Command {
            offset: 24,
            opcode: 39,
            words: vec![0, 64, 7, u32::MAX, 0],
            label: None,
        };
        let a = Request::decode(&bytes, &c, 64..128, 4, 2).unwrap();
        assert_eq!(a.mode, 7);
        assert_eq!(a.actor, -1);
        assert_eq!(a.effect.as_deref(), Some("fx"));
        assert!(a.node.is_none());
        c.words[1] = 63;
        assert!(Request::decode(&bytes, &c, 64..128, 4, 2).is_err());
        c.words[1] = 64;
        c.words[0] = 2;
        assert!(Request::decode(&bytes, &c, 64..128, 4, 2).is_err());
        c.words[0] = u32::MAX;
        assert_eq!(Request::decode(&bytes, &c, 64..128, 4, 2).unwrap().slot, -1);
    }
}

/// Admission of the measured FX02-only program. This says nothing about the
/// surrounding proxy constructor, transforms, scheduling or other FX commands.
pub fn terminator_only(program: &giants_formats::effect_program::Program) -> bool {
    program.consumed == 1
        && program.records.len() == 1
        && program.records[0].opcode == 2
        && program.records[0].bytes == [2]
}

#[derive(Debug, Clone, Copy)]
pub struct TerminatorStep {
    pub result: u32,
    pub cursor_advance: u32,
}
/// Null-object FX02 path. Native skip input is zero versus nonzero; this is a
/// function result, never a mission/cinematic readiness acknowledgement.
pub fn finish_terminator(
    program: &giants_formats::effect_program::Program,
    skip: u32,
) -> Result<TerminatorStep, &'static str> {
    if !terminator_only(program) {
        return Err("effect program requires execution beyond FX02");
    }
    Ok(TerminatorStep {
        result: u32::from(skip == 0),
        cursor_advance: u32::from(skip == 0),
    })
}
