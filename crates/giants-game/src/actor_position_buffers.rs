//! Shared model-position buffers for ordinary single/multi-frame updates.
//! Native procedural callbacks remain explicit errors. The caller supplies
//! the native epoch and force-cache gate.
use crate::{animation_channels::Channels, animation_frames, specification::Prepared};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    KeepPrimaryGuard,
    KeepEpoch,
    KeepTime,
    KeepNoFrames,
    Deformed,
}
#[derive(Debug)]
struct Buffer {
    geometry: crate::catalog_registry::Location,
    skeleton: Option<crate::catalog_registry::Location>,
    positions: Vec<[f32; 3]>,
    cache: crate::vertex_cache::Cache,
}
#[derive(Debug)]
struct AnimationCache {
    source: Vec<u8>,
    matrices: BTreeMap<usize, (u32, [f32; 16])>,
}
struct AnimationUpdate<'a> {
    name: String,
    source: &'a [u8],
    matrices: crate::animation_mixing::Memo,
}
struct Deformation<'a> {
    positions: Vec<[f32; 3]>,
    updates: Vec<AnimationUpdate<'a>>,
}
#[derive(Default, Debug)]
pub struct Bank {
    models: BTreeMap<u32, Buffer>,
    animations: BTreeMap<String, AnimationCache>,
}
impl Bank {
    fn mixed<'a>(
        &self,
        spec: &'a Prepared,
        slot: usize,
        frames: &[animation_frames::Frame],
        epoch: u32,
    ) -> Result<Deformation<'a>, String> {
        let model = spec
            .models
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("mixed model absent")?;
        let skin = model
            .prepared
            .skin
            .as_ref()
            .ok_or("mixed model skin absent")?;
        let mut descriptors = Vec::new();
        let mut bindings = Vec::new();
        let mut names = Vec::new();
        let mut sources = Vec::new();
        let mut matrices = Vec::new();
        let mut indices = Vec::new();
        for frame in frames {
            let descriptor = spec
                .animations
                .get(&frame.descriptor)
                .ok_or("mixed descriptor absent")?;
            let binding = descriptor
                .template
                .bindings
                .get(slot)
                .and_then(Option::as_ref)
                .ok_or("mixed model binding absent")?;
            if binding.objects.len() != skin.objects.len() {
                return Err("mixed skin object binding changed".into());
            }
            let name = descriptor.name.to_ascii_lowercase();
            let index = if let Some(index) = names.iter().position(|n| n == &name) {
                index
            } else {
                let retained = self.animations.get(&name);
                if retained.is_some_and(|c| c.source != descriptor.animation.bytes()) {
                    return Err("mixed animation rebound to different bytes".into());
                }
                let index = names.len();
                matrices.push(retained.map_or_else(
                    || {
                        descriptor
                            .animation
                            .nodes
                            .iter()
                            .map(|node| (node.offset, (0, [0.; 16])))
                            .collect()
                    },
                    |c| c.matrices.clone(),
                ));
                names.push(name);
                sources.push(&descriptor.animation);
                index
            };
            descriptors.push(descriptor);
            bindings.push(binding);
            indices.push(index);
        }
        let anchor = bindings.first().ok_or("mixed frame list empty")?;
        let mut values = BTreeMap::new();
        // The native multi-frame deformation visits objects in source order,
        // but their clusters in reverse order. Shared node cache writes matter.
        for (oi, object) in skin.objects.iter().enumerate() {
            for ci in (0..object.clusters.len()).rev() {
                let mut node_frames = Vec::new();
                for (fi, frame) in frames.iter().enumerate() {
                    let object = bindings[fi]
                        .objects
                        .get(oi)
                        .ok_or("mixed object binding absent")?;
                    let cluster = object
                        .clusters
                        .get(ci)
                        .ok_or("mixed cluster binding absent")?;
                    node_frames.push(crate::animation_mixing::Frame {
                        animation: indices[fi],
                        node: cluster.node,
                        tick: frame.tick,
                        backup: frame.backup,
                        transition: frame.transition,
                    });
                }
                let value =
                    crate::animation_mixing::matrix(&node_frames, &sources, epoch, &mut matrices)
                        .map_err(|e| e.to_string())?
                        .ok_or("mixed bone has no current frame")?;
                let node = anchor.objects[oi].clusters[ci]
                    .node
                    .ok_or("mixed anchor bone absent")?;
                values.insert(node, value);
            }
        }
        let positions = crate::skinning::positions_bound_with_matrices(
            skin,
            &descriptors[0].animation,
            anchor,
            &values,
        )
        .map_err(|e| e.to_string())?;
        let updates = names
            .into_iter()
            .zip(sources)
            .zip(matrices)
            .map(|((name, source), matrices)| AnimationUpdate {
                name,
                source: source.bytes(),
                matrices,
            })
            .collect();
        Ok(Deformation { positions, updates })
    }
    fn reference(spec: &Prepared, slot: usize) -> Result<u32, String> {
        spec.metadata
            .selection
            .slots
            .get(slot)
            .and_then(|s| s.reference)
            .filter(|id| *id != 0)
            .ok_or_else(|| "position buffer requires a registered model reference".into())
    }
    pub fn len(&self) -> usize {
        self.models.len()
    }
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
    pub fn clear(&mut self) {
        self.models.clear();
        self.animations.clear();
    }
    pub fn positions(&self, spec: &Prepared, slot: usize) -> Result<Option<&[[f32; 3]]>, String> {
        let key = Self::reference(spec, slot)?;
        let Some(buffer) = self.models.get(&key) else {
            return Ok(None);
        };
        let model = spec
            .models
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("position model absent")?;
        if model.geometry_source != buffer.geometry || model.skeleton_source != buffer.skeleton {
            return Err("position model reference rebound to different resources".into());
        }
        Ok(Some(&buffer.positions))
    }
    pub fn cache(
        &self,
        spec: &Prepared,
        slot: usize,
    ) -> Result<Option<crate::vertex_cache::Cache>, String> {
        self.positions(spec, slot)?;
        Ok(self
            .models
            .get(&Self::reference(spec, slot)?)
            .map(|b| b.cache))
    }
    pub fn update(
        &mut self,
        spec: &Prepared,
        slot: usize,
        channels: &Channels,
        epoch: u32,
        force: bool,
    ) -> Result<Update, String> {
        if !spec.needs.is_empty() {
            return Err("position specification has unresolved needs".into());
        }
        let key = Self::reference(spec, slot)?;
        let model = spec
            .models
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or("position model absent")?;
        self.positions(spec, slot)?;
        let primary = spec
            .models
            .first()
            .and_then(Option::as_ref)
            .ok_or("primary position model absent")?;
        let mut cache = self.models.get(&key).map_or(
            crate::vertex_cache::Cache {
                epoch: 0,
                seconds: 0.,
                animation: 0,
            },
            |b| b.cache,
        );
        let mut replacement = None;
        let mut animation_updates = Vec::new();
        let update = if primary.prepared.mesh.indices.is_empty() || primary.prepared.skin.is_none()
        {
            Update::KeepPrimaryGuard
        } else {
            let mut descriptors = vec![None; 256];
            for (&id, descriptor) in &spec.animations {
                descriptors[usize::from(id)] = Some(animation_frames::Descriptor {
                    timeline: descriptor.animation.timeline,
                    flags: descriptor.template.flags,
                });
            }
            let frames = animation_frames::select(
                channels,
                &descriptors,
                crate::arithmetic::Precision::Double53,
            )?;
            let action = if !force && cache.epoch == epoch {
                crate::vertex_cache::Action::KeepEpoch
            } else if frames.len() > 1 {
                cache.epoch = if force { 0 } else { epoch };
                cache.seconds = -1.;
                crate::vertex_cache::Action::Deform
            } else if let Some(frame) = frames.first() {
                let descriptor = &spec.animations[&frame.descriptor];
                cache.single(
                    force,
                    epoch,
                    f32::from_bits(channels[0][1]),
                    channels[0][0],
                    descriptor.template.flags,
                )?
            } else {
                cache.epoch = if force { 0 } else { epoch };
                cache.seconds = -1.;
                crate::vertex_cache::Action::Deform
            };
            match action {
                crate::vertex_cache::Action::KeepEpoch => Update::KeepEpoch,
                crate::vertex_cache::Action::KeepTime => Update::KeepTime,
                crate::vertex_cache::Action::Deform => match frames.first() {
                    None => Update::KeepNoFrames,
                    Some(_) if frames.len() > 1 => {
                        let result = self.mixed(spec, slot, &frames, epoch)?;
                        if result.positions.len() != model.prepared.mesh.positions.len() {
                            return Err("mixed position capacity changed".into());
                        }
                        replacement = Some(result.positions);
                        animation_updates.extend(result.updates);
                        Update::Deformed
                    }
                    Some(frame) => {
                        let descriptor = &spec.animations[&frame.descriptor];
                        let skin = model
                            .prepared
                            .skin
                            .as_ref()
                            .ok_or("selected position model has no skin")?;
                        let binding = descriptor
                            .template
                            .bindings
                            .get(slot)
                            .and_then(Option::as_ref)
                            .ok_or("selected position binding absent")?;
                        let name = descriptor.name.to_ascii_lowercase();
                        let retained = self.animations.get(&name);
                        if retained.is_some_and(|c| c.source != descriptor.animation.bytes()) {
                            return Err("retained animation name rebound to different bytes".into());
                        }
                        // The acquired ANM constructor profile leaves node
                        // epochs and matrix caches zero. In particular epoch0
                        // can return a retained zero matrix, not a bind pose.
                        let mut matrices = retained.map_or_else(
                            || {
                                descriptor
                                    .animation
                                    .nodes
                                    .iter()
                                    .map(|node| (node.offset, (0, [0.; 16])))
                                    .collect()
                            },
                            |c| c.matrices.clone(),
                        );
                        let evaluator = crate::animation::Evaluator {
                            animation: &descriptor.animation,
                        };
                        for object in &binding.objects {
                            for cluster in &object.clusters {
                                let node = cluster.node.ok_or("retained skin bone unresolved")?;
                                evaluator
                                    .retained_matrix(node, frame.tick, epoch, &mut matrices)
                                    .map_err(|e| e.to_string())?;
                            }
                        }
                        let values = matrices
                            .iter()
                            .map(|(&node, &(_, matrix))| (node, matrix))
                            .collect();
                        let positions = crate::skinning::positions_bound_with_matrices(
                            skin,
                            &descriptor.animation,
                            binding,
                            &values,
                        )
                        .map_err(|e| e.to_string())?;
                        if positions.len() != model.prepared.mesh.positions.len() {
                            return Err("position buffer capacity changed".into());
                        }
                        replacement = Some(positions);
                        animation_updates.push(AnimationUpdate {
                            name,
                            source: descriptor.animation.bytes(),
                            matrices,
                        });
                        Update::Deformed
                    }
                },
            }
        };
        let buffer = self.models.entry(key).or_insert_with(|| Buffer {
            geometry: model.geometry_source.clone(),
            skeleton: model.skeleton_source.clone(),
            positions: model.prepared.mesh.positions.clone(),
            cache: crate::vertex_cache::Cache {
                epoch: 0,
                seconds: 0.,
                animation: 0,
            },
        });
        buffer.cache = cache;
        if let Some(positions) = replacement {
            buffer.positions = positions;
        }
        for AnimationUpdate {
            name,
            source,
            matrices,
        } in animation_updates
        {
            self.animations
                .entry(name)
                .and_modify(|c| c.matrices = matrices.clone())
                .or_insert_with(|| AnimationCache {
                    source: source.to_vec(),
                    matrices,
                });
        }
        Ok(update)
    }
}
