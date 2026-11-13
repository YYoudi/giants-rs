use super::*;
fn candidate(position: [f32; 3], data: Data<u32>) -> Candidate<u32> {
    let reaper = matches!(data, Data::Reaperman { .. });
    Candidate {
        class: if reaper { 215 } else { 230 },
        state: if reaper { 51 } else { 23 },
        mode: 103,
        control_flags: 0,
        position,
        data,
    }
}
fn reaper(target: Option<u32>) -> Data<u32> {
    Data::Reaperman {
        opaque_0: 0,
        target,
        flags_1b8: 0,
        opaque_1f0: 0,
        opaque_228: 0,
    }
}
#[test]
fn horizontal_alerts_keep_distinct_edges_existing_targets_and_mode_gates() {
    let mut actors = [
        candidate([70., 0., 0.], Data::Ripper { target: None }),
        candidate([69., 0., 10000.], Data::Ripper { target: None }),
        candidate([69., 0., 0.], Data::Ripper { target: Some(42) }),
        candidate([70., 0., 10000.], reaper(Some(42))),
        candidate([1., 0., 0.], reaper(Some(42))),
    ];
    actors[4].mode = 102;
    alert_team_from_building([0.; 3], 70., 7, &mut actors).unwrap();
    alert_rippers([0.; 3], 70., 7, &mut actors).unwrap();
    assert!(matches!(actors[0].data, Data::Ripper { target: None }));
    assert!(matches!(actors[1].data, Data::Ripper { target: Some(7) }));
    assert!(matches!(actors[2].data, Data::Ripper { target: Some(42) }));
    assert!(matches!(
        actors[3].data,
        Data::Reaperman {
            target: Some(7),
            ..
        }
    ));
    assert_eq!(actors[3].control_flags, 0x21);
    assert!(matches!(
        actors[4].data,
        Data::Reaperman {
            target: Some(42),
            ..
        }
    ));
    assert_eq!(actors[4].control_flags, 0);
    actors[4].data = Data::Other;
    actors[1].data = Data::Ripper { target: None };
    assert!(alert_rippers([0.; 3], 70., 7, &mut actors).is_err());
    assert!(matches!(actors[1].data, Data::Ripper { target: None }));
}
#[derive(Default)]
struct Recorder {
    events: Vec<String>,
    fail_at_remaining: Option<i32>,
}
impl Services<u32> for Recorder {
    fn alert_team(&mut self, s: &State<u32>, _: f32) -> Result<(), String> {
        self.events.push(format!(
            "team:{}:{}:{}",
            s.phase,
            s.damage,
            s.target.unwrap()
        ));
        Ok(())
    }
    fn alert_rippers(&mut self, _: &State<u32>, _: f32) -> Result<(), String> {
        self.events.push("rippers".into());
        Ok(())
    }
    fn object_sound(&mut self, _: u32) -> Result<(), String> {
        self.events.push("hit sound".into());
        Ok(())
    }
    fn explosion(&mut self, kind: Explosion, _: [f32; 3]) -> Result<(), String> {
        self.events.push(kind.name().into());
        Ok(())
    }
    fn positional_sound(&mut self, _: u32, _: [f32; 3]) -> Result<(), String> {
        self.events.push("death sound".into());
        Ok(())
    }
    fn mission_trigger(&mut self, s: &State<u32>) -> Result<(), String> {
        self.events.push(format!("mission:{}", s.remaining));
        if self.fail_at_remaining == Some(s.remaining) {
            return Err("required mission consumer unavailable".into());
        }
        Ok(())
    }
    fn retire(&mut self) -> Result<(), String> {
        self.events.push("retire".into());
        Ok(())
    }
}
#[test]
fn lethal_hit_preserves_source_order_unclamped_damage_and_failed_prefix() {
    let mut state = State {
        position: [1., 2., 3.],
        target: Some(42),
        phase: 1,
        selector: 0,
        damage: 39.,
        limit: 40.,
        mark_flags: 2,
        remaining: 3,
    };
    let mut services = Recorder {
        fail_at_remaining: Some(2),
        ..Default::default()
    };
    assert!(hit(&mut state, 7., None, 7, 50., &mut services).is_err());
    assert_eq!(state.damage, 46.);
    assert_eq!(state.remaining, 2);
    assert_eq!(
        services.events,
        [
            "team:2:39:7",
            "rippers",
            "fx_minibarracks",
            "death sound",
            "mission:3",
            "mission:2"
        ]
    );
    let prefix = services.events.clone();
    assert!(!hit(&mut state, 7., None, 7, 50., &mut services).unwrap());
    assert_eq!(services.events, prefix);
}
