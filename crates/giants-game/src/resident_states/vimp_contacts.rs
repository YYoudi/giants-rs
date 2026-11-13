use super::*;

impl Session {
    pub fn vimp_wound_pose(&self, id: Id) -> Result<crate::contact_pose::Cached, String> {
        self.completed_vimp_frame(id)?;
        let fields = self
            .get(id)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("wound owner fields absent")?;
        let affine = self.actor_current_affine(id)?;
        crate::contact_pose::Cached::new(fields.object_flags, affine.flags, affine.matrix)
    }
    /// Current single-frame joint dictionary used by a Vimp wound attachment.
    /// Blended/procedural providers must be acquired before extending this
    /// explicitly unblended normal-preview profile.
    pub fn vimp_hit_point_joints(
        &self,
        id: Id,
    ) -> Result<Option<Vec<crate::skinned_hit_point::Joint<String>>>, String> {
        self.completed_vimp_frame(id)?;
        let actor = self.get(id).ok_or("wound actor absent")?;
        if actor.class != 8 || actor.seed.storage.state() != 21 {
            return Err("wound joints require the actual Vimp owner".into());
        }
        let channels = actor.channels.ok_or("wound actor channels absent")?;
        let specification = self.specification(id).ok_or("wound specification absent")?;
        let mut descriptors = vec![None; 256];
        for (&key, descriptor) in &specification.animations {
            descriptors[usize::from(key)] = Some(crate::animation_frames::Descriptor {
                timeline: descriptor.animation.timeline,
                flags: descriptor.template.flags,
            });
        }
        let frames = crate::animation_frames::select(
            &channels,
            &descriptors,
            crate::arithmetic::Precision::Double53,
        )?;
        let Some(frame) = frames.first() else {
            return Ok(None);
        };
        if frames.len() != 1 || !frame.uses_model_binding || frame.backup {
            return Err(
                "Vimp wound requires its acquired single current model-binding frame".into(),
            );
        }
        let descriptor = &specification.animations[&frame.descriptor];
        let binding = descriptor
            .template
            .bindings
            .first()
            .and_then(Option::as_ref)
            .ok_or("wound primary model binding absent")?;
        let tree = descriptor
            .animation
            .trees
            .iter()
            .find(|tree| tree.offset == binding.tree)
            .ok_or("wound current joint dictionary absent")?;
        let joints = tree
            .entries
            .iter()
            .map(|(key, node)| {
                Ok(crate::skinned_hit_point::Joint {
                    key: key.to_ascii_lowercase(),
                    matrix: crate::animation_references::node_matrix(
                        &descriptor.animation,
                        *node,
                        frame.tick,
                    )
                    .map_err(|e| e.to_string())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if joints.is_empty() {
            return Err("wound source frame has no named joints in the acquired profile".into());
        }
        Ok(Some(joints))
    }
    /// Copy the dedicated current collision model immediately after preparing
    /// its owned position buffer. Render LOD, bind pose and guessed extents are
    /// never substitutes for a missing completed frame or resource.
    pub fn vimp_contact_snapshot(
        &mut self,
        id: Id,
        resources: &crate::vimp_contacts::Resources,
    ) -> Result<crate::vimp_contacts::Snapshot, String> {
        let actor = self.get(id).ok_or("Vimp contact owner absent or foreign")?;
        let vimp = actor.vimp.ok_or("Vimp contact construction absent")?;
        if actor.class != 8
            || actor.seed.storage.state() != 21
            || actor.vimp_frame_stopped
            || !vimp.initialized
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("Vimp contact requires a completed unparented live owner".into());
        }
        // The common-frame seal checks physical/ANM consumption and finished
        // dispatch. The primary's pending marker is reset at the next entry.
        let completed = self.completed_vimp_frame(id)?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp contact fields absent")?
            .clone();
        let channels = actor.channels.ok_or("Vimp contact channels absent")?;
        let specification = self
            .specification(id)
            .ok_or("Vimp contact specification absent")?
            .clone();
        if !Arc::ptr_eq(&specification, &resources.specification) {
            return Err("Vimp contact resources belong to another specification".into());
        }
        let affine = self.actor_current_affine(id)?;
        let matrix = affine.matrix[..12].try_into().unwrap();
        let model_bounds = resources.bounds.bounds(channels[0][0])?;
        let bounds = crate::collision_bounds::cached(
            Some(crate::collision_bounds::Models {
                primary: model_bounds.primary,
                auxiliary: model_bounds.current_auxiliary,
                slot: channels[0][0],
            }),
            matrix,
            crate::arithmetic::Precision::Double53,
        )?;
        let (mesh, triangles) = self.contact_geometry(id, completed.animation_epoch)?;
        Ok(crate::vimp_contacts::Snapshot {
            actor: crate::projectile_contacts::Actor {
                id,
                parent: None,
                class: 8,
                state: 21,
                context: vimp.context,
                type_word: vimp.mode,
                flags: DirectFlags {
                    active: fields.object_flags,
                    personality: fields.configuration[14],
                },
            },
            frame: completed.number,
            response_flags: fields.response_flags,
            bounds: crate::volume_queries::Bounds {
                min: bounds[..3].try_into().unwrap(),
                max: bounds[3..].try_into().unwrap(),
            },
            mesh,
            triangles,
        })
    }
}
