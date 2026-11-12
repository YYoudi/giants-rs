//! Authored selector3 / state18 actors before ordinary Mecc pickup.
//! Prototype routes: `smartie_move`, `smartie_check_follow__FP6OBJECTf`,
//! `smartie_board_player__FP6OBJECTT0i`. Only proximity is probably attributed;
//! construction/pickup relationships remain unknown. No general Smartie AI claim.
use crate::{
    actor_fields::Fields,
    carried_actor,
    declared_residents::{Key, Scope},
    instances::Plan,
    resident_states::{Construction, Id, Session},
    specification, state_resources,
    state_storage::Profile,
};
use std::sync::Arc;

pub struct Prepared {
    pub key: Key,
    pub source_offset: usize,
    pub carry: Arc<carried_actor::Prepared>,
    fields: Fields,
    initial: crate::uncached_pose::Pose,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    flags: crate::object_lists::DirectFlags,
    writes: Vec<giants_formats::mission::StateWriteKind>,
}

impl Prepared {
    pub fn mission(plan: &Plan) -> Result<Vec<Self>, String> {
        let mut out = Vec::new();
        for (index, entry) in plan.entries.iter().enumerate() {
            let Ok(source) = &entry.source else { continue };
            if source.mission_selector != Some(3)
                || !source
                    .state_seed
                    .as_ref()
                    .is_ok_and(|s| s.storage.state() == 18)
            {
                continue;
            }
            if source.mission_word_174.is_some() || source.mission_values_f4.is_some() {
                return Err("declared Smartie requires additional placement services".into());
            }
            let mut carry = Arc::new(
                carried_actor::Prepared::load(plan.resources(), source.class)
                    .map_err(|e| e.to_string())?,
            );
            let specification = Arc::new(specification::Prepared::load_registered(
                plan.resources(),
                source.class,
            )?);
            let states = Arc::new(state_resources::Prepared::load(
                plan.resources(),
                source.class,
                Profile::Debug,
            )?);
            if !specification.needs.is_empty() || !states.needs().is_empty() {
                return Err("declared Smartie resources have unresolved dependencies".into());
            }
            let mut fields = Fields::load(plan.resources(), source.class, &specification)?.placed(
                crate::collider_init::DeclaredPose {
                    class: source.class as u32,
                    position: source.position,
                    yaw: source.yaw,
                    pitch_roll: source.pitch_roll,
                },
            )?;
            let initial = crate::uncached_pose::Pose {
                position: fields.position,
                angles: fields.angles,
                scale: fields.scale,
            };
            fields.set_declared_scale(source.scale)?;
            Arc::get_mut(&mut carry)
                .expect("unshared prepared carry")
                .scale = fields.scale;
            fields.action = 3;
            if fields.control_flags & 8 != 0 {
                return Err("declared Smartie requires network/special-action services".into());
            }
            let object = giants_formats::mission::ObjectRef::Placement(index);
            if plan
                .mission_object_results
                .iter()
                .any(|r| r.object == object && r.result.is_err())
            {
                return Err("declared Smartie has unresolved mission writes".into());
            }
            let writes = plan
                .mission_state_writes()
                .iter()
                .filter(|w| w.object == object)
                .map(|w| w.kind.clone())
                .collect();
            out.push(Self {
                key: Key {
                    scope: Scope::Mission,
                    index,
                },
                source_offset: source.source_offset,
                carry,
                fields,
                initial,
                specification,
                states,
                flags: source.direct_flags,
                writes,
            });
        }
        Ok(out)
    }
    pub fn construct(&self, session: &mut Session) -> Result<Id, String> {
        let writes = self.writes.iter().collect::<Vec<_>>();
        session.construct_mission_declaration_with_pose(
            self.key,
            Construction {
                class: self.carry.class,
                seed: &self.carry.seed,
                bindings: &self.carry.state_bindings,
                flags: self.flags,
                specification: self.specification.clone(),
                state_resources: self.states.clone(),
                fields: self.fields.instantiate(),
            },
            &writes,
            self.initial,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; source construction/proximity/pickup, no GPU or OS input"]
    fn installed_declared_wife_selects_actual_start_player_and_attaches_once() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let index = menu
            .entries
            .iter()
            .position(|e| e.name == "2b_2_returnwife")
            .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Story2", index).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let entries = Prepared::mission(&plan).unwrap();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        let mut session = Session::new(9921);
        let child = entry.construct(&mut session).unwrap();
        let source = plan.entries[entry.key.index].source.as_ref().unwrap();
        let fields = session
            .get(child)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!(fields.position, source.position);
        assert_eq!(fields.scale, source.scale);
        assert_eq!(fields.action, 3);
        assert!(session.get(child).unwrap().attachment_words().is_none());
        let count = session.declaration_count();
        assert!(entry.construct(&mut session).is_err());
        assert_eq!(session.declaration_count(), count);
        // Without an actual player the scan consumes its cooldown but creates no
        // attachment. The next scan uses that retained real payload timer.
        assert_eq!(
            session.select_declared_smartie_mecc(child, 0.).unwrap(),
            None
        );
        let start = crate::player_start::declared(
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
        )
        .unwrap();
        let resources = plan.resources();
        let spec =
            Arc::new(specification::Prepared::load_registered(resources, start.kind).unwrap());
        let states = Arc::new(
            state_resources::Prepared::load(resources, start.kind, Profile::Debug).unwrap(),
        );
        let program = resources
            .classes
            .models
            .first_program(start.kind)
            .unwrap()
            .program
            .as_ref()
            .unwrap();
        let seed = crate::state_storage::Seed::from_program_for(
            program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        let fields = Fields::load(resources, start.kind, &spec)
            .unwrap()
            .placed(crate::collider_init::DeclaredPose {
                class: start.kind as u32,
                position: start.position,
                yaw: start.angles[0],
                pitch_roll: Some([start.angles[1], start.angles[2]]),
            })
            .unwrap();
        let bindings = resources
            .class_state_bindings_for(start.kind, Profile::Debug)
            .unwrap();
        let parent = session
            .construct(Construction {
                class: start.kind,
                seed: &seed,
                bindings: &bindings,
                flags: crate::object_lists::DirectFlags::project(start.kind, program),
                specification: spec,
                state_resources: states,
                fields,
            })
            .unwrap();
        crate::mecc_special::Factory::new(resources)
            .initialize_player_fields(
                &mut session,
                parent,
                crate::animation_channels::Blending::Enabled,
            )
            .unwrap();
        assert_eq!(
            session.select_declared_smartie_mecc(child, 1.).unwrap(),
            None
        );
        assert_eq!(
            session.select_declared_smartie_mecc(child, 1.).unwrap(),
            Some(parent)
        );
        let point = session
            .constructor_point(parent, crate::smartie_carry::MECC_PARENT_REFERENCE as i32)
            .unwrap();
        let before_flags = session
            .get(child)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .object_flags;
        let services = session
            .pickup_declared_mecc(child, parent, point, 12.5f32.to_bits(), 25)
            .unwrap();
        assert_eq!(services.len(), 3);
        let actor = session.get(child).unwrap();
        let fields = actor.construction_fields.as_ref().unwrap();
        assert_eq!(fields.object_flags & 0x400, before_flags & 0x400);
        assert_eq!(fields.position, point);
        assert_eq!(
            fields.scale,
            (f64::from(source.scale) * f64::from(0.8f32)) as f32
        );
        assert_eq!(actor.attachment_words().unwrap().behavior, 19);
        assert_eq!(actor.channels.unwrap()[0][0], 41);
        assert!(actor.pickup_notice_pending);
        assert_ne!(
            session
                .get(parent)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .response_flags
                & 0x800000,
            0
        );
        assert!(session.select_declared_smartie_mecc(child, 1.).is_err());
        assert!(
            session
                .pickup_declared_mecc(child, parent, point, 0, 25)
                .is_err()
        );
    }
}
