use super::*;

fn state() -> State<u32, u32> {
    State {
        action_available: 17,
        item_available: 18,
        movement_ready: 19,
        point_available: 20,
        highlighted: Some(2),
        item_target: Some(2),
        terrain_selected: 22,
        cooldown: 0.,
        controls: 8,
        proxy: Some(1),
        candidate: Some(2),
        point: [-1.; 3],
        terrain_point: [-2.; 3],
        aim_point: [-3.; 3],
        pending: Some(2),
    }
}
fn input() -> Input {
    Input {
        local_owner: true,
        response_flags: 0,
        suppressed: false,
        animation: 0,
        selected: 1,
        selected_kind: 0x2c,
        selected_item: 2,
        item_kind: 0x17,
        pending_seconds: 0.,
        position: [0.; 3],
        scale: 1.,
        reference_scale: 1.,
        height_limit: 5.,
        reserve: 1.,
        readiness: readiness::Parameters {
            fallback_rate: 1.,
            exponent: 0.25,
        },
    }
}
#[derive(Default)]
struct Services {
    linked: bool,
    terrain_miss: bool,
    terrain_failure: bool,
    linked_failure: bool,
    item_failure: bool,
    hidden: bool,
    calls: Vec<&'static str>,
}
impl World<u32, u32> for Services {
    fn seconds(&mut self) -> Result<f64, String> {
        self.calls.push("seconds");
        Ok(0.125)
    }
    fn hide_proxy(&mut self, _: u32) -> Result<(), String> {
        self.calls.push("hide");
        self.hidden = true;
        Ok(())
    }
    fn projectile_range(&mut self, definition: i32) -> Result<f32, String> {
        assert!(matches!(definition, 0x2b | 0x17));
        self.calls.push("range");
        Ok(100.)
    }
    fn viewport_route(&mut self) -> Result<bool, String> {
        self.calls.push("view");
        Ok(false)
    }
    fn terrain(&mut self, _: bool, _: f32, point: &mut [f32; 3]) -> Result<Terrain, String> {
        self.calls.push("terrain");
        *point = [10., 20., 30.];
        if self.terrain_failure {
            return Err("terrain late failure".into());
        }
        Ok(Terrain {
            hit: !self.terrain_miss,
            distance: 100.5,
        })
    }
    fn query(
        &mut self,
        _: bool,
        distance: f32,
        point: &mut [f32; 3],
    ) -> Result<Option<u32>, String> {
        self.calls.push("query");
        assert_eq!(distance, 100.5);
        *point = if self.linked { [7., 8., 9.] } else { [0.; 3] };
        Ok(self.linked.then_some(1))
    }
    fn eligible(&mut self, _: u32) -> Result<bool, String> {
        self.calls.push("eligible");
        Ok(true)
    }
    fn personality(&mut self, _: u32) -> Result<u32, String> {
        self.calls.push("personality");
        Ok(0x400000)
    }
    fn camera_fallback(&mut self) -> Result<[f32; 3], String> {
        self.calls.push("camera");
        Ok([100.; 3])
    }
    fn pending(&mut self) -> Result<Option<u32>, String> {
        self.calls.push("pending");
        Ok(None)
    }
    fn item_eligible(&mut self, _: i32) -> Result<bool, String> {
        self.calls.push("item-eligible");
        Ok(true)
    }
    fn item_available(
        &mut self,
        _: i32,
        _: [f32; 3],
        target: &mut Option<u32>,
    ) -> Result<u32, String> {
        self.calls.push("item-available");
        *target = Some(2);
        if self.item_failure {
            return Err("item late failure".into());
        }
        Ok(7)
    }
    fn linked(&mut self, value: &mut Linked<u32, u32>) -> Result<(), String> {
        self.calls.push("linked");
        value.target = 2;
        value.proxy = Some(2);
        value.prefix.fallback = [40., 50., 60.];
        value.prefix.readiness = 1;
        if self.linked_failure {
            Err("linked late failure".into())
        } else {
            Ok(())
        }
    }
    fn physics_blocks_action(&mut self) -> Result<bool, String> {
        self.calls.push("physics");
        Ok(false)
    }
    fn busy(&mut self) -> Result<bool, String> {
        self.calls.push("busy");
        Ok(false)
    }
    fn class(&mut self, _: u32) -> Result<i32, String> {
        self.calls.push("class");
        Ok(11)
    }
    fn flags(&mut self, _: u32) -> Result<u32, String> {
        self.calls.push("flags");
        Ok(0)
    }
    fn highlight_in_range(&mut self, _: u32, _: i32, _: i32) -> Result<bool, String> {
        self.calls.push("highlight-range");
        Ok(true)
    }
}

#[test]
fn terrain_query_and_display_points_keep_distinct_lifetimes() {
    let mut state = state();
    let mut world = Services::default();
    advance(&mut state, input(), &mut world).unwrap();
    assert_eq!(state.terrain_point, [10., 20., 30.]);
    assert_eq!(state.aim_point, state.terrain_point);
    assert_eq!(state.point, [0.; 3]);
    assert_eq!(state.movement_ready, 1); // Cost uses query point, not display aim.
    assert_eq!(state.terrain_selected, 1);
    assert_eq!(state.candidate, None);
    assert_eq!(state.pending, None);
    assert_eq!(state.item_available, 7);
    assert_eq!(state.item_target, Some(2));
    assert!(world.hidden);
    assert_eq!(
        world.calls,
        [
            "hide",
            "range",
            "view",
            "terrain",
            "view",
            "query",
            "pending",
            "item-eligible",
            "item-available",
            "physics",
            "busy"
        ]
    );
}

#[test]
fn partial_terrain_and_linked_writes_survive_errors_without_recomputing_aim() {
    let mut state = state();
    state.cooldown = 0.5;
    let mut world = Services {
        terrain_failure: true,
        ..Default::default()
    };
    assert!(advance(&mut state, input(), &mut world).is_err());
    assert_eq!(state.cooldown, 0.375);
    assert_eq!(state.controls, 8);
    assert_eq!(state.terrain_point, [10., 20., 30.]);
    assert_eq!(state.point, [-1.; 3]);
    assert_eq!(state.candidate, Some(2));
    assert_eq!(state.movement_ready, 0);
    assert_eq!(state.highlighted, None);
    assert!(world.hidden);
    assert_eq!(world.calls.last(), Some(&"terrain"));

    let mut state = super::tests::state();
    let mut world = Services {
        linked: true,
        linked_failure: true,
        ..Default::default()
    };
    assert!(advance(&mut state, input(), &mut world).is_err());
    assert_eq!(state.point, [40., 50., 60.]);
    assert_eq!(state.candidate, Some(2));
    assert_eq!(state.proxy, Some(2));
    assert_eq!(state.movement_ready, 1);
    assert_eq!(state.terrain_point, [7., 8., 9.]);
    assert_eq!(state.aim_point, [7., 8., 9.]);
    assert_eq!(state.highlighted, Some(1));
    assert_eq!(state.item_available, 0);
    assert_eq!(world.calls.last(), Some(&"linked"));

    let mut state = super::tests::state();
    let mut world = Services {
        item_failure: true,
        ..Default::default()
    };
    assert!(advance(&mut state, input(), &mut world).is_err());
    assert_eq!(state.item_target, Some(2));
    assert_eq!(state.item_available, 0);
    assert_eq!(state.movement_ready, 1);
    assert_eq!(state.aim_point, [10., 20., 30.]);
    assert_eq!(world.calls.last(), Some(&"item-available"));
}

#[test]
fn inline_readiness_keeps_wide_squared_distance_unlike_linked_helper() {
    let mut state = state();
    state.movement_ready = 0;
    state.point = [1., 2_f32.powi(-13), 0.];
    let input = input();
    fallback(&mut state, input).unwrap();
    assert_eq!(state.movement_ready, 0);
    let mut linked = readiness::State {
        fallback: [0.; 3],
        readiness: 0,
    };
    assert!(
        readiness::prepare(
            &mut linked,
            input.position,
            state.point,
            input.reserve,
            input.readiness
        )
        .unwrap()
    );
    assert_eq!(linked.readiness, 1);
}

#[test]
fn nonlocal_owner_keeps_state_and_uses_no_providers() {
    let mut state = state();
    let mut input = input();
    input.local_owner = false;
    let mut world = Services::default();
    advance(&mut state, input, &mut world).unwrap();
    assert_eq!(state.movement_ready, 19);
    assert_eq!(state.cooldown, 0.);
    assert_eq!(state.controls, 8);
    assert_eq!(state.highlighted, Some(2));
    assert_eq!(state.aim_point, [-3.; 3]);
    assert!(world.calls.is_empty());
}

#[test]
fn inactive_special_search_retains_pending_and_class_gate_reads_both_words() {
    let mut s = state();
    let mut i = input();
    i.selected_kind = 0x2f;
    let mut world = Services {
        terrain_miss: true,
        ..Default::default()
    };
    advance(&mut s, i, &mut world).unwrap();
    assert_eq!(s.pending, Some(2));
    assert_eq!(s.action_available, 1);
    assert_eq!(s.aim_point, [100.; 3]);
    assert!(!world.calls.contains(&"pending"));

    let mut s = state();
    let mut world = Services {
        linked: true,
        ..Default::default()
    };
    advance(&mut s, input(), &mut world).unwrap();
    assert_eq!(s.action_available, 0);
    assert!(world.calls.ends_with(&["class", "personality", "flags"]));
    assert!(!world.calls.contains(&"highlight-range"));
}
