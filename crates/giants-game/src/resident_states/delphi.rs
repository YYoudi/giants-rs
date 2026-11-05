//! Delphi-specific owned mutations; ordinary Mecc admission stays unchanged.
use super::*;
use crate::delphi_special::{CopySource, Initialized, State};
impl Session {
    pub fn prepare_delphi_physical(
        &mut self,
        id: Id,
        resources: crate::delphi_physical::Resources,
    ) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("Delphi physical owner absent or foreign")?;
        if !actor.delphi_physics_present
            || actor.delphi_physical.is_some()
            || !self
                .specification(id)
                .is_some_and(|s| std::sync::Arc::ptr_eq(s, &resources.specification))
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.mecc_animation_stamp != 0
        {
            return Err(
                "Delphi physical resources require their fresh initialized owned player".into(),
            );
        }
        let cycle = crate::delphi_physical::Owned::fresh(
            resources,
            actor
                .construction_fields
                .as_ref()
                .ok_or("Delphi fields absent")?,
            actor.seed.storage.data().ok_or("Delphi payload absent")?,
        )?;
        self.residents.get_mut(&id.serial).unwrap().delphi_physical = Some(cycle);
        Ok(())
    }
    pub fn delphi_physical(&self, id: Id) -> Option<&crate::delphi_physical::View> {
        self.get(id)?.delphi_physical.as_ref().map(|c| &c.view)
    }
    pub fn advance_delphi_physical(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        self.advance_delphi_phases(id, e, ground, services, false)
    }
    /// Acquired physical/history -> dry sword secondary -> zero-time ANM.
    /// This does not execute the surrounding full primary or auxiliary callbacks.
    pub fn advance_delphi_locomotion(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        self.advance_delphi_phases(id, e, ground, services, true)
    }
    fn advance_delphi_phases(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
        locomotion: bool,
    ) -> Result<crate::delphi_physical::View, String> {
        let actor = self
            .get(id)
            .ok_or("Delphi physical owner absent or foreign")?;
        let mut cycle = actor
            .delphi_physical
            .clone()
            .ok_or("Delphi physical cycle unprepared")?;
        if cycle.resources.locomotion.is_some() != locomotion {
            return Err("Delphi phase entry differs from its prepared profile".into());
        }
        let before_cycle = cycle.clone();
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi fields absent")?
            .clone();
        if locomotion && fields.configuration[14] & 0x10 != 0 {
            return Err("Delphi locomotion requires its entered secondary dispatch profile".into());
        }
        let payload = actor
            .seed
            .storage
            .data()
            .ok_or("Delphi payload absent")?
            .to_vec();
        let group = self
            .delphi_special(id)
            .ok_or("Delphi group absent")?
            .clone();
        for copy in &group.copies {
            for child in [copy.actor, copy.held] {
                self.get(child)
                    .and_then(|a| a.construction_fields.as_ref())
                    .ok_or("Delphi physical group member absent")?;
            }
        }
        // Event descriptors are mutable class resources, shared by owners of
        // the same prepared specification rather than reset per actor.
        let specification = cycle.resources.specification.clone();
        let bank = self
            .delphi_events
            .clone()
            .unwrap_or_else(|| crate::mecc_animation_events::Bank::new(specification.clone()));
        if !bank.belongs_to(&specification) {
            return Err("Delphi event resources differ from shared class bank".into());
        }
        let mut pipeline = crate::actor_animation_phase::Pipeline {
            specification: cycle.resources.specification.clone(),
            references: self
                .specifications
                .get(&id.serial)
                .ok_or("Delphi references absent")?
                .0
                .clone(),
            bank,
            contexts: services.contexts.staged_copy(),
            random: *services.random,
            channels: actor.channels.ok_or("Delphi channels absent")?,
            owner: id,
            stamp: services.stamp,
            last_stamp: actor.mecc_animation_stamp,
            blending: services.blending,
            materials: Some(cycle.resources.materials.clone()),
        };
        let globals = e.globals;
        let result = cycle
            .advance(&fields, &payload, e, ground, |fields, view, seconds| {
                let pass = pipeline.advance_sample(
                    fields,
                    crate::actor_animation_phase::Sample {
                        model: view.model,
                        affine: view.affine,
                        support: view.support,
                        support_part: view.support_part,
                    },
                    seconds,
                )?;
                fields.channels = pass.channels;
                Ok(())
            })
            .and_then(|mut out| {
                if let Some(clips) = cycle.resources.locomotion {
                    let word = crate::delphi_physical::word;
                    if [0xdb4, 0xf20, 0xf24, 0xe2c, 0xfd0]
                        .into_iter()
                        .any(|at| word(&out.payload, at) != 0)
                    {
                        return Err(
                            "Delphi secondary equipment/context/effect producer pending".into()
                        );
                    }
                    let view = &cycle.view;
                    let selection = crate::delphi_locomotion::select(
                        crate::delphi_locomotion::State {
                            mode: word(&out.payload, 0x5c) as i32,
                            internal: word(&out.payload, 0),
                            active: out.fields.object_flags,
                            controls: out.fields.control_flags,
                            physics: word(&out.payload, 0xd00),
                            clip: pipeline.channels[0][0],
                            wrapped: pipeline.channels[0][8] & 2 != 0,
                            model: view.model,
                            velocity: view.velocity,
                            previous_velocity: view.previous_velocity,
                            position_z: out.fields.position[2],
                            floor: view.floor,
                            offset: f32::from_bits(out.fields.configuration[17]),
                            scale: out.fields.scale,
                            damage: view.accumulated_damage,
                            maximum_damage: f32::from_bits(out.fields.configuration[12]),
                        },
                        crate::delphi_locomotion::Parameters::from_globals(globals)?,
                        clips,
                        &mut pipeline.random,
                    )?;
                    for (at, value) in [(0, selection.internal), (0x5c, selection.mode as u32)] {
                        out.payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
                    }
                    out.fields.control_flags = selection.controls;
                    if let Some(request) = selection.request {
                        crate::animation_channels::select_primary(
                            &mut pipeline.channels,
                            request,
                            pipeline.blending,
                        )?;
                    }
                    out.fields.channels = pipeline.channels;
                    let pass = pipeline.advance_sample(
                        &out.fields,
                        crate::actor_animation_phase::Sample {
                            model: view.model,
                            affine: view.affine,
                            support: view.support,
                            support_part: view.support_part,
                        },
                        0.,
                    )?;
                    out.fields.channels = pass.channels;
                }
                Ok(out)
            });
        match result {
            Ok(out) => {
                self.specifications.get_mut(&id.serial).unwrap().0 = pipeline.references;
                self.delphi_events = Some(pipeline.bank);
                *services.contexts = pipeline.contexts;
                *services.random = pipeline.random;
                let actor = self.residents.get_mut(&id.serial).unwrap();
                actor.channels = Some(out.fields.channels);
                actor.mecc_animation_stamp = pipeline.last_stamp;
                actor.affine_cache = Some(OwnedAffine {
                    value: cycle.view.affine.ok_or("Delphi physical affine absent")?,
                    pose: affine_pose_words(&out.fields),
                    external_orientation: false,
                });
                actor.construction_fields = Some(out.fields);
                actor
                    .seed
                    .storage
                    .data_mut()
                    .unwrap()
                    .copy_from_slice(&out.payload);
                // Ordinary non-turbo Delphi hides the existing copies and held
                // objects; this does not execute their separate callbacks.
                for copy in group.copies {
                    for child in [copy.actor, copy.held] {
                        self.residents
                            .get_mut(&child.serial)
                            .unwrap()
                            .construction_fields
                            .as_mut()
                            .unwrap()
                            .object_flags |= 0x40000000;
                    }
                }
                let view = cycle.view.clone();
                self.residents.get_mut(&id.serial).unwrap().delphi_physical = Some(cycle);
                Ok(view)
            }
            Err(error) => {
                if cycle.view.failure.is_none() {
                    cycle = before_cycle;
                    cycle.view.failure = Some(error.clone());
                }
                self.residents.get_mut(&id.serial).unwrap().delphi_physical = Some(cycle);
                Err(error)
            }
        }
    }
    pub fn initialize_delphi_physics(
        &mut self,
        id: Id,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<crate::delphi_initial::Physics, String> {
        let actor = self
            .get(id)
            .ok_or("Delphi physics owner absent or foreign")?;
        if actor.class != 3
            || !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 3
            || !matches!(actor.delphi_special, Some(State::Complete(_)))
            || actor.delphi_physics_present
            || actor.mecc_physics_present
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.controlled_motion.is_some()
            || actor.ordinary_motion.is_some()
            || actor.environment.is_some()
            || actor.finalization.is_some()
            || actor.world_tick.is_some()
            || actor.world_tick_running
            || actor.affine_cache.is_some()
            || actor.mecc_animation_stamp != 0
        {
            return Err("Delphi physics requires its fresh unparented special group".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi physical fields absent")?;
        let initialized = crate::delphi_initial::initialize(
            actor
                .seed
                .storage
                .data()
                .ok_or("Delphi physical payload absent")?,
            fields.scale,
            crate::delphi_initial::Parameters::from_globals(globals)?,
        )?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor
            .seed
            .storage
            .data_mut()
            .unwrap()
            .copy_from_slice(&initialized.payload);
        actor.construction_fields.as_mut().unwrap().control_flags = initialized.control_flags;
        actor.delphi_physics_present = true;
        Ok(initialized.physics)
    }
    pub fn delphi_physics(&self, id: Id) -> Result<crate::delphi_initial::Physics, String> {
        let actor = self
            .get(id)
            .ok_or("Delphi physics owner absent or foreign")?;
        if !actor.delphi_physics_present {
            return Err("Delphi physics not initialized".into());
        }
        crate::delphi_initial::Physics::read(
            actor.seed.storage.data().ok_or("Delphi payload absent")?,
        )
    }
    pub(crate) fn finish_delphi_player_fields(
        &mut self,
        id: Id,
        cold: crate::animation_channels::Channels,
        selected: crate::animation_channels::Channels,
    ) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("Delphi player owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi player fields absent")?;
        if actor.class != 3
            || !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 3
            || !matches!(actor.delphi_special, Some(State::Complete(_)))
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.channels != Some(cold)
            || fields.channels != cold
            || fields.configuration[14] & 1 != 0
        {
            return Err(
                "Delphi player fields require their fresh completed cold special group".into(),
            );
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.channels = selected;
        fields.configuration[14] |= 1;
        actor.channels = Some(selected);
        Ok(())
    }
    pub fn delphi_special(&self, id: Id) -> Option<&Initialized> {
        match self.get(id)?.delphi_special.as_ref()? {
            State::Complete(value) => Some(value),
            _ => None,
        }
    }
    pub fn delphi_special_failure(&self, id: Id) -> Option<&str> {
        match self.get(id)?.delphi_special.as_ref()? {
            State::Failed(value) => Some(value),
            _ => None,
        }
    }
    pub fn delphi_copy_source(&self, id: Id) -> Option<CopySource> {
        self.get(id)?.delphi_copy_source
    }
    /// Acquired header parameter; its later rendering semantics are unverified.
    pub fn delphi_copy_parameter(&self, id: Id) -> Option<f32> {
        self.get(id)?.delphi_copy_parameter
    }
    pub(crate) fn construct_delphi_cold(
        &mut self,
        input: Construction<'_>,
        mask: u32,
    ) -> Result<Id, String> {
        if !matches!(
            (input.class, input.seed.storage.state(), mask),
            (3, 3, 0x80000000) | (311, 62, 4) | (669, 0, 0x20)
        ) || input.seed.storage.profile() != Profile::Debug
            || !input.seed.diagnostics.is_empty()
            || input.fields.object_flags != input.flags.active
            || input.fields.configuration[14] != input.flags.personality
        {
            return Err(
                "Delphi construction requires the acquired class/state/mask template".into(),
            );
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let id = staged.register_with_mask(
            input.class,
            input.seed,
            input.bindings,
            input.flags,
            mask,
        )?;
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = input.fields.channels;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.acquired_construction = true;
        self.publish_staged(staged, id)
    }
    pub(crate) fn begin_delphi_special(
        &mut self,
        id: Id,
        cold: &Seed,
        template: &crate::actor_fields::Fields,
    ) -> Result<(), String> {
        let actor = self.get(id).ok_or("Delphi owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi fields absent")?;
        if actor.class != 3
            || !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 3
            || actor.seed.storage.data() != cold.storage.data()
            || cold.storage.data().is_none_or(|p| p.len() != 4096)
            || actor.delphi_special.is_some()
            || actor.channels != Some(template.channels)
            || fields.channels != template.channels
            || fields.object_flags != template.object_flags
            || fields.control_flags != template.control_flags
            || fields.action != template.action
            || fields.configuration != template.configuration
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || fields.dispatch_context != 0
            || fields.object_flags & 0x8000 != 0
            || !fields
                .position
                .into_iter()
                .chain(fields.angles)
                .chain([fields.scale])
                .all(f32::is_finite)
        {
            return Err(
                "Delphi special phase requires a fresh acquired normal class3 owner".into(),
            );
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.delphi_special = Some(State::Initializing);
        actor.seed.storage.data_mut().unwrap()[0x68..0x6c].copy_from_slice(&u32::MAX.to_le_bytes());
        Ok(())
    }
    pub(crate) fn bind_delphi_copy(
        &mut self,
        copy: Id,
        owner: Id,
        index: u32,
        parameter: f32,
    ) -> Result<(), String> {
        if !matches!(
            self.get(owner).and_then(|a| a.delphi_special.as_ref()),
            Some(State::Initializing)
        ) {
            return Err("Delphi copy source is not initializing".into());
        }
        let source = self
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let position = source.position;
        let scale = source.scale;
        let context = source.dispatch_context;
        let actor = self.get(copy).ok_or("Delphi copy absent")?;
        if actor.class != 311
            || actor.seed.storage.state() != 62
            || actor.constructor_mask != 4
            || actor.delphi_copy_source.is_some()
            || !parameter.is_finite()
        {
            return Err("Delphi copy outside fresh source profile".into());
        }
        let actor = self.residents.get_mut(&copy.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.position = position;
        fields.current = position;
        fields.scale = scale;
        fields.dispatch_context = context;
        fields.configuration[14] |= 0xe00;
        fields.object_flags |= 0x40400010;
        actor.delphi_copy_source = Some(CopySource { owner, index });
        actor.delphi_copy_parameter = Some(parameter);
        Ok(())
    }
    pub(crate) fn bind_delphi_held(
        &mut self,
        child: Id,
        parent: Id,
        reference: i32,
    ) -> Result<(), String> {
        let actor = self.get(parent).ok_or("Delphi held parent absent")?;
        let allowed = matches!(actor.delphi_special, Some(State::Initializing))
            || actor.delphi_copy_source.is_some_and(|c| {
                matches!(
                    self.get(c.owner).and_then(|a| a.delphi_special.as_ref()),
                    Some(State::Initializing)
                )
            });
        if !allowed || reference != 1 || actor.wield_failure.is_some() {
            return Err("Delphi held requires its initializing owner or copy".into());
        }
        let source = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi held parent fields absent")?;
        let position = source.position;
        let yaw = source.angles[0];
        let actor = self.get(child).ok_or("Delphi held child absent")?;
        if actor.class != 669
            || actor.constructor_mask != 0x20
            || actor.wield_binding.is_some()
            || actor.attachment.is_some()
        {
            return Err("Delphi held requires its fresh acquired constructor".into());
        }
        let actor = self.residents.get_mut(&child.serial).unwrap();
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.position = position;
        fields.angles[0] = yaw;
        fields.object_flags |= 0x20400000;
        fields.dispatch_context = 0;
        actor.attachment = Some(Attachment {
            parent,
            child_reference: 0,
            parent_reference: 1,
            behavior: 0,
        });
        actor.presentation_parent = Some(parent);
        actor.wield_binding = Some(crate::mecc_wield::Binding {
            parent,
            reference: 1,
            context: 0,
        });
        Ok(())
    }
    pub(crate) fn finish_delphi_copy_held(&mut self, id: Id, parameter: f32) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Delphi copy held absent")?;
        let fields = actor.construction_fields.as_mut().unwrap();
        fields.object_flags |= 0x40000010;
        fields.configuration[14] |= 0xe00;
        actor.delphi_copy_parameter = Some(parameter);
        Ok(())
    }
    pub(crate) fn initialize_delphi_equipment(
        &mut self,
        id: Id,
        charges: i32,
    ) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Delphi equipment owner absent")?;
        if actor.class != 3 || !matches!(actor.delphi_special, Some(State::Initializing)) {
            return Err("Delphi equipment requires its initializing cold owner".into());
        }
        let body = actor
            .seed
            .storage
            .data_mut()
            .ok_or("Delphi payload absent")?;
        for at in [
            0xd3c, 0xd48, 0xd54, 0xd60, 0xd6c, 0xd78, 0xd80, 0xd88, 0xd90, 0xd98, 0xda0, 0xdb0,
            0xdb4,
        ] {
            body[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        for at in [0xd7c, 0xd84, 0xd8c, 0xd94, 0xd9c, 0xda4] {
            body[at..at + 4].fill(0);
        }
        body[0xd78..0xd7c].copy_from_slice(&44u32.to_le_bytes());
        body[0xd7c..0xd80].copy_from_slice(&charges.to_le_bytes());
        Ok(())
    }
    pub(crate) fn finish_delphi_special(&mut self, id: Id) -> Result<(), String> {
        let actor = self
            .residents
            .get_mut(&id.serial)
            .ok_or("Delphi owner absent at completion")?;
        actor.seed.storage.data_mut().unwrap()[0xdb4..0xdb8].fill(0);
        Ok(())
    }
    pub(crate) fn end_delphi_special(&mut self, id: Id, result: &Result<Initialized, String>) {
        if let Some(actor) = self.residents.get_mut(&id.serial) {
            actor.delphi_special = Some(match result {
                Ok(value) => State::Complete(value.clone()),
                Err(error) => State::Failed(error.clone()),
            });
        }
    }
}

#[cfg(test)]
mod physical_tests {
    use super::*;

    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT; silent owned CPU composition, no live controller"]
    fn source_delphi_physical_owns_motion_shares_events_and_quarantines_failure() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English")
                .unwrap();
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let weapons = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
            giants_formats::weapons::Profile::Debug,
        )
        .unwrap();
        let factory = crate::delphi_special::Factory::load(&resources, &globals, &weapons).unwrap();
        let mut session = Session::new(1281);
        let mut owners = Vec::new();
        for profile in 0..2 {
            let id = factory
                .construct_player_fields(
                    &mut session,
                    crate::delphi_player::ColdFrame {
                        position: [0., 0., 10.],
                        current: [0., 0., 10.],
                        previous: [0., 0., 10.],
                        angles: [0.; 3],
                    },
                    crate::animation_channels::Blending::Disabled,
                )
                .unwrap()
                .owner;
            session.initialize_delphi_physics(id, &globals).unwrap();
            let prepared = crate::delphi_physical::Resources::load(
                &resources,
                session.specification(id).unwrap().clone(),
                Arc::new(
                    crate::mecc_contact_materials::Catalog::from_prepared(&[], Default::default())
                        .unwrap(),
                ),
            )
            .unwrap();
            let prepared = if profile == 0 {
                prepared
            } else {
                let executable = std::env::var_os("GIANTS_DELPHI_EXECUTABLE")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| root.join("Giants.exe"));
                let definitions =
                    crate::delphi_animation_definitions::Definitions::read(&executable).unwrap();
                prepared
                    .with_locomotion(definitions.sword_locomotion().unwrap())
                    .unwrap()
            };
            session
                .prepare_delphi_physical(id, prepared.clone())
                .unwrap();
            assert!(session.prepare_delphi_physical(id, prepared).is_err());
            owners.push(id);
        }
        let mut contexts =
            crate::attached_effect_context::Pool::from_active_resources(1281, &resources);
        let mut random = crate::audio_parameters::Random(1);
        let mut stamp = crate::animation_epoch::Stamp::default();
        let mut landed = false;
        let mut jumped = false;
        for number in 1..=64 {
            for (index, &id) in owners.iter().enumerate() {
                let commands = if (25..=30).contains(&number) {
                    [0, 0, 0, 0, 0, 2, 0, 0]
                } else {
                    [0; 8]
                };
                let environment = crate::delphi_physical::Environment {
                    number,
                    actor_key: index as u32 + 1,
                    globals: &globals,
                    global_seconds: 1. / 60.,
                    areas: &[],
                    commands,
                    launch_blocked: false,
                    objects: &[],
                };
                let ground = |_| {
                    Ok(crate::physical_ground::Ground {
                        height: 0.,
                        plane: [0., 0., 1., 0.],
                        key: [5, 0],
                    })
                };
                let mut services = crate::actor_animation_phase::Services {
                    contexts: &mut contexts,
                    random: &mut random,
                    stamp: &mut stamp,
                    blending: crate::animation_channels::Blending::Disabled,
                };
                let view = if index == 0 {
                    session.advance_delphi_physical(id, environment, ground, &mut services)
                } else {
                    session.advance_delphi_locomotion(id, environment, ground, &mut services)
                }
                .unwrap();
                let fields = session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap();
                // Terrain contact has no supporting actor identity.
                landed |= fields.position[2] == 0. && view.velocity[2] == 0.;
                jumped |= view.velocity[2] > 1. && fields.position[2] > 0.;
                assert!(
                    session
                        .delphi_events
                        .as_ref()
                        .unwrap()
                        .belongs_to(session.specification(id).unwrap())
                );
                assert!(session.reference_cache_storage(id).unwrap().sampled);
            }
        }
        assert!(landed && jumped);
        let id = owners[0];
        let before = session
            .get(id)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let contexts_before = contexts.active().to_vec();
        let random_before = random;
        assert!(
            session
                .advance_delphi_physical(
                    id,
                    crate::delphi_physical::Environment {
                        number: 65,
                        actor_key: 1,
                        globals: &globals,
                        global_seconds: 1. / 60.,
                        areas: &[],
                        commands: [0; 8],
                        launch_blocked: true,
                        objects: &[],
                    },
                    |_| panic!("Rejected profile must not query terrain"),
                    &mut crate::actor_animation_phase::Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending: crate::animation_channels::Blending::Disabled,
                    }
                )
                .is_err()
        );
        assert_eq!(
            session.get(id).unwrap().seed.storage.data().unwrap(),
            before
        );
        assert!(session.delphi_physical(id).unwrap().failure.is_some());
        assert_eq!(random.0, random_before.0);
        assert_eq!(contexts.active(), contexts_before);
        // A deliberately malformed secondary mode fails after staged physical
        // work. This is an owned quarantine control, not native error parity.
        let malformed = owners[1];
        session
            .residents
            .get_mut(&malformed.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0x5c..0x60]
            .copy_from_slice(&99u32.to_le_bytes());
        let retained = session
            .get(malformed)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let reference_words = |s: &Session| {
            s.reference_cache_storage(malformed)
                .unwrap()
                .matrices
                .iter()
                .flat_map(|m| std::iter::once(m.flags).chain(m.values.map(f32::to_bits)))
                .collect::<Vec<_>>()
        };
        let references_before = reference_words(&session);
        assert!(
            session
                .advance_delphi_locomotion(
                    malformed,
                    crate::delphi_physical::Environment {
                        number: 65,
                        actor_key: 2,
                        globals: &globals,
                        global_seconds: 1. / 60.,
                        areas: &[],
                        commands: [0; 8],
                        launch_blocked: false,
                        objects: &[],
                    },
                    |_| Ok(crate::physical_ground::Ground {
                        height: 0.,
                        plane: [0., 0., 1., 0.],
                        key: [5, 0]
                    }),
                    &mut crate::actor_animation_phase::Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending: crate::animation_channels::Blending::Disabled
                    }
                )
                .is_err()
        );
        assert_eq!(session.delphi_physical(malformed).unwrap().number, 64);
        assert!(
            session
                .delphi_physical(malformed)
                .unwrap()
                .failure
                .is_some()
        );
        assert_eq!(
            session.get(malformed).unwrap().seed.storage.data().unwrap(),
            retained
        );
        assert_eq!(reference_words(&session), references_before);
        assert_eq!(random.0, random_before.0);
        session.clear();
        assert!(session.delphi_events.is_none());
        assert!(session.delphi_physical(id).is_none());
        assert_eq!(session.counts().residents, 0);
    }
}
