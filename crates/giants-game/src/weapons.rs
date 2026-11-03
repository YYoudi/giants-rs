//! Weapon declarations linked to each measured parent's resource context.
//! Projectile construction, trajectories, damage and firing callbacks are absent.
use crate::{child_creation::InitialPlan, instances::Plan, static_geometry::SourceKey};
use std::path::Path;

#[derive(Debug)]
pub struct Effect {
    pub name: String,
    pub commands: Result<usize, String>,
}
#[derive(Debug)]
pub struct Binding {
    pub parent: SourceKey,
    pub selector: i32,
    pub parameter: u32,
    pub class: i32,
    pub interval: f32,
    pub aim_response: f32,
    pub class_registered: bool,
    pub effects: Vec<Effect>,
}
#[derive(Debug)]
pub struct Registry {
    pub catalog: giants_formats::weapons::Catalog,
    pub bindings: Vec<Binding>,
    pub errors: Vec<String>,
}
impl Registry {
    pub fn load(root: &Path, initial: &InitialPlan, plan: &Plan) -> Result<Self, String> {
        let data = std::fs::read(root.join("Bin/weapondefs.bin")).map_err(|e| e.to_string())?;
        let catalog = giants_formats::weapons::parse(&data).map_err(|e| e.to_string())?;
        let mut out = Self {
            catalog,
            bindings: Vec::new(),
            errors: Vec::new(),
        };
        for pair in &initial.pairs {
            let result = (|| -> Result<Binding, String> {
                let selector = pair.variant.selector as i32;
                let projectile = out
                    .catalog
                    .projectiles
                    .get(&selector)
                    .ok_or("projectile declaration missing")?;
                let resources = if pair.parent.0 {
                    plan.world_entries
                        .iter()
                        .find(|p| p.declaration.source_offset == pair.parent.1)
                        .ok_or("world weapon parent missing")?
                        .resources()
                } else {
                    plan.entries
                        .iter()
                        .find(|p| p.serialized.source_offset == pair.parent.1)
                        .ok_or("mission weapon parent missing")?
                        .resources()
                };
                let class_registered = resources
                    .classes
                    .models
                    .first_program(projectile.class)
                    .is_some_and(|p| p.program.is_ok());
                let effects = projectile
                    .effects
                    .iter()
                    .flatten()
                    .map(|name| {
                        let commands = (|| -> Result<usize, String> {
                            let image = resources
                                .classes
                                .models
                                .effects
                                .last()
                                .ok_or("active effect image missing")?;
                            let target = resources.effect_target(&image.path, name)?;
                            let program = resources.bound_effect_program(&target)?;
                            Ok(program.records.len())
                        })();
                        Effect {
                            name: name.clone(),
                            commands,
                        }
                    })
                    .collect();
                Ok(Binding {
                    parent: pair.parent,
                    selector,
                    parameter: pair.variant.parameter,
                    class: projectile.class,
                    interval: projectile.interval,
                    aim_response: projectile.aim_response,
                    class_registered,
                    effects,
                })
            })();
            match result {
                Ok(binding) => out.bindings.push(binding),
                Err(e) => out.errors.push(format!("{:?}: {e}", pair.parent)),
            }
        }
        Ok(out)
    }
}
