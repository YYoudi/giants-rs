use super::*;
use crate::kabuto_resource_phase::{Clock, Network, Request, Services};
struct World {
    seconds: f32,
    clocks: Vec<u32>,
    events: Vec<u32>,
    fail: bool,
    clear: bool,
    dead: bool,
    guard_timer: f32,
}
fn snapshot(s: &Session, id: Id) -> Vec<u32> {
    super::super::motion_tests::snapshot(s, id)
}
impl Services for World {
    fn local_owner(&mut self, _: &Session, _: Id) -> Result<bool, String> {
        assert!(self.dead, "cold damage must skip local guard");
        Ok(true)
    }
    fn timer(&mut self, _: &Session) -> Result<f32, String> {
        assert!(self.dead);
        Ok(self.guard_timer)
    }
    fn global_seconds(&mut self, _: &Session) -> Result<f32, String> {
        assert!(self.dead);
        Ok(self.seconds)
    }
    fn store_timer(&mut self, _: &Session, value: f32) -> Result<(), String> {
        assert!(self.dead);
        self.guard_timer = value;
        Ok(())
    }
    fn mode(&mut self, _: &Session) -> Result<u32, String> {
        assert!(self.dead);
        Ok(1)
    }
    fn clock(&mut self, s: &Session, id: Id) -> Result<Clock, String> {
        self.clocks.extend(snapshot(s, id));
        Ok(Clock {
            global_seconds: self.seconds,
            areas: vec![],
        })
    }
    fn network(&mut self, _: &Session, _: Id) -> Result<Network, String> {
        Ok(Network {
            session_mode: 4,
            peer_ids: vec![11, 22],
            local_peer: 11,
            owner_word: 0,
        })
    }
    fn execute(&mut self, s: &mut Session, id: Id, request: Request) -> Result<(), String> {
        let (kind, value) = match request {
            Request::StartSound(id) => (0, id),
            Request::NotifySound(id) => (1, id),
            Request::NotifyGrowth(v) => (3, u32::from(v)),
            Request::Reinitialize(class) => (2, class),
        };
        self.events.extend([kind, value]);
        self.events.extend(snapshot(s, id));
        if self.clear {
            s.clear();
            return Ok(());
        }
        if matches!(request, Request::StartSound(_)) {
            let a = s.residents.get_mut(&id.serial).unwrap();
            let f = a.construction_fields.as_mut().unwrap();
            let limit = f32::from_bits(f.configuration[12]);
            a.mecc_damage = Some(limit * 0.25);
            f.configuration[12] = (limit * 2.).to_bits();
            a.kabuto_group.as_mut().unwrap().motion.commands[4] ^= 0x10;

            // Declared entry-provider mutation, visible to later clock/transport
            // boundaries. This is not execution of the original sound backend.
            s.residents
                .get_mut(&id.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap()[0x128..0x12c]
                .copy_from_slice(&0.25f32.to_le_bytes());
        }
        if matches!(request, Request::Reinitialize(_)) {
            Err("required lifecycle service unavailable".into())
        } else if self.fail {
            Err("required source sound failed".into())
        } else {
            Ok(())
        }
    }
}
#[derive(Default)]
pub(super) struct Controls {
    inputs: String,
    outputs: String,
    cases: usize,
}
impl Controls {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn advance(
        &mut self,
        factory: &crate::kabuto_initial::Factory,
        foreign: &crate::kabuto_initial::Factory,
        s: &mut Session,
        id: Id,
        number: u64,
        seconds: f32,
        globals: &giants_formats::global_data::GlobalData,
    ) {
        let before = snapshot(s, id);
        let slot = s.get(id).unwrap().channels.unwrap()[0][0] as u8;
        let spec = s.specification(id).unwrap();
        let descriptor = spec.animations.get(&slot).unwrap();
        let mut input = [
            0x9e8, 0x10bc, 0x10b8, 0x10b4, 0x10b0, 0x11a0, 0x11a4, 0x11a8, 0x9e0, 0x9e4, 0x9ec,
            0x9f0,
        ]
        .map(|at| globals.words[at / 4])
        .to_vec();
        input.extend(descriptor.animation.timeline.map(|v| v as u32));
        input.extend([seconds.to_bits(), 4, 11, 0]);
        input.extend(&before);
        self.inputs.push_str(&words_line(input));
        let mut w = World {
            seconds,
            clocks: vec![],
            events: vec![],
            fail: false,
            clear: false,
            dead: false,
            guard_timer: 0.,
        };
        assert!(foreign.advance_resources(s, id, number, &mut w).is_err());
        assert!(
            factory
                .advance_resources(s, id, number + 1, &mut w)
                .is_err()
        );
        assert_eq!(before, snapshot(s, id));
        assert!(w.clocks.is_empty() && w.events.is_empty());
        let view = factory.advance_resources(s, id, number, &mut w).unwrap();
        let r = view.resources.unwrap();
        let mut output = snapshot(s, id);
        output.extend(view.guard.forward.map(f32::to_bits));
        output.extend([
            r.previous_frame as u32,
            r.current_frame as u32,
            r.activity,
            r.initial_mode,
            r.previous_selection,
            r.slot,
            u32::from(r.reverse),
            u32::from(r.wrapped),
            r.charge.to_bits(),
            r.interval.to_bits(),
            (w.clocks.len() / 404) as u32,
            w.events.len() as u32,
        ]);
        output.extend(&w.events);
        output.extend(&w.clocks);
        self.outputs.push_str(&words_line(output));
        self.cases += 1;
        let g = s.kabuto_group(id).unwrap();
        assert_eq!(g.resource_phase_entered, Some(number));
        assert_eq!(g.resource_phase_completed, Some(number));
        assert!(g.resource_phase_view.unwrap().resources.is_some());
        assert_eq!(g.motion_history, Some(number));
        assert_eq!(g.pending.len(), 4);
        assert!(factory.advance_resources(s, id, number, &mut w).is_err());
        assert!(
            s.begin_kabuto_motion_history(id, number + 1, [0; 8])
                .is_err()
        );
        if self.cases == 512 {
            // Separate controlled phase entry tests the real late failure path;
            // it is not part of the successful original interval comparison.
            let a = s.residents.get_mut(&id.serial).unwrap();
            let p = a.seed.storage.data_mut().unwrap();
            p[0x12c..0x130].copy_from_slice(&1u32.to_le_bytes());
            p[0x128..0x12c].copy_from_slice(&0f32.to_le_bytes());
            let g = a.kabuto_group.as_mut().unwrap();
            g.motion.model[1] = 1.;
            g.resource_phase_entered = None;
            g.resource_phase_completed = None;
            w.fail = true;
            assert!(factory.advance_resources(s, id, number, &mut w).is_err());
            let a = s.get(id).unwrap();
            let p = a.seed.storage.data().unwrap();
            assert_eq!(
                f32::from_le_bytes(p[0x128..0x12c].try_into().unwrap()),
                0.25
            );
            assert_eq!(u32::from_le_bytes(p[0x12c..0x130].try_into().unwrap()), 1);
            assert!(
                a.kabuto_group
                    .as_ref()
                    .unwrap()
                    .geometry_transition_failure
                    .is_some()
            );
            assert_eq!(
                a.kabuto_group.as_ref().unwrap().resource_phase_completed,
                None
            );
            assert!(factory.advance_resources(s, id, number, &mut w).is_err());
            let a = s.residents.get_mut(&id.serial).unwrap();
            let f = a.construction_fields.as_mut().unwrap();
            a.mecc_damage = Some(f32::from_bits(f.configuration[12]));
            f.object_flags |= 0x80000;
            let g = a.kabuto_group.as_mut().unwrap();
            g.motion.commands[4..].copy_from_slice(&4u32.to_le_bytes());
            g.geometry_transition_failure = None;
            g.resource_phase_entered = None;
            g.resource_phase_view = None;
            w.fail = false;
            w.dead = true;
            w.guard_timer = 3.;
            w.clocks.clear();
            let event_start = w.events.len();
            let error = factory
                .advance_resources(s, id, number, &mut w)
                .unwrap_err();
            assert!(error.contains("lifecycle service unavailable"), "{error}");
            assert!(w.clocks.is_empty());
            assert!(
                w.events[event_start..]
                    .as_chunks::<406>()
                    .0
                    .iter()
                    .any(|e| e[0] == 2)
            );
            assert_eq!(w.guard_timer, (3f64 + f64::from(seconds)) as f32);
            assert_eq!(s.kabuto_group(id).unwrap().resource_phase_completed, None);
            assert!(factory.advance_resources(s, id, number, &mut w).is_err());
            let a = s.residents.get_mut(&id.serial).unwrap();
            let p = a.seed.storage.data_mut().unwrap();
            p[0x128..0x12c].copy_from_slice(&0f32.to_le_bytes());
            p[0x12c..0x130].copy_from_slice(&1u32.to_le_bytes());
            let g = a.kabuto_group.as_mut().unwrap();
            g.geometry_transition_failure = None;
            g.resource_phase_entered = None;
            w.fail = false;
            w.dead = true;
            w.clear = true;
            assert!(factory.advance_resources(s, id, number, &mut w).is_err());
            assert!(s.get(id).is_none());
            assert_eq!(s.counts().residents, 0);
        }
    }
    pub(super) fn finish(self) {
        assert_eq!(self.cases, 512);
        if let Some(p) = std::env::var_os("GIANTS_KABUTO_RESOURCE_INPUTS") {
            std::fs::write(p, self.inputs).unwrap();
        }
        if let Some(p) = std::env::var_os("GIANTS_KABUTO_RESOURCE_RECEIPT") {
            std::fs::write(p, self.outputs).unwrap();
        }
    }
}

pub(super) fn refuse_failed_animation(
    factory: &crate::kabuto_initial::Factory,
    s: &mut Session,
    id: Id,
    number: u64,
) {
    let mut w = World {
        seconds: 0.02,
        clocks: vec![],
        events: vec![],
        fail: false,
        clear: false,
        dead: false,
        guard_timer: 0.,
    };
    let before = snapshot(s, id);
    assert!(factory.advance_resources(s, id, number, &mut w).is_err());
    assert_eq!(before, snapshot(s, id));
    assert!(w.clocks.is_empty() && w.events.is_empty());
}
