//! Ordered activation boundaries for a prepared burst. Unlike a birth projection,
//! real services run between draws; a late failure preserves the executed prefix.
//! Constructor completion is separate from ordinary preparation/world callbacks.
use super::{Body, ChildServiceNeed, Op, Plan, random_sound};
use crate::audio_parameters::Random;

#[derive(Clone, Debug)]
pub enum Command {
    Lifetime([f32; 3]),
    Size { values: [f32; 2], interpolate: bool },
    AngularRates([f32; 3]),
    Speed(f32),
    Gravity(f32),
    Damping([f32; 2]),
    Bounce(f32),
    Attribute(u32),
    ParameterB0(u32),
    Height(f32),
    Hidden,
    TextureAnimation(crate::effect_texture_animation::Configuration),
    ChildService(ChildServiceNeed),
}

/// The backend owns actual actors, current reference poses and source services.
/// No projected spawn is a substitute for construction or command completion.
pub trait Services {
    type Actor: Copy;
    type Context: Copy;
    /// Prepare the actual direct-offset immediate child context before its
    /// nested activation. The pool owns its preparation draws/allocation.
    fn prepare_child(
        &mut self,
        parent: Self::Actor,
        source_offset: u32,
        reference: u32,
        own_reference: u32,
        random: &mut Random,
    ) -> Result<Self::Context, String>;
    fn positional_sound(&mut self, sound: u32, random: &mut Random) -> Result<(), String>;
    fn begin(
        &mut self,
        class: i32,
        context: Self::Context,
        random: &mut Random,
    ) -> Result<Self::Actor, String>;
    fn command(&mut self, actor: Self::Actor, command: Command) -> Result<(), String>;
    fn attached_sound(
        &mut self,
        actor: Self::Actor,
        sound: u32,
        retained: bool,
        random: &mut Random,
    ) -> Result<(), String>;
    /// Finish the source object block; ordinary ANM/placement stays separate.
    fn finish(&mut self, actor: Self::Actor, context: Self::Context) -> Result<(), String>;
}

#[derive(Default)]
pub struct Execution {
    started: bool,
    finished: bool,
    failure: Option<String>,
    bodies_started: usize,
}
impl Execution {
    pub fn finished(&self) -> bool {
        self.finished
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn bodies_started(&self) -> usize {
        self.bodies_started
    }
    /// The pool must have performed the named null-object preparation already.
    /// This transaction is single-use even if a service rejects its first call.
    pub fn activate<C: Copy>(
        &mut self,
        plan: &Plan,
        random: &mut Random,
        services: &mut impl Services<Context = C>,
        root: C,
    ) -> Result<(), String> {
        if self.started {
            return Err("burst activation already started; replay is forbidden".into());
        }
        self.started = true;
        let result: Result<(), String> = (|| {
            if let Some(range) = plan.sound {
                let sound = random_sound(&mut random.0, range);
                services.positional_sound(sound, random)?;
            }
            for body in &plan.bodies {
                self.body(body, root, random, services)?;
            }
            Ok(())
        })();
        match &result {
            Ok(()) => self.finished = true,
            Err(error) => self.failure = Some(error.clone()),
        }
        result
    }
    fn body<S: Services>(
        &mut self,
        body: &Body,
        context: S::Context,
        random: &mut Random,
        services: &mut S,
    ) -> Result<(), String> {
        if self.bodies_started >= 512 {
            return Err("burst live construction budget".into());
        }
        let actor = services.begin(body.class, context, random)?;
        self.bodies_started += 1;
        for op in &body.ops {
            let values = if matches!(op, Op::RandomSound(_)) {
                Vec::new()
            } else {
                op.sample(&mut random.0)
            };
            let command = match op {
                Op::Lifetime(v) => Command::Lifetime(*v),
                Op::RandomLifetime(_) => Command::Lifetime([values[0], values[1], values[2]]),
                Op::Size(v) => Command::Size {
                    values: [*v; 2],
                    interpolate: false,
                },
                Op::Sizes(v) => Command::Size {
                    values: *v,
                    interpolate: v[0] != v[1],
                },
                Op::RandomSize(_) => Command::Size {
                    values: [values[0]; 2],
                    interpolate: false,
                },
                Op::SizeRange(_) => Command::Size {
                    values: [values[0], values[1]],
                    interpolate: values[0] != values[1],
                },
                Op::Spin(_) => Command::AngularRates([values[1], values[0], values[2]]),
                Op::FixedSpin(v) => Command::AngularRates([v[1], v[0], v[2]]),
                Op::Speed(_) => Command::Speed(values[0]),
                Op::RandomGravity(_) => Command::Gravity(values[0]),
                Op::Gravity(v) => Command::Gravity(*v),
                Op::Damping(v) => Command::Damping(*v),
                Op::Bounce(v) => Command::Bounce(*v),
                Op::Attribute(v) => Command::Attribute(*v),
                Op::PayloadB0(v) => Command::ParameterB0(*v),
                Op::Height(v) => Command::Height(*v),
                Op::Hidden => Command::Hidden,
                Op::TextureAnimation(v) => Command::TextureAnimation(*v),
                Op::ChildService(v) => Command::ChildService(v.clone()),
                Op::Sound(v) => {
                    services.attached_sound(actor, *v, false, random)?;
                    continue;
                }
                Op::RandomSound(range) => {
                    let sound = random_sound(&mut random.0, *range);
                    services.attached_sound(actor, sound, true, random)?;
                    continue;
                }
                Op::Child {
                    source_offset,
                    reference,
                    own_reference,
                    body,
                } => {
                    let child = services.prepare_child(
                        actor,
                        *source_offset,
                        *reference,
                        *own_reference,
                        random,
                    )?;
                    self.body(body, child, random, services)?;
                    continue;
                }
            };
            services.command(actor, command)?;
        }
        services.finish(actor, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_MECC_WORLD_ROOT; source VM boundary recorder, supplied parent pose/flags and disabled audio, no canonical actors/world/rendering"]
    fn source_pistol_burst_keeps_native_nested_order_and_random_stream() {
        use crate::attached_effect_context::{Creation, Id, Immediate, ImmediateInput, Pool};
        let root = std::env::var_os("GIANTS_MECC_WORLD_ROOT").expect("GIANTS_MECC_WORLD_ROOT");
        let bytes =
            std::fs::read(std::path::Path::new(&root).join("Bin/fx_world_dust.bin")).unwrap();
        let image = std::sync::Arc::new(giants_formats::effect_image::parse(&bytes).unwrap());
        let mut pool = Pool::<u32>::new(11, Some(image));
        let mut random = Random(17);
        let (Creation::Prepared(root), _) = pool
            .prepare_positional_invariant("fx_bullet_sound", [3., 4., 7.], &mut random)
            .unwrap()
        else {
            panic!("source burst absent")
        };
        assert_eq!(
            random.0, 6169232,
            "root preparation must not prepare child contexts"
        );
        let context = pool.get(root).unwrap();
        let plan = Plan::from_records(
            &context.source_image(),
            &context.program.as_ref().unwrap().records,
            2,
        )
        .unwrap();
        struct Recorder<'a> {
            pool: &'a mut Pool<u32>,
            root: Id,
            classes: Vec<i32>,
        }
        impl Services for Recorder<'_> {
            type Actor = u32;
            type Context = Id;
            fn prepare_child(
                &mut self,
                parent: u32,
                source_offset: u32,
                reference: u32,
                own_reference: u32,
                random: &mut Random,
            ) -> Result<Id, String> {
                let (Creation::Prepared(id), _) = self.pool.prepare_immediate_offset_invariant(
                    source_offset,
                    ImmediateInput {
                        binding: Immediate {
                            parent,
                            reference,
                            own_reference,
                        },
                        position: [3., 4., 7.],
                        object_flags: 0,
                        render_flags: 0,
                    },
                    random,
                )?
                else {
                    return Err("source child absent".into());
                };
                assert_eq!(self.pool.get(id).unwrap().flags(), 2);
                Ok(id)
            }
            fn positional_sound(&mut self, _: u32, _: &mut Random) -> Result<(), String> {
                // Matches the explicitly disabled native audio fixture. The
                // application must bind its real device instead of this recorder.
                Ok(())
            }
            fn begin(&mut self, class: i32, context: Id, _: &mut Random) -> Result<u32, String> {
                self.pool
                    .get(context)
                    .ok_or("source context retired too early")?;
                self.classes.push(class);
                Ok(self.classes.len() as u32)
            }
            fn command(&mut self, _: u32, _: Command) -> Result<(), String> {
                Ok(())
            }
            fn attached_sound(
                &mut self,
                _: u32,
                _: u32,
                _: bool,
                _: &mut Random,
            ) -> Result<(), String> {
                Err("source fixture has no attached-sound command".into())
            }
            fn finish(&mut self, _: u32, context: Id) -> Result<(), String> {
                if context != self.root {
                    self.pool.release_prepared(context)?;
                }
                Ok(())
            }
        }
        let mut recorder = Recorder {
            pool: &mut pool,
            root,
            classes: vec![],
        };
        let mut execution = Execution::default();
        execution
            .activate(&plan, &mut random, &mut recorder, root)
            .unwrap();
        assert_eq!(
            recorder.classes,
            [794, 818, 913, 913, 913, 873, 867, 867, 867, 867]
        );
        assert_eq!(
            random.0, 4259069417,
            "matches the original complete source activation fixture"
        );
        assert!(execution.finished());
        assert_eq!(recorder.pool.active(), &[root]);
        recorder.pool.release_prepared(root).unwrap();
        assert!(recorder.pool.active().is_empty());
    }
    #[derive(Default)]
    struct Backend {
        events: Vec<String>,
        values: Vec<Command>,
        fail_class: Option<i32>,
        fail_sound: bool,
        reject_child: bool,
        sound_draws: usize,
        next: u32,
    }
    impl Services for Backend {
        type Actor = u32;
        type Context = u32;
        fn prepare_child(
            &mut self,
            parent: u32,
            source_offset: u32,
            reference: u32,
            own_reference: u32,
            random: &mut Random,
        ) -> Result<u32, String> {
            self.events.push(format!(
                "prepare:{parent}:{source_offset}:{reference}:{own_reference}"
            ));
            crate::timed_effect::sample(&mut random.0, [1., 2.]);
            if self.reject_child {
                Err("child context allocation rejected".into())
            } else {
                Ok(source_offset)
            }
        }

        fn positional_sound(&mut self, sound: u32, random: &mut Random) -> Result<(), String> {
            self.events.push(format!("positional:{sound}"));
            for _ in 0..self.sound_draws {
                random.next_15();
            }
            if self.fail_sound {
                Err("device rejected after its parameter draws".into())
            } else {
                Ok(())
            }
        }
        fn begin(&mut self, class: i32, context: u32, _: &mut Random) -> Result<u32, String> {
            assert_eq!(context, if class == 2 { 24 } else { 0 });
            self.events.push(format!("begin:{class}"));
            if self.fail_class == Some(class) {
                return Err("constructor rejected".into());
            }
            self.next += 1;
            Ok(self.next)
        }
        fn command(&mut self, _: u32, command: Command) -> Result<(), String> {
            self.values.push(command);
            Ok(())
        }
        fn attached_sound(
            &mut self,
            actor: u32,
            sound: u32,
            retained: bool,
            _: &mut Random,
        ) -> Result<(), String> {
            self.events
                .push(format!("attached:{actor}:{sound}:{retained}"));
            Ok(())
        }
        fn finish(&mut self, actor: u32, _: u32) -> Result<(), String> {
            self.events.push(format!("finish:{actor}"));
            Ok(())
        }
    }
    fn plan() -> Plan {
        Plan {
            sound: Some([10, 12]),
            bodies: vec![
                Body {
                    class: 1,
                    ops: vec![
                        Op::Size(2.),
                        Op::Child {
                            source_offset: 24,
                            reference: 3,
                            own_reference: 4,
                            body: Body {
                                class: 2,
                                ops: vec![Op::RandomSize([1., 2.]), Op::Lifetime([1., 0., 0.])],
                            },
                        },
                        Op::Lifetime([2., 0., 0.]),
                    ],
                },
                Body {
                    class: 3,
                    ops: vec![Op::Lifetime([1., 0., 0.])],
                },
            ],
        }
    }
    #[test]
    fn immediate_child_prepares_its_own_context_before_nested_actor_construction() {
        let mut backend = Backend::default();
        let mut random = Random(17);
        let mut execution = Execution::default();
        execution
            .activate(&plan(), &mut random, &mut backend, 0)
            .unwrap();
        assert!(execution.finished());
        assert!(execution.failure().is_none());
        assert_eq!(execution.bodies_started(), 3);
        assert_eq!(
            &backend.events[1..],
            [
                "begin:1",
                "prepare:1:24:3:4",
                "begin:2",
                "finish:2",
                "finish:1",
                "begin:3",
                "finish:3"
            ]
        );
        let before = (random.0, backend.events.len());
        assert!(
            execution
                .activate(&plan(), &mut random, &mut backend, 0)
                .is_err()
        );
        assert_eq!(before, (random.0, backend.events.len()));
    }
    #[test]
    fn rejected_positional_sound_keeps_its_draws_without_constructing_future_objects() {
        let mut backend = Backend {
            fail_sound: true,
            sound_draws: 2,
            ..Default::default()
        };
        let mut random = Random(9);
        let mut expected = random;
        random_sound(&mut expected.0, [10, 12]);
        expected.next_15();
        expected.next_15();
        let mut execution = Execution::default();
        assert!(
            execution
                .activate(&plan(), &mut random, &mut backend, 0)
                .is_err()
        );
        assert_eq!(random.0, expected.0);
        assert_eq!(execution.bodies_started(), 0);
        assert_eq!(backend.events.len(), 1);
        assert!(execution.failure().unwrap().contains("device"));
        assert!(!execution.finished());
    }
    #[test]
    fn child_context_failure_keeps_preparation_draws_and_parent_prefix() {
        let mut backend = Backend {
            reject_child: true,
            ..Default::default()
        };
        let mut random = Random(31);
        let mut expected = random;
        random_sound(&mut expected.0, [10, 12]);
        crate::timed_effect::sample(&mut expected.0, [1., 2.]);
        let mut execution = Execution::default();
        assert!(
            execution
                .activate(&plan(), &mut random, &mut backend, 0)
                .is_err()
        );
        assert_eq!(random.0, expected.0);
        assert_eq!(execution.bodies_started(), 1);
        assert_eq!(backend.values.len(), 1);
        assert!(!backend.events.iter().any(|e| e.starts_with("finish")));
        assert!(execution.failure().unwrap().contains("context allocation"));
    }
    #[test]
    fn audio_boundary_precedes_child_preparation_and_nested_active_draws() {
        let mut backend = Backend {
            sound_draws: 4,
            ..Default::default()
        };
        let mut random = Random(3);
        let mut expected = random;
        random_sound(&mut expected.0, [10, 12]);
        for _ in 0..4 {
            expected.next_15();
        }
        crate::timed_effect::sample(&mut expected.0, [1., 2.]);
        Execution::default()
            .activate(&plan(), &mut random, &mut backend, 0)
            .unwrap();
        let size = crate::timed_effect::sample(&mut expected.0, [1., 2.]);
        assert_eq!(random.0, expected.0);
        assert!(
            matches!(backend.values[1], Command::Size { values, interpolate: false } if values == [size; 2])
        );
    }
}
