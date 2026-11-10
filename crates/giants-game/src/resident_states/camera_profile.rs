use super::*;

impl Session {
    fn camera_profile_actor(&self, id: Id) -> Result<&Resident, String> {
        let actor = self
            .get(id)
            .ok_or("camera profile actor absent or foreign")?;
        if actor.seed.storage.profile() != Profile::Debug {
            return Err("camera profile actor outside acquired PC profile".into());
        }
        Ok(actor)
    }
    fn camera_profile_fields(&self, id: Id) -> Result<&crate::actor_fields::Fields, String> {
        self.camera_profile_actor(id)?
            .construction_fields
            .as_ref()
            .ok_or_else(|| "camera profile header absent".into())
    }
}
impl crate::camera_profile::World<Id> for Session {
    fn object_flags(&self, id: Id) -> Result<u32, String> {
        Ok(self.camera_profile_fields(id)?.object_flags)
    }
    fn class(&self, id: Id) -> Result<i32, String> {
        Ok(self.camera_profile_fields(id)?.class)
    }
    fn control_flags(&self, id: Id) -> Result<u32, String> {
        Ok(self.camera_profile_fields(id)?.control_flags)
    }
    fn parent(&self, id: Id) -> Result<Option<Id>, String> {
        Ok(self.camera_profile_actor(id)?.attachment.map(|a| a.parent))
    }
    fn state_kind(&self, id: Id) -> Result<i32, String> {
        Ok(i32::from(
            self.camera_profile_actor(id)?.seed.storage.state(),
        ))
    }
}
