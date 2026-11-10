//! Acquired class-669 normal held-cache publication. Animation, alternate lock
//! profiles and nonempty position footprints remain separate services.
use super::*;
use crate::procedural_aim::Matrix;

#[derive(Debug)]
pub(super) struct Owned {
    number: u64,
    binding: crate::mecc_wield::Binding,
    channels: crate::animation_channels::Channels,
    pose: [u32; 7],
    cache: crate::actor_affine::Cache,
    reference: [u32; 17],
}
fn pose(fields: &crate::actor_fields::Fields) -> [u32; 7] {
    let mut out = [0; 7];
    out[..3].copy_from_slice(&fields.position.map(f32::to_bits));
    out[3..6].copy_from_slice(&fields.angles.map(f32::to_bits));
    out[6] = fields.scale.to_bits();
    out
}
fn reference_words(m: Matrix) -> [u32; 17] {
    let mut words = [0; 17];
    words[0] = m.flags;
    words[1..].copy_from_slice(&m.values.map(f32::to_bits));
    words
}
impl Session {
    /// Read the acquired held-origin route while the owned local caller runs.
    /// Its first turn reads constructor zeros; later turns read the preceding
    /// completed placement. Ordinary completed-frame readers keep their guards.
    pub fn delphi_prephysical_target_ray(
        &self,
        owner: Id,
        number: u64,
        threshold: f32,
    ) -> Result<crate::delphi_target_ray::Sample, String> {
        use crate::delphi_target_ray::{Sample, Source};
        let actor = self
            .get(owner)
            .ok_or("Delphi pre-physical ray owner absent or foreign")?;
        let cycle = actor
            .delphi_physical
            .as_ref()
            .ok_or("Delphi physical cycle absent")?;
        let target = actor
            .delphi_local_target
            .as_ref()
            .ok_or("Delphi local caller absent")?;
        if !self.delphi_target_running
            || !cycle.resources.local_target_phase
            || cycle.view.failure.is_some()
            || cycle.view.number.checked_add(1) != Some(number)
            || target.number != number
            || target.phase != super::DelphiLocalTargetPhase::Running
            || target.failure.is_some()
            || actor.class != 3
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 3
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err("Delphi pre-physical ray requires its running local caller".into());
        }
        let child = self.delphi_held_child(owner)?;
        if cycle.view.number != 0 {
            let ray = self.delphi_target_ray(owner, cycle.view.number, threshold)?;
            let matrix = self.actor_current_affine(owner)?.matrix;
            return Ok(Sample {
                source: Source::PreviousFrame(cycle.view.number),
                ray,
                second_column: [matrix[1], matrix[5], matrix[9]],
            });
        }
        let initialized = self
            .delphi_special(owner)
            .ok_or("Delphi cold group incomplete")?;
        let held = self.get(child).ok_or("Delphi cold held actor absent")?;
        if !actor.acquired_construction
            || initialized.held != child
            || actor.affine_cache.is_some()
            || !held.acquired_construction
            || held.class != 669
            || held.seed.storage.profile() != Profile::Debug
            || held.seed.storage.state() != 0
            || held.affine_cache.is_some()
            || held.delphi_held_pose.is_some()
            || self.reference_cache_storage(child)?.sampled
        {
            return Err(
                "Delphi cold ray requires its unchanged acquired constructor caches".into(),
            );
        }
        // These two header caches are zero in the acquired cold constructors;
        // unlike an evaluated affine, they have no implicit identity last row.
        let zero = Matrix {
            flags: 0,
            values: [0.; 16],
        };
        let reference = self.delphi_held_reference(child)?;
        Ok(Sample {
            source: Source::ColdConstructor,
            ray: crate::delphi_target_ray::from_constructor_caches(
                zero, zero, reference, threshold,
            )?,
            second_column: [0.; 3],
        })
    }
    fn delphi_held_reference(&self, child: Id) -> Result<Matrix, String> {
        let storage = self.reference_cache_storage(child)?;
        if storage.sampled {
            return self
                .current_reference_matrices(child)?
                .first()
                .copied()
                .ok_or_else(|| "Delphi held evaluated reference absent".into());
        }
        let reference = storage
            .matrices
            .first()
            .copied()
            .ok_or("Delphi held constructor reference absent")?;
        if storage.matrices.len() != 1 || reference_words(reference) != [0; 17] {
            return Err(
                "Delphi held unevaluated reference outside acquired zeroed constructor storage"
                    .into(),
            );
        }
        Ok(reference)
    }

    fn delphi_held_child(&self, owner: Id) -> Result<Id, String> {
        self.wield_list()?
            .into_iter()
            .find(|c| c.flags & 0x8000 == 0 && c.parent == Some(owner) && c.reference == 1)
            .map(|c| c.id)
            .ok_or_else(|| "Delphi target ray held owner/reference absent".into())
    }
    /// Publish the acquired normal held matrix and actual position history after
    /// a completed parent frame. This does not advance child ANM or a world tick.
    pub fn advance_delphi_held_pose(&mut self, owner: Id, number: u64) -> Result<Id, String> {
        let parent = self
            .get(owner)
            .ok_or("Delphi held parent absent or foreign")?;
        let view = self
            .delphi_physical(owner)
            .ok_or("Delphi held parent frame absent")?;
        if parent.seed.storage.profile() != Profile::Debug
            || parent.class != 3
            || parent.seed.storage.state() != 3
            || view.number != number
            || number == 0
            || view.failure.is_some()
        {
            return Err("Delphi held pose requires its completed unfailed parent frame".into());
        }
        let parent_fields = parent
            .construction_fields
            .as_ref()
            .ok_or("Delphi held parent fields absent")?;
        let parent_cache = self.actor_current_affine(owner)?;
        let parent_reference = *self
            .current_reference_matrices(owner)?
            .get(1)
            .ok_or("Delphi held parent reference absent")?;
        let child = self.delphi_held_child(owner)?;
        let actor = self.get(child).ok_or("Delphi held child absent")?;
        let binding = self
            .wield_binding(child)
            .ok_or("Delphi held binding absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi held fields absent")?;
        if actor.seed.storage.profile() != Profile::Debug
            || actor.class != 669
            || actor.seed.storage.state() != 0
            || binding.context != 0
            || actor.channels != Some(fields.channels)
            || fields.object_flags & !(0x20 | 0x40 | 0x800 | 0x40000000) != 0x20400000
            || fields.response_flags & 0x02009000 != 0
            || actor.environment.is_some()
            || actor.ordinary_motion.is_some()
            || actor.finalization.is_some()
            || actor.surface_link.is_some()
            || actor
                .delphi_held_pose
                .as_ref()
                .is_some_and(|p| p.number.checked_add(1) != Some(number))
        {
            return Err("Delphi held pose outside acquired normal zero-footprint profile".into());
        }
        let reference = self.delphi_held_reference(child)?;
        let anchor = Matrix {
            flags: parent_cache.flags,
            values: parent_cache.matrix,
        }
        .mul(parent_reference)?;
        let mut matrix = crate::effect_attachment::normal_matrix(
            anchor,
            fields.scale,
            parent_fields.scale,
            reference,
        )?;
        matrix.values[11] = (f64::from(matrix.values[11])
            + f64::from(f32::from_bits(fields.configuration[17])))
            as f32;
        if !matrix.values.into_iter().all(f32::is_finite) {
            return Err("Delphi held pose outside finite affine profile".into());
        }
        let parameter = (fields.object_flags & 0x20000000 != 0)
            .then(|| self.delphi_copy_parameter(owner).unwrap_or(0.));
        let cache = crate::actor_affine::Cache {
            flags: matrix.flags,
            matrix: matrix.values,
        };
        let actor = self
            .residents
            .get_mut(&child.serial)
            .expect("validated Delphi held child");
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.position = [matrix.values[3], matrix.values[7], matrix.values[11]];
        fields.previous = fields.current;
        fields.current = fields.position;
        if let Some(value) = parameter {
            actor.delphi_copy_parameter = Some(value);
        }
        actor.affine_cache = Some(OwnedAffine {
            value: cache,
            pose: affine_pose_words(fields),
            external_orientation: fields.object_flags & 0x10000 != 0,
        });
        actor.delphi_held_pose = Some(Owned {
            number,
            binding,
            channels: fields.channels,
            pose: pose(fields),
            cache,
            reference: reference_words(reference),
        });
        Ok(child)
    }
    /// Read the actually published held cache; parented caches are not admitted
    /// through the ordinary unparented-affine reader.
    pub fn delphi_held_affine(
        &self,
        owner: Id,
        number: u64,
    ) -> Result<crate::actor_affine::Cache, String> {
        let child = self.delphi_held_child(owner)?;
        let actor = self.get(child).ok_or("Delphi held child absent")?;
        let owned = actor
            .delphi_held_pose
            .as_ref()
            .ok_or("Delphi held cache not produced")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi held fields absent")?;
        let parent = self
            .delphi_physical(owner)
            .ok_or("Delphi held parent frame absent")?;
        if owned.number != number
            || parent.number != number
            || parent.failure.is_some()
            || self.wield_binding(child) != Some(owned.binding)
            || actor.channels != Some(owned.channels)
            || pose(fields) != owned.pose
        {
            return Err("Delphi held cache frame, binding or channels are stale".into());
        }
        if reference_words(self.delphi_held_reference(child)?) != owned.reference {
            return Err("Delphi held reference storage changed after pose publication".into());
        }
        let cache = actor
            .affine_cache
            .as_ref()
            .ok_or("Delphi held affine storage absent")?;
        if cache.value.flags != owned.cache.flags
            || cache.value.matrix.map(f32::to_bits) != owned.cache.matrix.map(f32::to_bits)
        {
            return Err("Delphi held affine storage was replaced".into());
        }
        Ok(cache.value)
    }
    pub fn delphi_target_ray(
        &self,
        owner: Id,
        number: u64,
        threshold: f32,
    ) -> Result<crate::delphi_target_ray::Ray, String> {
        let held = self.delphi_held_affine(owner, number)?;
        let owner_cache = self.actor_current_affine(owner)?;
        let child = self.delphi_held_child(owner)?;
        let reference = self.delphi_held_reference(child)?;
        crate::delphi_target_ray::from_poses(
            Matrix {
                flags: owner_cache.flags,
                values: owner_cache.matrix,
            },
            Matrix {
                flags: held.flags,
                values: held.matrix,
            },
            reference,
            threshold,
        )
    }
}
