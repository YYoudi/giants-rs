//! Explicit primary obligation and post-world normal-pistol composition.
use super::*;
use crate::mecc_subjective::{
    self as subjective, Phase, State, WorldOutput, WorldPost, WorldServices,
};

impl Session {
    fn require_subjective_geometry(&self, state: &State<Id>, number: u64) -> Result<(), String> {
        let source = state.completed_owned_source(number)?;
        let actor = self
            .get(source)
            .ok_or("subjective geometry source absent or foreign")?;
        let cycle = actor
            .mecc_physical_cycle
            .as_ref()
            .ok_or("subjective geometry cycle absent")?;
        if actor.class != 2
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 2
            || cycle.view.number != number
            || cycle.failure.is_some()
            || cycle.subjective_pending.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor
                .world_tick
                .as_ref()
                .is_none_or(|t| t.phase() != crate::actor_world_tick::Phase::Finished)
            || !Arc::ptr_eq(
                state.specification(),
                self.mecc_subjective_specification(source)?,
            )
        {
            return Err("subjective geometry source/frame/resource binding changed".into());
        }
        Ok(())
    }

    /// Update the shared model-position bank from the completed virtual channels.
    /// The force gate and model-slot admission remain explicit caller inputs.
    /// This does not acknowledge visibility, materials, GPU upload or drawing.
    pub fn prepare_mecc_subjective_positions(
        &mut self,
        state: &State<Id>,
        number: u64,
        slot: usize,
        force: bool,
    ) -> Result<crate::actor_position_buffers::Update, String> {
        self.require_subjective_geometry(state, number)?;
        self.position_buffers.update(
            state.specification(),
            slot,
            state.channels(),
            state.header().unwrap().animation_stamp,
            force,
        )
    }

    pub fn mecc_subjective_model_positions(
        &self,
        state: &State<Id>,
        number: u64,
        slot: usize,
    ) -> Result<Option<&[[f32; 3]]>, String> {
        self.require_subjective_geometry(state, number)?;
        self.position_buffers.positions(state.specification(), slot)
    }

    pub fn mecc_subjective_model_cache(
        &self,
        state: &State<Id>,
        number: u64,
        slot: usize,
    ) -> Result<Option<crate::vertex_cache::Cache>, String> {
        self.require_subjective_geometry(state, number)?;
        self.position_buffers.cache(state.specification(), slot)
    }

    pub(crate) fn mecc_camera_phase_input(
        &self,
        source: Id,
        frame: u64,
        inner: bool,
        first_person: bool,
        seconds: f32,
    ) -> Result<crate::mecc_camera_update::Input, String> {
        let actor = self.get(source).ok_or("camera source absent or foreign")?;
        let cycle = actor
            .mecc_physical_cycle
            .as_ref()
            .ok_or("camera source physical cycle absent")?;
        if actor.class != 2
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 2
            || cycle.view.number != frame
            || cycle.failure.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor
                .world_tick
                .as_ref()
                .is_none_or(|t| t.phase() != crate::actor_world_tick::Phase::Finished)
            || if inner {
                cycle.subjective_pending != Some(frame)
            } else {
                cycle.subjective_pending.is_some()
            }
        {
            return Err(
                "camera requires its completed ordinary source and matching world phase".into(),
            );
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("camera source fields absent")?;
        Ok(crate::mecc_camera_update::Input {
            number: 0,
            position: fields.position,
            yaw: fields.angles[0],
            pitch: cycle.view.look_pitch,
            look_height: f32::from_bits(fields.configuration[22]),
            scale: fields.scale,
            skip_floor: fields.object_flags & 0x10 != 0,
            seconds,
            profile: crate::mecc_camera_update::Profile::normal(first_person),
        })
    }
    /// Defer this prepared source's actual fire while retaining fire-enabled
    /// primary/secondary commands. A completed body blocks its next history
    /// pass until the matching post phase has finished or the world is cleared.
    pub fn advance_mecc_subjective_world(
        &mut self,
        state: &mut State<Id>,
        environment: crate::mecc_physical_cycle::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: crate::mecc_primary::Services<'_, '_, '_>,
    ) -> Result<crate::mecc_world::Projection, String> {
        let source = state
            .header()
            .ok_or("subjective primary header absent")?
            .source;
        let number = environment.number;
        state.prepared_for_primary(self, source, number)?;
        if services.fire.is_some() {
            return Err("subjective primary must not also provide immediate fire".into());
        }
        state.primary_started();
        let result = self.advance_mecc_world_offline_route(
            source,
            environment,
            Some(source),
            ground,
            services,
            true,
        );
        match result {
            Ok(projection)
                if projection.outcome == crate::actor_world_tick::Outcome::UpdatedByPrimary
                    && projection.primary.is_some() =>
            {
                self.residents
                    .get_mut(&source.serial)
                    .unwrap()
                    .mecc_physical_cycle
                    .as_mut()
                    .unwrap()
                    .subjective_pending = Some(number);
                state.primary_completed();
                Ok(projection)
            }
            Ok(_) => {
                let error =
                    "subjective primary did not complete its owned world iteration".to_string();
                state.fail(error.clone());
                Err(error)
            }
            Err(error) => {
                state.fail(error.clone());
                Err(error)
            }
        }
    }

    pub fn advance_mecc_subjective_post(
        &mut self,
        state: &mut State<Id>,
        post: WorldPost,
        history: &mut subjective::History,
        services: WorldServices<'_, '_, '_>,
    ) -> Result<WorldOutput, String> {
        self.advance_mecc_subjective_post_inner(state, post, history, services, None)
    }

    /// Select the virtual animation, produce its inner camera, place and publish
    /// references, execute actual local fire, then finish zero-time ANM.
    pub fn advance_mecc_subjective_post_produced(
        &mut self,
        state: &mut State<Id>,
        post: subjective::ProducedWorldPost,
        history: &mut subjective::History,
        services: WorldServices<'_, '_, '_>,
        camera: &mut crate::mecc_world_camera::State<Id>,
        world: &mut impl crate::mecc_camera_update::World,
    ) -> Result<WorldOutput, String> {
        if post.camera_owner != post.controlled {
            return Err("produced subjective camera requires the controlled source owner".into());
        }
        camera.check_inner(self, post.controlled, post.number)?;
        let previous = camera.placement_previous()?;
        let mut producer = crate::mecc_world_camera::Inner {
            state: camera,
            first_person: services.fire.view.first_person,
            seconds: services.global_seconds,
            world,
        };
        self.advance_mecc_subjective_post_inner(
            state,
            WorldPost {
                number: post.number,
                controlled: post.controlled,
                camera_owner: Some(post.camera_owner),
                previous_camera: previous,
                current_camera: previous,
            },
            history,
            services,
            Some(&mut producer),
        )
    }

    fn advance_mecc_subjective_post_inner(
        &mut self,
        state: &mut State<Id>,
        post: WorldPost,
        history: &mut subjective::History,
        services: WorldServices<'_, '_, '_>,
        camera: Option<&mut dyn crate::mecc_world_camera::Producer>,
    ) -> Result<WorldOutput, String> {
        let source = state
            .header()
            .ok_or("subjective post header absent")?
            .source;
        let number = state.frame_number();
        if state.phase() != Phase::PrimaryComplete
            || post.number != number
            || post.controlled != source
            || post.camera_owner.is_some_and(|id| id.ticket != self.ticket)
            || self
                .get(source)
                .and_then(|a| a.mecc_physical_cycle.as_ref())
                .is_none_or(|c| c.subjective_pending != Some(number) || c.failure.is_some())
            || !Arc::ptr_eq(
                state.specification(),
                self.mecc_subjective_specification(source)?,
            )
        {
            return Err(
                "subjective post requires the matching completed primary obligation".into(),
            );
        }
        services.held.require_current(self, source, number)?;
        if !services.global_seconds.is_finite()
            || services.global_seconds < 0.
            || services.areas.iter().any(|a| {
                !a.center.into_iter().all(f32::is_finite)
                    || !a.radius_squared.is_finite()
                    || a.radius_squared < 0.
                    || !a.factor.is_finite()
                    || a.factor < 0.
            })
        {
            return Err("subjective clock requires the actual finite world inputs".into());
        }
        let actor = self.get(source).unwrap();
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("subjective source fields absent")?;
        let seconds = crate::object_time::seconds(
            services.global_seconds,
            Some(crate::object_time::Sample {
                owner: fields.dispatch_context,
                position: fields.position,
            }),
            None,
            services.areas,
        );
        let weapon = self.mecc_selected_weapon(source)?;
        // Weapon-changing input remains outside the admitted primary profile.
        // Consequently the selector's inventory-presence branch is not entered.
        if weapon != 1 || state.header().unwrap().secondary_flags & 0x30 != 0 {
            return Err("subjective fire outside acquired normal-pistol input profile".into());
        }
        let icon = services
            .catalog
            .icons
            .get(&weapon)
            .ok_or("subjective selected icon absent")?;
        let mut family = None;
        for offset in &icon.records {
            let record = services
                .catalog
                .records
                .iter()
                .find(|r| r.offset == *offset)
                .ok_or("subjective icon record absent")?;
            if record.opcode == 0x26 {
                family = Some(
                    *record
                        .args
                        .first()
                        .ok_or("subjective icon family truncated")?,
                );
            }
        }
        let icon_index = family.ok_or("subjective icon animation family undeclared")? as usize;
        if icon_index >= 4 {
            return Err("subjective icon animation family outside acquired profile".into());
        }
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("subjective source payload absent")?;
        let offset = std::array::from_fn(|i| {
            f32::from_le_bytes(body[0x74 + i * 4..0x78 + i * 4].try_into().unwrap())
        });
        let controlled_flags = fields.control_flags;
        let auxiliary_reset = services.globals.scalar(0x8f4).map_err(|e| e.to_string())?;
        if !seconds.is_finite() || !auxiliary_reset.is_finite() {
            return Err("subjective post time/reset outside finite profile".into());
        }
        if post.camera_owner == Some(source) {
            services.fire.preflight_subjective(source, number)?;
            if camera.is_none()
                && (services.fire.view.camera.position.map(f32::to_bits)
                    != post.current_camera.position.map(f32::to_bits)
                    || services.fire.view.camera.angles.map(f32::to_bits)
                        != post.current_camera.angles.map(f32::to_bits))
            {
                return Err(
                    "subjective fire view must use the supplied current inner camera".into(),
                );
            }
        }
        let WorldServices {
            animation,
            fire,
            held,
            ..
        } = services;
        let crate::mecc_animation_phase::Services {
            contexts,
            random,
            stamp,
            blending,
        } = animation;
        let mut adapter = PostAdapter {
            session: self,
            fire,
            held,
            contexts,
            random,
            seconds,
            auxiliary_reset,
            output: None,
            camera,
            camera_output: None,
        };
        let result = state.post(
            subjective::Post {
                controlled: post.controlled,
                camera_owner: post.camera_owner,
                controlled_flags,
                icon_index,
                weapon,
                inventory_present: false,
                previous_camera: post.previous_camera,
                current_camera: post.current_camera,
                offset,
                seconds: seconds as f32,
                blending,
            },
            history,
            stamp,
            &mut adapter,
        );
        match result {
            Ok(animation) => {
                adapter
                    .session
                    .residents
                    .get_mut(&source.serial)
                    .unwrap()
                    .mecc_physical_cycle
                    .as_mut()
                    .unwrap()
                    .subjective_pending = None;
                if animation.is_some() {
                    state.acknowledge_owned_post();
                }
                Ok(WorldOutput {
                    animation,
                    fire: adapter.output,
                    camera: adapter.camera_output,
                })
            }
            Err(error) => {
                adapter
                    .session
                    .residents
                    .get_mut(&source.serial)
                    .unwrap()
                    .mecc_physical_cycle
                    .as_mut()
                    .unwrap()
                    .failure = Some(error.clone());
                Err(error)
            }
        }
    }
}

struct PostAdapter<'a, 's, 'r, 't, 'c> {
    session: &'s mut Session,
    fire: crate::mecc_primary::FireServices<'a, 'r, 't>,
    held: &'a crate::mecc_wield_pose::Producer,
    contexts: &'a mut crate::attached_effect_context::Pool<Id>,
    random: &'a mut crate::audio_parameters::Random,
    seconds: f64,
    auxiliary_reset: f32,
    output: Option<crate::mecc_fire_primary::Output>,
    camera: Option<&'c mut dyn crate::mecc_world_camera::Producer>,
    camera_output: Option<crate::mecc_world_camera::Publication<Id>>,
}
impl subjective::Services<Id> for PostAdapter<'_, '_, '_, '_, '_> {
    fn cameras(
        &mut self,
        source: Id,
        number: u64,
        previous: crate::first_person::Camera,
        current: crate::first_person::Camera,
    ) -> Result<(crate::first_person::Camera, crate::first_person::Camera), String> {
        let Some(producer) = self.camera.as_mut() else {
            return Ok((previous, current));
        };
        let publication = producer.publish(self.session, source, number)?;
        let previous = publication
            .previous()
            .ok_or("inner camera previous globals absent")?;
        let output = publication.output();
        self.fire.view.camera = output.camera;
        self.fire.view.viewport_origin = output.viewport_origin;
        self.fire.view.viewport_forward = output.viewport_forward;
        let current = output.camera;
        self.camera_output = Some(publication);
        Ok((previous, current))
    }
    fn references(
        &mut self,
        source: Id,
        context: &mut crate::actor_references::Owned,
        channels: &crate::animation_channels::Channels,
        epoch: u32,
    ) -> Result<(), String> {
        self.session
            .update_mecc_subjective_references(source, context, channels, epoch)?;
        Ok(())
    }
    fn fire(&mut self, source: Id, number: u64) -> Result<(), String> {
        let actor = self
            .session
            .get(source)
            .ok_or("subjective fire source removed")?;
        let frame = crate::mecc_fire_primary::Frame {
            fields: actor
                .construction_fields
                .as_ref()
                .ok_or("subjective fire fields absent")?
                .clone(),
            payload: actor
                .seed
                .storage
                .data()
                .ok_or("subjective fire payload absent")?
                .to_vec(),
        };
        let view = self.session.mecc_physical_cycle(source)?.clone();
        let mut muzzle = CurrentMuzzle {
            held: self.held,
            number,
            definition: self.fire.factory.definition(),
            other: &mut *self.fire.other,
        };
        let mut fire = crate::mecc_primary::FireServices {
            factory: self.fire.factory,
            resources: self.fire.resources,
            terrain: self.fire.terrain,
            view: self.fire.view,
            other: &mut muzzle,
        };
        self.output = Some(fire.run_subjective(
            self.session,
            crate::mecc_primary::FireFrame {
                frame,
                buttons: u32::from_le_bytes(view.commands[4..8].try_into().unwrap()),
                view: &view,
                seconds: self.seconds,
                auxiliary_reset: self.auxiliary_reset,
            },
            self.contexts,
            self.random,
        )?);
        Ok(())
    }
}
struct CurrentMuzzle<'a> {
    held: &'a crate::mecc_wield_pose::Producer,
    number: u64,
    definition: &'a crate::projectile_definition::Definition,
    other: &'a mut dyn crate::mecc_shot::Services,
}
impl crate::mecc_shot::Services for CurrentMuzzle<'_> {
    fn aim(
        &mut self,
        s: &mut Session,
        id: Id,
        p: [f32; 3],
        range: f32,
    ) -> Result<[f32; 3], String> {
        self.other.aim(s, id, p, range)
    }
    fn muzzle(
        &mut self,
        s: &Session,
        binding: crate::mecc_fire_lock::Binding<Id>,
    ) -> Result<[f32; 3], String> {
        let (actual, point) = self
            .held
            .current_world_point(s, self.number, self.definition)?;
        if actual != binding {
            return Err("subjective muzzle binding changed after held pass".into());
        }
        Ok(point)
    }
    fn effect(
        &mut self,
        s: &mut Session,
        id: Id,
        name: &str,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::attached_effect_context::Creation, String> {
        self.other.effect(s, id, name, random)
    }
    fn sound(
        &mut self,
        s: &mut Session,
        id: Id,
        cue: u32,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.other.sound(s, id, cue)
    }
}
