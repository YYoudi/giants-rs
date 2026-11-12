//! Source declaration coverage, not ordinary encounter acceptance.
use super::*;

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; declaration admission across authored Story2 scenarios, no live AI"]
fn source_ripper_declarations_follow_each_authored_scenario() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let world = giants_game::world_source::resolve(&root, "Story2").unwrap();
    let menu = giants_game::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin").join(world.catalog)).unwrap(),
        u32::MAX,
        true,
    )
    .unwrap();
    for (index, expected) in [
        (0, (7, 4)),
        (1, (4, 1)),
        (2, (7, 1)),
        (3, (2, 0)),
        (4, (2, 1)),
    ] {
        let request = menu.request("Story2", index).unwrap();
        let packet = giants_game::mission_loading::prepare(&root, request).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let ticket = 9200 + index as u64;
        let mut session = Session::new(ticket);
        let report = giants_game::declared_residents::construct_acquired(&plan, &mut session);
        let mut component = State::prepare(&plan).unwrap().unwrap();
        let view = component.bind(ticket, &report, &session).unwrap();
        assert_eq!((view.class73, view.class230), expected);
        assert_eq!(
            session
                .list(0)
                .iter()
                .filter(|id| {
                    session
                        .get(**id)
                        .is_some_and(|actor| matches!(actor.class, 73 | 230))
                })
                .count(),
            expected.0 + expected.1
        );
        session.clear();
        assert!(component.view(&session, ticket).is_err());
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; exact source mission declarations and constructor identity only"]
fn source_wife_ripper_bindings_use_declared_owners_and_survive_no_ticket_reuse() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let world = giants_game::world_source::resolve(&root, "Story2").unwrap();
    let menu = giants_game::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin").join(world.catalog)).unwrap(),
        u32::MAX,
        true,
    )
    .unwrap();
    let request = menu.request("Story2", 1).unwrap();
    assert_eq!(request.source_id, 1);
    assert_eq!(request.mission_name, "2b_1_savewife");
    let packet = giants_game::mission_loading::prepare(&root, request).unwrap();
    let plan = packet.instance_plan(&root, "English").unwrap();
    let world = packet.world_asset(&root).unwrap();
    let mut old: Option<State> = None;
    for ticket in [9101, 9102] {
        let mut session = Session::new(ticket);
        let mut report = giants_game::declared_residents::construct_acquired(&plan, &mut session);
        let mut state = State::prepare(&plan).unwrap().unwrap();
        assert_eq!(state.expected.len(), 5);
        assert!(state.expected.iter().all(|d| d.key.scope == Scope::Mission));
        let view = state.bind(ticket, &report, &session).unwrap();
        assert_eq!(
            view,
            View {
                class73: 4,
                class230: 1
            }
        );
        assert!(state.bind(ticket, &report, &session).is_err());
        assert!(state.view(&session, ticket + 1).is_err());
        if let Some(previous) = &old {
            assert!(previous.view(&session, ticket).is_err());
        }
        state
            .initialize(&mut session, ticket, plan.resources(), &world)
            .unwrap();
        assert!(
            state
                .initialize(&mut session, ticket, plan.resources(), &world)
                .is_err()
        );
        for owner in &state.bound.as_ref().unwrap().1 {
            let fields = session
                .get(owner.id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            let declaration = plan.entries[owner.declaration.key.index]
                .source
                .as_ref()
                .unwrap();
            assert_eq!(
                fields.position.map(f32::to_bits),
                declaration.position.map(f32::to_bits)
            );
            assert_eq!(fields.angles[0].to_bits(), declaration.yaw.to_bits());
            assert!(session.ripper_primary(owner.id).is_err());
            assert_eq!(
                session.actor_finalization(owner.id).unwrap().phase(),
                giants_game::resident_finalization::Phase::Idle
            );
            assert!(!session.actor_motion(owner.id).unwrap().awaiting_motion());
        }
        // Replacing a report ID by another real Ripper must not be accepted.
        let first = state.expected[0].key;
        let other = state.bound.as_ref().unwrap().1[1].id;
        report
            .records
            .iter_mut()
            .find(|r| r.key == first)
            .unwrap()
            .result = Ok(other);
        let mut bad = State::prepare(&plan).unwrap().unwrap();
        assert!(bad.bind(ticket, &report, &session).is_err());
        assert!(bad.bound.is_none());
        session.clear();
        assert!(state.view(&session, ticket).is_err());
        old = Some(state);
    }
}
