//! Real process/configuration fields, scoped by the current parsed source
//! scenario and actual retirement receipts. No portable completion snapshot.
use super::*;
use giants_game::mission_guard_state::Scope;
impl Runtime {
    pub(super) fn bind_normal_combat_globals(&mut self, ticket: u64) -> Result<(), String> {
        let a = self
            .active
            .as_ref()
            .filter(|a| a.ticket == ticket)
            .ok_or("combat source mission absent")?;
        let owner = a.player_owner()?;
        if a.owned_mecc.is_some() || a.controlled_mecc_owner != Some(owner) {
            return Err("combat process requires actual ordinary controlled Mecc".into());
        }
        let c = a
            .campaign
            .as_ref()
            .ok_or("combat source objective owner absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let packet = &a.payload.packet;
        let world = packet.world.as_ref().ok_or("combat source world absent")?;
        let mission = packet
            .mission
            .as_ref()
            .ok_or("combat source mission stream absent")?;
        let scenario = packet
            .scenario_index
            .ok_or("combat source scenario identity absent")?;
        let record = world
            .scenarios
            .get(scenario)
            .ok_or("combat source scenario record absent")?;
        if record.kind != 2
            || record.name != packet.request.mission_name
            || c.program.scenario_index != scenario
        {
            return Err("combat source scenario differs from its actual objective owner".into());
        }
        let slot = giants_game::mission_slot::index(world, &packet.request.mission_name)?;
        // Command18 is a separate native failure-timer writer. The present
        // mission reader rejects it; retain the barrier if support is added.
        if mission.commands.iter().any(|&(_, op)| op == 18) {
            return Err("combat requires actual timed-failure producer".into());
        }
        let reached_completion = (c.progress.counter as i32) >= c.program.required;
        let scope = Scope {
            ticket,
            scenario,
            slot,
        };
        let process = self
            .normal_combat_globals
            .as_mut()
            .ok_or("combat process requires actual launch configuration owner")?;
        if let Some(previous) = process.scope()
            && self.retired.contains(&previous.ticket)
        {
            process.leave_scenario(previous.ticket)?;
        }
        match process.scope() {
            Some(previous) if previous != scope => {
                return Err("combat process still belongs to another live source scenario".into());
            }
            None => process.begin_scenario(scope)?,
            _ => {}
        }
        if reached_completion {
            process.require_unbound_transition(
                ticket,
                "source completion precheck/clock/health-suppression writer",
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_owner_is_moved_once_and_absence_never_creates_clear_guards() {
        let mut app = App::new();
        app.insert_resource(crate::startup_configuration::Configuration::begin());
        app.add_plugins(super::super::Plugin);
        assert!(
            app.world()
                .resource::<Runtime>()
                .normal_combat_globals
                .is_some()
        );
        assert!(
            app.world_mut()
                .resource_mut::<crate::startup_configuration::Configuration>()
                .take_combat_process()
                .is_none()
        );
        let mut absent = App::new();
        absent.add_plugins(super::super::Plugin);
        assert!(
            absent
                .world()
                .resource::<Runtime>()
                .normal_combat_globals
                .is_none()
        );
    }
}
