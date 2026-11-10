//! Shared mode command consuming the actual Session owner and header.
use super::*;
use crate::{camera_mode as mode, camera_owner, camera_profile};

impl Session {
    pub fn camera_look_angle(&self, id: Id) -> Result<f32, String> {
        self.get(id)
            .ok_or("Camera look-angle owner absent or foreign")?
            .camera_look_angle
            .ok_or_else(|| "Camera look angle has no acquired producer".into())
    }

    /// Uses one shared mode state. Each profile query reads the current checked
    /// owner and live Session fields again; no cached selection is substituted.
    /// The projection closure must perform the actual backend publication.
    /// This does not select a camera owner or acknowledge player initialization.
    pub fn change_camera_mode(
        &mut self,
        state: &mut mode::State,
        owners: &camera_owner::State<Id>,
        command: mode::Command<Id>,
        catalog: &impl camera_profile::Catalog,
        projection: &mut impl FnMut(f32) -> Result<(), String>,
    ) -> Result<(), String> {
        let owner = owners
            .owner_from_session(self)?
            .map(|source| -> Result<mode::Owner<Id>, String> {
                Ok(mode::Owner {
                    source,
                    object_flags: camera_profile::World::object_flags(self, source)?,
                })
            })
            .transpose()?;
        struct Bound<'a, C, F> {
            session: &'a mut Session,
            owners: &'a camera_owner::State<Id>,
            command: mode::Command<Id>,
            catalog: &'a C,
            projection: &'a mut F,
        }
        impl<C: camera_profile::Catalog, F: FnMut(f32) -> Result<(), String>> mode::Services<Id>
            for Bound<'_, C, F>
        {
            fn profile(&mut self, _: u32, view: u32) -> Result<mode::Profile, String> {
                camera_profile::select(
                    camera_profile::Input {
                        owner: self.owners.owner_from_session(self.session)?,
                        controlled: self.command.controlled,
                        view,
                        scaled_input: self.command.scaled_input,
                    },
                    self.session,
                )?
                .resolve(self.catalog)
            }
            fn scale(&mut self, source: Id, pitch: f32) -> Result<(), String> {
                let actor = self
                    .session
                    .residents
                    .get_mut(&source.serial)
                    .ok_or("Camera look-angle owner retired during command")?;
                actor.camera_look_angle = Some(pitch);
                Ok(())
            }
            fn projection(&mut self, fov: f32) -> Result<(), String> {
                (self.projection)(fov)
            }
        }
        state.change(
            command.request,
            command.extended,
            owner,
            &mut Bound {
                session: self,
                owners,
                command,
                catalog,
                projection,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires GIANTS_KABUTO_ROOT and GIANTS_KABUTO_EXECUTABLE; real cold owner and external profiles, supplied owner/mode/projection context; no gameplay or GPU claim"]
    fn source_kabuto_camera_command_updates_exact_owned_look_angle() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
        let executable =
            std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_EXECUTABLE").unwrap());
        let source = crate::world_source::resolve(&root, "Kstory1").unwrap();
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Kstory1", 0).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let resources = crate::kabuto_resources::Resources::load(
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
        )
        .unwrap();
        let catalog = camera_profile::ExecutableCatalog::read(&executable).unwrap();
        let cold = crate::camera_initial::Cold::read(&executable).unwrap();
        let mut session = Session::new(1943);
        let first = resources.construct_ordinary(&mut session).unwrap();
        let second = resources.construct_ordinary(&mut session).unwrap();
        assert!(session.camera_look_angle(first).is_err());
        resources
            .initialize_player_fields(
                &mut session,
                first,
                crate::kabuto_player::ColdFrame {
                    position: resources.start.position,
                    current: resources.start.position,
                    previous: resources.start.position,
                    angles: resources.start.angles,
                },
                crate::animation_channels::Blending::Enabled,
            )
            .unwrap();
        assert_eq!(session.camera_look_angle(first).unwrap(), 0.);
        let fields = session
            .get(first)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let body = (fields.scale, fields.angles);
        // Exact loader-image words seed this cold prefix; this does not prove
        // that every mission-entry caller leaves those words unchanged.
        let mut owners = cold.owners();
        owners
            .select_from_session(
                &mut session,
                cold.owner_input(Some(first)),
                &mut |_, _, _| panic!("negative cold reference does not sample a reference point"),
            )
            .unwrap();
        assert_eq!(owners.owner(), Some(first));
        let mut modes = cold.modes().unwrap();
        let command = mode::Command {
            request: 1,
            extended: false,
            controlled: Some(second),
            scaled_input: false,
        };
        let mut projections = Vec::new();
        session
            .change_camera_mode(&mut modes, &owners, command, &catalog, &mut |fov| {
                let output = crate::camera_projection::build(fov, cold.projection([1500., 1000.]))?;
                projections.push(output);
                Ok(())
            })
            .unwrap();
        let selected =
            camera_profile::from_session(&session, &owners, &modes, Some(second), false).unwrap();
        let record = catalog.record(selected).unwrap();
        assert!(record.pitch < 0.);
        assert_eq!(session.camera_look_angle(first).unwrap(), record.pitch);
        assert!(session.camera_look_angle(second).is_err());
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].vertical_fov, record.camera.vertical_fov);
        assert_eq!((modes.mode(), modes.view(), modes.transition()), (0, 0, 0.));
        let fields = session
            .get(first)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!((fields.scale, fields.angles), body);

        // A later projection failure preserves the completed pitch store and
        // transition reset, and cannot replay through this mode state.
        assert!(
            session
                .change_camera_mode(
                    &mut modes,
                    &owners,
                    mode::Command {
                        request: 2,
                        ..command
                    },
                    &catalog,
                    &mut |_| Err("supplied projection failure".into()),
                )
                .is_err()
        );
        let angle = session.camera_look_angle(first).unwrap();
        let mut callbacks = 0;
        assert!(
            session
                .change_camera_mode(&mut modes, &owners, command, &catalog, &mut |_| {
                    callbacks += 1;
                    Ok(())
                },)
                .is_err()
        );
        assert_eq!(callbacks, 0);
        assert_eq!(session.camera_look_angle(first).unwrap(), angle);
        assert_eq!(modes.transition(), 0.);
        assert!(modes.failure().is_some());

        session.clear();
        assert!(session.camera_look_angle(first).is_err());
        let mut modes = mode::State::new(2, 2, 0.375).unwrap();
        assert!(
            session
                .change_camera_mode(&mut modes, &owners, command, &catalog, &mut |_| panic!(
                    "retired owner"
                ),)
                .is_err()
        );
        assert_eq!(
            (modes.mode(), modes.view(), modes.transition()),
            (2, 2, 0.375)
        );
    }
}
