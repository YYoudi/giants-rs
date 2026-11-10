use super::held_reference_tests::Case;
use super::*;
use crate::{
    actor_animation_phase::Services, animation_channels::Request, kabuto_animation_commit as commit,
};

#[derive(Default)]
pub(super) struct Controls {
    inputs: String,
    outputs: String,
    cases: usize,
    resolved: bool,
    held: usize,
    skipped: usize,
}
fn line(words: impl IntoIterator<Item = u32>) -> String {
    words
        .into_iter()
        .map(|v| format!("{v:08x}"))
        .collect::<Vec<_>>()
        .join(" ")
        + "\n"
}
fn references(s: &Session, owner: Id) -> Vec<u32> {
    s.reference_cache_storage(owner)
        .unwrap()
        .matrices
        .iter()
        .flat_map(|m| std::iter::once(m.flags).chain(m.values.map(f32::to_bits)))
        .collect()
}
fn packet(r: Request) -> [u32; 5] {
    [
        r.reference_speed.to_bits(),
        r.blend_seconds.to_bits(),
        u32::from(r.slot),
        r.flags,
        r.speed_source as u32,
    ]
}
struct World<'a> {
    factory: &'a crate::kabuto_initial::Factory,
    number: u64,
    selection: commit::Selection,
    mode: u32,
    owner_word: u32,
    kind: Case,
    prefix: Vec<u32>,
    calls: usize,
}
impl commit::World for World<'_> {
    fn network<'a>(
        &'a self,
        _: &Session,
        _: Id,
    ) -> Result<crate::kabuto_geometry_mode::NotificationContext<'a>, String> {
        Ok(crate::kabuto_geometry_mode::NotificationContext {
            session_mode: self.mode,
            peer_ids: &[11, 22],
            local_peer: 11,
            owner_word: self.owner_word,
        })
    }
    fn notify(&mut self, s: &mut Session, owner: Id, request: Request) -> Result<(), String> {
        self.calls += 1;
        assert_eq!(
            request.flags,
            self.selection.flags & !self.selection.clear_flags
        );
        let a = s.get(owner).unwrap();
        assert_eq!(
            a.channels,
            Some(a.construction_fields.as_ref().unwrap().channels)
        );
        assert_eq!(a.channels.unwrap()[0][0], u32::from(request.slot));
        self.prefix.extend(packet(request));
        self.prefix.extend(super::motion_tests::snapshot(s, owner));
        let mut contexts = crate::attached_effect_context::Pool::new(owner.ticket(), None);
        let mut random = crate::audio_parameters::Random(1);
        let mut stamp = crate::animation_epoch::Stamp(0);
        let mut services = Services {
            contexts: &mut contexts,
            random: &mut random,
            stamp: &mut stamp,
            blending: Blending::Disabled,
        };
        assert!(
            self.factory
                .commit_animation_selection(
                    s,
                    owner,
                    self.number,
                    commit::Frame {
                        selection: self.selection,
                        materials: None
                    },
                    &mut services,
                    self
                )
                .is_err()
        );
        // Declared mutating transport entry, not the native delivery backend.
        let a = s.residents.get_mut(&owner.serial).unwrap();
        let f = a.construction_fields.as_mut().unwrap();
        f.action ^= 0x0080_0000;
        f.channels[0][1] = 0.125f32.to_bits();
        f.channels[0][2] = 0.0625f32.to_bits();
        a.channels = Some(f.channels);
        a.kabuto_group.as_mut().unwrap().motion.mode = 99;
        let p = a.seed.storage.data_mut().unwrap();
        p[0x90..0x94].copy_from_slice(&123u32.to_le_bytes());
        p[0x300..0x304].copy_from_slice(&0x12345678u32.to_le_bytes());
        if matches!(self.kind, Case::CommitClear) {
            s.clear();
            Ok(())
        } else if matches!(self.kind, Case::CommitFailure) {
            Err("declared late animation notification failure".into())
        } else {
            Ok(())
        }
    }
}
impl Controls {
    pub(super) fn advance(
        &mut self,
        factories: (
            &crate::kabuto_initial::Factory,
            &crate::kabuto_initial::Factory,
        ),
        s: &mut Session,
        owner: Id,
        number: u64,
        kind: Case,
        services: &mut Services<'_>,
    ) {
        let (factory, foreign) = factories;
        let index = (number - 1) as usize;
        let selection = commit::Selection {
            slot: [2, 115, 116, 1][index % 4],
            flags: [0xa1, 0x31, 0x21, 1][index % 4],
            clear_flags: [32, 0, 0, 0][index % 4],
            speed_source: [1, 2, 2, -1][index % 4],
            reference_speed: [40., -24., 24., 0.][index % 4],
            blend_seconds: 0.25,
            behavior: [3, 4, 4, 0][index % 4],
            definition: u32::MAX,
            retained_time: 0.25f32.to_bits(),
        };
        let resolved = matches!(kind, Case::ActionGate);
        self.resolved |= resolved;
        let gate = s.kabuto_group(owner).unwrap().action_gate_next;
        let selection =
            if resolved && gate == Some(crate::kabuto_action_select::Continuation::Commit) {
                let q = s
                    .kabuto_group(owner)
                    .unwrap()
                    .action_select_scratch
                    .unwrap();
                commit::Selection {
                    slot: u8::try_from(q.slot).unwrap(),
                    flags: q.flags,
                    clear_flags: q.clear_flags,
                    speed_source: q.speed_source,
                    reference_speed: q.reference_speed,
                    blend_seconds: q.blend_seconds,
                    behavior: q.behavior,
                    definition: q.definition,
                    retained_time: q.retained_time,
                }
            } else {
                selection
            };
        let mut world = World {
            factory,
            number,
            selection,
            mode: if index.is_multiple_of(4) { 0 } else { 4 },
            owner_word: u32::from(index % 4 == 2),
            kind,
            prefix: vec![],
            calls: 0,
        };
        let captured = s
            .kabuto_group(owner)
            .unwrap()
            .resource_phase_view
            .unwrap()
            .resources
            .unwrap();
        let before = super::motion_tests::snapshot(s, owner);
        let refs = references(s, owner);
        if resolved && gate != Some(crate::kabuto_action_select::Continuation::Commit) {
            match gate.unwrap() {
                crate::kabuto_action_select::Continuation::HeldReferences => self.held += 1,
                crate::kabuto_action_select::Continuation::SkipCommit => self.skipped += 1,
                _ => unreachable!(),
            }
            assert!(
                factory
                    .commit_resolved_actions(s, owner, number, None, services, &mut world)
                    .is_err()
            );
            assert_eq!(before, super::motion_tests::snapshot(s, owner));
            assert_eq!(refs, references(s, owner));
            assert_eq!(world.calls, 0);
            assert!(
                s.kabuto_group(owner)
                    .unwrap()
                    .animation_commit_entered
                    .is_none()
            );
            return;
        }
        let mut input = vec![
            u32::from(selection.slot),
            selection.flags,
            selection.clear_flags,
            selection.speed_source as u32,
            selection.reference_speed.to_bits(),
            selection.blend_seconds.to_bits(),
            selection.behavior,
            selection.definition,
            selection.retained_time,
            u32::from(services.blending == Blending::Enabled),
            world.mode,
            world.owner_word,
            captured.slot,
            services.stamp.0,
            services.random.0,
        ];
        input.extend(&before);
        input.push((refs.len() / 17) as u32);
        input.extend(refs);
        assert!(
            foreign
                .commit_animation_selection(
                    s,
                    owner,
                    number,
                    commit::Frame {
                        selection,
                        materials: None
                    },
                    services,
                    &mut world
                )
                .is_err()
        );
        assert!(
            factory
                .commit_animation_selection(
                    s,
                    owner,
                    number + 1,
                    commit::Frame {
                        selection,
                        materials: None
                    },
                    services,
                    &mut world
                )
                .is_err()
        );
        assert_eq!(before, super::motion_tests::snapshot(s, owner));
        assert_eq!(world.calls, 0);
        if resolved {
            assert!(
                foreign
                    .commit_resolved_actions(s, owner, number, None, services, &mut world)
                    .is_err()
            );
            assert!(
                factory
                    .commit_resolved_actions(s, owner, number + 1, None, services, &mut world)
                    .is_err()
            );
            assert_eq!(before, super::motion_tests::snapshot(s, owner));
        }
        let result = if resolved {
            factory.commit_resolved_actions(s, owner, number, None, services, &mut world)
        } else {
            factory.commit_animation_selection(
                s,
                owner,
                number,
                commit::Frame {
                    selection,
                    materials: None,
                },
                services,
                &mut world,
            )
        };
        self.cases += 1;
        if matches!(kind, Case::CommitClear) {
            assert!(result.is_err());
            assert!(s.get(owner).is_none());
            assert_eq!(s.counts().residents, 0);
            return;
        }
        let g = s.kabuto_group(owner).unwrap();
        assert_eq!(g.animation_commit_entered, Some(number));
        assert!(!g.animation_commit_running);
        assert_eq!(g.motion_history, Some(number));
        assert_eq!(g.pending.len(), 4);
        if matches!(kind, Case::CommitFailure) {
            assert!(result.unwrap_err().contains("late animation notification"));
            assert!(g.geometry_transition_failure.is_some());
            assert_eq!(g.animation_commit_completed, None);
            assert_eq!(g.motion.mode, 99);
            assert_eq!(
                u32::from_le_bytes(
                    s.get(owner).unwrap().seed.storage.data().unwrap()[0x90..0x94]
                        .try_into()
                        .unwrap()
                ),
                123
            );
        } else {
            let view = result.unwrap_or_else(|e| panic!("commit context {index}: {e}"));
            assert_eq!(g.animation_commit_completed, Some(number));
            assert_eq!(g.motion.mode, selection.behavior);
            assert!(view.animation_processed);
            let old = s
                .specification(owner)
                .unwrap()
                .animations
                .get(&(captured.slot as u8))
                .unwrap();
            let expected = crate::animation_frames::frame_at_time(
                crate::animation_frames::Descriptor {
                    timeline: old.animation.timeline,
                    flags: old.template.flags,
                },
                f32::from_bits(s.get(owner).unwrap().channels.unwrap()[0][1]),
            )
            .unwrap();
            assert_eq!(view.current_frame, expected);
            assert_eq!(
                g.resource_phase_view
                    .unwrap()
                    .resources
                    .unwrap()
                    .previous_frame,
                captured.previous_frame
            );
            let p = s.get(owner).unwrap().seed.storage.data().unwrap();
            assert_eq!(&p[0x90..0x94], &selection.definition.to_le_bytes());
            assert_eq!(&p[0xe0..0xe4], &[0; 4]);
            let mut output = super::motion_tests::snapshot(s, owner);
            output.extend([services.stamp.0, services.random.0]);
            output.extend(references(s, owner));
            output.extend([
                view.current_frame as u32,
                u32::from(view.animation_processed),
                view.event_visits,
                view.stamp,
                world.calls as u32,
            ]);
            output.extend(&world.prefix);
            self.inputs.push_str(&line(input));
            self.outputs.push_str(&line(output));
        }
        assert!(
            factory
                .commit_animation_selection(
                    s,
                    owner,
                    number,
                    commit::Frame {
                        selection,
                        materials: None
                    },
                    services,
                    &mut world
                )
                .is_err()
        );
    }
    pub(super) fn finish(self) {
        if self.resolved {
            assert_eq!(self.cases + self.held + self.skipped, 512);
            assert!(self.cases > 0);
            println!(
                "kabuto_selection_gate committed={} held={} skipped={} primary_complete=false",
                self.cases, self.held, self.skipped
            );
        }
        if self.cases == 512 || self.resolved {
            let names = if self.resolved {
                [
                    "GIANTS_KABUTO_RESOLVED_COMMIT_INPUTS",
                    "GIANTS_KABUTO_RESOLVED_COMMIT_RECEIPT",
                ]
            } else {
                [
                    "GIANTS_KABUTO_COMMIT_INPUTS",
                    "GIANTS_KABUTO_COMMIT_RECEIPT",
                ]
            };
            for (name, text) in names.into_iter().zip([self.inputs, self.outputs]) {
                if let Some(path) = std::env::var_os(name) {
                    assert!(std::path::Path::new(&path).is_absolute());
                    std::fs::write(path, text).unwrap();
                }
            }
        }
    }
}
