//! Source-ordered canonical construction and retained wait for the acquired
//! mixed explosion profile. Prototype PS2 `fx_process_one`, `fx_logic` and
//! `fx_processOpCodes` are unknown PC attribution routes, not imported layouts.
//! Body callbacks, ordinary preparation, vertex use and retirement are separate.
use super::{Creation, Id, Immediate, ImmediateInput, Pool};
use crate::{
    audio_parameters::Random,
    resident_states::{Id as Actor, Session},
    scene_resources::SceneResources,
    timed_effect_object::Timed,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Waiting { remaining: f32 },
    Complete,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(session: &Session, id: Actor) -> Vec<u32> {
        let actor = session.get(id).unwrap();
        let f = actor.construction_fields.as_ref().unwrap();
        let d = actor.seed.storage.data().unwrap();
        let word = |at| u32::from_le_bytes(d[at..at + 4].try_into().unwrap());
        let mut words = vec![
            actor.class as u32,
            f.object_flags,
            f.response_flags,
            f.control_flags,
            session.timed_effect_render_flags(id).unwrap(),
            f.scale.to_bits(),
            session.timed_effect_transparency(id).unwrap().to_bits(),
        ];
        words.extend(
            f.position
                .into_iter()
                .chain(f.current)
                .chain(f.previous)
                .chain(f.angles)
                .map(f32::to_bits),
        );
        let matrix = session.timed_effect_matrix(id).unwrap();
        words.push(matrix.flags);
        words.extend(matrix.values.map(f32::to_bits));
        words.extend([4, 8, 0x10, 0x14].map(word));
        words.extend(session.timed_effect_fade(id).unwrap().map(f32::to_bits));
        words.extend((0xa0..0xdc).step_by(4).map(word));
        words
    }
    #[test]
    #[ignore = "Requires GIANTS_NPC_ROOT and private GIANTS_RIPPER_EXECUTION_CONTROLS; actual canonical births/wait/context cleanup, supplied clocks/resources, no body scheduling/GPU/combat"]
    fn installed_mixed_executor_matches_native_births_wait_rng_and_context_retirement() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
        let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let text =
            std::fs::read_to_string(std::env::var_os("GIANTS_RIPPER_EXECUTION_CONTROLS").unwrap())
                .unwrap();
        let mut cases = std::collections::BTreeMap::<String, Vec<Vec<u32>>>::new();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let name = parts.next().unwrap().to_owned();
            let words = parts.map(|v| v.parse::<u32>().unwrap()).collect::<Vec<_>>();
            assert_eq!(words.len(), 63);
            cases.entry(name).or_default().push(words);
        }
        let mut count = 0;
        for (case, (name, rows)) in cases.into_iter().enumerate() {
            let mut session = Session::new(980 + case as u64);
            let mut pool = Pool::<Actor>::from_active_resources(session.ticket(), &resources);
            let mut random = Random(17); // Explicit native-fixture VM stream boundary.
            let request = crate::enemy_projectile::impact_effect(
                u32::from(name == "fx_ripper_large_explode"),
                [1.25, -2.5, 3.75],
            )
            .unwrap();
            assert_eq!(request.name, name);
            let (Creation::Prepared(origin), _) = pool
                .prepare_enemy_projectile_impact_invariant(&request, &mut random)
                .unwrap()
            else {
                panic!("native source absent")
            };
            let mut executor = Execution::new(origin);
            let wait = f32::from_bits(rows[0][1]);
            let first_count = rows.len() - 1;
            assert_eq!(
                executor
                    .advance(&mut pool, &mut session, &resources, 0., &mut random)
                    .unwrap(),
                Step::Waiting { remaining: wait }
            );
            assert_eq!(executor.actors().len(), first_count);
            assert_eq!(
                random.0, rows[0][2],
                "actual full first-phase VM stream {name}"
            );
            assert_eq!(pool.active(), [origin]); // Every nested child context really retired.
            assert!(executor.pending_child().is_none());
            assert!(pool.release_prepared(origin).is_err());
            for (&id, row) in executor.actors().iter().zip(&rows) {
                assert_eq!(
                    &snapshot(&session, id),
                    &row[4..],
                    "actual first-phase body {name} ordinal{}",
                    row[0]
                );
                assert_eq!(session.timed_effect_failure(id), None);
                if session.get(id).unwrap().class == 796 {
                    assert!(session.timed_effect_immediate_binding(id).is_some());
                    assert!(pool.get(session.timed_effect_origin(id).unwrap()).is_none());
                }
            }
            // Another wrapper cannot consume the same actual context while it waits.
            let mut duplicate = Execution::new(origin);
            let seed = random.0;
            assert!(
                duplicate
                    .advance(&mut pool, &mut session, &resources, 0., &mut random)
                    .is_err()
            );
            assert!(duplicate.failure().is_some());
            assert!(duplicate.actors().is_empty());
            assert_eq!(random.0, seed);
            assert_eq!(pool.active(), [origin]);
            for delta in [0., wait / 2.] {
                let remaining = executor.remaining().unwrap() - delta;
                assert_eq!(
                    executor
                        .advance(&mut pool, &mut session, &resources, delta, &mut random)
                        .unwrap(),
                    Step::Waiting { remaining }
                );
                assert_eq!(random.0, seed);
                assert_eq!(executor.actors().len(), first_count);
            }
            assert_eq!(
                executor
                    .advance(&mut pool, &mut session, &resources, wait / 2., &mut random)
                    .unwrap(),
                Step::Complete
            );
            assert_eq!(
                random.0, rows[0][3],
                "actual delayed-phase VM stream {name}"
            );
            assert_eq!(executor.actors().len(), rows.len());
            let last = *executor.actors().last().unwrap();
            assert_eq!(&snapshot(&session, last), &rows.last().unwrap()[4..]);
            assert_eq!(session.get(last).unwrap().class, 814);
            assert!(executor.complete());
            assert!(executor.failure().is_none());
            assert!(pool.get(origin).is_none());
            assert!(pool.active().is_empty());
            let seed = random.0;
            assert!(
                executor
                    .advance(&mut pool, &mut session, &resources, 0., &mut random)
                    .is_err()
            );
            assert_eq!(random.0, seed);
            assert_eq!(session.list(0).len(), rows.len());
            count += rows.len();
        }
        assert_eq!(count, 44);
    }
    #[test]
    #[ignore = "Requires GIANTS_NPC_ROOT; real declared Wife terrain/collision preparation and ordinary timed callbacks, supplied clocks/prototype progress policy, no GPU/native world-walk acceptance"]
    fn installed_mixed_bodies_use_real_ordinary_world_preparation() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let world = packet.world_asset(&root).unwrap();
        let prepared =
            crate::cinematic_timed::world::Prepared::load(&plan, &world.terrain).unwrap();
        let position = packet.player_start_candidate(&root).unwrap().position;
        let resources = plan.resources();
        for action in [0, 1] {
            let mut session = Session::new(985 + u64::from(action));
            let mut pool = Pool::<Actor>::from_active_resources(session.ticket(), resources);
            let mut random = Random(17); // Supplied fixture stream, not a live game seed.
            let request = crate::enemy_projectile::impact_effect(action, position).unwrap();
            let (Creation::Prepared(origin), _) = pool
                .prepare_enemy_projectile_impact_invariant(&request, &mut random)
                .unwrap()
            else {
                panic!("declared source absent")
            };
            let mut executor = Execution::new(origin);
            executor
                .advance(&mut pool, &mut session, resources, 0., &mut random)
                .unwrap();
            let mut stamp = crate::animation_epoch::Stamp::default();
            let mut services = crate::cinematic_timed::world::World::new(
                &world.terrain,
                &prepared.objects,
                &[],
                &prepared.unresolved,
                0.01,
                &[],
            )
            .unwrap();
            let mut callbacks = 0;
            let mut geometry =
                std::collections::BTreeMap::<Actor, crate::smoke_particle_draw::Geometry>::new();
            for &id in executor.actors() {
                assert!(session.particle_draw_ids().contains(&id));
                assert!(session.particle_draw_state(id).unwrap().is_none());
            }
            let view = [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ]; // Supplied CPU camera.
            let mut render_frames = 0;
            for frame in 0..4 {
                for &id in executor.actors() {
                    session.begin_timed_positioned_frame(id).unwrap();
                    let context = session
                        .get(id)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .dispatch_context;
                    let step = session
                        .prepare_timed_positioned_source(
                            id,
                            crate::resident_states::TimedEffectSourcePreparation {
                                seconds: 0.01,
                                finalization_time: crate::actor_finalization::Time {
                                    global_seconds: 0.01,
                                    context,
                                    areas: &[],
                                },
                                progress:
                                    crate::actor_finalization::PhysicalProgress::PrototypePs2Zero,
                                pc_argument: None,
                            },
                            &mut stamp,
                            &mut services,
                        )
                        .unwrap();
                    assert_eq!(
                        step,
                        if frame == 0 {
                            crate::timed_effect_object::Step::FirstFrame
                        } else {
                            crate::timed_effect_object::Step::Updated
                        }
                    );
                    assert!(session.timed_effect_ordinary_header(id).unwrap().prepared);
                    assert!(session.timed_effect_failure(id).is_none());
                    assert!(session.timed_effect_immediate_binding(id).is_none());
                    if session.timed_effect_hidden(id) == Some(true) {
                        assert!(session.particle_draw_state(id).unwrap().is_none());
                    } else {
                        let model = geometry.entry(id).or_insert_with(|| {
                            crate::smoke_particle_draw::Geometry::load(resources, &session, id, 0)
                                .unwrap()
                        });
                        let draw = model.frame(&mut session, id, view).unwrap().unwrap();
                        assert_eq!(draw.uv.len(), model.model.mesh.uv.len());
                        assert_eq!(draw.colors.len(), model.model.mesh.rgb.len());
                        assert_eq!(draw.vertex_alpha, model.class == 965);
                        if draw.vertex_alpha {
                            if frame == 0 {
                                assert!(draw.colors.iter().all(|c| c[3] == 0.));
                            } else {
                                assert!(draw.colors.iter().any(|c| c[3] > 0.));
                                assert_ne!(draw.uv, model.model.mesh.uv);
                            }
                        }
                        render_frames += 1;
                    }
                    callbacks += 1;
                }
            }
            assert_eq!(callbacks, executor.actors().len() * 4);
            assert_eq!(render_frames, (executor.actors().len() - 1) * 4);
            eprintln!(
                "mixed ordinary action={action} bodies={} callbacks={callbacks} render_input_frames={render_frames} colliders={} unprepared_platforms={} environments={} physical={} ground_queries={} gpu=false",
                executor.actors().len(),
                prepared.objects.len(),
                prepared.unresolved.len(),
                services.calls.environment,
                services.calls.motion,
                services.calls.callback_ground_queries
            );
            assert!(services.calls.environment > 0);
            assert!(services.calls.callback_ground_queries > 0);
            assert!(services.calls.motion > 0);
            assert!(services.calls.gravity > 0);
            // The separate VM wait is still retained; body preparation cannot
            // consume it or manufacture completion of the whole effect.
            assert!(!executor.complete());
            assert!(pool.get(origin).is_some());
        }
    }
}

#[cfg(test)]
mod lifetime_tests {
    use super::*;
    #[test]
    #[ignore = "Requires GIANTS_NPC_ROOT and private GIANTS_RIPPER_EXPIRY_CONTROLS; source-world lifetime/resource cleanup and original expiry counts, supplied clocks/progress, no GPU/global native walk or NPC death acceptance"]
    fn installed_mixed_lifetimes_release_actual_payloads_references_and_registrations() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let asset = packet.world_asset(&root).unwrap();
        let prepared =
            crate::cinematic_timed::world::Prepared::load(&plan, &asset.terrain).unwrap();
        let platforms =
            crate::declared_platform_scene::Prepared::load(&plan, &asset.terrain).unwrap();
        let position = packet.player_start_candidate(&root).unwrap().position;
        let resources = plan.resources();
        let controls =
            std::fs::read_to_string(std::env::var_os("GIANTS_RIPPER_EXPIRY_CONTROLS").unwrap())
                .unwrap();
        let mut expected = std::collections::BTreeMap::new();
        for line in controls.lines() {
            let p = line.split_whitespace().collect::<Vec<_>>();
            expected.insert(
                (p[0].to_owned(), p[1].parse::<usize>().unwrap()),
                (p[2].parse::<i32>().unwrap(), p[3].parse::<usize>().unwrap()),
            );
        }
        let mut total = 0;
        let mut total_callbacks = 0;
        for action in [0, 1] {
            let mut session = Session::new(990 + u64::from(action));
            let mut pool = Pool::<Actor>::from_active_resources(session.ticket(), resources);
            let mut random = Random(17); // Explicit source-control VM stream, not live-game initialization.
            let request = crate::enemy_projectile::impact_effect(action, position).unwrap();
            let (Creation::Prepared(origin), _) = pool
                .prepare_enemy_projectile_impact_invariant(&request, &mut random)
                .unwrap()
            else {
                panic!()
            };
            let mut execution = Execution::new(origin);
            execution
                .advance(&mut pool, &mut session, resources, 0., &mut random)
                .unwrap();
            let mut services = crate::cinematic_timed::world::World::new(
                &asset.terrain,
                &prepared.objects,
                &[],
                &prepared.unresolved,
                0.01,
                &[],
            )
            .unwrap()
            .with_declared_platforms(&platforms)
            .unwrap();
            let mut stamp = crate::animation_epoch::Stamp::default();
            let mut calls = std::collections::BTreeMap::<Actor, usize>::new();
            let mut geometry =
                std::collections::BTreeMap::<Actor, crate::smoke_particle_draw::Geometry>::new();
            let mut weak = std::collections::BTreeMap::new();
            let view = [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ];
            for frame in 0..650 {
                if frame > 0 && !execution.complete() {
                    execution
                        .advance(&mut pool, &mut session, resources, 0.01, &mut random)
                        .unwrap();
                }
                let mut expired = Vec::new();
                for (ordinal, &id) in execution.actors().iter().enumerate() {
                    let Some(actor) = session.get(id) else {
                        continue;
                    };
                    let class = actor.class;
                    weak.entry(id).or_insert_with(|| {
                        (
                            std::sync::Arc::downgrade(session.specification(id).unwrap()),
                            std::sync::Arc::downgrade(session.state_resources(id).unwrap()),
                        )
                    });
                    assert!(session.destroy_expired_mixed_particle(id, &pool).is_err());
                    session.begin_timed_positioned_frame(id).unwrap();
                    let context = session
                        .get(id)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .dispatch_context;
                    let step = session
                        .prepare_timed_positioned_source(
                            id,
                            crate::resident_states::TimedEffectSourcePreparation {
                                seconds: 0.01,
                                finalization_time: crate::actor_finalization::Time {
                                    global_seconds: 0.01,
                                    context,
                                    areas: &[],
                                },
                                progress:
                                    crate::actor_finalization::PhysicalProgress::PrototypePs2Zero,
                                pc_argument: None,
                            },
                            &mut stamp,
                            &mut services,
                        )
                        .unwrap();
                    *calls.entry(id).or_default() += 1;
                    if step == crate::timed_effect_object::Step::Marked {
                        assert_eq!(
                            (class, calls[&id]),
                            expected[&(request.name.to_owned(), ordinal)],
                            "actual expiry {} body{ordinal}",
                            request.name
                        );
                        assert!(session.particle_draw_state(id).unwrap().is_none());
                        expired.push(id);
                    } else if session.particle_draw_state(id).unwrap().is_some() {
                        let model = geometry.entry(id).or_insert_with(|| {
                            crate::smoke_particle_draw::Geometry::load(resources, &session, id, 0)
                                .unwrap()
                        });
                        assert!(model.frame(&mut session, id, view).unwrap().is_some());
                    }
                }
                let before = session.counts().residents;
                for id in &expired {
                    let result = session.destroy_expired_mixed_particle(*id, &pool).unwrap();
                    assert_eq!(result.payload_bytes, 268);
                    assert!(session.get(*id).is_none());
                    let (spec, state) = weak.remove(id).unwrap();
                    assert!(spec.upgrade().is_none());
                    assert!(state.upgrade().is_none());
                    if let Some(model) = geometry.remove(id) {
                        assert!(model.frame(&mut session, *id, view).is_err());
                    }
                    assert!(session.destroy_expired_mixed_particle(*id, &pool).is_err());
                    total += 1;
                }
                assert_eq!(session.counts().residents, before - expired.len());
                if execution.complete() && session.list(0).is_empty() {
                    break;
                }
            }
            assert!(execution.complete());
            assert!(pool.active().is_empty());
            assert!(session.list(0).is_empty());
            assert_eq!(session.counts().payload_bytes, 0);
            assert!(session.particle_draw_ids().is_empty());
            assert!(geometry.is_empty());
            assert!(weak.is_empty());
            total_callbacks += calls.values().sum::<usize>();
        }
        assert_eq!(total, 44);
        assert_eq!(total_callbacks, 9853);
        eprintln!(
            "mixed source lifetime bodies={total} callbacks={total_callbacks} resources_released=true gpu=false npc_death=false"
        );
    }
}

/// Own the real context across activations. A failure retains constructed
/// actors, pending child context and consumed random prefix; replay is forbidden.
/// Completion means source construction/context retirement, not effect lifetime.
#[derive(Debug)]
pub struct Execution {
    context: Id,
    started: bool,
    complete: bool,
    cursor: usize,
    remaining: Option<f32>,
    actors: Vec<Actor>,
    pending_child: Option<Id>,
    failure: Option<String>,
}
impl Execution {
    pub fn new(context: Id) -> Self {
        Self {
            context,
            started: false,
            complete: false,
            cursor: 0,
            remaining: None,
            actors: Vec::new(),
            pending_child: None,
            failure: None,
        }
    }
    pub fn context(&self) -> Id {
        self.context
    }
    /// Actual identities in birth order; a later body retirement does not erase
    /// its historical identity. Presence must still be checked in Session.
    pub fn actors(&self) -> &[Actor] {
        &self.actors
    }
    pub fn pending_child(&self) -> Option<Id> {
        self.pending_child
    }
    pub fn complete(&self) -> bool {
        self.complete
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn remaining(&self) -> Option<f32> {
        self.remaining
    }
    /// The composing owner supplies this context's independently resolved clock
    /// and existing VM random stream. No new seed, cadence or world readiness.
    pub fn advance(
        &mut self,
        pool: &mut Pool<Actor>,
        session: &mut Session,
        resources: &SceneResources,
        seconds: f32,
        random: &mut Random,
    ) -> Result<Step, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.complete {
            return Err("mixed context already completed; replay is forbidden".into());
        }
        let result = self.advance_inner(pool, session, resources, seconds, random);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn advance_inner(
        &mut self,
        pool: &mut Pool<Actor>,
        session: &mut Session,
        resources: &SceneResources,
        seconds: f32,
        random: &mut Random,
    ) -> Result<Step, String> {
        if !seconds.is_finite() || seconds < 0. || session.ticket() != self.context.ticket {
            return Err("mixed context requires its current Session and finite clock".into());
        }
        let context = pool
            .get(self.context)
            .ok_or("mixed context absent or stale")?;
        if !matches!(context.flags(), 0 | 0x8000) || context.immediate().is_some() {
            return Err("mixed context outside acquired unparented positional profile".into());
        }
        let program = context.program.as_ref().map_err(Clone::clone)?.clone();
        let image = context.source_image();
        if context.position().is_none()
            || !crate::effect_initialization::ripper_explosion(
                &program.records.iter().map(|r| r.opcode).collect::<Vec<_>>(),
            )
        {
            return Err("mixed execution requires the complete acquired source profile".into());
        }
        if !self.started {
            pool.claim_positional_impact(self.context)?;
            self.started = true;
        }
        if let Some(remaining) = self.remaining {
            let remaining = (f64::from(remaining) - f64::from(seconds)) as f32;
            if remaining > 0. {
                self.remaining = Some(remaining);
                return Ok(Step::Waiting { remaining });
            }
            self.remaining = None;
        }
        loop {
            let record = program
                .records
                .get(self.cursor)
                .ok_or("mixed cursor absent")?;
            match record.opcode {
                3 => {
                    let start = self.cursor;
                    let end = program.records[start..]
                        .iter()
                        .position(|r| r.opcode == 7)
                        .map(|n| start + n)
                        .ok_or("mixed body end absent")?;
                    // The acquired parent frames perform their own commands
                    // before all immediate child calls. No projection or second
                    // sampling pass supplies those commands.
                    let body = Timed::mixed_root_object(
                        resources,
                        self.context,
                        pool.get(self.context).unwrap(),
                        record.offset,
                        random,
                    )?;
                    let parent = session.adopt_timed_positioned(body)?;
                    self.actors.push(parent);
                    self.cursor += 2; // Actual source3/6 pair, checked above/by constructor.
                    while self.cursor < end {
                        let record = &program.records[self.cursor];
                        if record.opcode == 0x10 {
                            let bytes = &record.bytes;
                            let end_name = bytes
                                .get(5..)
                                .and_then(|v| v.iter().position(|&b| b == 0))
                                .map(|n| n + 5)
                                .ok_or("mixed child name absent")?;
                            let label = std::str::from_utf8(&bytes[5..end_name])
                                .map_err(|_| "mixed child name invalid")?;
                            let source_offset = image
                                .find(label)
                                .ok_or("mixed child source absent")?
                                .1
                                .offset;
                            let word = |at| -> Result<u32, String> {
                                Ok(u32::from_le_bytes(
                                    bytes
                                        .get(at..at + 4)
                                        .ok_or("mixed child operand absent")?
                                        .try_into()
                                        .unwrap(),
                                ))
                            };
                            let binding = Immediate {
                                parent,
                                reference: word(1)?,
                                own_reference: word(end_name + 1)?,
                            };
                            let fields = session
                                .get(parent)
                                .and_then(|a| a.construction_fields.as_ref())
                                .ok_or("mixed parent fields absent")?;
                            let (creation, _) = pool.prepare_immediate_offset_invariant(
                                source_offset,
                                ImmediateInput {
                                    binding,
                                    position: fields.position,
                                    object_flags: fields.object_flags,
                                    render_flags: session
                                        .timed_effect_render_flags(parent)
                                        .ok_or("mixed parent render owner absent")?,
                                },
                                random,
                            )?;
                            let Creation::Prepared(child_context) = creation else {
                                return Err("declared mixed child source did not prepare".into());
                            };
                            self.pending_child = Some(child_context);
                            pool.claim_positional_impact(child_context)?;
                            let child = Timed::immediate_particle_object(
                                resources,
                                child_context,
                                pool.get(child_context).unwrap(),
                                random,
                            )?;
                            self.actors.push(session.adopt_timed_positioned(child)?);
                            // The complete acquired child frame has no wait or
                            // trailing service. Retire its real context now.
                            pool.complete_positional_impact(child_context)?;
                            self.pending_child = None;
                        }
                        self.cursor += 1;
                    }
                    self.cursor = end + 1;
                }
                0x0b => {
                    let remaining = f32::from_le_bytes(
                        record
                            .bytes
                            .get(1..5)
                            .ok_or("mixed wait operand absent")?
                            .try_into()
                            .unwrap(),
                    );
                    if !remaining.is_finite() || remaining < 0. {
                        return Err("mixed wait outside acquired finite profile".into());
                    }
                    self.cursor += 1;
                    self.remaining = Some(remaining);
                    return Ok(Step::Waiting { remaining });
                }
                2 => {
                    pool.complete_positional_impact(self.context)?;
                    self.cursor += 1;
                    self.complete = true;
                    return Ok(Step::Complete);
                }
                _ => return Err("mixed source cursor has an unowned service".into()),
            }
        }
    }
}
