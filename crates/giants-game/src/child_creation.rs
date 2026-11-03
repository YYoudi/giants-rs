//! Child inheritance and a bounded initial constructor profile. This does not run
//! the full state44 callback, native allocation layout or effect lifetime.
use crate::{instances::Plan, scene_resources::SceneResources};
use std::collections::BTreeMap;

pub const ALLOCATION_FLAGS: u32 = 0x80000000;

/// Opaque scalar properties retain their measured copy semantics. A parent ID
/// belongs to the independent runtime; it is never an original process pointer.
#[derive(Clone, Debug)]
pub struct Fields {
    pub position: [f32; 3],
    pub yaw: f32,
    pub owner_value: u32,
    pub active: u32,
    pub parent: u64,
    pub binding: u32,
    pub parameter: u32,
    pub optional_value: u32,
}
pub fn inherit(
    parent_id: u64,
    parent: &Fields,
    child: &mut Fields,
    binding: u32,
    parameter: u32,
    optional_value: u32,
) {
    child.position = parent.position;
    child.yaw = parent.yaw;
    child.owner_value = parent.owner_value;
    child.parent = parent_id;
    child.binding = binding;
    child.parameter = parameter;
    child.active |= 0x400000;
    if optional_value != 0 {
        child.optional_value = optional_value;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variant {
    pub flags: u32,
    pub extra_flags: u32,
    pub parameter: u32,
    pub selector: u32,
}
/// Class dispatch inside initial state44 preparation. Unmatched classes keep
/// their input values. Common trigonometry and subsequent AI are not executed.
pub fn state44_variant(class: u32, value: &mut Variant) {
    let (parameter, selector, flags) = match class {
        585 => (2, 14, 0x822),
        625 => (1, 8, 0x3008),
        831 | 940 => (12, 9, 0x1008),
        833 => (12, 10, 0x1008),
        832 => (12, 11, 0x1008),
        60 => {
            value.extra_flags |= 1;
            (2, 14, 0x800)
        }
        1062 | 1186 => (2, 30, 0x1008),
        1061 | 1185 => (2, 29, 0x40a),
        1187 => (2, 46, 0x1008),
        1188 => (2, 47, 0x40a),
        _ => return,
    };
    value.parameter = parameter;
    value.selector = selector;
    value.flags |= flags;
}

#[derive(Debug)]
pub struct Model {
    pub slot: usize,
    pub name: String,
    pub vertices: usize,
    pub parts: usize,
}
#[derive(Debug)]
pub struct Class {
    pub state: u8,
    pub models: Vec<Model>,
    pub constructor: Result<Template, String>,
}
#[derive(Debug)]
pub struct Template {
    pub flags: crate::object_lists::DirectFlags,
    pub scale: f32,
    pub animation: Option<u8>,
    pub seed: crate::state_storage::Seed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildId(pub u32);
#[derive(Debug)]
pub struct Instance {
    pub id: ChildId,
    pub parent: crate::static_geometry::SourceKey,
    pub class: i32,
    pub fields: Fields,
    pub scale: f32,
    pub personality: u32,
    pub animation: Option<u8>,
    pub seed: crate::state_storage::Seed,
    pub remaining: f32,
}
#[derive(Default, Debug)]
pub struct Runtime {
    pub instances: Vec<Instance>,
    pub pairs: Vec<[ChildId; 2]>,
    pub links: crate::object_links::Links,
    pub errors: Vec<String>,
}
/// State44's initial owner fallback, before child inheritance. This word remains
/// opaque here; no team, networking or ownership policy is inferred from its name.
pub fn initial_owner(value: u32) -> u32 {
    if value == 0 { 100 } else { value }
}

impl Runtime {
    /// Execute the measured countdown tail using an explicitly supplied global
    /// delta and ordered local areas. Firing/trigger callbacks are not run here.
    pub fn step_children(&mut self, global: f32, areas: &[crate::object_time::SlowArea]) -> usize {
        for pair in &self.pairs {
            let (left, right) = self.instances.split_at_mut((pair[1].0 - 1) as usize);
            let first = &mut left[(pair[0].0 - 1) as usize];
            let second = &mut right[0];
            let delta = crate::object_time::seconds(
                global,
                Some(crate::object_time::Sample {
                    owner: first.fields.owner_value,
                    position: first.fields.position,
                }),
                None,
                areas,
            );
            crate::object_time::step_pair(
                &mut first.remaining,
                &mut first.fields.active,
                &mut second.fields.active,
                delta,
            );
        }
        self.pairs.len()
    }
    /// Execute only the measured initial child profile: independent state seed,
    /// class fields, constructor memberships, inheritance and initial hidden bit.
    /// Native UID encoding, reference transforms and effect callbacks are absent.
    pub fn construct(initial: &InitialPlan, plan: &Plan) -> Self {
        let mut out = Self::default();
        for pair in &initial.pairs {
            let result = (|| -> Result<(), String> {
                let (source, resources) = if pair.parent.0 {
                    let p = plan
                        .world_entries
                        .iter()
                        .find(|p| p.declaration.source_offset == pair.parent.1)
                        .ok_or("world parent missing")?;
                    (p.class_input()?, p.resources())
                } else {
                    let p = plan
                        .entries
                        .iter()
                        .find(|p| p.serialized.source_offset == pair.parent.1)
                        .ok_or("mission parent missing")?;
                    (p.source.as_ref().map_err(Clone::clone)?, p.resources())
                };
                let definition = resources
                    .classes
                    .models
                    .first_program(source.class)
                    .ok_or("parent program missing")?;
                let program = definition.program.as_ref().map_err(Clone::clone)?;
                if program.records.iter().any(|r| {
                    !matches!(
                        r.opcode,
                        1 | 0x13 | 0x17 | 0x1e | 0x2a | 0x31 | 0x36 | 0x4f | 0x50 | 0xff
                    )
                }) {
                    return Err(
                        "parent owner/pose initialization outside measured source-command profile"
                            .into(),
                    );
                }
                let parent = Fields {
                    position: source.position,
                    yaw: source.yaw,
                    owner_value: initial_owner(0),
                    active: source.direct_flags.active,
                    parent: 0,
                    binding: 0,
                    parameter: 0,
                    optional_value: 0,
                };
                let parent_id = (u64::from(pair.parent.0) << 63)
                    | u64::try_from(pair.parent.1.checked_add(1).ok_or("parent key overflow")?)
                        .map_err(|e| e.to_string())?;
                let mut prepared = Vec::new();
                for class in pair.children {
                    let template = initial
                        .classes
                        .get(&class)
                        .ok_or("child class missing")?
                        .as_ref()
                        .map_err(Clone::clone)?
                        .constructor
                        .as_ref()
                        .map_err(Clone::clone)?;
                    prepared.push((class, template, template.seed.instantiate()?));
                }
                let start = out.instances.len();
                for (class, template, seed) in prepared {
                    let id =
                        ChildId(u32::try_from(out.instances.len() + 1).map_err(|e| e.to_string())?);
                    let mut fields = Fields {
                        position: [0.; 3],
                        yaw: 0.,
                        owner_value: 0,
                        active: template.flags.active,
                        parent: 0,
                        binding: 0,
                        parameter: 0,
                        optional_value: id.0,
                    };
                    out.links.construct(
                        u64::from(id.0),
                        class,
                        u32::from(seed.storage.state()),
                        template.flags,
                        pair.allocation_flags,
                    )?;
                    inherit(
                        parent_id,
                        &parent,
                        &mut fields,
                        pair.binding,
                        pair.variant.parameter,
                        0,
                    );
                    fields.active |= 0x40000000;
                    out.instances.push(Instance {
                        id,
                        parent: pair.parent,
                        class,
                        fields,
                        scale: template.scale,
                        personality: template.flags.personality,
                        animation: template.animation,
                        seed,
                        remaining: 0.,
                    });
                }
                out.pairs
                    .push([out.instances[start].id, out.instances[start + 1].id]);
                Ok(())
            })();
            if let Err(e) = result {
                out.errors.push(format!("{:?}: {e}", pair.parent));
            }
        }
        out
    }
}
#[derive(Debug)]
pub struct Pair {
    pub parent: crate::static_geometry::SourceKey,
    pub parent_class: i32,
    pub variant: Variant,
    pub children: [i32; 2],
    pub binding: u32,
    pub allocation_flags: u32,
}
#[derive(Default, Debug)]
pub struct InitialPlan {
    pub pairs: Vec<Pair>,
    pub classes: BTreeMap<i32, Result<Class, String>>,
    pub diagnostics: Vec<String>,
}
impl InitialPlan {
    /// Enabled initial-callback projection for the measured benchmark context.
    /// These requests await a complete allocator and runtime callback system;
    /// resolving their models does not make the effects visible or simulated.
    pub fn prepare_enabled(plan: &Plan, secondary_flags: u32) -> Self {
        let mut out = Self::default();
        for p in &plan.entries {
            if let Ok(source) = &p.source {
                out.add(
                    (false, p.serialized.source_offset),
                    source,
                    p.resources(),
                    secondary_flags,
                );
            }
        }
        for p in &plan.world_entries {
            if let Ok(source) = p.class_input() {
                out.add(
                    (true, p.declaration.source_offset),
                    source,
                    p.resources(),
                    secondary_flags,
                );
            }
        }
        out
    }
    fn add(
        &mut self,
        parent: crate::static_geometry::SourceKey,
        source: &crate::scene_resources::SourcePlacement,
        resources: &SceneResources,
        secondary_flags: u32,
    ) {
        let Ok(seed) = &source.state_seed else {
            return;
        };
        if seed.storage.state() != 44 {
            return;
        }
        let result = (|| {
            let data = seed.storage.data().ok_or("state44 storage missing")?;
            if data.len() != 76 {
                return Err("state44 storage size mismatch");
            }
            let word = |o| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
            if word(0x2c) & 1 != 0 {
                return Err("state44 initial guard already set; current child links unavailable");
            }
            let mut variant = Variant {
                flags: word(0x2c)
                    | 1
                    | if secondary_flags & 0x1000 != 0 {
                        0x200
                    } else {
                        0
                    },
                extra_flags: word(0x10),
                parameter: word(0x30),
                selector: word(0x34),
            };
            state44_variant(source.class as u32, &mut variant);
            Ok(Pair {
                parent,
                parent_class: source.class,
                variant,
                children: [623, 627],
                binding: u32::MAX,
                allocation_flags: ALLOCATION_FLAGS,
            })
        })();
        match result {
            Err(e) => self.diagnostics.push(format!("{parent:?}: {e}")),
            Ok(pair) => {
                for class in pair.children {
                    self.classes.entry(class).or_insert_with(|| {
                        let seed = resources
                            .class_state_seed(class)
                            .map_err(|e| e.to_string())?;
                        let binding = resources
                            .classes
                            .models
                            .first_selection(class)
                            .ok_or("child model declaration missing")?
                            .as_ref()
                            .map_err(Clone::clone)?;
                        if !binding.selection.diagnostics.is_empty() {
                            return Err("child model declaration reports diagnostics".into());
                        }
                        let mut models = Vec::new();
                        for (slot, declaration) in binding.selection.slots.iter().enumerate() {
                            if declaration.name.is_some() {
                                let (name, mesh) = resources
                                    .model_mesh_slot(class, slot)
                                    .map_err(|e| e.to_string())?
                                    .ok_or("child model unexpectedly empty")?;
                                models.push(Model {
                                    slot,
                                    name,
                                    vertices: mesh.positions.len(),
                                    parts: mesh.subobjects.len(),
                                });
                            }
                        }
                        let constructor=(|| -> Result<Template,String> {
                            let definition=resources.classes.models.first_program(class).ok_or("child class program missing")?;
                            let program=definition.program.as_ref().map_err(Clone::clone)?;
                            if seed.storage.state()!=56 || program.records.iter().any(|r|!matches!(r.opcode,1|0x13|0x16|0x17|0x1d|0x1e|0x22|0xff)) || !seed.diagnostics.is_empty() || !seed.unresolved.is_empty() {
                                return Err("child constructor outside measured stateless source-command profile".into());
                            }
                            Ok(Template {flags:crate::object_lists::DirectFlags::project(class,program),scale:program.declared_scale().map_err(|e|e.to_string())?,animation:crate::model_pose::declared_loop(program).map_err(String::from)?,seed:seed.instantiate()?})
                        })();
                        Ok(Class {
                            state: seed.storage.state(),
                            models,
                            constructor,
                        })
                    });
                }
                self.pairs.push(pair);
            }
        }
    }
}
