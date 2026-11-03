//! Source-built initial target/reference records; no native inputs or pointers.
//! This is an initial idle profile, not enemy acquisition or full state44 execution.
use crate::{
    animation::Evaluator,
    child_creation::{InitialPlan, initial_owner},
    instances::Plan,
    procedural_aim::{Axes, Matrix},
    static_geometry::SourceKey,
    weapons,
};
use std::collections::BTreeMap;
#[derive(Debug)]
pub struct Instance {
    pub parent: SourceKey,
    pub spec: u32,
    pub selector: i32,
    pub response: f32,
    pub owner: u32,
    pub target: [f32; 4],
    pub world: Matrix,
    pub references: Vec<Matrix>,
    pub tick: i32,
    pub phase: f32,
    pub delta: f32,
}
#[derive(Default, Debug)]
pub struct Runtime {
    pub instances: Vec<Instance>,
    pub errors: Vec<String>,
}
impl Runtime {
    /// The independent caller owns phase/delta. No normal-game clock equivalence
    /// is inferred from the explicit initialization profile.
    pub fn initialize(
        initial: &InitialPlan,
        plan: &Plan,
        weapons: &weapons::Registry,
        phase: f32,
        delta: f32,
    ) -> Self {
        let mut out = Self::default();
        for pair in &initial.pairs {
            let result = (|| -> Result<Instance, String> {
                if pair.parent.0 {
                    return Err("world authored target pose not yet projected".into());
                }
                if pair.variant.flags & 0x400 == 0 || pair.variant.flags & 0x2000 != 0 {
                    return Err("state44 targeting variant outside idle profile".into());
                }
                let entry = plan
                    .entries
                    .iter()
                    .find(|p| p.serialized.source_offset == pair.parent.1)
                    .ok_or("target parent missing")?;
                let source = entry.source.as_ref().map_err(Clone::clone)?;
                let resources = entry.resources();
                let weapon = weapons
                    .bindings
                    .iter()
                    .find(|b| b.parent == pair.parent)
                    .ok_or("target weapon binding missing")?;
                let selection = resources
                    .classes
                    .models
                    .first_selection(source.class)
                    .ok_or("target model selection missing")?
                    .as_ref()
                    .map_err(Clone::clone)?;
                let definition = resources
                    .classes
                    .models
                    .first_program(source.class)
                    .ok_or("target class missing")?;
                let class_program = definition.program.as_ref().map_err(Clone::clone)?;
                if class_program.records.iter().any(|r| {
                    !matches!(
                        r.opcode,
                        1 | 0x13 | 0x17 | 0x1e | 0x2a | 0x31 | 0x36 | 0x4f | 0x50 | 0xff
                    )
                }) {
                    return Err(
                        "target initial owner/angle fields outside measured constructor profile"
                            .into(),
                    );
                }
                let slot = crate::model_pose::declared_loop(class_program)?
                    .ok_or("target loop slot missing")?;
                let animations: Vec<_> = selection
                    .selection
                    .animations
                    .iter()
                    .filter(|a| a.slot == slot)
                    .collect();
                if animations.len() != 1 {
                    return Err("target animation binding missing or ambiguous".into());
                }
                let animation = resources
                    .animation(&animations[0].name)
                    .map_err(|e| e.to_string())?;
                if animation.trees.len() != 1 || animation.trees[0].offset != animation.default_tree
                {
                    return Err("target requires single default animation tree".into());
                }
                let specs = resources
                    .classes
                    .models
                    .specs
                    .get(&selection.spec_id)
                    .ok_or("target spec missing")?;
                let candidates: Vec<_> = specs
                    .iter()
                    .filter(|s| s.catalog == selection.spec_catalog)
                    .collect();
                let program = candidates
                    .first()
                    .ok_or("bound target spec missing")?
                    .program
                    .as_ref()
                    .map_err(Clone::clone)?;
                let bytes: Vec<_> = program
                    .records
                    .iter()
                    .flat_map(|r| r.bytes.iter().copied())
                    .collect();
                for candidate in &candidates {
                    let other = candidate.program.as_ref().map_err(Clone::clone)?;
                    if other
                        .records
                        .iter()
                        .flat_map(|r| r.bytes.iter().copied())
                        .collect::<Vec<_>>()
                        != bytes
                    {
                        return Err("bound target spec definitions differ".into());
                    }
                }
                if program.records.iter().any(|r| r.opcode == 0x54) {
                    return Err("target reference post-correction unsupported".into());
                }
                let names = program.reference_names().map_err(|e| e.to_string())?;
                let definitions = crate::procedural_bindings::bindings(selection.spec_id, slot)?;
                let bindings: BTreeMap<usize, Axes> = definitions
                    .into_iter()
                    .map(|b| {
                        Ok((
                            animation
                                .default_node(b.node)
                                .map_err(|e| e.to_string())?
                                .offset,
                            b.axes,
                        ))
                    })
                    .collect::<Result<_, String>>()?;
                if bindings.is_empty() {
                    return Err("target profile has no supported procedural binding".into());
                }
                let owner = initial_owner(0);
                let local = crate::object_time::seconds(
                    delta,
                    Some(crate::object_time::Sample {
                        owner,
                        position: source.position,
                    }),
                    None,
                    &[],
                );
                let previous = crate::targeting::initial_direction([0., 0.])?;
                let target = crate::targeting::idle(
                    previous,
                    source.position,
                    phase,
                    local,
                    weapon.aim_response,
                )?;
                let world = Matrix {
                    flags: 17 | if source.scale != 1. { 2 } else { 0 },
                    values: source.unadjusted_matrix()?,
                };
                let (_, tick) = crate::model_pose::loop_warmup_time(animation.timeline, delta)?;
                let matrices = Evaluator {
                    animation: &animation,
                }
                .aimed_matrices(tick, world, Some((target, 0)), &bindings)
                .map_err(|e| e.to_string())?;
                let references = names
                    .iter()
                    .map(|name| {
                        let name = name
                            .as_ref()
                            .ok_or("target reference requires prior unbound slot")?;
                        let node = animation.default_node(name).map_err(|e| e.to_string())?;
                        matrices
                            .get(&node.offset)
                            .copied()
                            .ok_or_else(|| "target reference matrix missing".into())
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(Instance {
                    parent: pair.parent,
                    spec: selection.spec_id,
                    selector: weapon.selector,
                    response: weapon.aim_response,
                    owner,
                    target,
                    world,
                    references,
                    tick,
                    phase,
                    delta,
                })
            })();
            match result {
                Ok(instance) => out.instances.push(instance),
                Err(e) => out.errors.push(format!("{:?}: {e}", pair.parent)),
            }
        }
        out
    }
}
