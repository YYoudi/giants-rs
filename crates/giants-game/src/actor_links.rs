//! Source-ordered declaration/link/origin projection. Other commands remain recorded
//! as unexecuted. Clocks and local transforms are explicit caller inputs; this
//! is not a VM, live actor registration, visibility or playback timing.
use crate::{
    actor_context::{Class, Prepared},
    procedural_aim::Matrix,
    scene_resources::SceneResources,
};
use giants_formats::effect_image::Image;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct Factory<'a> {
    resources: &'a SceneResources,
    effects: &'a Image,
    classes: BTreeMap<i32, Arc<Class>>,
}
impl<'a> Factory<'a> {
    pub fn new(resources: &'a SceneResources, effects: &'a Image) -> Self {
        Self {
            resources,
            effects,
            classes: BTreeMap::new(),
        }
    }
    pub fn project(&mut self, bytes: &[u8]) -> Result<Projection, String> {
        let metadata =
            giants_formats::flick::discover_dependencies(bytes).map_err(|e| e.to_string())?;
        let mut result = Projection {
            actors: BTreeMap::new(),
            links: Vec::new(),
            origins: Vec::new(),
            unexecuted: Vec::new(),
        };
        let pool = metadata
            .commands
            .iter()
            .find(|c| c.opcode == 5)
            .map(|c| c.offset + 8)
            .ok_or("source projection terminator absent")?;
        for c in &metadata.commands {
            match c.opcode {
                1 => {} // Prepared::declaration checks the preceding binding.
                2 => {
                    let class = c.words[3] as i32;
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        self.classes.entry(class)
                    {
                        entry.insert(Arc::new(Class::load(self.resources, class, self.effects)?));
                    }
                    let actor =
                        Prepared::declaration(Arc::clone(&self.classes[&class]), bytes, c.offset)?;
                    if result.actors.insert(actor.actor_slot, actor).is_some() {
                        return Err("redeclared actor outside source projection profile".into());
                    }
                }
                3 => {
                    if c.words.len() != 4 {
                        return Err("invalid actor link payload".into());
                    }
                    let parent = c.words[0] as usize;
                    let child = c.words[2] as usize;
                    let p = result
                        .actors
                        .get(&parent)
                        .ok_or("link parent has not been constructed")?;
                    let ch = result
                        .actors
                        .get(&child)
                        .ok_or("link child has not been constructed")?;
                    let parent_reference = reference(
                        p,
                        label(
                            bytes,
                            c.words[1] as usize,
                            pool,
                            metadata.animation_region_offset,
                        )?,
                    )?;
                    let child_reference = reference(
                        ch,
                        label(
                            bytes,
                            c.words[3] as usize,
                            pool,
                            metadata.animation_region_offset,
                        )?,
                    )?;
                    result.links.push(Link {
                        offset: c.offset,
                        parent,
                        child,
                        parent_reference,
                        child_reference,
                    });
                }
                25 => {
                    if c.words.len() != 2 {
                        return Err("invalid actor origin payload".into());
                    }
                    if (c.words[0] as i32) < 0 {
                        continue;
                    }
                    let slot = c.words[0] as usize;
                    let actor = result.actors.get(&slot).ok_or("origin actor absent")?;
                    let reference =
                        if actor.class.specification.is_none() || actor.channels[0][0] == 254 {
                            None
                        } else {
                            reference(
                                actor,
                                label(
                                    bytes,
                                    c.words[1] as usize,
                                    pool,
                                    metadata.animation_region_offset,
                                )?,
                            )?
                        };
                    result.origins.push(Origin {
                        offset: c.offset,
                        slot,
                        reference,
                    });
                }
                _ => result.unexecuted.push((c.offset, c.opcode)),
            }
        }
        Ok(result)
    }
}
fn label(bytes: &[u8], pointer: usize, pool: usize, end: usize) -> Result<Option<&str>, String> {
    if pointer == 0 {
        return Ok(None);
    }
    if pointer < pool || pointer >= end || end > bytes.len() {
        return Err("link name outside source pool".into());
    }
    let bytes = &bytes[pointer..end.min(pointer + 128)];
    let stop = bytes
        .iter()
        .position(|b| *b == 0)
        .ok_or("unterminated link name")?;
    if stop == 0 || !bytes[..stop].is_ascii() {
        return Err("invalid link name".into());
    }
    Ok(Some(
        std::str::from_utf8(&bytes[..stop]).map_err(|e| e.to_string())?,
    ))
}
#[derive(Debug)]
pub struct Reference {
    pub animation_slot: u8,
    pub image: String,
    pub node: usize,
}
impl Reference {
    pub fn identity(&self) -> String {
        format!("{}:{}", self.image, self.node)
    }
}
fn reference(actor: &Prepared, name: Option<&str>) -> Result<Option<Reference>, String> {
    let Some(name) = name else { return Ok(None) };
    let slot = u8::try_from(actor.channels[0][0]).map_err(|e| e.to_string())?;
    if slot == 254 {
        return Ok(None);
    }
    if actor.inline.is_none() && actor.class.specification.is_none() {
        return Ok(None);
    }
    let (animation, _) = actor
        .descriptor(slot)
        .ok_or("selected actor animation descriptor absent")?;
    let node = animation
        .named_node(actor.label.as_deref(), name)
        .map_err(|e| e.to_string())?
        .offset;
    let image = if slot == 0 && actor.inline.is_some() {
        "inline".into()
    } else {
        actor
            .class
            .specification
            .as_ref()
            .ok_or("reference specification absent")?
            .animations[&slot]
            .name
            .clone()
    };
    Ok(Some(Reference {
        animation_slot: slot,
        image,
        node,
    }))
}
pub struct Link {
    pub offset: usize,
    pub parent: usize,
    pub child: usize,
    pub parent_reference: Option<Reference>,
    pub child_reference: Option<Reference>,
}
pub struct Projection {
    pub actors: BTreeMap<usize, Prepared>,
    pub links: Vec<Link>,
    pub origins: Vec<Origin>,
    pub unexecuted: Vec<(usize, u32)>,
}
pub struct Origin {
    pub offset: usize,
    pub slot: usize,
    pub reference: Option<Reference>,
}
impl Projection {
    /// Explicit-clock attachment at this command's link prefix. The optional
    /// reference is looked up in the actor's selected initial animation. This
    /// does not execute later animation-selection or local-transform commands.
    pub fn attachment_matrix(
        &self,
        offset: usize,
        slot: usize,
        name: Option<&str>,
        seconds: &BTreeMap<usize, f32>,
        locals: &BTreeMap<usize, Option<Matrix>>,
    ) -> Result<(Matrix, Option<Reference>), String> {
        let actor = self.actors.get(&slot).ok_or("attachment actor absent")?;
        if actor.command_offset >= offset {
            return Err("attachment actor has no prior declaration".into());
        }
        let reference = reference(actor, name)?;
        let mut matrix = self.matrix(offset, slot, seconds, locals)?;
        if let Some(reference) = &reference {
            matrix = matrix
                .mul(self.node(slot, reference, seconds)?)
                .map_err(str::to_owned)?;
        }
        Ok((matrix, reference))
    }
    /// Effect-slot support before class-specific FX construction. The original
    /// helper composes the actor's draw/link matrix and its named reference.
    /// This does not inherit a scale/angle for the camera-facing FX object.
    pub fn effect_anchor(
        &self,
        offset: usize,
        actor: i32,
        name: Option<&str>,
        seconds: &BTreeMap<usize, f32>,
        locals: &BTreeMap<usize, Option<Matrix>>,
    ) -> Result<Matrix, String> {
        if actor < 0 {
            return Ok(Matrix::identity());
        }
        let slot = actor as usize;
        let mut matrix = self.matrix(offset, slot, seconds, locals)?;
        let parent = self.actors.get(&slot).ok_or("effect carrier absent")?;
        if let Some(reference) = reference(parent, name)? {
            matrix = matrix
                .mul(self.node(slot, &reference, seconds)?)
                .map_err(str::to_owned)?;
        }
        Ok(matrix)
    }
    /// Director placement/bounds matrix after its optional origin reference.
    /// This is separate from the object's draw matrix: the native director
    /// consumes this result to derive position/orientation and bounds.
    pub fn placement_matrix(
        &self,
        offset: usize,
        slot: usize,
        seconds: &BTreeMap<usize, f32>,
        locals: &BTreeMap<usize, Option<Matrix>>,
    ) -> Result<Matrix, String> {
        let matrix = self.matrix(offset, slot, seconds, locals)?;
        if let Some(reference) = self
            .origins
            .iter()
            .rev()
            .find(|o| o.slot == slot && o.offset <= offset)
            .and_then(|o| o.reference.as_ref())
        {
            matrix
                .mul(self.node(slot, reference, seconds)?)
                .map_err(str::to_owned)
        } else {
            Ok(matrix)
        }
    }
    /// Evaluate the link prefix at this command offset. Every visited actor
    /// requires a supplied clock and optional local matrix (None explicitly
    /// means no local transform). Callers must account for unexecuted commands.
    pub fn matrix(
        &self,
        offset: usize,
        slot: usize,
        seconds: &BTreeMap<usize, f32>,
        locals: &BTreeMap<usize, Option<Matrix>>,
    ) -> Result<Matrix, String> {
        self.matrix_inner(offset, slot, seconds, locals, &mut BTreeSet::new())
    }
    fn matrix_inner(
        &self,
        offset: usize,
        slot: usize,
        seconds: &BTreeMap<usize, f32>,
        locals: &BTreeMap<usize, Option<Matrix>>,
        visiting: &mut BTreeSet<usize>,
    ) -> Result<Matrix, String> {
        if !visiting.insert(slot) || visiting.len() > 32 {
            return Err("cyclic/deep actor parent graph".into());
        }
        let actor = self.actors.get(&slot).ok_or("actor matrix slot absent")?;
        let clock = *seconds.get(&slot).ok_or("actor clock input absent")?;
        if !clock.is_finite() || clock < 0. {
            return Err("invalid actor clock input".into());
        }

        let link = self
            .links
            .iter()
            .rev()
            .find(|l| l.offset <= offset && l.child == slot);
        let mut result = if let Some(l) = link {
            let mut parent = self.matrix_inner(offset, l.parent, seconds, locals, visiting)?;
            if let Some(r) = &l.parent_reference {
                parent = parent
                    .mul(self.node(l.parent, r, seconds)?)
                    .map_err(str::to_owned)?;
            }
            parent
        } else {
            Matrix::identity()
        };
        if let Some(local) = locals
            .get(&slot)
            .ok_or("actor local-transform input absent")?
        {
            result = result.mul(*local).map_err(str::to_owned)?;
        }
        if actor.director_slot.is_none() {
            let scale = if let Some(l) = link {
                (f64::from(actor.class.scale) / f64::from(self.actors[&l.parent].class.scale))
                    as f32
            } else {
                actor.class.scale
            };
            if !scale.is_finite() || scale <= 0. {
                return Err("actor scale outside finite positive profile".into());
            }
            let mut matrix = Matrix::identity();
            matrix.flags = 2;
            for i in [0, 5, 10] {
                matrix.values[i] = scale;
            }
            result = result.mul(matrix).map_err(str::to_owned)?;
        }
        if let Some(r) = link.and_then(|l| l.child_reference.as_ref()) {
            result = result
                .mul(
                    self.node(slot, r, seconds)?
                        .inverse()
                        .map_err(str::to_owned)?,
                )
                .map_err(str::to_owned)?;
        }
        visiting.remove(&slot);
        Ok(result)
    }
    fn node(
        &self,
        slot: usize,
        reference: &Reference,
        seconds: &BTreeMap<usize, f32>,
    ) -> Result<Matrix, String> {
        let actor = &self.actors[&slot];
        let (animation, _) = actor
            .descriptor(reference.animation_slot)
            .ok_or("matrix animation absent")?;
        let seconds = *seconds.get(&slot).ok_or("actor clock input absent")?;
        let tick =
            crate::model_pose::seconds_tick(animation.timeline, seconds).map_err(str::to_owned)?;
        crate::animation_references::node_matrix(animation, reference.node, tick)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn role(slot: usize) -> Prepared {
        Prepared {
            class: Arc::new(Class {
                declaration_class: -1,
                specification: None,
                flags: crate::object_lists::DirectFlags::default(),
                state: 0,
                scale: 1.,
                declared_loop: None,
            }),
            command_offset: 0,
            actor_slot: slot,
            director_slot: None,
            label: None,
            inline: None,
            channels: [[0; 9]; 4],
            needs: Vec::new(),
        }
    }
    #[test]
    fn attachments_require_prior_declarations_and_explicit_clocks() {
        let mut actor = role(0);
        actor.command_offset = 40;
        let p = Projection {
            actors: BTreeMap::from([(0, actor)]),
            links: Vec::new(),
            origins: Vec::new(),
            unexecuted: Vec::new(),
        };
        let locals = BTreeMap::from([(0, None)]);
        assert!(
            p.attachment_matrix(39, 0, None, &BTreeMap::new(), &locals)
                .unwrap_err()
                .contains("prior declaration")
        );
        assert!(
            p.attachment_matrix(40, 0, None, &BTreeMap::new(), &locals)
                .unwrap_err()
                .contains("prior declaration")
        );
        assert!(
            p.attachment_matrix(41, 0, None, &BTreeMap::new(), &locals)
                .unwrap_err()
                .contains("clock input absent")
        );
        assert!(
            p.attachment_matrix(41, 0, None, &BTreeMap::from([(0, 0.)]), &locals)
                .is_ok()
        );
    }
    #[test]
    fn cycles_and_unknown_local_inputs_fail_before_returning_a_matrix() {
        let mut p = Projection {
            actors: BTreeMap::from([(0, role(0)), (1, role(1))]),
            links: Vec::new(),
            origins: Vec::new(),
            unexecuted: Vec::new(),
        };
        let clocks = BTreeMap::from([(0, 0.), (1, 0.)]);
        assert!(
            p.matrix(0, 0, &clocks, &BTreeMap::new())
                .unwrap_err()
                .contains("local-transform input absent")
        );
        p.links = vec![
            Link {
                offset: 0,
                parent: 1,
                child: 0,
                parent_reference: None,
                child_reference: None,
            },
            Link {
                offset: 1,
                parent: 0,
                child: 1,
                parent_reference: None,
                child_reference: None,
            },
        ];
        let locals = BTreeMap::from([(0, None), (1, None)]);
        assert!(
            p.matrix(1, 0, &clocks, &locals)
                .unwrap_err()
                .contains("cyclic/deep")
        );
    }
    #[test]
    fn a_link_name_must_be_in_the_pool_and_terminated() {
        let bytes = b"command\0ref_center\0";
        assert!(label(bytes, 0, 8, bytes.len()).unwrap().is_none());
        assert!(label(bytes, 1, 8, bytes.len()).is_err());
        assert!(label(bytes, 8, 8, bytes.len() - 1).is_err());
        assert_eq!(label(bytes, 8, 8, bytes.len()).unwrap(), Some("ref_center"));
    }
    #[test]
    fn placement_origin_is_offset_scoped_and_never_changes_the_draw_matrix() {
        let mut p = Projection {
            actors: BTreeMap::from([(0, role(0))]),
            links: Vec::new(),
            origins: vec![Origin {
                offset: 10,
                slot: 0,
                reference: Some(Reference {
                    animation_slot: 0,
                    image: "missing".into(),
                    node: 0,
                }),
            }],
            unexecuted: Vec::new(),
        };
        let clocks = BTreeMap::from([(0, 0.)]);
        let locals = BTreeMap::from([(0, None)]);
        let draw = p.matrix(20, 0, &clocks, &locals).unwrap();
        assert_eq!(draw.values, Matrix::identity().values);
        assert!(p.placement_matrix(9, 0, &clocks, &locals).is_ok());
        assert!(p.placement_matrix(10, 0, &clocks, &locals).is_err());
        p.origins.push(Origin {
            offset: 20,
            slot: 0,
            reference: None,
        });
        assert_eq!(
            p.placement_matrix(20, 0, &clocks, &locals).unwrap().values,
            draw.values
        );
        assert!(p.placement_matrix(19, 0, &clocks, &locals).is_err());
    }
}
