//! Per-object reference matrices over immutable class resources. Only the
//! ordinary single/mixed update is implemented; actor callbacks remain separate.
use crate::{animation_references::Resolution, procedural_aim::Matrix, specification::Prepared};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Update {
    pub updated: bool,
    pub evaluated: usize,
    pub cached_nodes: usize,
    pub missing_nodes: usize,
}
#[derive(Clone)]
pub struct Owned {
    specification: Arc<Prepared>,
    matrices: Vec<Matrix>,
    nodes: BTreeMap<u8, Vec<Option<usize>>>,
    sampled: bool,
    sample: Option<(u8, u32)>,
    current_sample: Option<crate::animation_channels::Channels>,
    mixed: BTreeMap<String, crate::animation_mixing::FlagMemo>,
    mixed_poses: BTreeMap<String, crate::animation_mixing::PoseMemo>,
}
impl std::fmt::Debug for Owned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorReferences")
            .field("class", &self.specification.class)
            .field("references", &self.matrices.len())
            .field("sampled", &self.sampled)
            .finish_non_exhaustive()
    }
}
impl Owned {
    pub fn new(specification: Arc<Prepared>) -> Result<Self, &'static str> {
        if !specification.needs.is_empty() {
            return Err("reference resources have unresolved preparation needs");
        }
        let count = specification.metadata.references.names.len();
        if count > 127 {
            return Err("reference count exceeds the acquired ordinary profile");
        }
        Ok(Self {
            specification,
            // Native allocation clears all 17 words, including undeclared slots.
            matrices: vec![
                Matrix {
                    flags: 0,
                    values: [0.; 16]
                };
                count
            ],
            nodes: BTreeMap::new(),
            sampled: false,
            sample: None,
            current_sample: None,
            mixed: BTreeMap::new(),
            mixed_poses: BTreeMap::new(),
        })
    }
    pub fn specification(&self) -> &Arc<Prepared> {
        &self.specification
    }
    pub fn matrices(&self) -> &[Matrix] {
        &self.matrices
    }
    pub fn sampled(&self) -> bool {
        self.sampled
    }
    /// The caller supplies an actually selected single clip and its owned clock.
    /// This does not implement native channel blending or advance the clock.
    /// Failed updates preserve the previous matrices and node cache atomically.
    pub fn update_unblended(&mut self, slot: u8, seconds: f32) -> Result<Update, String> {
        if slot == 254 {
            return Ok(Update::default());
        }
        let spec = &self.specification;
        let descriptor = spec
            .animations
            .get(&slot)
            .ok_or("reference clip descriptor absent")?;
        let tick = crate::model_pose::seconds_tick(descriptor.animation.timeline, seconds)?;
        let mut nodes = self
            .nodes
            .get(&slot)
            .cloned()
            .unwrap_or_else(|| vec![None; self.matrices.len()]);
        let mut matrices = self.matrices.clone();
        let mut update = Update {
            updated: true,
            ..Default::default()
        };
        for (index, name) in spec.metadata.references.names.iter().enumerate() {
            if name.is_none() {
                continue;
            }
            let resolution = spec
                .metadata
                .references
                .resolve_cached(&descriptor.animation, &mut nodes, index)
                .map_err(|e| e.to_string())?;
            matrices[index] = match resolution.node() {
                Some(node) => {
                    update.evaluated += 1;
                    update.cached_nodes += usize::from(matches!(resolution, Resolution::Cached(_)));
                    crate::animation_references::node_matrix(&descriptor.animation, node, tick)
                        .map_err(|e| e.to_string())?
                }
                None => {
                    update.missing_nodes += 1;
                    Matrix::identity()
                }
            };
        }
        for (index, matrix) in matrices.iter_mut().enumerate() {
            *matrix = crate::animation_links::apply(index as u8, *matrix, &spec.corrections)
                .map_err(|e| e.to_string())?;
        }
        self.matrices = matrices;
        self.nodes.insert(slot, nodes);
        self.sampled = true;
        self.sample = Some((slot, seconds.to_bits()));
        self.current_sample = None;
        Ok(update)
    }
    fn frames(
        &self,
        channels: &crate::animation_channels::Channels,
    ) -> Result<Vec<crate::animation_frames::Frame>, String> {
        let mut descriptors = vec![None; 256];
        for (&slot, d) in &self.specification.animations {
            descriptors[usize::from(slot)] = Some(crate::animation_frames::Descriptor {
                timeline: d.animation.timeline,
                flags: d.template.flags,
            });
        }
        crate::animation_frames::select(
            channels,
            &descriptors,
            crate::arithmetic::Precision::Double53,
        )
        .map_err(Into::into)
    }
    /// Follow the selected current/backup frames with native epoch caches.
    /// All writes are staged. Procedural callbacks remain explicit errors.
    pub fn update_current(
        &mut self,
        channels: &crate::animation_channels::Channels,
        epoch: u32,
    ) -> Result<Update, String> {
        if epoch == 0 {
            return Err("mixed reference epoch must be nonzero".into());
        }
        if channels[0][0] == 254 {
            return Ok(Update::default());
        }
        let frames = self.frames(channels)?;
        if frames.is_empty() {
            let mut matrices = self.matrices.clone();
            for (reference, matrix) in matrices.iter_mut().enumerate() {
                *matrix = crate::animation_links::apply(
                    reference as u8,
                    *matrix,
                    &self.specification.corrections,
                )
                .map_err(|e| e.to_string())?;
            }
            self.matrices = matrices;
            self.sample = None;
            self.current_sample = None;
            return Ok(Update::default());
        }
        let spec = &self.specification;
        let mut names = Vec::<String>::new();
        let mut sources = Vec::<&giants_formats::anm::Animation>::new();
        let mut memos = Vec::new();
        let mut poses = Vec::new();
        let mut indices = Vec::new();
        for frame in &frames {
            let d = spec
                .animations
                .get(&frame.descriptor)
                .ok_or("mixed reference descriptor absent")?;
            let name = d.name.to_ascii_lowercase();
            let index = if let Some(index) = names.iter().position(|n| n == &name) {
                if sources[index].bytes() != d.animation.bytes() {
                    return Err("mixed reference animation identity rebound".into());
                }
                index
            } else {
                let index = names.len();
                memos.push(self.mixed.get(&name).cloned().unwrap_or_default());
                poses.push(self.mixed_poses.get(&name).cloned().unwrap_or_default());
                sources.push(&d.animation);
                names.push(name);
                index
            };
            indices.push(index);
        }
        let mut nodes = self.nodes.clone();
        let mut matrices = self.matrices.clone();
        let mut update = Update {
            updated: true,
            ..Default::default()
        };
        for (reference, name) in spec.metadata.references.names.iter().enumerate() {
            if name.is_none() {
                continue;
            }
            let mut node_frames = Vec::new();
            for (i, frame) in frames.iter().enumerate() {
                let d = spec.animations.get(&frame.descriptor).unwrap();
                let cache = nodes
                    .entry(frame.descriptor)
                    .or_insert_with(|| vec![None; matrices.len()]);
                let resolution = spec
                    .metadata
                    .references
                    .resolve_cached(&d.animation, cache, reference)
                    .map_err(|e| e.to_string())?;
                update.cached_nodes += usize::from(matches!(resolution, Resolution::Cached(_)));
                node_frames.push(crate::animation_mixing::Frame {
                    animation: indices[i],
                    node: resolution.node(),
                    tick: frame.tick,
                    backup: frame.backup,
                    transition: frame.transition,
                });
            }
            matrices[reference] = match crate::animation_mixing::matrix_with_flags(
                &node_frames,
                &sources,
                epoch,
                &mut memos,
                &mut poses,
            )
            .map_err(|e| e.to_string())?
            {
                Some(matrix) => {
                    update.evaluated += 1;
                    matrix
                }
                None => {
                    update.missing_nodes += 1;
                    Matrix::identity()
                }
            };
        }
        for (reference, matrix) in matrices.iter_mut().enumerate() {
            *matrix = crate::animation_links::apply(reference as u8, *matrix, &spec.corrections)
                .map_err(|e| e.to_string())?;
        }
        self.matrices = matrices;
        self.nodes = nodes;
        for ((name, memo), pose) in names.into_iter().zip(memos).zip(poses) {
            self.mixed.insert(name.clone(), memo);
            self.mixed_poses.insert(name, pose);
        }
        self.sampled = true;
        self.sample = None;
        self.current_sample = Some(*channels);
        Ok(update)
    }
    fn single_channel(
        &self,
        channels: &crate::animation_channels::Channels,
    ) -> Result<(u8, f32), String> {
        if channels[0][0] == 254 {
            return Err("current reference primary channel absent".into());
        }
        let frames = self.frames(channels)?;
        let [frame] = frames.as_slice() else {
            return Err("current reference requires an acquired single frame".into());
        };
        if frame.backup || frame.transition != Some(0.) {
            return Err("current reference requires the unblended frame profile".into());
        }
        Ok((frame.descriptor, f32::from_bits(channels[frame.channel][1])))
    }
    /// Evaluate the actually selected ordinary channel; never choose an arbitrary
    /// clip when a mixed/procedural profile needs a different producer.
    pub fn update_current_unblended(
        &mut self,
        channels: &crate::animation_channels::Channels,
    ) -> Result<Update, String> {
        let (slot, seconds) = self.single_channel(channels)?;
        let update = self.update_unblended(slot, seconds)?;
        self.current_sample = Some(*channels);
        Ok(update)
    }
    /// The current consumer must not reuse an inspection sample from another
    /// clock or descriptor. Missing/mixed current frames are explicit errors.
    pub fn current_matrices(
        &self,
        channels: &crate::animation_channels::Channels,
    ) -> Result<&[Matrix], String> {
        if self.sampled && self.current_sample.as_ref() == Some(channels) {
            return Ok(&self.matrices);
        }
        let (slot, seconds) = self.single_channel(channels)?;
        if self.sample != Some((slot, seconds.to_bits())) {
            return Err("current reference matrices are stale or unprepared".into());
        }
        Ok(&self.matrices)
    }
    pub fn matrix(&self, reference: usize) -> Result<Matrix, &'static str> {
        if !self.sampled {
            return Err("actor reference matrices have not been updated");
        }
        if self
            .specification
            .metadata
            .references
            .names
            .get(reference)
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err("actor reference is outside or undeclared");
        }
        self.matrices
            .get(reference)
            .copied()
            .ok_or("actor reference outside cache")
    }
}
