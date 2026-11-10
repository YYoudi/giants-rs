use super::held_reference_tests::Case;
use super::*;
use crate::{
    audio_parameters::Random,
    kabuto_action_select::{self as select, Next, PacketWord, Scratch, Sound},
};

#[derive(Default)]
pub(super) struct Controls {
    definitions: Option<crate::kabuto_animation_cache::Definitions>,
    inputs: String,
    outputs: String,
    gate_inputs: String,
    gate_outputs: String,
    cases: usize,
}
fn words(s: Scratch) -> [u32; 10] {
    [
        s.packet_word,
        s.slot,
        s.retained_time,
        s.flags,
        s.clear_flags,
        s.behavior,
        s.definition,
        s.blend_seconds.to_bits(),
        s.reference_speed.to_bits(),
        s.speed_source as u32,
    ]
}
fn snapshot(s: &Session, owner: Id, scratch: Scratch, mode: u32, random: u32) -> Vec<u32> {
    let mut out = super::motion_tests::snapshot(s, owner);
    let state = s.kabuto_reference_state(owner).unwrap();
    let key = |id: Option<Id>| match id {
        None => 0,
        Some(id) if id == owner => 1,
        _ => panic!("actor outside supplied self-reference profile"),
    };
    out[193 + 0xc / 4] = key(state.reference);
    out[193 + 0x10 / 4] = key(state.held[0]);
    out[193 + 0x14 / 4] = key(state.held[1]);
    out.extend([1, mode, random]);
    out.extend(words(scratch));
    out
}
fn line(v: impl IntoIterator<Item = u32>) -> String {
    v.into_iter()
        .map(|w| format!("{w:08x}"))
        .collect::<Vec<_>>()
        .join(" ")
        + "\n"
}
struct World<'a> {
    factory: &'a crate::kabuto_initial::Factory,
    definitions: &'a crate::kabuto_animation_cache::Definitions,
    random: &'a mut Random,
    mode: u32,
    number: u64,
    kind: Case,
    scripts: [[u32; 19]; 4],
    trace: Vec<Vec<u32>>,
    prefixes: Vec<Vec<u32>>,
}
impl World<'_> {
    fn event(&mut self, s: &mut Session, owner: Id, kind: usize, id: u32) {
        let scratch = s
            .kabuto_group(owner)
            .unwrap()
            .action_select_scratch
            .unwrap();
        self.trace.push(vec![kind as u32, id]);
        let mut prefix = vec![kind as u32];
        prefix.extend(snapshot(s, owner, scratch, self.mode, self.random.0));
        self.prefixes.push(prefix);
        let q = self.scripts[kind];
        let scale = s.kabuto_reference_state(owner).unwrap().reference_scale;
        self.factory
            .publish_reference(s, owner, self.number, (q[1] != 0).then_some(owner), scale)
            .unwrap();
        let a = s.residents.get_mut(&owner.serial).unwrap();
        let g = a.kabuto_group.as_mut().unwrap();
        g.motion.model[1] = f32::from_bits(q[4]);
        g.motion.model[2] = f32::from_bits(q[5]);
        g.motion.commands[4..].copy_from_slice(&q[6].to_le_bytes());
        let p = a.seed.storage.data_mut().unwrap();
        for (at, v) in [0x15c, 0x158, 0x168, 0x16c, 0x174, 0x178, 0x17c]
            .into_iter()
            .zip(&q[8..15])
        {
            p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        self.mode = q[16];
        self.random.0 = q[17];
    }
}
impl select::Services for World<'_> {
    fn other_reference_present(&self, s: &Session, owner: Id) -> Result<bool, String> {
        let p = s
            .get(owner)
            .ok_or("owner absent")?
            .seed
            .storage
            .data()
            .ok_or("payload absent")?;
        Ok(u32::from_le_bytes(p[8..12].try_into().unwrap()) != 0)
    }
    fn definition_slot(&self, _: &Session, _: Id, key: u32) -> Result<u32, String> {
        self.definitions
            .entries()
            .get(key as usize)
            .map(|d| u32::from(d.animation))
            .ok_or_else(|| "source definition absent".into())
    }
    fn random(&mut self, _: &Session, _: Id, upper: u32) -> Result<u32, String> {
        assert!([3, 100].contains(&upper));
        Ok((self.random.next_15() * upper) >> 15)
    }
    fn sound(&mut self, s: &mut Session, owner: Id, kind: Sound, id: u32) -> Result<(), String> {
        // Exercise real reentry protection with separate scratch/services only
        // after the actual preceding caller stores have been published.
        let mut retry = s
            .kabuto_group(owner)
            .unwrap()
            .action_select_scratch
            .unwrap();
        let factory = self.factory;
        assert!(
            factory
                .select_actions(s, owner, self.number, &mut retry, self)
                .is_err()
        );
        self.event(
            s,
            owner,
            match kind {
                Sound::Start => 0,
                Sound::Stop => 1,
            },
            id,
        );
        if matches!(self.kind, Case::ActionsClear) {
            s.clear();
            Ok(())
        } else if matches!(self.kind, Case::ActionsFailure) {
            Err("declared late action sound failure".into())
        } else {
            Ok(())
        }
    }
    fn network<'a>(
        &'a self,
        _: &Session,
        _: Id,
    ) -> Result<crate::kabuto_geometry_mode::NotificationContext<'a>, String> {
        Ok(crate::kabuto_geometry_mode::NotificationContext {
            session_mode: self.mode,
            peer_ids: &[11, 22],
            local_peer: 11,
            owner_word: 0,
        })
    }
    fn notify(
        &mut self,
        s: &mut Session,
        owner: Id,
        kind: Sound,
        alias: PacketWord,
        packet: &mut u32,
    ) -> Result<(), String> {
        let scratch = s
            .kabuto_group(owner)
            .unwrap()
            .action_select_scratch
            .unwrap();
        assert_eq!(
            *packet,
            match alias {
                PacketWord::RetainedTime => scratch.retained_time,
                PacketWord::Temporary => scratch.packet_word,
            }
        );
        let k = match kind {
            Sound::Start => 2,
            Sound::Stop => 3,
        };
        self.event(s, owner, k, *packet);
        *packet = self.scripts[k][18];
        Ok(())
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
        random: &mut Random,
    ) {
        let (factory, foreign) = factories;
        let index = (number - 1) as usize;
        if self.definitions.is_none() {
            let path = std::env::var("GIANTS_KABUTO_EXECUTABLE")
                .expect("explicit matching definition-data provider required");
            self.definitions = Some(
                crate::kabuto_animation_cache::Definitions::read(std::path::Path::new(&path))
                    .unwrap(),
            );
        }
        let definitions = self.definitions.as_ref().unwrap();
        let scripts = std::array::from_fn(|k| {
            [
                0,
                u32::from(k % 2 == 1),
                0,
                0,
                [0f32, 40.][k % 2].to_bits(),
                0f32.to_bits(),
                [0x80u32, 0x100, 0x200, 0x400][(index + k) % 4]
                    | u32::from(
                        matches!(kind, Case::Tail | Case::TailFailure | Case::TailClear)
                            && index % 2 == 1,
                    ),
                0,
                4f32.to_bits(),
                2f32.to_bits(),
                10f32.to_bits(),
                30f32.to_bits(),
                20f32.to_bits(),
                30f32.to_bits(),
                40f32.to_bits(),
                1,
                if (index + k).is_multiple_of(2) { 4 } else { 0 },
                0x12345000 + index as u32 + k as u32,
                0x33445500 + index as u32 + k as u32,
            ]
        });
        let mut w = World {
            factory,
            definitions,
            random,
            mode: if index.is_multiple_of(2) { 4 } else { 0 },
            number,
            kind,
            scripts,
            trace: vec![],
            prefixes: vec![],
        };
        let mut scratch = Scratch {
            packet_word: 0,
            slot: u32::MAX,
            retained_time: 0.25f32.to_bits(),
            flags: 32,
            clear_flags: 0,
            behavior: 1,
            definition: u32::MAX,
            blend_seconds: 0.25,
            reference_speed: 0.,
            speed_source: -1,
        };
        let captured = s
            .kabuto_group(owner)
            .unwrap()
            .resource_phase_view
            .unwrap()
            .resources
            .unwrap();
        if !matches!(
            kind,
            Case::Actions | Case::ActionGate | Case::Tail | Case::TailFailure | Case::TailClear
        ) {
            assert_eq!(
                captured.activity & 3,
                0,
                "late-error fixture must reach genuine idle activity"
            );
        }
        let before = snapshot(s, owner, scratch, w.mode, w.random.0);
        let mut input = before[..407].to_vec();
        input.extend([
            captured.initial_mode,
            captured.activity,
            u32::from(captured.wrapped) * 2,
        ]);
        input.extend(words(scratch));
        input.extend(w.scripts.iter().flatten().copied());
        input.extend(
            [0x2f, 0x30, 0x31, 0x32, 0x35, 0x36]
                .map(|k| u32::from(definitions.entries()[k].animation)),
        );
        assert_eq!(input.len(), 502);
        assert!(
            foreign
                .select_actions(s, owner, number, &mut scratch, &mut w)
                .is_err()
        );
        assert!(
            factory
                .select_actions(s, owner, number + 1, &mut scratch, &mut w)
                .is_err()
        );
        assert_eq!(before, snapshot(s, owner, scratch, w.mode, w.random.0));
        let result = factory.select_actions(s, owner, number, &mut scratch, &mut w);
        self.cases += 1;
        if matches!(kind, Case::ActionsClear) {
            assert!(result.is_err());
            assert!(s.get(owner).is_none());
            assert_eq!(s.counts().residents, 0);
            return;
        }
        let g = s.kabuto_group(owner).unwrap();
        assert_eq!(g.action_select_entered, Some(number));
        assert!(!g.action_select_running);
        assert_eq!(g.motion_history, Some(number));
        assert_eq!(g.pending.len(), 4);
        assert!(g.animation_commit_entered.is_none());
        assert!(g.held_reference_entered.is_none());
        if matches!(kind, Case::ActionsFailure) {
            assert!(result.unwrap_err().contains("late action sound"));
            assert_eq!(g.action_select_completed, None);
            assert!(g.geometry_transition_failure.is_some());
            assert!(!w.trace.is_empty());
            assert_eq!(w.random.0, w.scripts[w.trace[0][0] as usize][17]);
        } else {
            let next = result.unwrap_or_else(|e| panic!("selection context{index}: {e}"));
            assert_eq!(g.action_select_completed, Some(number));
            assert_eq!(g.action_select_next, Some(next));
            assert_eq!(words(g.action_select_scratch.unwrap()), words(scratch));
            let mut output = vec![match next {
                Next::HeldReferences => 0,
                Next::CheckSelection => 1,
                Next::ChooseSelection => 2,
            }];
            output.extend(snapshot(s, owner, scratch, w.mode, w.random.0));
            for groups in [&w.trace, &w.prefixes] {
                output.push(groups.len() as u32);
                for item in groups {
                    output.push(item.len() as u32);
                    output.extend(item)
                }
            }
            self.inputs.push_str(&line(input.iter().copied()));
            self.outputs.push_str(&line(output));
            if matches!(
                kind,
                Case::ActionGate | Case::Tail | Case::TailFailure | Case::TailClear
            ) {
                assert!(foreign.resolve_actions(s, owner, number).is_err());
                assert!(factory.resolve_actions(s, owner, number + 1).is_err());
                let before_gate = snapshot(s, owner, scratch, w.mode, w.random.0);
                let continuation = factory.resolve_actions(s, owner, number).unwrap();
                let g = s.kabuto_group(owner).unwrap();
                let selected = g.action_select_scratch.unwrap();
                assert_eq!(g.action_gate_completed, Some(number));
                assert_eq!(g.action_gate_next, Some(continuation));
                assert_eq!(g.motion_history, Some(number));
                assert_eq!(g.pending.len(), 4);
                assert!(g.animation_commit_entered.is_none());
                assert_eq!(
                    &before_gate[..407],
                    &snapshot(s, owner, selected, w.mode, w.random.0)[..407]
                );
                assert!(factory.resolve_actions(s, owner, number).is_err());
                let mut gate_input = input;
                gate_input.push(captured.slot);
                self.gate_inputs.push_str(&line(gate_input));
                let mut output = vec![match continuation {
                    select::Continuation::HeldReferences => 0,
                    select::Continuation::Commit => 1,
                    select::Continuation::SkipCommit => 2,
                }];
                output.extend(snapshot(s, owner, selected, w.mode, w.random.0));
                for groups in [&w.trace, &w.prefixes] {
                    output.push(groups.len() as u32);
                    for item in groups {
                        output.push(item.len() as u32);
                        output.extend(item);
                    }
                }
                self.gate_outputs.push_str(&line(output));
            }
        }
        assert!(
            factory
                .select_actions(s, owner, number, &mut scratch, &mut w)
                .is_err()
        );
    }
    pub(super) fn finish(self) {
        if self.cases == 512 {
            for (name, text) in [
                ("GIANTS_KABUTO_ACTION_INPUTS", self.inputs),
                ("GIANTS_KABUTO_ACTION_RECEIPT", self.outputs),
                ("GIANTS_KABUTO_GATE_INPUTS", self.gate_inputs),
                ("GIANTS_KABUTO_GATE_RECEIPT", self.gate_outputs),
            ] {
                if let Some(path) = std::env::var_os(name) {
                    assert!(std::path::Path::new(&path).is_absolute());
                    std::fs::write(path, text).unwrap();
                }
            }
        }
    }
}
