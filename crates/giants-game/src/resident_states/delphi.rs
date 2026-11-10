//! Delphi-specific owned mutations; ordinary Mecc admission stays unchanged.
use super::*;
use crate::delphi_special::{CopySource, Initialized, State};
enum Entry<'a> {
    Physical,
    Locomotion,
    Controlled {
        manual: crate::delphi_physical::ManualRotation,
        camera: Option<crate::delphi_camera::Context<'a>>,
        post: Option<crate::delphi_post_physical::Context<'a>>,
        reserve: Option<ReserveEntry<'a>>,
        primary_tail: Option<TailEntry<'a>>,
    },
}
struct TailEntry<'a> {
    session_mode: u32,
    peer_ids: &'a [u32],
    local_peer: u32,
    owner_word: Option<u32>,
    proximity: Option<f32>,
}
enum TailProfile<'a> {
    Items(crate::delphi_items::Context<'a>),
    Proximity(crate::delphi_proximity::Context<'a>),
}
impl TailEntry<'_> {
    fn eligible(&self) -> Result<bool, String> {
        if self.session_mode != 4 {
            return Ok(true);
        }
        crate::kabuto_geometry_mode::NotificationContext {
            session_mode: self.session_mode,
            peer_ids: self.peer_ids,
            local_peer: self.local_peer,
            owner_word: self
                .owner_word
                .ok_or("Delphi item authority owner identity absent")?,
        }
        .eligible()
        .map_err(str::to_owned)
    }
}
struct ItemWorld<'a> {
    session: &'a Session,
    owner_context: u32,
    time: crate::object_time::Sample,
    global_seconds: f32,
    areas: &'a [crate::object_time::SlowArea],
    context: &'a TailEntry<'a>,
}
impl crate::delphi_items::World for ItemWorld<'_> {
    fn actor_seconds(&mut self) -> Result<f64, String> {
        Ok(crate::object_time::seconds(
            self.global_seconds,
            Some(self.time),
            None,
            self.areas,
        ))
    }
    fn global_seconds(&mut self) -> Result<f64, String> {
        Ok(crate::object_time::seconds(
            self.global_seconds,
            None,
            None,
            self.areas,
        ))
    }
    fn eligible(&mut self) -> Result<bool, String> {
        self.context.eligible()
    }
    fn action(
        &mut self,
        action: crate::delphi_items::Action,
        state: &mut crate::delphi_items::State,
    ) -> Result<(), String> {
        if action == crate::delphi_items::Action::TargetMovement && !state.movement_ready {
            return Ok(());
        }
        Err("Delphi item action producer unprepared".into())
    }
    fn scan(&mut self, _: &mut crate::delphi_items::State) -> Result<bool, String> {
        for id in self.session.list(0) {
            let actor = self
                .session
                .get(id)
                .ok_or("Delphi item scan has an unowned actor")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("Delphi item scan actor fields absent")?;
            if fields.object_flags & 0x8000 != 0 || actor.seed.storage.state() != 90 {
                continue;
            }
            if fields.dispatch_context != 0 && fields.dispatch_context != self.owner_context {
                continue;
            }
            // The source next requires distance, then held/interaction services.
            // Refuse before inventing a target outcome or clearing the latch.
            return Err(
                "Delphi eligible item target distance/interaction services unprepared".into(),
            );
        }
        Ok(false)
    }
}
struct ProximityWorld<'a> {
    session: &'a Session,
    context: &'a TailEntry<'a>,
    global_seconds: f32,
    areas: &'a [crate::object_time::SlowArea],
}
fn unprepared_proximity<T>() -> Result<T, String> {
    Err("Delphi state102 current interaction provider unprepared".into())
}
impl crate::delphi_proximity::World<Id> for ProximityWorld<'_> {
    fn authority(&mut self) -> Result<bool, String> {
        self.context.eligible()
    }
    fn global_seconds(&mut self) -> Result<f64, String> {
        Ok(crate::object_time::seconds(
            self.global_seconds,
            None,
            None,
            self.areas,
        ))
    }
    fn actors(&mut self) -> Result<Vec<Id>, String> {
        Ok(self.session.list(0))
    }
    fn state(&mut self, actor: Id) -> Result<u32, String> {
        Ok(u32::from(
            self.session
                .get(actor)
                .ok_or("Delphi proximity actor absent or foreign")?
                .seed
                .storage
                .state(),
        ))
    }
    fn context(&mut self, actor: Id) -> Result<u32, String> {
        Ok(self
            .session
            .get(actor)
            .and_then(|v| v.construction_fields.as_ref())
            .ok_or("Delphi proximity actor fields absent")?
            .dispatch_context)
    }
    fn damage_and_limit(&mut self, _: Id) -> Result<(f32, f32), String> {
        unprepared_proximity()
    }
    fn anchor(&mut self, _: Id) -> Result<[f32; 3], String> {
        unprepared_proximity()
    }
    fn network_guard(&mut self) -> Result<bool, String> {
        Ok(self.context.session_mode == 4 && self.context.eligible()?)
    }
    fn notify(&mut self, _: Id, _: &mut u32) -> Result<(), String> {
        unprepared_proximity()
    }
    fn activate(&mut self, _: Id, _: &mut u32) -> Result<(), String> {
        unprepared_proximity()
    }
    fn select(&mut self, _: Id) -> Result<(), String> {
        unprepared_proximity()
    }
    fn alignment_angle(&mut self, _: Id) -> Result<f32, String> {
        unprepared_proximity()
    }
    fn alignment_position(&mut self, _: &mut crate::delphi_proximity::State) -> Result<(), String> {
        unprepared_proximity()
    }
    fn move_owner(&mut self, _: &mut crate::delphi_proximity::State) -> Result<(), String> {
        unprepared_proximity()
    }
}
struct ReserveEntry<'a> {
    recharge_inhibited: bool,
    services: &'a mut dyn crate::delphi_reserve::Services,
}
struct ReserveWorld<'a, 'b> {
    global_seconds: f32,
    actor: crate::object_time::Sample,
    areas: &'a [crate::object_time::SlowArea],
    services: &'b mut dyn crate::delphi_reserve::Services,
}
impl crate::delphi_reserve::Services for ReserveWorld<'_, '_> {
    fn retire(&mut self, present: &mut bool) -> Result<(), String> {
        self.services.retire(present)
    }
    fn splash(&mut self, position: [f32; 3]) -> Result<(), String> {
        self.services.splash(position)
    }
    fn movement(&mut self) -> Result<(), String> {
        self.services.movement()
    }
}
impl crate::delphi_reserve::World for ReserveWorld<'_, '_> {
    fn seconds(&mut self) -> Result<f64, String> {
        Ok(crate::object_time::seconds(
            self.global_seconds,
            Some(self.actor),
            None,
            self.areas,
        ))
    }
}
struct DryResetWorld<'a, 'b> {
    session: &'a mut Session,
    owner: Id,
    stop_sound: &'b mut dyn FnMut(u32) -> Result<(), String>,
}
impl crate::delphi_dry_reset::World for DryResetWorld<'_, '_> {
    type Held = Id;
    fn held(&mut self) -> Result<Option<Id>, String> {
        Ok(self
            .session
            .wield_list()?
            .into_iter()
            .find(|child| {
                child.flags & 0x8000 == 0
                    && child.parent == Some(self.owner)
                    && child.reference == 1
            })
            .map(|child| child.id))
    }
    fn reveal(&mut self, held: Id) -> Result<(), String> {
        self.session
            .residents
            .get_mut(&held.serial)
            .ok_or("Delphi reset held owner absent")?
            .construction_fields
            .as_mut()
            .ok_or("Delphi reset held fields absent")?
            .object_flags &= !0x40000000;
        Ok(())
    }
    fn stop_sound(&mut self, cue: u32) -> Result<(), String> {
        (self.stop_sound)(cue)
    }
}
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
        self.advance_delphi_phases(id, e, ground, services, Entry::Physical)
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
        self.advance_delphi_phases(id, e, ground, services, Entry::Locomotion)
    }
    /// Entered prefix -> manual heading/look -> jump/body alignment/physical/history ->
    /// dry sword secondary -> zero-time ANM. Full primary remains separate.
    pub fn advance_delphi_controlled(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        manual: crate::delphi_physical::ManualRotation,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        self.advance_delphi_phases(
            id,
            e,
            ground,
            services,
            Entry::Controlled {
                manual,
                camera: None,
                post: None,
                reserve: None,
                primary_tail: None,
            },
        )
    }
    /// Entered prefix -> authored camera -> heading/look -> jump/body/physics/
    /// history -> dry sword secondary -> zero-time ANM. Full primary is separate.
    pub fn advance_delphi_camera_controlled(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        manual: crate::delphi_physical::ManualRotation,
        camera: crate::delphi_camera::Context<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        self.advance_delphi_phases(
            id,
            e,
            ground,
            services,
            Entry::Controlled {
                manual,
                camera: Some(camera),
                post: None,
                reserve: None,
                primary_tail: None,
            },
        )
    }
    /// Controlled physical/history -> acquired dry post-physical callback ->
    /// secondary -> zero-time ANM. Later service failures retain the completed
    /// physical/post effects and quarantine the owner. Full primary is separate.
    pub fn advance_delphi_post_physical_controlled(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        manual: crate::delphi_physical::ManualRotation,
        post: crate::delphi_post_physical::Context<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        self.advance_delphi_phases(
            id,
            e,
            ground,
            services,
            Entry::Controlled {
                manual,
                camera: None,
                post: Some(post),
                reserve: None,
                primary_tail: None,
            },
        )
    }
    /// Controlled physical/reset/post -> reserve callback -> secondary/zeroANM.
    /// The level flag and effects are supplied actual world services. This does
    /// not admit active turbo movement, immersion, item inputs or live missions.
    pub fn advance_delphi_reserve_controlled(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        manual: crate::delphi_physical::ManualRotation,
        context: crate::delphi_reserve::Context<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        self.advance_delphi_phases(
            id,
            e,
            ground,
            services,
            Entry::Controlled {
                manual,
                camera: None,
                post: Some(context.post),
                reserve: Some(ReserveEntry {
                    recharge_inhibited: context.recharge_inhibited,
                    services: context.services,
                }),
                primary_tail: None,
            },
        )
    }
    /// Physical/reset/post/reserve, acquired item tail, absent auxiliary route,
    /// then secondary/zero ANM. Required active actions are not acknowledged.
    pub fn advance_delphi_tail_controlled(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        manual: crate::delphi_physical::ManualRotation,
        context: crate::delphi_items::Context<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        self.advance_delphi_tail_profile(
            id,
            e,
            manual,
            TailProfile::Items(context),
            ground,
            services,
        )
    }
    /// Explicit physical/item/proximity/secondary component sequence. State102
    /// interaction providers and the surrounding full primary remain refused.
    pub fn advance_delphi_proximity_controlled(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        manual: crate::delphi_physical::ManualRotation,
        context: crate::delphi_proximity::Context<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        self.advance_delphi_tail_profile(
            id,
            e,
            manual,
            TailProfile::Proximity(context),
            ground,
            services,
        )
    }
    fn advance_delphi_tail_profile(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        manual: crate::delphi_physical::ManualRotation,
        profile: TailProfile<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        let (context, proximity) = match profile {
            TailProfile::Items(items) => (items, None),
            TailProfile::Proximity(context) => {
                (context.items, Some(context.squared_distance_limit))
            }
        };
        self.advance_delphi_phases(
            id,
            e,
            ground,
            services,
            Entry::Controlled {
                manual,
                camera: None,
                post: Some(context.reserve.post),
                reserve: Some(ReserveEntry {
                    recharge_inhibited: context.reserve.recharge_inhibited,
                    services: context.reserve.services,
                }),
                primary_tail: Some(TailEntry {
                    session_mode: context.session_mode,
                    peer_ids: context.peer_ids,
                    local_peer: context.local_peer,
                    owner_word: context.owner_word,
                    proximity,
                }),
            },
        )
    }
    fn advance_delphi_phases(
        &mut self,
        id: Id,
        e: crate::delphi_physical::Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        services: &mut crate::actor_animation_phase::Services<'_>,
        entry: Entry<'_>,
    ) -> Result<crate::delphi_physical::View, String> {
        if self.delphi_target_running {
            return Err("Delphi physical entry during local targeting is unprepared".into());
        }
        let (locomotion, manual, mut camera, mut post, mut reserve, mut primary_tail) = match entry
        {
            Entry::Physical => (false, None, None, None, None, None),
            Entry::Locomotion => (true, None, None, None, None, None),
            Entry::Controlled {
                manual,
                camera,
                post,
                reserve,
                primary_tail,
            } => (true, Some(manual), camera, post, reserve, primary_tail),
        };
        let actor = self
            .get(id)
            .ok_or("Delphi physical owner absent or foreign")?;
        if let Some(target) = &actor.delphi_local_target {
            target.check_physical(e.number)?;
        }
        let mut cycle = actor
            .delphi_physical
            .clone()
            .ok_or("Delphi physical cycle unprepared")?;
        if cycle.resources.local_target_phase && actor.delphi_local_target.is_none() {
            return Err("Delphi physical profile requires its owned local target phase".into());
        }
        if cycle.resources.locomotion.is_some() != locomotion
            || cycle.resources.manual_rotation != manual.is_some()
            || cycle.resources.authored_camera != camera.is_some()
            || cycle.resources.post_physical != post.is_some()
            || cycle.resources.reserve != reserve.is_some()
            || cycle.resources.primary_tail != primary_tail.is_some()
            || cycle.resources.proximity_continuation
                != primary_tail.as_ref().is_some_and(|v| v.proximity.is_some())
        {
            return Err("Delphi phase entry differs from its prepared profile".into());
        }
        let before_cycle = cycle.clone();
        let paused = manual.is_some_and(|m| m.paused);
        let prior_affine = actor.affine_cache.clone();
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
        let commands = e.commands;
        let global_seconds = e.global_seconds;
        let areas = e.areas;
        let keep_completed = post.is_some();
        let mut retained = None;
        if let Some(target) = self
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .delphi_local_target
            .as_mut()
        {
            target.phase = super::DelphiLocalTargetPhase::Consumed;
        }
        let result = cycle
            .advance(&fields, &payload, e, manual, ground, |phase| match phase {
                crate::delphi_physical::Phase::BeforeHeading => {
                    let context = camera
                        .as_mut()
                        .ok_or("Delphi entered camera context absent")?;
                    let manual = manual.ok_or("Delphi entered camera control absent")?;
                    // Prefix history/rate bookkeeping does not change the
                    // channels or prior caches read by this camera getter.
                    self.advance_delphi_camera(
                        id,
                        crate::delphi_camera::Environment {
                            local_owner: manual.local_owner,
                            mode: context.mode,
                            buttons: u32::from_le_bytes(commands[4..8].try_into().unwrap()),
                            aspect: context.aspect,
                            target: context.target,
                        },
                        context.state,
                        pipeline.stamp,
                    )
                }
                crate::delphi_physical::Phase::DryReset {
                    fields,
                    payload,
                    physics_flags,
                    damage,
                } => {
                    let context = post
                        .as_mut()
                        .ok_or("Delphi physical reset context absent")?;
                    let word = crate::delphi_physical::word;
                    let mut state = crate::delphi_dry_reset::State {
                        controls: fields.control_flags,
                        internal: word(payload, 0),
                        second_context: word(payload, 0xf24),
                        timed_word: f32::from_bits(word(payload, 0xf1c)),
                    };
                    let result = crate::delphi_dry_reset::advance(
                        &mut state,
                        crate::delphi_dry_reset::Input {
                            physics_flags,
                            object_flags: fields.object_flags,
                            mode: word(payload, 0x5c) as i32,
                            first_context: word(payload, 0xf20),
                            equipment_context: word(payload, 0xe2c),
                            view_mode: context.view_mode,
                            damage,
                            limit: f32::from_bits(fields.configuration[12]),
                            reset: globals.scalar(0x828).map_err(|e| e.to_string())?,
                        },
                        &mut DryResetWorld {
                            session: self,
                            owner: id,
                            stop_sound: context.stop_sound,
                        },
                    );
                    fields.control_flags = state.controls;
                    for (at, value) in [
                        (0, state.internal),
                        (0xf24, state.second_context),
                        (0xf1c, state.timed_word.to_bits()),
                    ] {
                        payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
                    }
                    result
                }
                crate::delphi_physical::Phase::Animation {
                    fields,
                    view,
                    seconds,
                } => {
                    let pass = pipeline.advance_sample(
                        fields,
                        crate::actor_animation_phase::Sample {
                            model: view.model,
                            affine: view.affine.filter(|_| {
                                !paused
                                    || prior_affine.as_ref().is_some_and(|cache| {
                                        cache.pose == affine_pose_words(fields)
                                    })
                            }),
                            support: view.support,
                            support_part: view.support_part,
                        },
                        seconds,
                    )?;
                    fields.channels = pass.channels;
                    Ok(())
                }
            })
            .and_then(|mut out| {
                let followup = (|| -> Result<(), String> {
                    if let Some(context) = post.as_mut() {
                        let word = crate::delphi_physical::word;
                        let mut state = crate::delphi_post_physical::State {
                            timed_word: f32::from_bits(word(&out.payload, 0xf1c)),
                            controls: out.fields.control_flags,
                            response: out.fields.response_flags,
                            buttons: u32::from_le_bytes(commands[4..8].try_into().unwrap()),
                            internal: word(&out.payload, 0),
                            attack_serial: word(&out.payload, 4),
                            cleanup_context: word(&out.payload, 0xe38),
                        };
                        let result = crate::delphi_post_physical::advance(
                            &mut state,
                            crate::delphi_post_physical::Input {
                                global_seconds,
                                actor: crate::object_time::Sample {
                                    owner: out.fields.dispatch_context,
                                    position: out.fields.position,
                                },
                                areas,
                                local_owner: manual.unwrap().local_owner,
                                paused,
                                inhibited: false,
                                physics_flags: word(&out.payload, 0xd00),
                                spell: word(&out.payload, 0xdb4) as i32,
                                mode: word(&out.payload, 0x5c) as i32,
                                busy_contexts: [
                                    word(&out.payload, 0xf20),
                                    word(&out.payload, 0xf24),
                                    word(&out.payload, 0xe2c),
                                ],
                            },
                            context.world,
                        );
                        out.fields.control_flags = state.controls;
                        out.fields.response_flags = state.response;
                        cycle.view.post_buttons = Some(state.buttons);
                        for (at, value) in [
                            (0xf1c, state.timed_word.to_bits()),
                            (0, state.internal),
                            (4, state.attack_serial),
                            (0xe38, state.cleanup_context),
                        ] {
                            out.payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
                        }
                        result?;
                    }
                    if let Some(context) = reserve.as_mut() {
                        let word = crate::delphi_physical::word;
                        let mut state = crate::delphi_reserve::State {
                            reserve: f32::from_bits(word(&out.payload, 0x28)),
                            delay: f32::from_bits(word(&out.payload, 0x2c)),
                            movement_seconds: f32::from_bits(word(&out.payload, 0x4c)),
                            controls: out.fields.control_flags,
                            internal: word(&out.payload, 0),
                            gravity_mass: f32::from_bits(word(&out.payload, 0xd04)),
                            mass: f32::from_bits(word(&out.payload, 0xd08)),
                            context_present: word(&out.payload, 0xfec) != 0,
                        };
                        let result = crate::delphi_reserve::advance(
                            &mut state,
                            crate::delphi_reserve::Input {
                                physics_flags: word(&out.payload, 0xd00),
                                hit: cycle.view.hit != 0,
                                damage: cycle.view.accumulated_damage,
                                limit: f32::from_bits(out.fields.configuration[12]),
                                linked_actor_present: word(&out.payload, 8) != 0,
                                recharge_inhibited: context.recharge_inhibited,
                                position: out.fields.position,
                            },
                            crate::delphi_reserve::Parameters::from_globals(globals)?,
                            &mut ReserveWorld {
                                global_seconds,
                                actor: crate::object_time::Sample {
                                    owner: out.fields.dispatch_context,
                                    position: out.fields.position,
                                },
                                areas,
                                services: context.services,
                            },
                        );
                        out.fields.control_flags = state.controls;
                        for (at, value) in [
                            (0x28, state.reserve),
                            (0x2c, state.delay),
                            (0x4c, state.movement_seconds),
                            (0xd04, state.gravity_mass),
                            (0xd08, state.mass),
                        ] {
                            out.payload[at..at + 4].copy_from_slice(&value.to_bits().to_le_bytes());
                        }
                        out.payload[..4].copy_from_slice(&state.internal.to_le_bytes());
                        if !state.context_present {
                            out.payload[0xfec..0xff0].fill(0);
                        }
                        result?;
                    }
                    if let Some(context) = primary_tail.as_mut() {
                        let word = crate::delphi_physical::word;
                        let mut state = crate::delphi_items::State {
                            buttons: cycle
                                .view
                                .post_buttons
                                .ok_or("Delphi item tail requires completed post input")?,
                            controls: out.fields.control_flags,
                            physics_flags: word(&out.payload, 0xd00),
                            movement_ready: word(&out.payload, 0xf40) != 0,
                            selected: word(&out.payload, 0xdb0) as i32,
                            items: std::array::from_fn(|i| {
                                word(&out.payload, 0xd3c + i * 12) as i32
                            }),
                            spells: std::array::from_fn(|i| {
                                [
                                    word(&out.payload, 0xd98 + i * 8) as i32,
                                    word(&out.payload, 0xd9c + i * 8) as i32,
                                ]
                            }),
                            elapsed: f32::from_bits(word(&out.payload, 0xfa0)),
                            scan_remaining: f32::from_bits(word(&out.payload, 0xfa8)),
                        };
                        let result = crate::delphi_items::advance(
                            &mut state,
                            globals.scalar(0x1568).map_err(|e| e.to_string())?,
                            &mut ItemWorld {
                                session: self,
                                owner_context: out.fields.dispatch_context,
                                time: crate::object_time::Sample {
                                    owner: out.fields.dispatch_context,
                                    position: out.fields.position,
                                },
                                global_seconds,
                                areas,
                                context,
                            },
                        );
                        out.fields.control_flags = state.controls;
                        cycle.view.post_buttons = Some(state.buttons);
                        for (at, value) in [
                            (0xd00, state.physics_flags),
                            (0xdb0, state.selected as u32),
                            (0xfa0, state.elapsed.to_bits()),
                            (0xfa8, state.scan_remaining.to_bits()),
                        ] {
                            out.payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
                        }
                        result?;
                        if word(&out.payload, 0xcfc) != 0 {
                            return Err("Delphi auxiliary resource callback unprepared".into());
                        }
                        if let Some(squared_distance_limit) = context.proximity {
                            let mut current_commands = commands;
                            current_commands[4..8]
                                .copy_from_slice(&cycle.view.post_buttons.unwrap().to_le_bytes());
                            let mut state = crate::delphi_proximity::State {
                                remaining: f32::from_bits(word(&out.payload, 0xfb0)),
                                controls: out.fields.control_flags,
                                heading: out.fields.angles[0],
                                position: out.fields.position,
                                commands: current_commands,
                            };
                            let result = crate::delphi_proximity::advance(
                                &mut state,
                                crate::delphi_proximity::Input {
                                    owner_context: out.fields.dispatch_context,
                                    owner_word: context.owner_word,
                                    squared_distance_limit,
                                },
                                &mut ProximityWorld {
                                    session: self,
                                    context,
                                    global_seconds,
                                    areas,
                                },
                            );
                            out.fields.control_flags = state.controls;
                            out.fields.angles[0] = state.heading;
                            out.fields.position = state.position;
                            out.payload[0xfb0..0xfb4]
                                .copy_from_slice(&state.remaining.to_bits().to_le_bytes());
                            cycle.view.post_buttons =
                                Some(u32::from_le_bytes(state.commands[4..8].try_into().unwrap()));
                            cycle.view.post_commands = Some(state.commands);
                            result?;
                        }
                    }
                    if let Some(clips) = cycle.resources.locomotion {
                        let word = crate::delphi_physical::word;
                        if [0xdb4, 0xf20, 0xf24, 0xe2c, 0xfd0]
                            .into_iter()
                            .any(|at| word(&out.payload, at) != 0)
                        {
                            return Err(
                                "Delphi secondary equipment/context/effect producer pending".into(),
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
                        for (at, value) in [(0, selection.internal), (0x5c, selection.mode as u32)]
                        {
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
                                affine: view.affine.filter(|_| {
                                    !paused
                                        || prior_affine.as_ref().is_some_and(|cache| {
                                            cache.pose == affine_pose_words(&out.fields)
                                        })
                                }),
                                support: view.support,
                                support_part: view.support_part,
                            },
                            0.,
                        )?;
                        out.fields.channels = pass.channels;
                    }
                    Ok(())
                })();
                if keep_completed {
                    retained = Some(out.clone());
                }
                followup.map(|()| out)
            });
        let failure = result.as_ref().err().cloned();
        let completed = match result {
            Ok(out) => Some(out),
            Err(_) => retained,
        };
        match completed {
            Some(out) => {
                if let Some(error) = &failure {
                    cycle.view.failure = Some(error.clone());
                }

                self.specifications.get_mut(&id.serial).unwrap().0 = pipeline.references;
                self.delphi_events = Some(pipeline.bank);
                *services.contexts = pipeline.contexts;
                *services.random = pipeline.random;
                let actor = self.residents.get_mut(&id.serial).unwrap();
                actor.channels = Some(out.fields.channels);
                actor.mecc_animation_stamp = pipeline.last_stamp;
                actor.affine_cache = if paused {
                    prior_affine
                } else {
                    cycle.view.affine.map(|value| OwnedAffine {
                        value,
                        pose: affine_pose_words(&out.fields),
                        external_orientation: false,
                    })
                };
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
                match failure {
                    Some(error) => Err(error),
                    None => Ok(view),
                }
            }
            None => {
                let error = failure.unwrap();
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
        for profile in 0..7 {
            let id = factory
                .construct_player_fields(
                    &mut session,
                    crate::delphi_player::ColdFrame {
                        position: [0., 0., 10.],
                        current: [0., 0., 10.],
                        previous: [0., 0., 10.],
                        angles: [0., 0., if profile >= 2 { 30. } else { 0. }],
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
            let prepared = if profile >= 2 {
                prepared.with_manual_rotation().unwrap()
            } else {
                prepared
            };
            let prepared = if profile == 3 {
                prepared.with_authored_camera().unwrap()
            } else {
                prepared
            };
            let prepared = if profile >= 4 {
                prepared.with_post_physical().unwrap()
            } else {
                prepared
            };
            let prepared = if profile >= 5 {
                prepared.with_reserve().unwrap()
            } else {
                prepared
            };
            let prepared = if profile == 6 {
                prepared.with_primary_tail().unwrap()
            } else {
                prepared
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
        let mut entered_camera = crate::delphi_camera::State::default();
        let mut post_world = crate::delphi_post_physical::World::default();
        let mut stop_cues = Vec::new();
        let mut ordinary_camera = crate::mecc_camera_update::State::default();
        let mut ordinary_view = crate::delphi_ordinary_view::State::default();
        let executable = std::env::var_os("GIANTS_DELPHI_EXECUTABLE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("Giants.exe"));
        let camera_profiles = crate::delphi_camera::OrdinaryProfiles::read(&executable).unwrap();
        let view_controls = |number| crate::delphi_ordinary_view::Controls {
            seconds: 1. / 60.,
            transition_input: number % 16 < 10,
            scaled_input: number % 32 >= 16,
            input_scale: 0.8,
            projection: crate::camera_projection::Parameters {
                aspect: None,
                viewport: [1920., 1080.],
                near: 1.,
                far: 10_000.,
                limit_far: true,
                depth_reference: [0.; 3],
                depth_axis: [1., 0., 0.],
            },
        };
        #[derive(Default)]
        struct CameraWorld(u32);
        impl crate::mecc_camera_update::World for CameraWorld {
            fn height(&mut self, _: [f32; 2]) -> Result<f32, String> {
                self.0 += 1;
                Ok(0.)
            }
            fn clip(&mut self, _: &mut [f32; 3], _: [f32; 3]) -> Result<(), String> {
                self.0 += 1;
                Ok(())
            }
        }
        let mut camera_world = CameraWorld::default();
        // Supplied synthetic profile; this test does not produce world profiles.
        let camera_profile = crate::mecc_camera_update::Profile {
            distance: 20.,
            height: 5.,
            side: 1.,
            along: 0.,
            vertical_fov: 70.,
        };
        let mut landed = false;
        let mut jumped = false;
        for number in 1..=64 {
            for (index, &id) in owners.iter().enumerate() {
                let mut commands = if (25..=30).contains(&number) {
                    [0, 0, 0, 0, 0, 2, 0, 0]
                } else {
                    [0; 8]
                };
                let paused = index >= 2 && (35..=40).contains(&number);
                let prior_position = session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .position;
                if index >= 2 {
                    commands[0] = 32;
                    commands[1] = 16;
                }
                if index == 3 && (4..=18).contains(&number) {
                    commands[5] |= 0x80;
                }
                if index == 6 && number % 16 == 5 {
                    commands[4] |= 0x10;
                }
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
                    assert!(!paused, "Paused physical phase must not query terrain");
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
                } else if index == 1 {
                    session.advance_delphi_locomotion(id, environment, ground, &mut services)
                } else if index == 2 {
                    session.advance_delphi_controlled(
                        id,
                        environment,
                        crate::delphi_physical::ManualRotation {
                            sensitivity: 1.,
                            local_owner: true,
                            paused,
                        },
                        ground,
                        &mut services,
                    )
                } else if index == 3 {
                    session.advance_delphi_camera_controlled(
                        id,
                        environment,
                        crate::delphi_physical::ManualRotation {
                            sensitivity: 1.,
                            local_owner: true,
                            paused,
                        },
                        crate::delphi_camera::Context {
                            state: &mut entered_camera,
                            mode: -1,
                            aspect: 16. / 9.,
                            target: None,
                        },
                        ground,
                        &mut services,
                    )
                } else if index == 4 {
                    session.advance_delphi_post_physical_controlled(
                        id,
                        environment,
                        crate::delphi_physical::ManualRotation {
                            sensitivity: 1.,
                            local_owner: true,
                            paused,
                        },
                        crate::delphi_post_physical::Context {
                            world: &mut post_world,
                            view_mode: 0,
                            stop_sound: &mut |cue| {
                                stop_cues.push(cue);
                                Ok(())
                            },
                        },
                        ground,
                        &mut services,
                    )
                } else if index == 5 {
                    session.advance_delphi_reserve_controlled(
                        id,
                        environment,
                        crate::delphi_physical::ManualRotation {
                            sensitivity: 1.,
                            local_owner: true,
                            paused,
                        },
                        crate::delphi_reserve::Context {
                            post: crate::delphi_post_physical::Context {
                                world: &mut post_world,
                                view_mode: 0,
                                stop_sound: &mut |cue| {
                                    stop_cues.push(cue);
                                    Ok(())
                                },
                            },
                            recharge_inhibited: (10..=14).contains(&number),
                            services: &mut crate::delphi_reserve::EmptyContext,
                        },
                        ground,
                        &mut services,
                    )
                } else {
                    session.advance_delphi_tail_controlled(
                        id,
                        environment,
                        crate::delphi_physical::ManualRotation {
                            sensitivity: 1.,
                            local_owner: true,
                            paused,
                        },
                        crate::delphi_items::Context {
                            reserve: crate::delphi_reserve::Context {
                                post: crate::delphi_post_physical::Context {
                                    world: &mut post_world,
                                    view_mode: 0,
                                    stop_sound: &mut |cue| {
                                        stop_cues.push(cue);
                                        Ok(())
                                    },
                                },
                                recharge_inhibited: false,
                                services: &mut crate::delphi_reserve::EmptyContext,
                            },
                            session_mode: 0,
                            peer_ids: &[],
                            local_peer: 0,
                            owner_word: None,
                        },
                        ground,
                        &mut services,
                    )
                }
                .unwrap();
                if index == 4 {
                    assert!(view.post_buttons.is_some());
                }
                if index == 6 {
                    assert!(session.delphi_target_ray(id, number, 1e-4).is_err());
                    let held = session.advance_delphi_held_pose(id, number).unwrap();
                    let ray = session.delphi_target_ray(id, number, 1e-4).unwrap();
                    assert!(
                        ray.origin
                            .into_iter()
                            .chain(ray.direction)
                            .all(f32::is_finite)
                    );
                    assert!(session.advance_delphi_held_pose(id, number).is_err());
                    assert!(session.delphi_target_ray(id, number + 1, 1e-4).is_err());
                    let storage = session.reference_cache_storage(held).unwrap();
                    assert!(!storage.sampled);
                    assert_eq!(storage.matrices.len(), 1);
                    assert_eq!(storage.matrices[0].flags, 0);
                    assert_eq!(storage.matrices[0].values.map(f32::to_bits), [0; 16]);
                }

                let fields = session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap();
                if index >= 2 {
                    assert!(view.look > 0.);
                    if number == 1 {
                        assert!(fields.angles[2] > 0. && fields.angles[2] < 30.);
                    } else if number >= 20 {
                        assert_eq!(fields.angles[2], 0.);
                    }
                    if paused {
                        assert_eq!(fields.position, prior_position);
                    }
                } else {
                    assert_eq!(view.look, 0.);
                    assert_eq!(fields.angles[0], 0.);
                }
                // Terrain contact has no supporting actor identity.
                landed |= fields.position[2] == 0. && view.velocity[2] == 0.;
                jumped |= view.velocity[2] > 1. && fields.position[2] > 0.;
                if index == 2 {
                    let output = session
                        .advance_delphi_ordinary_camera(
                            id,
                            number,
                            1. / 60.,
                            camera_profile,
                            &mut ordinary_camera,
                            &mut camera_world,
                        )
                        .unwrap();
                    assert_eq!(output.camera.angles, [fields.angles[0], view.look, 0.]);
                    assert!(output.view.into_iter().all(f32::is_finite));
                    let output = session
                        .advance_delphi_ordinary_view(
                            id,
                            number,
                            view_controls(number),
                            camera_profiles,
                            &mut ordinary_view,
                            &mut camera_world,
                        )
                        .unwrap();
                    assert_eq!(output.pose.camera.angles, [fields.angles[0], view.look, 0.]);
                    assert!(output.projection.matrix.into_iter().all(f32::is_finite));
                    let transition = ordinary_view.transition();
                    let calls = camera_world.0;
                    assert!(
                        session
                            .advance_delphi_ordinary_camera(
                                id,
                                number + 1,
                                1. / 60.,
                                camera_profile,
                                &mut ordinary_camera,
                                &mut camera_world,
                            )
                            .is_err()
                    );
                    assert_eq!(camera_world.0, calls);
                    assert!(ordinary_camera.failure().is_none());
                    assert!(
                        session
                            .advance_delphi_ordinary_view(
                                id,
                                number + 1,
                                view_controls(number + 1),
                                camera_profiles,
                                &mut ordinary_view,
                                &mut camera_world,
                            )
                            .is_err()
                    );
                    assert_eq!(camera_world.0, calls);
                    assert_eq!(ordinary_view.transition(), transition);
                    assert!(ordinary_view.failure().is_none());
                }
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
        let held = session
            .wield_list()
            .unwrap()
            .into_iter()
            .find(|c| c.parent == Some(owners[6]) && c.reference == 1 && c.flags & 0x8000 == 0)
            .unwrap()
            .id;
        let held_position = session
            .get(held)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        session
            .residents
            .get_mut(&held.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position[0] += 1.;
        assert!(session.delphi_target_ray(owners[6], 64, 1e-4).is_err());
        session
            .residents
            .get_mut(&held.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .position = held_position;
        assert!(session.delphi_target_ray(owners[6], 64, 1e-4).is_ok());
        session
            .update_current_references_at(held, stamp.0 + 1)
            .unwrap();
        assert!(session.delphi_target_ray(owners[6], 64, 1e-4).is_err());
        let tail_owner = owners[6];
        let payload = session
            .get(tail_owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap();
        let elapsed = f32::from_bits(crate::delphi_physical::word(payload, 0xfa0));
        assert!(elapsed > 0.);
        let scan = f32::from_bits(crate::delphi_physical::word(payload, 0xfa8));
        assert!((0. ..=0.5).contains(&scan));
        session
            .residents
            .get_mut(&tail_owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0xcfc..0xd00]
            .copy_from_slice(&1u32.to_le_bytes());
        let stamp_before = stamp.0;
        let tail_environment = |number| crate::delphi_physical::Environment {
            number,
            actor_key: 7,
            globals: &globals,
            global_seconds: 1. / 60.,
            areas: &[],
            commands: [0; 8],
            launch_blocked: false,
            objects: &[],
        };
        let mut tail_retained = None;
        for number in [65, 66] {
            let result = session.advance_delphi_tail_controlled(
                tail_owner,
                tail_environment(number),
                crate::delphi_physical::ManualRotation {
                    sensitivity: 1.,
                    local_owner: true,
                    paused: true,
                },
                crate::delphi_items::Context {
                    reserve: crate::delphi_reserve::Context {
                        post: crate::delphi_post_physical::Context {
                            world: &mut post_world,
                            view_mode: 0,
                            stop_sound: &mut |_| panic!("Paused tail cannot stop sound"),
                        },
                        recharge_inhibited: false,
                        services: &mut crate::delphi_reserve::EmptyContext,
                    },
                    session_mode: 0,
                    peer_ids: &[],
                    local_peer: 0,
                    owner_word: None,
                },
                |_| panic!("Paused tail cannot query terrain"),
                &mut crate::actor_animation_phase::Services {
                    contexts: &mut contexts,
                    random: &mut random,
                    stamp: &mut stamp,
                    blending: crate::animation_channels::Blending::Disabled,
                },
            );
            assert!(result.is_err());
            if number == 65 {
                assert!(result.unwrap_err().contains("auxiliary"));
            }
            let p = session
                .get(tail_owner)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap();
            assert_eq!(
                crate::delphi_physical::word(p, 0xfa0),
                ((f64::from(elapsed) + f64::from(1_f32 / 60.)) as f32).to_bits()
            );
            assert_eq!(session.delphi_physical(tail_owner).unwrap().number, 65);
            if let Some((ref bytes, epoch)) = tail_retained {
                assert_eq!(p, bytes);
                assert_eq!(stamp.0, epoch);
            } else {
                tail_retained = Some((p.to_vec(), stamp.0));
            }
        }
        assert!(stamp.0 > stamp_before);
        assert!(session.advance_delphi_held_pose(owners[6], 65).is_err());
        assert!(session.delphi_target_ray(owners[6], 64, 1e-4).is_err());
        let reserve_owner = owners[5];
        let payload = session
            .get(reserve_owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap();
        let old_delay = f32::from_bits(crate::delphi_physical::word(payload, 0x2c));
        assert!(old_delay < 0.);
        assert!(f32::from_bits(crate::delphi_physical::word(payload, 0x28)) > 0.);
        let reserve_environment = |number| crate::delphi_physical::Environment {
            number,
            actor_key: 6,
            globals: &globals,
            global_seconds: 1. / 60.,
            areas: &[],
            commands: [0; 8],
            launch_blocked: false,
            objects: &[],
        };
        let reserve_manual = crate::delphi_physical::ManualRotation {
            sensitivity: 1.,
            local_owner: true,
            paused: true,
        };
        let stamp_before = stamp.0;
        post_world.local_word = 91;
        assert!(
            session
                .advance_delphi_post_physical_controlled(
                    reserve_owner,
                    reserve_environment(65),
                    reserve_manual,
                    crate::delphi_post_physical::Context {
                        world: &mut post_world,
                        view_mode: 0,
                        stop_sound: &mut |_| panic!("Wrong profile cannot stop sound")
                    },
                    |_| panic!("Wrong profile cannot query terrain"),
                    &mut crate::actor_animation_phase::Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending: crate::animation_channels::Blending::Disabled
                    },
                )
                .is_err()
        );
        assert_eq!((post_world.local_word, stamp.0), (91, stamp_before));
        assert!(
            session
                .delphi_physical(reserve_owner)
                .unwrap()
                .failure
                .is_none()
        );
        session
            .residents
            .get_mut(&reserve_owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0xdb4..0xdb8]
            .copy_from_slice(&1_u32.to_le_bytes());
        assert!(
            session
                .advance_delphi_reserve_controlled(
                    reserve_owner,
                    reserve_environment(65),
                    reserve_manual,
                    crate::delphi_reserve::Context {
                        post: crate::delphi_post_physical::Context {
                            world: &mut post_world,
                            view_mode: 0,
                            stop_sound: &mut |_| panic!("Paused reset cannot stop sound")
                        },
                        recharge_inhibited: true,
                        services: &mut crate::delphi_reserve::EmptyContext,
                    },
                    |_| panic!("Paused physical cannot query terrain"),
                    &mut crate::actor_animation_phase::Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending: crate::animation_channels::Blending::Disabled
                    },
                )
                .unwrap_err()
                .contains("secondary")
        );
        let payload = session
            .get(reserve_owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap();
        assert_eq!(crate::delphi_physical::word(payload, 0x28), 0);
        assert_eq!(
            crate::delphi_physical::word(payload, 0x2c),
            ((f64::from(old_delay) - f64::from(1_f32 / 60.)) as f32).to_bits()
        );
        let retained = payload.to_vec();
        let retained_stamp = stamp.0;
        assert_eq!(session.delphi_physical(reserve_owner).unwrap().number, 65);
        assert!(
            session
                .delphi_physical(reserve_owner)
                .unwrap()
                .failure
                .is_some()
        );
        post_world.local_word = 92;
        assert!(
            session
                .advance_delphi_reserve_controlled(
                    reserve_owner,
                    reserve_environment(66),
                    reserve_manual,
                    crate::delphi_reserve::Context {
                        post: crate::delphi_post_physical::Context {
                            world: &mut post_world,
                            view_mode: 0,
                            stop_sound: &mut |_| panic!("Failed cycle cannot stop sound")
                        },
                        recharge_inhibited: false,
                        services: &mut crate::delphi_reserve::EmptyContext
                    },
                    |_| panic!("Failed cycle cannot query terrain"),
                    &mut crate::actor_animation_phase::Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending: crate::animation_channels::Blending::Disabled
                    },
                )
                .is_err()
        );
        assert_eq!((post_world.local_word, stamp.0), (92, retained_stamp));
        assert_eq!(
            session
                .get(reserve_owner)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap(),
            retained
        );
        assert!(!stop_cues.is_empty() && stop_cues.iter().all(|&cue| cue == 0x34c));
        let post_owner = owners[4];
        let environment = |number| crate::delphi_physical::Environment {
            number,
            actor_key: 5,
            globals: &globals,
            global_seconds: 1. / 60.,
            areas: &[],
            commands: [0, 0, 0, 0, 0, 0, 0, 2],
            launch_blocked: false,
            objects: &[],
        };
        let manual = crate::delphi_physical::ManualRotation {
            sensitivity: 1.,
            local_owner: true,
            paused: true,
        };
        let stamp_before = stamp.0;
        assert!(
            session
                .advance_delphi_controlled(
                    post_owner,
                    environment(65),
                    manual,
                    |_| panic!("Wrong frozen entry must not query terrain"),
                    &mut crate::actor_animation_phase::Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending: crate::animation_channels::Blending::Disabled
                    },
                )
                .is_err()
        );
        assert_eq!(stamp.0, stamp_before);
        assert!(
            session
                .delphi_physical(post_owner)
                .unwrap()
                .failure
                .is_none()
        );
        let payload = session
            .residents
            .get_mut(&post_owner.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap();
        let old_timed = f32::from_bits(crate::delphi_physical::word(payload, 0xf1c));
        payload[0xdb4..0xdb8].copy_from_slice(&1_u32.to_le_bytes());
        post_world.local_word = 77;
        assert!(
            session
                .advance_delphi_post_physical_controlled(
                    post_owner,
                    environment(65),
                    manual,
                    crate::delphi_post_physical::Context {
                        world: &mut post_world,
                        view_mode: 0,
                        stop_sound: &mut |_| panic!("Paused physical must not stop sound")
                    },
                    |_| panic!("Paused physical must not query terrain"),
                    &mut crate::actor_animation_phase::Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending: crate::animation_channels::Blending::Disabled
                    },
                )
                .unwrap_err()
                .contains("secondary")
        );
        let view = session.delphi_physical(post_owner).unwrap();
        assert_eq!(view.number, 65);
        assert!(view.failure.is_some());
        assert_eq!(view.post_buttons, Some(0));
        assert_eq!(post_world.local_word, 0);
        assert!(stamp.0 > stamp_before);
        let payload = session
            .get(post_owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap();
        assert_eq!(
            crate::delphi_physical::word(payload, 0xf1c),
            ((f64::from(old_timed) - f64::from(1_f32 / 60.)) as f32).to_bits()
        );
        let retained_payload = payload.to_vec();
        let retained_stamp = stamp.0;
        post_world.local_word = 88;
        assert!(
            session
                .advance_delphi_post_physical_controlled(
                    post_owner,
                    environment(66),
                    manual,
                    crate::delphi_post_physical::Context {
                        world: &mut post_world,
                        view_mode: 0,
                        stop_sound: &mut |_| panic!("Failed cycle must not stop sound")
                    },
                    |_| panic!("Failed cycle must not query terrain"),
                    &mut crate::actor_animation_phase::Services {
                        contexts: &mut contexts,
                        random: &mut random,
                        stamp: &mut stamp,
                        blending: crate::animation_channels::Blending::Disabled
                    },
                )
                .is_err()
        );
        assert_eq!(post_world.local_word, 88);
        assert_eq!(stamp.0, retained_stamp);
        assert_eq!(
            session
                .get(post_owner)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap(),
            retained_payload
        );
        let valid_owner = owners[2];
        let fields = session
            .get(valid_owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let original_kind = fields.configuration[20];
        let original_flags = fields.object_flags;
        let calls = camera_world.0;
        let transition = ordinary_view.transition();
        for (kind, flags) in [
            (4, original_flags),
            (original_kind, original_flags | 0x8000),
        ] {
            let fields = session
                .residents
                .get_mut(&valid_owner.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap();
            fields.configuration[20] = kind;
            fields.object_flags = flags;
            let error = session
                .advance_delphi_ordinary_view(
                    valid_owner,
                    64,
                    view_controls(64),
                    camera_profiles,
                    &mut ordinary_view,
                    &mut camera_world,
                )
                .unwrap_err();
            assert!(error.contains("selected kind/profile"));
            assert_eq!(ordinary_view.transition(), transition);
            assert!(ordinary_view.failure().is_none());
            assert_eq!(camera_world.0, calls);
        }
        let fields = session
            .residents
            .get_mut(&valid_owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        fields.configuration[20] = original_kind;
        fields.object_flags = original_flags;
        // The standalone camera callback consumes real owned pose/reference
        // storage. Its mode/target inputs remain supplied world boundaries.
        let camera_owner = owners[0];
        let foreign = Id {
            ticket: 1252,
            serial: camera_owner.serial,
        };
        let calls = camera_world.0;
        for invalid in [foreign, camera_owner] {
            assert!(
                session
                    .advance_delphi_ordinary_view(
                        invalid,
                        65,
                        view_controls(65),
                        camera_profiles,
                        &mut ordinary_view,
                        &mut camera_world,
                    )
                    .is_err()
            );
            assert!(
                session
                    .advance_delphi_ordinary_camera(
                        invalid,
                        64,
                        1. / 60.,
                        camera_profile,
                        &mut ordinary_camera,
                        &mut camera_world,
                    )
                    .is_err()
            );
        }
        assert_eq!(camera_world.0, calls);
        let mut camera = crate::delphi_camera::State::default();
        let request =
            |local_owner, mode, buttons, aspect, target| crate::delphi_camera::Environment {
                local_owner,
                mode,
                buttons,
                aspect,
                target,
            };
        let before_stamp = stamp.0;
        session
            .advance_delphi_camera(
                camera_owner,
                request(true, -1, 0x8000, 16. / 9., None),
                &mut camera,
                &mut stamp,
            )
            .unwrap();
        assert!(camera.active_view().unwrap().is_some());
        assert!(camera.vertical_fov.is_some_and(|angle| angle > 0.));
        // This actor still uses animation1; its current reference buffer is
        // consumed without advancing the fallback hierarchy epoch.
        assert_eq!(stamp.0, before_stamp);
        let view_words = |camera: &crate::delphi_camera::State| {
            camera.retained_view.map(|view| {
                std::iter::once(view.flags)
                    .chain(view.values.map(f32::to_bits))
                    .collect::<Vec<_>>()
            })
        };
        let retained = view_words(&camera);
        let retained_fov = camera.vertical_fov;
        for (local, mode) in [(false, -1), (true, 0)] {
            session
                .advance_delphi_camera(
                    camera_owner,
                    request(local, mode, 0x8000, f32::NAN, Some(foreign)),
                    &mut camera,
                    &mut stamp,
                )
                .unwrap();
            assert_eq!(view_words(&camera), retained);
            assert_eq!(camera.vertical_fov, retained_fov);
            assert_eq!(stamp.0, before_stamp);
        }
        session
            .advance_delphi_camera(
                camera_owner,
                request(true, -1, 0, f32::NAN, Some(foreign)),
                &mut camera,
                &mut stamp,
            )
            .unwrap();
        assert!(camera.active_view().unwrap().is_none());
        assert!(camera.vertical_fov.is_none());
        assert_eq!(view_words(&camera), retained);
        // A supplied self-target uses hand reference0, not a camera node; the
        // unused lens/aspect is not read or replaced by a default camera.
        session
            .advance_delphi_camera(
                camera_owner,
                request(true, -1, 0x8000, f32::NAN, Some(camera_owner)),
                &mut camera,
                &mut stamp,
            )
            .unwrap();
        assert!(camera.active_view().unwrap().is_some());
        assert!(camera.vertical_fov.is_none());
        let retained = view_words(&camera);
        assert!(
            session
                .advance_delphi_camera(
                    camera_owner,
                    request(true, -1, 0x8000, 1., Some(foreign)),
                    &mut camera,
                    &mut stamp
                )
                .is_err()
        );
        assert_eq!(view_words(&camera), retained);
        assert!(camera.active_view().is_err());
        assert!(
            session
                .advance_delphi_camera(
                    camera_owner,
                    request(true, -1, 0, 1., None),
                    &mut camera,
                    &mut stamp
                )
                .is_err()
        );
        assert_eq!(stamp.0, before_stamp);
        assert!(
            session
                .sample_reference_camera(camera_owner, 255, usize::MAX, f32::NAN, &mut stamp)
                .unwrap()
                .is_none()
        );
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
        let controlled = owners[2];
        let camera_owner = owners[3];
        let refused_environment = |actor_key| crate::delphi_physical::Environment {
            number: 65,
            actor_key,
            globals: &globals,
            global_seconds: 1. / 60.,
            areas: &[],
            commands: [0, 0, 0, 0, 0, 0x80, 0, 0],
            launch_blocked: false,
            objects: &[],
        };
        let mut refused_camera = crate::delphi_camera::State::default();
        let stamp_before_refusal = stamp.0;
        {
            let mut services = crate::actor_animation_phase::Services {
                contexts: &mut contexts,
                random: &mut random,
                stamp: &mut stamp,
                blending: crate::animation_channels::Blending::Disabled,
            };
            assert!(
                session
                    .advance_delphi_camera_controlled(
                        controlled,
                        refused_environment(3),
                        crate::delphi_physical::ManualRotation {
                            sensitivity: 1.,
                            local_owner: true,
                            paused: false
                        },
                        crate::delphi_camera::Context {
                            state: &mut refused_camera,
                            mode: -1,
                            aspect: 1.,
                            target: None
                        },
                        |_| panic!("Unprepared camera profile must not query terrain"),
                        &mut services
                    )
                    .is_err()
            );
            assert!(
                session
                    .advance_delphi_controlled(
                        camera_owner,
                        refused_environment(4),
                        crate::delphi_physical::ManualRotation {
                            sensitivity: 1.,
                            local_owner: true,
                            paused: false
                        },
                        |_| panic!("Camera profile requires its camera entry"),
                        &mut services
                    )
                    .is_err()
            );
        }
        assert_eq!(stamp.0, stamp_before_refusal);
        assert!(refused_camera.retained_view.is_none() && refused_camera.failure.is_none());
        assert!(
            session
                .delphi_physical(controlled)
                .unwrap()
                .failure
                .is_none()
        );
        assert!(
            session
                .delphi_physical(camera_owner)
                .unwrap()
                .failure
                .is_none()
        );
        let rotation_before = session.delphi_physical(controlled).unwrap().clone();
        let fields_before = session
            .get(controlled)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .clone();
        let payload_before = session
            .get(controlled)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let environment = || crate::delphi_physical::Environment {
            number: 65,
            actor_key: 3,
            globals: &globals,
            global_seconds: 1. / 60.,
            areas: &[],
            commands: [32, 16, 0, 0, 0, 0, 0, 0],
            launch_blocked: false,
            objects: &[],
        };
        let mut services = crate::actor_animation_phase::Services {
            contexts: &mut contexts,
            random: &mut random,
            stamp: &mut stamp,
            blending: crate::animation_channels::Blending::Disabled,
        };
        assert!(
            session
                .advance_delphi_locomotion(
                    controlled,
                    environment(),
                    |_| panic!("Wrong entry must not query terrain"),
                    &mut services,
                )
                .is_err()
        );
        assert!(
            session
                .delphi_physical(controlled)
                .unwrap()
                .failure
                .is_none()
        );
        assert!(
            session
                .advance_delphi_controlled(
                    controlled,
                    environment(),
                    crate::delphi_physical::ManualRotation {
                        sensitivity: f32::NAN,
                        local_owner: true,
                        paused: false
                    },
                    |_| panic!("Invalid rotation must not query terrain"),
                    &mut services,
                )
                .is_err()
        );
        let retained = session.delphi_physical(controlled).unwrap();
        assert!(retained.failure.is_some());
        assert_eq!(retained.number, rotation_before.number);
        assert_eq!(retained.look, rotation_before.look);
        assert_eq!(retained.model, rotation_before.model);
        assert_eq!(
            session
                .get(controlled)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .angles,
            fields_before.angles
        );
        assert_eq!(
            session
                .get(controlled)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap(),
            payload_before
        );
        assert_eq!(random.0, random_before.0);
        // Camera publication completes before a subsequent invalid rotation.
        // The owned frame is quarantined; replay cannot consume another epoch.
        let camera_owner = owners[3];
        let payload_before = session
            .get(camera_owner)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        assert!(entered_camera.active_view().unwrap().is_none());
        {
            let mut services = crate::actor_animation_phase::Services {
                contexts: &mut contexts,
                random: &mut random,
                stamp: &mut stamp,
                blending: crate::animation_channels::Blending::Disabled,
            };
            assert!(
                session
                    .advance_delphi_camera_controlled(
                        camera_owner,
                        refused_environment(4),
                        crate::delphi_physical::ManualRotation {
                            sensitivity: f32::NAN,
                            local_owner: true,
                            paused: false
                        },
                        crate::delphi_camera::Context {
                            state: &mut entered_camera,
                            mode: -1,
                            aspect: 16. / 9.,
                            target: None
                        },
                        |_| panic!("Rotation failure precedes terrain"),
                        &mut services
                    )
                    .is_err()
            );
        }
        assert!(entered_camera.active_view().unwrap().is_some());
        assert!(entered_camera.failure.is_none());
        assert_eq!(session.delphi_physical(camera_owner).unwrap().number, 64);
        assert!(
            session
                .delphi_physical(camera_owner)
                .unwrap()
                .failure
                .is_some()
        );
        assert_eq!(
            session
                .get(camera_owner)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap(),
            payload_before
        );
        assert_eq!(random.0, random_before.0);
        let stamp_after_failure = stamp.0;
        let retained = view_words(&entered_camera);
        {
            let mut services = crate::actor_animation_phase::Services {
                contexts: &mut contexts,
                random: &mut random,
                stamp: &mut stamp,
                blending: crate::animation_channels::Blending::Disabled,
            };
            assert!(
                session
                    .advance_delphi_camera_controlled(
                        camera_owner,
                        refused_environment(4),
                        crate::delphi_physical::ManualRotation {
                            sensitivity: 1.,
                            local_owner: true,
                            paused: false
                        },
                        crate::delphi_camera::Context {
                            state: &mut entered_camera,
                            mode: -1,
                            aspect: 1.,
                            target: None
                        },
                        |_| panic!("Quarantined frame must not query terrain"),
                        &mut services
                    )
                    .is_err()
            );
        }
        assert_eq!(stamp.0, stamp_after_failure);
        assert_eq!(view_words(&entered_camera), retained);
        session.clear();
        assert!(session.delphi_events.is_none());
        assert!(session.delphi_physical(id).is_none());
        assert_eq!(session.counts().residents, 0);
    }
}
