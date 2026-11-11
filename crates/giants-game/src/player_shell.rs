//! Acquired generic player actor sequence before mission type activation.
//! World-global reset/session scalar, the persistent camera lifecycle and later
//! retyping remain upstream. Callback completion is not playable admission.
use crate::{
    actor_fields::Fields,
    object_lists::DirectFlags,
    resident_states::{Id, Session},
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};

pub struct Factory {
    seed: Seed,
    flags: DirectFlags,
    fields: Fields,
}
#[derive(Clone, Copy, Debug)]
pub struct WorldPose {
    pub position: [f32; 3],
    pub heading: f32,
}
/// Required real camera operations. The controlled shell is distinct from the
/// currently selected camera owner. Neither callback may silently omit its work.
pub trait Camera {
    fn change_view(
        &mut self,
        session: &mut Session,
        controlled: Id,
        request: i32,
    ) -> Result<(), String>;
    fn update_camera(&mut self, session: &mut Session, controlled: Id) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checkpoint {
    Constructed,
    ViewApplied,
    PoseApplied,
    CameraApplied,
}
/// Own the actual constructor identity even after a later service fails.
/// This capability is not cloneable; an entered sequence cannot be replayed.
pub struct Entry {
    owner: Id,
    checkpoint: Checkpoint,
    started: bool,
    failure: Option<String>,
}
impl Factory {
    pub fn load(resources: &SceneResources) -> Result<Self, String> {
        let declaration = resources
            .classes
            .models
            .first_program(0)
            .ok_or("player shell class0 absent")?;
        let program = declaration.program.as_ref().map_err(Clone::clone)?;
        let seed = Seed::from_program_for(
            program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )?;
        if seed.storage.state() != 7
            || seed.storage.data().is_some()
            || !seed.diagnostics.is_empty()
            || !seed.unresolved.is_empty()
        {
            return Err("player shell requires its acquired unmodeled state7".into());
        }
        Ok(Self {
            seed,
            flags: DirectFlags::project(0, program),
            fields: Fields::load_player_shell(resources)?,
        })
    }
    pub fn construct(&self, session: &mut Session) -> Result<Entry, String> {
        let owner =
            session.construct_player_shell(&self.seed, self.flags, self.fields.instantiate())?;
        Ok(Entry {
            owner,
            checkpoint: Checkpoint::Constructed,
            started: false,
            failure: None,
        })
    }
}
impl Entry {
    pub fn owner(&self) -> Id {
        self.owner
    }
    pub fn checkpoint(&self) -> Checkpoint {
        self.checkpoint
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    /// Camera command precedes context/pose/history; camera update follows them.
    /// Required callbacks use the caller's actual persistent owner/mode/backend.
    pub fn initialize(
        &mut self,
        session: &mut Session,
        pose: WorldPose,
        camera: &mut impl Camera,
    ) -> Result<(), String> {
        if self.started {
            return Err("player shell sequence already entered".into());
        }
        if !pose
            .position
            .into_iter()
            .chain([pose.heading])
            .all(f32::is_finite)
        {
            return Err("player shell world pose is not finite".into());
        }
        session.check_player_shell(self.owner)?;
        self.started = true;
        let result: Result<(), String> = (|| {
            camera.change_view(session, self.owner, 1)?;
            self.checkpoint = Checkpoint::ViewApplied;
            session.publish_player_shell_pose(self.owner, pose)?;
            self.checkpoint = Checkpoint::PoseApplied;
            camera.update_camera(session, self.owner)?;
            self.checkpoint = Checkpoint::CameraApplied;
            Ok(())
        })();
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
}
