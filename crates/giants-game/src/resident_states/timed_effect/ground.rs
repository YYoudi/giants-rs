//! Flagged physical-ground stage of the actual effect callback, before its
//! affine writer and gravity. Mesh/platform finalization remains a later owner.
use super::*;
use crate::physical_ground;

impl Session {
    pub(crate) fn timed_effect_needs_ground(&self, id: Id) -> bool {
        self.get(id)
            .and_then(|a| a.timed_effect.as_ref())
            .is_some_and(|m| m.runtime.ground.is_some() || m.runtime.particle_contact)
    }

    /// Execute the acquired repeated terrain query on the live body. Resetting
    /// contact bits is an actual prefix store; failures retain it and quarantine
    /// this callback. A consumed query sequence can never be retried.
    pub fn resolve_timed_effect_ground(
        &mut self,
        id: Id,
        query: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
    ) -> Result<physical_ground::Output, String> {
        let actor = self
            .get(id)
            .ok_or("ground effect owner absent or foreign")?;
        let m = actor
            .timed_effect
            .as_ref()
            .ok_or("ground effect metadata absent")?;
        if !m.runtime.awaiting_physical()
            || (m.runtime.ground.is_none() && !m.runtime.particle_contact)
            || m.runtime.failure.is_some()
            || !m.motion_done
            || m.ground_started
        {
            return Err("ground effect terrain stage is not pending".into());
        }
        let f = actor
            .construction_fields
            .as_ref()
            .ok_or("ground effect fields absent")?;
        if f.response_flags & 0x800 != 0 {
            return Err("ground effect requires its extra response service".into());
        }
        let p = crate::effect_actor::physical_data(&actor.seed)?;
        let word = |i: usize| u32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap());
        let input = physical_ground::Input {
            position: f.position,
            previous: f.current,
            velocity: m.physical.velocity,
            scale: f.scale,
            ground_offset: f32::from_bits(f.configuration[17]),
            center_offset: f32::from_bits(f.configuration[22]),
            gravity_mass: f32::from_bits(word(1)),
            restitution: f32::from_bits(word(3)),
            retention: f32::from_bits(word(4)),
            force_z: f32::from_bits(word(8)),
            flags: word(0) & 0xffff_ff81,
            floor: m.ordinary.floor,
            plane: m.physical.plane,
        };
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.timed_effect.as_mut().unwrap().ground_started = true;
        actor.seed.storage.data_mut().unwrap()[0xa0..0xa4]
            .copy_from_slice(&input.flags.to_le_bytes());
        let output = physical_ground::resolve_preserving_flags(
            input,
            crate::arithmetic::Precision::Double53,
            query,
        );
        let output = match output {
            Ok(output) => output,
            Err(error) => {
                self.residents
                    .get_mut(&id.serial)
                    .unwrap()
                    .timed_effect
                    .as_mut()
                    .unwrap()
                    .runtime
                    .failure = Some(error.into());
                return Err(error.into());
            }
        };
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let f = actor.construction_fields.as_mut().unwrap();
        f.position = output.position;
        f.current = output.previous;
        let m = actor.timed_effect.as_mut().unwrap();
        m.physical.velocity = output.velocity;
        m.physical.plane = output.plane;
        m.ordinary.floor = output.floor;
        let data = actor.seed.storage.data_mut().unwrap();
        data[0xa0..0xa4].copy_from_slice(&output.flags.to_le_bytes());
        data[0xc0..0xc4].copy_from_slice(&output.force_z.to_le_bytes());
        Ok(output)
    }

    /// The post-gravity contact consumer sees the same body and inline budget.
    /// This does not certify a query, child factory or retirement on its own.
    pub fn timed_effect_contact_fields(
        &mut self,
        id: Id,
    ) -> Result<(&mut Seed, &mut crate::actor_fields::Fields), String> {
        let actor = self
            .get(id)
            .ok_or("ground contact owner absent or foreign")?;
        let m = actor
            .timed_effect
            .as_ref()
            .ok_or("ground contact metadata absent")?;
        if m.runtime.ground.is_none()
            || !m.runtime.awaiting_gravity()
            || m.runtime.failure.is_some()
            || !m.motion_done
            || !m.gravity_done
            || !m.ground_started
        {
            return Err("ground effect contact consumer is not pending".into());
        }
        let actor = self.residents.get_mut(&id.serial).unwrap();
        Ok((&mut actor.seed, actor.construction_fields.as_mut().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::preparation::SourceInput;
    use super::*;
    use crate::actor_finalization;
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; supplied source-pose/frames, not FLK dispatch or GPU acceptance"]
    fn installed_blood_source_pose_uses_real_callback_terrain_before_child_refusal() {
        source_pose(false);
    }
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; real terrain/factory with supplied source-pose/frames, no GPU acceptance"]
    fn installed_blood_contact_constructs_real_surface_child_and_retires_parent() {
        source_pose(true);
    }
    fn source_pose(bind_factory: bool) {
        use crate::{
            attached_effect_context::{Creation, Pool},
            audio_parameters::Random,
        };
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let resources = plan.resources();
        let world = packet.world_asset(&root).unwrap();
        let prepared =
            crate::cinematic_timed::world::Prepared::load(&plan, &world.terrain).unwrap();
        let image = &resources.classes.models.effects.last().unwrap().image;
        let bytes = std::fs::read(crate::flick::sequence_path(&root, "story2_savewife_6").unwrap())
            .unwrap();
        let metadata = giants_formats::flick::discover_dependencies(&bytes).unwrap();
        let actors = crate::actor_links::Factory::new(resources, image)
            .project(&bytes)
            .unwrap();
        let clocks = actors.actors.keys().map(|&slot| (slot, 0.)).collect();
        let locals = actors.actors.keys().map(|&slot| (slot, None)).collect();
        let requests = crate::cinematic_fx::declarations(&bytes, &metadata).unwrap();
        let request = requests
            .iter()
            .find(|r| r.effect.as_deref() == Some("fxb_blood"))
            .unwrap();
        let position = request
            .placement(&actors, &clocks, &locals)
            .unwrap()
            .unwrap()
            .position;
        let scope = CinematicScope {
            ticket: 81,
            director: 4,
            generation: 1,
        };
        let mut pool = Pool::<CinematicSupport>::from_active_resources(scope.ticket, resources);
        let mut random = Random(29); // Explicit dependency fixture stream.
        let offset = image.find("fxb_blood").unwrap().1.offset;
        let (Creation::Prepared(context), _) = pool
            .prepare_retirement_offset_invariant(Some(offset), position, 0, 0x1000, &mut random)
            .unwrap()
        else {
            panic!("source context absent")
        };
        let body = crate::timed_effect_object::Timed::ground_positioned(
            resources,
            context,
            pool.get(context).unwrap(),
            &mut random,
        )
        .unwrap();
        let mut session = Session::new(scope.ticket);
        let actor = session
            .adopt_cinematic_timed_positioned(body, scope)
            .unwrap();
        pool.release_prepared(context).unwrap();
        let mut stamp = crate::animation_epoch::Stamp::default();
        let mut queries = 0;
        let mut refused_child = false;
        let mut newborns = Vec::new();
        for frame in 0..360 {
            let seconds = if frame == 0 { 0. } else { 1. / 60. };
            session.begin_cinematic_timed_frame(actor, scope).unwrap();
            let mut factory = crate::cinematic_timed::ground_factory::Factory {
                scope,
                contexts: &mut pool,
                resources,
                random: &mut random,
                newborns: &mut newborns,
            };
            let mut services = crate::cinematic_timed::world::World::new(
                &world.terrain,
                &prepared.objects,
                &[],
                &prepared.unresolved,
                seconds,
                &[],
            )
            .unwrap();
            if bind_factory {
                services = services.with_ground_effects(&mut factory);
            }
            let result = session.prepare_cinematic_timed_positioned_source(
                actor,
                scope,
                SourceInput {
                    seconds,
                    finalization_time: actor_finalization::Time {
                        global_seconds: seconds,
                        context: 0,
                        areas: &[],
                    },
                    progress: actor_finalization::PhysicalProgress::PrototypePs2Zero,
                    pc_argument: None,
                },
                &mut stamp,
                &mut services,
            );
            queries += services.calls.callback_ground_queries;
            if let Err(error) = result {
                assert!(!bind_factory, "actual ground factory refused: {error}");
                assert!(
                    error.contains("real surface-child factory"),
                    "frame={frame} position={position:?}: {error}"
                );
                assert!(session.timed_effect_failure(actor).is_some());
                refused_child = true;
                break;
            }
            if session
                .get(actor)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000
                != 0
            {
                break;
            }
        }
        assert!(
            queries > 0,
            "actual callback terrain stage was not exercised"
        );
        assert_eq!(refused_child, !bind_factory);
        let budget = &session.get(actor).unwrap().seed.storage.data().unwrap()[0x20..0x24];
        assert_eq!(
            i32::from_le_bytes(budget.try_into().unwrap()),
            if bind_factory { 19 } else { 20 }
        );
        if bind_factory {
            assert_eq!(newborns.len(), 1);
            assert_eq!(
                session
                    .get(actor)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags
                    & 0x8000,
                0x8000
            );
            let child = newborns[0];
            assert_eq!(session.get(child).unwrap().class, 938);
            assert_eq!(session.cinematic_timed_scope(child), Some(scope));
            assert_eq!(
                session
                    .get(child)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .control_flags
                    & 0x80,
                0
            );
            let before = random.0;
            session.begin_cinematic_timed_frame(actor, scope).unwrap();
            let mut factory = crate::cinematic_timed::ground_factory::Factory {
                scope,
                contexts: &mut pool,
                resources,
                random: &mut random,
                newborns: &mut newborns,
            };
            let mut services = crate::cinematic_timed::world::World::new(
                &world.terrain,
                &prepared.objects,
                &[],
                &prepared.unresolved,
                1. / 60.,
                &[],
            )
            .unwrap()
            .with_ground_effects(&mut factory);
            let step = session
                .prepare_cinematic_timed_positioned_source(
                    actor,
                    scope,
                    SourceInput {
                        seconds: 1. / 60.,
                        finalization_time: actor_finalization::Time {
                            global_seconds: 1. / 60.,
                            context: 0,
                            areas: &[],
                        },
                        progress: actor_finalization::PhysicalProgress::PrototypePs2Zero,
                        pc_argument: None,
                    },
                    &mut stamp,
                    &mut services,
                )
                .unwrap();
            assert_eq!(step, crate::timed_effect_object::Step::Inactive);
            assert_eq!(random.0, before);
            assert_eq!(newborns.len(), 1, "marked parent emitted its child again");
            let mut expired = false;
            for _ in 0..360 {
                let seconds = 1. / 60.;
                session.begin_cinematic_timed_frame(child, scope).unwrap();
                let mut services = crate::cinematic_timed::world::World::new(
                    &world.terrain,
                    &prepared.objects,
                    &[],
                    &prepared.unresolved,
                    seconds,
                    &[],
                )
                .unwrap();
                session
                    .prepare_cinematic_timed_positioned_source(
                        child,
                        scope,
                        SourceInput {
                            seconds,
                            finalization_time: actor_finalization::Time {
                                global_seconds: seconds,
                                context: 0,
                                areas: &[],
                            },
                            progress: actor_finalization::PhysicalProgress::PrototypePs2Zero,
                            pc_argument: None,
                        },
                        &mut stamp,
                        &mut services,
                    )
                    .unwrap();
                if session
                    .get(child)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags
                    & 0x8000
                    != 0
                {
                    expired = true;
                    break;
                }
            }
            assert!(expired, "source surface child did not expire");
            assert!(pool.active().is_empty());
        }
        session.cancel_cinematic_scope(scope).unwrap();
        assert!(session.get(actor).is_none());
        for child in newborns {
            assert!(session.get(child).is_none());
        }
        println!(
            "blood_callback_real_terrain queries={queries} child_factory_bound={bind_factory} gpu_verified=false"
        );
    }
}
