//! Common source dependencies for unnamed cinematic actors. Decoded resources
//! and calculable matrices are not native creation, visibility or drawing proof.
use crate::{
    catalog_registry::Location,
    scene_resources::{ModelAsset, SceneResources},
};
use giants_formats::{anm::Animation, cast::AttachmentDeclaration};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

pub struct Source {
    pub class_catalog: PathBuf,
    pub spec_catalog: PathBuf,
    pub mesh: Location,
    pub animation: Option<Location>,
    pub textures: BTreeMap<String, Location>,
}
pub struct Asset {
    pub model: ModelAsset,
    pub animation: Option<Animation>,
    pub child_handle_policy: ChildHandlePolicy,
    pub scale: Result<f32, String>,
    pub source: Source,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildHandlePolicy {
    SelectedAnimation,
    /// Initial class profile without animation/state/child callbacks. Native
    /// actor index 254 suppresses named-child lookup and retains a null handle.
    AbsentInitialAnimation,
    Unresolved,
}
impl ChildHandlePolicy {
    pub fn initial(
        program: &giants_formats::class_program::Program,
        selected_animation: bool,
        any_declared_animation: bool,
    ) -> Self {
        if selected_animation {
            return Self::SelectedAnimation;
        }
        if !any_declared_animation
            && program
                .records
                .iter()
                .all(|r| matches!(r.opcode, 1 | 0x14 | 0x16 | 0x1d | 0x22 | 0x36 | 0x17 | 0xff))
        {
            Self::AbsentInitialAnimation
        } else {
            Self::Unresolved
        }
    }
    fn child_node(self, requested: Option<&str>) -> Result<Option<&str>, &'static str> {
        match (requested, self) {
            (None, _) | (_, Self::AbsentInitialAnimation) => Ok(None),
            (Some(name), Self::SelectedAnimation) => Ok(Some(name)),
            (Some(_), Self::Unresolved) => Err("attachment child animation state unresolved"),
        }
    }
}
#[derive(Clone)]
pub struct Request {
    pub offset: usize,
    pub slot: usize,
    pub class: i32,
    pub links: Vec<AttachmentDeclaration>,
    pub asset: Result<Arc<Asset>, String>,
    pub binding: Result<Placement, String>,
    offset_program: Result<Arc<giants_formats::flick::Program>, String>,
    constructed_child_policy: Option<ChildHandlePolicy>,
}
#[derive(Clone)]
pub enum Placement {
    Attached(Bound),
    /// No director, parent or handle in the checked
    /// initial-field profile. Own SKN/ANM coordinates remain in the geometry.
    Unparented,
}
#[derive(Clone)]
pub struct Bound {
    pub parent_tree: Option<String>,
    pub parent_scale: f32,
    pub link: AttachmentDeclaration,
}
impl Request {
    /// Resolve only the initially unknown null-handle case from this exact
    /// constructed declaration. This is not a later animation/state update.
    pub fn resolve_initial_child_handle(
        &mut self,
        actor: &crate::actor_context::Prepared,
    ) -> Result<bool, String> {
        if actor.actor_slot != self.slot
            || actor.class.declaration_class != self.class
            || actor.command_offset != self.offset
        {
            return Err("child handle context differs from source declaration".into());
        }
        let asset = self.asset.as_ref().map_err(Clone::clone)?;
        let resolved = asset.child_handle_policy == ChildHandlePolicy::Unresolved
            && self.links.len() == 1
            && self.links[0].child_node.is_some()
            && actor.inline.is_none()
            && actor.channels[0][0] == 254;
        self.constructed_child_policy =
            resolved.then_some(ChildHandlePolicy::AbsentInitialAnimation);
        Ok(resolved)
    }
    pub fn matrix(&self, director: &Animation, tick: i32) -> Result<([f32; 16], u32), String> {
        self.matrix_with_playback(director, tick, None)
    }
    /// Own-loop link reference, separate from the director's clock and parent.
    /// A supplied playback must be the already admitted instance of this actor.
    pub fn matrix_with_playback(
        &self,
        director: &Animation,
        tick: i32,
        playback: Option<&crate::effect_animation::Playback>,
    ) -> Result<([f32; 16], u32), String> {
        let asset = self.asset.as_ref().map_err(Clone::clone)?;
        let placement = self.binding.as_ref().map_err(Clone::clone)?;
        let needs_timeline = matches!(placement, Placement::Unparented)
            || self
                .offset_program
                .as_ref()
                .is_ok_and(|p| p.commands.iter().any(|c| c.opcode == 33));
        let timeline = if needs_timeline {
            let program = self.offset_program.as_ref().map_err(Clone::clone)?;
            let mut timeline = crate::flick::Timeline::with_start(
                program.as_ref().clone(),
                director.timeline[2],
                director.timeline[0],
            )
            .map_err(str::to_owned)?;
            timeline.step(tick).map_err(str::to_owned)?;
            Some(timeline)
        } else {
            None
        };
        let declared_scale = asset.scale.clone()?;
        let current_scale = timeline
            .as_ref()
            .and_then(|t| t.actor_scale(self.slot, declared_scale))
            .unwrap_or(declared_scale);
        let bound = match placement {
            Placement::Attached(bound) => bound,
            Placement::Unparented => {
                return timeline
                    .as_ref()
                    .ok_or("offset timeline absent")?
                    .actor_offset(self.slot)
                    .ok_or("offset actor slot missing")?
                    .matrix(current_scale)
                    .map_err(str::to_owned);
            }
        };
        let binding = crate::attachments::Binding {
            parent_tree: bound.parent_tree.as_deref(),
            parent_node: &bound.link.parent_node,
            child_node: self
                .constructed_child_policy
                .unwrap_or(asset.child_handle_policy)
                .child_node(bound.link.child_node.as_deref())?,
            scale: current_scale,
            parent_scale: timeline
                .as_ref()
                .and_then(|t| t.actor_scale(bound.link.parent_slot, bound.parent_scale))
                .unwrap_or(bound.parent_scale),
        };
        if let Some(playback) = playback {
            let child = binding
                .child_node
                .map(|name| playback.named_reference(name))
                .transpose()?;
            crate::attachments::binding_matrix_with_reference(binding, director, tick, child)
        } else {
            crate::attachments::binding_matrix(binding, director, asset.animation.as_ref(), tick)
        }
        .map_err(|e| e.to_string())
    }
}
pub struct Registered<'a> {
    resources: &'a SceneResources,
    assets: BTreeMap<i32, Result<Arc<Asset>, String>>,
}
fn scale(resources: &SceneResources, class: i32) -> Result<f32, String> {
    resources
        .classes
        .models
        .first_program(class)
        .ok_or("class program absent")?
        .program
        .as_ref()
        .map_err(Clone::clone)?
        .declared_scale()
        .map_err(|e| e.to_string())
}
fn inline_parent_profile(
    parent: &giants_formats::cast::Actor,
    child_slot: usize,
    metadata: &giants_formats::flick::DependencyMetadata,
    links: &[AttachmentDeclaration],
) -> Result<(), String> {
    if parent.name.is_some() || parent.director != 0 || parent.object_type >= 0 {
        return Err("unnamed parent outside inline placeholder profile".into());
    }
    if links.iter().any(|link| link.slot == parent.slot) {
        return Err("recursive inline parent binding unresolved".into());
    }
    if metadata.commands.iter().any(|command| {
        let slot = match command.opcode {
            25 | 33 => command.words.first(),
            32 => command.words.last(),
            _ => None,
        };
        slot.is_some_and(|slot| *slot as usize == parent.slot || *slot as usize == child_slot)
    }) {
        return Err("inline attachment local animation/offset/scale context unresolved".into());
    }
    Ok(())
}
fn unparented_initial(
    program: &giants_formats::class_program::Program,
    director: i32,
) -> Result<Placement, String> {
    if director >= 0 {
        return Err("unparented actor director context unresolved".into());
    }
    if program.records.iter().any(|r| {
        !matches!(
            r.opcode,
            1 | 0x13
                | 0x14
                | 0x16
                | 0x17
                | 0x1b
                | 0x1d
                | 0x22
                | 0x23
                | 0x2a
                | 0x36
                | 0x39
                | 0x52
                | 0x53
                | 0xff
        ) || r.opcode == 0x13 && r.bytes.get(1) != Some(&1)
    }) {
        return Err("unparented actor class outside checked initial-field profile".into());
    }
    Ok(Placement::Unparented)
}
fn load(resources: &SceneResources, class: i32) -> Result<Asset, Box<dyn std::error::Error>> {
    let selected = resources
        .classes
        .models
        .first_selection(class)
        .ok_or("attachment class absent")?
        .as_ref()
        .map_err(Clone::clone)?;
    let model = resources
        .model_slot_with_encoding(
            class,
            0,
            crate::scene_resources::TextureEncoding::Rgb565OrArgb4444,
        )?
        .ok_or("attachment slot zero has no geometry")?;
    let animations: Vec<_> = selected
        .selection
        .animations
        .iter()
        .filter(|a| a.slot == 1)
        .collect();
    if animations.len() > 1 {
        return Err("ambiguous attachment animation slot one".into());
    }
    let animation_name = animations.first().map(|a| a.name.as_str());
    let animation = animation_name
        .map(|name| resources.animation(name))
        .transpose()?;
    let program = resources
        .classes
        .models
        .first_program(class)
        .ok_or("attachment class program absent")?
        .program
        .as_ref()
        .map_err(Clone::clone)?;
    let child_handle_policy = ChildHandlePolicy::initial(
        program,
        animation.is_some(),
        !selected.selection.animations.is_empty(),
    );
    let mut textures = BTreeMap::new();
    for t in &model.textures {
        textures.insert(
            t.name.clone(),
            resources
                .archives
                .select_bin(&format!("{}.tga", t.name), &resources.language)?
                .clone(),
        );
    }
    let source = Source {
        class_catalog: selected.class_catalog.clone(),
        spec_catalog: selected.spec_catalog.clone(),
        mesh: resources
            .archives
            .select_bin(&format!("{}.gbs", model.name), &resources.language)?
            .clone(),
        animation: animation_name
            .map(|name| {
                resources
                    .archives
                    .select_bin(name, &resources.language)
                    .cloned()
            })
            .transpose()?,
        textures,
    };
    Ok(Asset {
        model,
        animation,
        child_handle_policy,
        scale: scale(resources, class),
        source,
    })
}
impl<'a> Registered<'a> {
    pub fn new(resources: &'a SceneResources) -> Self {
        Self {
            resources,
            assets: BTreeMap::new(),
        }
    }
    pub fn asset(&mut self, class: i32) -> Result<Arc<Asset>, String> {
        self.assets
            .entry(class)
            .or_insert_with(|| {
                load(self.resources, class)
                    .map(Arc::new)
                    .map_err(|e| e.to_string())
            })
            .clone()
    }
    pub fn sequence(&mut self, bytes: &[u8]) -> Result<Vec<Request>, giants_formats::Error> {
        let metadata = giants_formats::flick::discover_dependencies(bytes)?;
        let declarations = giants_formats::cast::attachment_declarations(bytes)?;
        let offset_program = giants_formats::flick::parse_preview(bytes)
            .map(Arc::new)
            .map_err(|e| e.to_string());
        let mut requests = Vec::new();
        for actor in metadata
            .actors
            .iter()
            .filter(|a| a.object_type >= 0 && a.name.is_none())
        {
            let asset = self.asset(actor.object_type);
            let links: Vec<_> = declarations
                .iter()
                .filter(|l| l.slot == actor.slot)
                .cloned()
                .collect();
            let binding = if links.is_empty() {
                self.resources
                    .classes
                    .models
                    .first_program(actor.object_type)
                    .ok_or_else(|| "class program absent".to_string())
                    .and_then(|p| p.program.as_ref().map_err(Clone::clone))
                    .and_then(|p| unparented_initial(p, actor.director))
            } else if links.len() != 1 {
                Err(format!(
                    "expected one node binding; found {} (placement/commands unresolved)",
                    links.len()
                ))
            } else {
                let link = &links[0];
                let parent = &metadata.actors[link.parent_slot];
                if let Some(tree) = parent.name.as_ref() {
                    scale(self.resources, parent.object_type).map(|parent_scale| {
                        Placement::Attached(Bound {
                            parent_tree: Some(tree.clone()),
                            parent_scale,
                            link: link.clone(),
                        })
                    })
                } else {
                    inline_parent_profile(parent, actor.slot, &metadata, &declarations).map(|()| {
                        // Native negative-class initialization fixes the placeholder
                        // scale to one, independently of the class-zero lookup.
                        Placement::Attached(Bound {
                            parent_tree: None,
                            parent_scale: 1.,
                            link: link.clone(),
                        })
                    })
                }
            };
            let offset = metadata
                .commands
                .iter()
                .find(|c| c.opcode == 2 && c.words.get(2) == Some(&(actor.slot as u32)))
                .ok_or(giants_formats::Error("unnamed actor origin absent"))?
                .offset;
            requests.push(Request {
                offset,
                slot: actor.slot,
                class: actor.object_type,
                links,
                asset,
                binding,
                offset_program: offset_program.clone(),
                constructed_child_policy: None,
            });
        }
        Ok(requests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_placeholder_does_not_accept_recursive_or_locally_modified_parents() {
        let mut parent = giants_formats::cast::Actor {
            slot: 1,
            director: 0,
            name: None,
            object_type: -1,
        };
        let mut metadata = giants_formats::flick::DependencyMetadata {
            actors: Vec::new(),
            commands: Vec::new(),
            next: None,
            opaque_blocks: 0,
            animation_region_offset: 40,
            runtime_profile_supported: false,
        };
        assert!(inline_parent_profile(&parent, 2, &metadata, &[]).is_ok());
        parent.object_type = 0;
        assert!(inline_parent_profile(&parent, 2, &metadata, &[]).is_err());
        parent.object_type = -1;
        parent.director = -1;
        assert!(inline_parent_profile(&parent, 2, &metadata, &[]).is_err());
        parent.director = 0;
        let recursive = AttachmentDeclaration {
            offset: 0,
            slot: 1,
            parent_slot: 0,
            parent_node: "anchor".into(),
            child_node: None,
        };
        assert!(inline_parent_profile(&parent, 2, &metadata, &[recursive]).is_err());
        for (opcode, words) in [
            (25, vec![1, 0]),
            (32, vec![0, 0, 0, 1]),
            (33, vec![1, 1_f32.to_bits()]),
            (25, vec![2, 0]),
            (32, vec![0, 0, 0, 2]),
            (33, vec![2, 1_f32.to_bits()]),
        ] {
            metadata.commands = vec![giants_formats::flick::Command {
                offset: 0,
                opcode,
                words,
                label: None,
            }];
            assert!(inline_parent_profile(&parent, 2, &metadata, &[]).is_err());
        }
    }
    #[test]
    fn unparented_profile_rejects_directors_and_unmeasured_state() {
        let initial = giants_formats::class_program::parse(&[
            1, 1, 0, 0, 0, 0x13, 1, 0x53, 0x52, 0x1d, 0x36, 7, 0xff,
        ])
        .unwrap();
        assert!(matches!(
            unparented_initial(&initial, -1),
            Ok(Placement::Unparented)
        ));
        assert!(unparented_initial(&initial, 0).is_err());
        let state = giants_formats::class_program::parse(&[1, 1, 0, 0, 0, 0x1e, 1, 0xff]).unwrap();
        assert!(unparented_initial(&state, -1).is_err());
        let other_animation = giants_formats::class_program::parse(&[0x13, 2, 0xff]).unwrap();
        assert!(unparented_initial(&other_animation, -1).is_err());
        for scale in [0., -1., f32::NAN, f32::INFINITY] {
            assert!(crate::attachments::unparented_matrix(scale).is_err());
        }
    }
    #[test]
    fn absent_initial_animation_does_not_hide_state_or_missing_named_nodes() {
        let simple = giants_formats::class_program::parse(&[1, 1, 0, 0, 0, 0x36, 8, 0xff]).unwrap();
        let absent = ChildHandlePolicy::initial(&simple, false, false);
        assert_eq!(absent, ChildHandlePolicy::AbsentInitialAnimation);
        assert_eq!(absent.child_node(Some("requested_handle")), Ok(None));
        assert_eq!(
            ChildHandlePolicy::initial(&simple, false, true),
            ChildHandlePolicy::Unresolved
        );
        let selected = ChildHandlePolicy::initial(&simple, true, true);
        assert_eq!(
            selected.child_node(Some("must_resolve")),
            Ok(Some("must_resolve"))
        );
        let state = giants_formats::class_program::parse(&[1, 1, 0, 0, 0, 0x13, 1, 0xff]).unwrap();
        assert!(
            ChildHandlePolicy::initial(&state, false, false)
                .child_node(Some("handle"))
                .is_err()
        );
        assert_eq!(ChildHandlePolicy::Unresolved.child_node(None), Ok(None));
    }
}
