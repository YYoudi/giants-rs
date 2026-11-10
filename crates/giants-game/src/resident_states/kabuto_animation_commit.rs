use super::*;
use crate::{
    actor_animation_phase, animation_channels, animation_frames, kabuto_animation_commit as commit,
};

impl Session {
    pub(crate) fn commit_kabuto_animation(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        frame: commit::Frame,
        services: &mut actor_animation_phase::Services<'_>,
        world: &mut impl commit::World,
    ) -> Result<commit::View, String> {
        self.check_kabuto_reference_owner(owner, resources, number)?;
        let a = self.get(owner).unwrap();
        let g = a.kabuto_group.as_ref().unwrap();
        if g.animation_commit_running || g.animation_commit_entered == Some(number) {
            return Err("Kabuto animation commit reentry or replay".into());
        }
        let captured = g
            .resource_phase_view
            .and_then(|v| v.resources)
            .ok_or("Kabuto animation commit resource sample absent")?;
        // The caller retains this earlier template across selection and ANM.
        let old_slot = u8::try_from(captured.slot)
            .map_err(|_| "Kabuto retained descriptor slot outside table")?;
        let old = resources
            .specification
            .animations
            .get(&old_slot)
            .ok_or("Kabuto retained animation descriptor absent")?;
        let descriptor = animation_frames::Descriptor {
            timeline: old.animation.timeline,
            flags: old.template.flags,
        };
        let f = a.construction_fields.as_ref().unwrap();
        if a.channels != Some(f.channels) {
            return Err("Kabuto animation commit channel mirror differs".into());
        }
        let selection = frame.selection;
        let scaled = (f64::from(selection.reference_speed) * f64::from(f.scale)) as f32;
        if !selection.reference_speed.is_finite() || !f.scale.is_finite() || !scaled.is_finite() {
            return Err("Kabuto animation commit reference speed outside finite profile".into());
        }
        let request = animation_channels::Request {
            slot: selection.slot,
            flags: selection.flags & !selection.clear_flags,
            speed_source: selection.speed_source,
            reference_speed: scaled,
            blend_seconds: selection.blend_seconds,
        };
        let mut channels = f.channels;
        let notifying = world
            .network(self, owner)?
            .eligible()
            .map_err(str::to_owned)?;
        animation_channels::select_primary(&mut channels, request, services.blending)
            .map_err(str::to_owned)?;
        let a = self.residents.get_mut(&owner.serial).unwrap();
        let g = a.kabuto_group.as_mut().unwrap();
        g.animation_commit_entered = Some(number);
        g.animation_commit_running = true;
        a.channels = Some(channels);
        a.construction_fields.as_mut().unwrap().channels = channels;

        let result: Result<commit::View, String> = (|| {
            if notifying {
                // Selection is already visible; later stores must not restore a
                // stale header after transport mutates or removes this owner.
                let sent = world.notify(self, owner, request);
                self.check_kabuto_reference_owner(owner, resources, number)?;
                sent?;
            }
            let a = self.residents.get_mut(&owner.serial).unwrap();
            a.kabuto_group.as_mut().unwrap().motion.mode = selection.behavior;
            let p = a.seed.storage.data_mut().unwrap();
            p[0x90..0x94].copy_from_slice(&selection.definition.to_le_bytes());
            let f = a.construction_fields.as_mut().unwrap();
            if a.channels != Some(f.channels) {
                return Err("Kabuto post-notification channel mirror differs".into());
            }
            if request.flags & 0x80 != 0 {
                f.channels[0][1] = selection.retained_time;
                a.channels = Some(f.channels);
            }
            let pass =
                self.kabuto_commit_animation_sample(owner, resources, frame.materials, services)?;
            let channels = self.get(owner).unwrap().channels.unwrap();
            // The first conversion executes but its return does not replace the
            // caller's earlier previous-frame local. Keep that distinction.
            let _ = animation_frames::frame_at_time(descriptor, f32::from_bits(channels[0][2]))
                .map_err(str::to_owned)?;
            let current_frame =
                animation_frames::frame_at_time(descriptor, f32::from_bits(channels[0][1]))
                    .map_err(str::to_owned)?;
            self.residents
                .get_mut(&owner.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap()[0xe0..0xe4]
                .copy_from_slice(&0u32.to_le_bytes());
            Ok(commit::View {
                current_frame,
                animation_processed: pass.processed,
                event_visits: pass.visits,
                stamp: pass.stamp,
            })
        })();
        if let Some(g) = self
            .residents
            .get_mut(&owner.serial)
            .and_then(|a| a.kabuto_group.as_mut())
            .filter(|g| {
                g.animation_commit_entered == Some(number)
                    && Arc::ptr_eq(&g.declared_class, &resources.specification)
            })
        {
            g.animation_commit_running = false;
            match &result {
                Ok(_) => g.animation_commit_completed = Some(number),
                Err(error) => g.geometry_transition_failure = Some(error.clone()),
            }
        }
        result
    }

    fn kabuto_commit_animation_sample(
        &mut self,
        owner: Id,
        resources: &Resources,
        materials: Option<Arc<crate::mecc_contact_materials::Catalog>>,
        services: &mut actor_animation_phase::Services<'_>,
    ) -> Result<actor_animation_phase::Pass, String> {
        let a = self.get(owner).ok_or("Kabuto commit ANM owner absent")?;
        let f = a
            .construction_fields
            .as_ref()
            .ok_or("Kabuto commit ANM fields absent")?;
        if a.channels != Some(f.channels) {
            return Err("Kabuto commit ANM channel mirror differs".into());
        }
        let g = a.kabuto_group.as_ref().unwrap();
        let sample = actor_animation_phase::Sample {
            model: g.motion.model,
            affine: a.affine_cache.as_ref().map(|c| c.value),
            support: g.contact.support,
            support_part: g.contact.support_part,
        };
        let bank = self.kabuto_events.clone().unwrap_or_else(|| {
            crate::mecc_animation_events::Bank::new(resources.specification.clone())
        });
        if !bank.belongs_to(&resources.specification) {
            return Err("Kabuto commit ANM event bank specification differs".into());
        }
        let mut pipeline = actor_animation_phase::Pipeline {
            specification: resources.specification.clone(),
            references: self
                .specifications
                .get(&owner.serial)
                .ok_or("Kabuto commit ANM references absent")?
                .0
                .clone(),
            bank,
            contexts: services.contexts.staged_copy(),
            random: *services.random,
            channels: f.channels,
            owner,
            stamp: services.stamp,
            last_stamp: a.mecc_animation_stamp,
            blending: services.blending,
            materials,
        };
        let result = pipeline.advance_sample(f, sample, 0.);
        // Preserve completed ANM/ref/epoch/context/RNG stores on late refusal.
        self.specifications.get_mut(&owner.serial).unwrap().0 = pipeline.references;
        self.kabuto_events = Some(pipeline.bank);
        *services.contexts = pipeline.contexts;
        *services.random = pipeline.random;
        let a = self.residents.get_mut(&owner.serial).unwrap();
        a.channels = Some(pipeline.channels);
        a.construction_fields.as_mut().unwrap().channels = pipeline.channels;
        a.mecc_animation_stamp = pipeline.last_stamp;
        result
    }
}
