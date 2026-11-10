//! Cold normal Delphi special phase and its owned auxiliary copies.
//! This does not initialize the complete local player, execute copy callbacks,
//! advance animation/physics or admit gameplay.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    specification, state_resources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub position: [f32; 3],
    pub angles: [f32; 3],
    pub scale: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub copies: usize,
    pub first: f32,
    pub last: f32,
}
impl Parameters {
    pub fn from_globals(g: &giants_formats::global_data::GlobalData) -> Result<Self, String> {
        let result = Self {
            copies: *g.words.get(0x4fc / 4).ok_or("Delphi copy count absent")? as usize,
            first: g.scalar(0x504).map_err(|e| e.to_string())?,
            last: g.scalar(0x508).map_err(|e| e.to_string())?,
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(self) -> Result<(), String> {
        if self.copies > 64 || self.copies == 1 || !self.first.is_finite() || !self.last.is_finite()
        {
            return Err("Delphi copy parameters outside acquired finite profile".into());
        }
        Ok(())
    }
    pub fn value(self, index: usize) -> Result<f32, String> {
        self.validate()?;
        if index >= self.copies {
            return Err("Delphi copy index outside prepared group".into());
        }
        Ok(((f64::from(self.last) - f64::from(self.first))
            * (index as f64 / (self.copies - 1) as f64)
            + f64::from(self.first)) as f32)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Copy {
    pub actor: Id,
    pub held: Id,
    pub index: u32,
    pub parameter: f32,
}
#[derive(Clone, Debug)]
pub struct Initialized {
    pub copies: Vec<Copy>,
    pub held: Id,
}
#[derive(Clone, Copy, Debug)]
pub struct CopySource {
    pub owner: Id,
    pub index: u32,
}
#[derive(Debug)]
pub(crate) enum State {
    Initializing,
    Complete(Initialized),
    Failed(String),
}

struct Template {
    class: i32,
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    fields: Fields,
}
impl Template {
    fn load(r: &SceneResources, class: i32, state: u8) -> Result<Self, String> {
        let program = r
            .classes
            .models
            .first_program(class)
            .ok_or("Delphi constructor class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let specification = Arc::new(specification::Prepared::load_registered(r, class)?);
        let states = Arc::new(state_resources::Prepared::load(r, class, Profile::Debug)?);
        let fields = Fields::load(r, class, &specification)?;
        if seed.storage.state() != state
            || !seed.diagnostics.is_empty()
            || !specification.needs.is_empty()
            || !states.needs().is_empty()
        {
            return Err("Delphi constructor resources outside acquired cold profile".into());
        }
        Ok(Self {
            class,
            seed,
            bindings: r.class_state_bindings_for(class, Profile::Debug)?,
            flags: DirectFlags::project(class, program),
            specification,
            states,
            fields,
        })
    }
    fn input(&self) -> Construction<'_> {
        Construction {
            class: self.class,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.specification.clone(),
            state_resources: self.states.clone(),
            fields: self.fields.instantiate(),
        }
    }
}
pub struct Factory {
    owner: Template,
    copy: Option<Template>,
    held: Template,
    parameters: Parameters,
    charges: i32,
}
impl Factory {
    pub fn load(
        r: &SceneResources,
        g: &giants_formats::global_data::GlobalData,
        catalog: &giants_formats::weapons::Catalog,
    ) -> Result<Self, String> {
        let parameters = Parameters::from_globals(g)?;
        let charges = initial_charges(catalog)?;
        Ok(Self {
            owner: Template::load(r, 3, 3)?,
            copy: if parameters.copies > 0 {
                Some(Template::load(r, 311, 62)?)
            } else {
                None
            },
            held: Template::load(r, 669, 0)?,
            parameters,
            charges,
        })
    }
    /// Real class construction and supplied placement, before special init.
    pub fn construct(&self, s: &mut Session, pose: Pose) -> Result<Id, String> {
        let mut input = self.owner.input();
        input.fields = self
            .owner
            .fields
            .placed(crate::collider_init::DeclaredPose {
                class: 3,
                position: pose.position,
                yaw: pose.angles[0],
                pitch_roll: Some([pose.angles[1], pose.angles[2]]),
            })?;
        input.fields.set_declared_scale(pose.scale)?;
        s.construct_delphi_cold(input, 0x80000000)
    }
    /// Cold generic-to-Delphi field projection and complete special group.
    /// The supplied frame is not a mission-marker or live type-change producer.
    pub fn construct_player_fields(
        &self,
        s: &mut Session,
        frame: crate::delphi_player::ColdFrame,
        blending: crate::animation_channels::Blending,
    ) -> Result<crate::delphi_player::Initialized, String> {
        self.construct_player_fields_retained(s, frame, blending)
            .result
    }
    pub fn construct_player_fields_retained(
        &self,
        s: &mut Session,
        frame: crate::delphi_player::ColdFrame,
        blending: crate::animation_channels::Blending,
    ) -> crate::delphi_player::Attempt {
        let mut published = None;
        let result = (|| {
            let mut input = self.owner.input();
            input.fields = crate::delphi_player::prepare(&self.owner.fields, frame)?;
            let channels = crate::delphi_player::selected(input.fields.channels, blending)?;
            if !self.owner.specification.animations.contains_key(&1) {
                return Err("Delphi initial player animation absent".into());
            }
            let owner = s.construct_delphi_cold(input, 0x80000000)?;
            published = Some(owner);
            let special = self.initialize(s, owner)?;
            s.finish_delphi_player_fields(owner, self.owner.fields.channels, channels)?;
            Ok(crate::delphi_player::Initialized {
                owner,
                special,
                camera_projection_reset_required: true,
            })
        })();
        crate::delphi_player::Attempt {
            owner: published,
            result,
        }
    }
    /// Declared placement with the actual source constructor's shell history.
    /// This does not prove native relocation or acknowledge live readiness.
    pub(crate) fn construct_declared_player_fields(
        &self,
        s: &mut Session,
        start: &crate::player_start::Declared,
        blending: crate::animation_channels::Blending,
    ) -> crate::delphi_player::Attempt {
        self.construct_player_fields_retained(
            s,
            crate::delphi_player::ColdFrame {
                position: start.position,
                current: self.owner.fields.current,
                previous: self.owner.fields.previous,
                angles: start.angles,
            },
            blending,
        )
    }
    pub fn initialize(&self, s: &mut Session, owner: Id) -> Result<Initialized, String> {
        s.begin_delphi_special(owner, &self.owner.seed, &self.owner.fields)?;
        let result = (|| {
            let mut copies = Vec::new();
            for index in 0..self.parameters.copies {
                let actor = s.construct_delphi_cold(
                    self.copy
                        .as_ref()
                        .ok_or("Delphi copy template absent")?
                        .input(),
                    4,
                )?;
                let parameter = self.parameters.value(index)?;
                s.bind_delphi_copy(actor, owner, index as u32, parameter)?;
                let held = self.bind_held(s, actor)?;
                s.finish_delphi_copy_held(held, parameter)?;
                copies.push(Copy {
                    actor,
                    held,
                    index: index as u32,
                    parameter,
                });
            }
            s.initialize_delphi_equipment(owner, self.charges)?;
            let held = self.bind_held(s, owner)?;
            s.finish_delphi_special(owner)?;
            Ok(Initialized { copies, held })
        })();
        s.end_delphi_special(owner, &result);
        result
    }
    fn bind_held(&self, s: &mut Session, parent: Id) -> Result<Id, String> {
        let mut world = HeldWorld {
            session: s,
            template: &self.held,
            constructed: None,
        };
        crate::mecc_wield::replace(&mut world, parent, 1, 669)?;
        world
            .constructed
            .ok_or("cold Delphi held construction absent".into())
    }
}
fn initial_charges(catalog: &giants_formats::weapons::Catalog) -> Result<i32, String> {
    let icon = catalog
        .icons
        .get(&44)
        .ok_or("Delphi cold equipment declaration absent")?;
    let mut charges = 333;
    for offset in &icon.records {
        let record = catalog
            .records
            .iter()
            .find(|r| r.offset == *offset)
            .ok_or("Delphi equipment record outside catalog")?;
        if record.opcode == 0x2e {
            charges = *record.args.first().ok_or("Delphi charge operand absent")? as i32;
        }
    }
    Ok(charges)
}
struct HeldWorld<'a> {
    session: &'a mut Session,
    template: &'a Template,
    constructed: Option<Id>,
}
impl crate::mecc_wield::World<Id> for HeldWorld<'_> {
    fn children(&self) -> Result<Vec<crate::mecc_wield::Child<Id>>, String> {
        self.session.wield_list()
    }
    fn destroy(&mut self, _: Id) -> Result<(), String> {
        Err("cold Delphi held replacement cannot destroy an existing child".into())
    }
    fn construct(&mut self, class: i32, mask: u32) -> Result<Id, String> {
        if class != 669 || mask != 0x20 {
            return Err("Delphi held identity changed".into());
        }
        let id = self
            .session
            .construct_delphi_cold(self.template.input(), mask)?;
        self.constructed = Some(id);
        Ok(id)
    }
    fn bind(&mut self, child: Id, parent: Id, reference: i32) -> Result<(), String> {
        self.session.bind_delphi_held(child, parent, reference)
    }
    fn is_mecc(&self, _: Id) -> Result<bool, String> {
        Ok(false)
    }
    fn cancel_footprint(&mut self, _: Id) -> Result<(), String> {
        Err("Delphi held cannot invoke the Mecc footprint service".into())
    }
    fn find_equipment(&mut self, _: Id, _: i32, _: i32) -> Result<Option<Id>, String> {
        Err("Delphi special equipment query outside cold profile".into())
    }
    fn set_equipment_active(&mut self, _: Id, _: bool) -> Result<(), String> {
        Err("Delphi special equipment activation outside cold profile".into())
    }
    fn refresh_equipment(&mut self, _: Id) -> Result<(), String> {
        Err("Delphi equipment refresh outside cold profile".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalog(records: &[(u32, &[u32])]) -> giants_formats::weapons::Catalog {
        let mut words = vec![1, 0];
        for &(op, args) in records {
            words.extend([op, args.len() as u32 + 2]);
            words.extend(args);
        }
        words.extend([0, 2]);
        words[1] = words.len() as u32 * 4;
        let raw = words
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        giants_formats::weapons::parse_with_profile(&raw, giants_formats::weapons::Profile::Debug)
            .unwrap()
    }
    #[test]
    fn cold_charge_uses_its_own_ordered_field_and_declaration_reset() {
        let data = catalog(&[
            (0x24, &[44]),
            (0x2e, &[5]),
            (0x2d, &[123]),
            (0x2e, &[u32::MAX]),
            (0x25, &[]),
            (0x2e, &[99]),
        ]);
        assert_eq!(initial_charges(&data).unwrap(), -1);
        let reset = catalog(&[(0x24, &[44]), (0x2e, &[7]), (0x24, &[44])]);
        assert_eq!(initial_charges(&reset).unwrap(), 333);
        assert!(initial_charges(&catalog(&[(0x24, &[43])])).is_err());
    }
    #[test]
    fn copy_parameters_refuse_singular_or_unbounded_pools() {
        let mut p = Parameters {
            copies: 2,
            first: 0.75,
            last: -0.5,
        };
        assert_eq!(p.value(0).unwrap(), 0.75);
        assert_eq!(p.value(1).unwrap(), -0.5);
        assert!(p.value(2).is_err());
        for count in [1, 65, usize::MAX] {
            p.copies = count;
            assert!(p.validate().is_err());
        }
        p.copies = 0;
        assert!(p.validate().is_ok());
        assert!(p.value(0).is_err());
        p.first = f32::NAN;
        assert!(p.validate().is_err());
    }
    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT; actual constructor identity after a supplied late child-resource failure"]
    fn source_delphi_retained_player_attempt_keeps_published_owner_after_failure() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let r = SceneResources::load(&root, "w_Rstory1.bin", "English").unwrap();
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let weapons = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
            giants_formats::weapons::Profile::Debug,
        )
        .unwrap();
        let mut factory = Factory::load(&r, &globals, &weapons).unwrap();
        let mut session = Session::new(1045);
        let frame = crate::delphi_player::ColdFrame {
            position: [7., 8., 9.],
            current: factory.owner.fields.current,
            previous: factory.owner.fields.previous,
            angles: [0.; 3],
        };
        let bad = factory.construct_player_fields_retained(
            &mut session,
            crate::delphi_player::ColdFrame {
                position: [f32::NAN; 3],
                ..frame
            },
            crate::animation_channels::Blending::Enabled,
        );
        assert!(bad.owner.is_none());
        assert!(bad.result.is_err());
        assert_eq!(session.counts().residents, 0);
        assert!(factory.parameters.copies > 0);
        factory.copy = None;
        let attempt = factory.construct_player_fields_retained(
            &mut session,
            frame,
            crate::animation_channels::Blending::Enabled,
        );
        let owner = attempt
            .owner
            .expect("constructor published before child resource failure");
        assert!(attempt.result.unwrap_err().contains("copy template absent"));
        assert_eq!(session.get(owner).unwrap().class, 3);
        assert!(
            session
                .delphi_special_failure(owner)
                .unwrap()
                .contains("copy template absent")
        );
        assert_eq!(session.counts().residents, 1);
        session.clear();
        assert!(session.get(owner).is_none());
    }

    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT with user-supplied resources; cold CPU phase only"]
    fn source_delphi_special_group_owns_references_and_refuses_replay() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let r = SceneResources::load(&root, "w_Rstory1.bin", "English").unwrap();
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let catalog = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
            giants_formats::weapons::Profile::Debug,
        )
        .unwrap();
        let factory = Factory::load(&r, &globals, &catalog).unwrap();
        let pose = Pose {
            position: [7., -11., 13.],
            angles: [37., -12., 8.],
            scale: 0.5,
        };
        let mut s = Session::new(1041);
        let owner = factory.construct(&mut s, pose).unwrap();
        let result = factory.initialize(&mut s, owner).unwrap();
        assert_eq!(s.counts().residents, 2 * result.copies.len() + 2);
        assert_eq!(
            s.incoming_actor_references(owner).unwrap().len(),
            result.copies.len() + 1
        );
        assert!(s.mecc_health(owner).is_err());
        assert_eq!(
            s.get(owner)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .configuration[14]
                & 1,
            0
        );
        let counts = s.counts();
        let payload = s.get(owner).unwrap().seed.storage.data().unwrap().to_vec();
        assert!(factory.initialize(&mut s, owner).is_err());
        assert_eq!(s.counts(), counts);
        assert_eq!(s.get(owner).unwrap().seed.storage.data().unwrap(), payload);
        assert!(s.delphi_special_failure(owner).is_none());
        s.clear();
        assert_eq!(s.counts().residents, 0);
        assert_eq!(s.counts().registrations, 0);
        assert!(s.delphi_special(owner).is_none());
        let frame = crate::delphi_player::ColdFrame {
            position: [7., -11., 13.],
            current: [6., -10., 12.],
            previous: [5., -9., 11.],
            angles: [37., -12., 8.],
        };
        for blending in [
            crate::animation_channels::Blending::Disabled,
            crate::animation_channels::Blending::Enabled,
        ] {
            let player = factory
                .construct_player_fields(&mut s, frame, blending)
                .unwrap();
            assert!(player.camera_projection_reset_required);
            assert_eq!(s.counts().residents, 2 * player.special.copies.len() + 2);
            let fields = s
                .get(player.owner)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            assert_eq!(fields.previous, frame.current);
            assert_eq!(fields.current, frame.position);
            assert_eq!(fields.angles, [37., 0., 8.]);
            assert_eq!(fields.scale, factory.owner.fields.scale);
            assert_eq!(fields.channels[0][0], 1);
            assert_eq!(fields.configuration[14] & 1, 1);
            let channels = fields.channels;
            let retained = s.counts();
            assert!(
                s.finish_delphi_player_fields(
                    player.owner,
                    factory.owner.fields.channels,
                    channels
                )
                .is_err()
            );
            assert!(factory.initialize(&mut s, player.owner).is_err());
            assert_eq!(s.counts(), retained);
            assert!(s.delphi_physics(player.owner).is_err());
            let physics = s.initialize_delphi_physics(player.owner, &globals).unwrap();
            assert_eq!(s.delphi_physics(player.owner).unwrap(), physics);
            assert_eq!(s.counts(), retained);
            let payload = s
                .get(player.owner)
                .unwrap()
                .seed
                .storage
                .data()
                .unwrap()
                .to_vec();
            assert!(s.initialize_delphi_physics(player.owner, &globals).is_err());
            assert_eq!(
                s.get(player.owner).unwrap().seed.storage.data().unwrap(),
                payload
            );
            assert!(s.mecc_physics(player.owner).is_err());
            s.clear();
            assert!(s.delphi_physics(player.owner).is_err());
            assert_eq!(s.counts().residents, 0);
            assert_eq!(s.counts().registrations, 0);
        }
        let mut invalid = frame;
        invalid.previous[0] = f32::INFINITY;
        assert!(
            factory
                .construct_player_fields(
                    &mut s,
                    invalid,
                    crate::animation_channels::Blending::Disabled
                )
                .is_err()
        );
        assert_eq!(s.counts().residents, 0);
        // A deliberately unbound list6 member is an owned-backend failure
        // control, not a native error-path comparison.
        let mut malformed = crate::state_storage::Seed {
            storage: crate::state_storage::Storage::new(Profile::Debug),
            transitions: vec![],
            unexecuted: vec![],
            diagnostics: vec![],
            unresolved: vec![],
        };
        malformed.storage.transition(6).unwrap();
        s.register(900, &malformed, &[], DirectFlags::default())
            .unwrap();
        let failed = factory.construct(&mut s, pose).unwrap();
        assert!(factory.initialize(&mut s, failed).is_err());
        assert!(s.delphi_special_failure(failed).is_some());
        let retained = s.counts();
        assert!(factory.initialize(&mut s, failed).is_err());
        assert_eq!(s.counts(), retained);
        s.clear();
        assert_eq!(s.counts().residents, 0);
    }
}
