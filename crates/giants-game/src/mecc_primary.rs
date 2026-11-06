//! Required services for the acquired normal offline Mecc primary profile.
//! This does not admit actors, run their children or acknowledge a whole world.
pub struct Services<'a, 'r, 'c> {
    pub animation: crate::mecc_animation_phase::Services<'a>,
    pub jet: crate::mecc_jet_owned::Services<'a, 'r>,
    /// Required if the actual periodic scan selects an eligible owned item.
    pub collection: Option<crate::mecc_collection::Services<'c>>,
    /// Optional acquired local pistol phase. Other buttons remain refused.
    pub fire: Option<FireServices<'a, 'r, 'c>>,
}

#[derive(Clone, Copy)]
pub struct FireView {
    pub source: crate::resident_states::Id,
    pub number: u64,
    pub camera: crate::first_person::Camera,
    pub viewport_origin: [f32; 3],
    pub viewport_forward: [f32; 3],
    pub first_person: bool,
    /// The actual pre-world status, never inferred from `first_person`.
    pub display: crate::first_person::Display<crate::resident_states::Id>,
    pub vertex_epoch: u32,
    pub force_vertices: bool,
    pub input_mode: i32,
    pub fire_disabled: bool,
    pub counter: u32,
}
/// The primary supplies frame storage, time, event/FX pool and RNG. The caller
/// still supplies the actual pre-primary view and muzzle/audio services.
pub struct FireServices<'a, 'r, 't> {
    pub factory: &'r crate::mecc_shot::Factory,
    pub resources: &'r crate::scene_resources::SceneResources,
    pub terrain: &'a crate::terrain_queries::StaticTerrainQuery<'t>,
    pub view: FireView,
    pub other: &'a mut dyn crate::mecc_shot::Services,
}
pub(crate) struct FireFrame<'a> {
    pub frame: crate::mecc_fire_primary::Frame,
    pub view: &'a crate::mecc_physical_cycle::View,
    pub buttons: u32,
    pub seconds: f64,
    pub auxiliary_reset: f32,
}
impl FireServices<'_, '_, '_> {
    pub(crate) fn preflight(
        &self,
        source: crate::resident_states::Id,
        number: u64,
    ) -> Result<(), String> {
        self.preflight_route(source, number, false)
    }
    pub(crate) fn preflight_subjective(
        &self,
        source: crate::resident_states::Id,
        number: u64,
    ) -> Result<(), String> {
        self.preflight_route(source, number, true)
    }
    fn preflight_route(
        &self,
        source: crate::resident_states::Id,
        number: u64,
        subjective: bool,
    ) -> Result<(), String> {
        if self.view.source != source || self.view.number != number || self.view.vertex_epoch == 0 {
            return Err("primary fire requires the current owned view/frame binding".into());
        }
        if self
            .view
            .display
            .camera_owner
            .is_some_and(|id| id.ticket() != source.ticket())
        {
            return Err("primary fire display belongs to another mission".into());
        }
        if self.view.display.owns(&source) && !subjective {
            // The native primary defers this owner's fire to its subjective
            // post-world phase. That phase is not bound by this primary API.
            return Err("primary fire requires the unbound subjective post-world phase".into());
        }
        if subjective && !self.view.display.owns(&source) {
            return Err("subjective fire requires the matching active display".into());
        }
        Ok(())
    }
    pub(crate) fn run(
        &mut self,
        session: &mut crate::resident_states::Session,
        input: FireFrame<'_>,
        contexts: &mut crate::attached_effect_context::Pool<crate::resident_states::Id>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::mecc_fire_primary::Output, String> {
        self.run_route(session, input, contexts, random, false)
    }
    pub(crate) fn run_subjective(
        &mut self,
        session: &mut crate::resident_states::Session,
        input: FireFrame<'_>,
        contexts: &mut crate::attached_effect_context::Pool<crate::resident_states::Id>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::mecc_fire_primary::Output, String> {
        self.run_route(session, input, contexts, random, true)
    }
    fn run_route(
        &mut self,
        session: &mut crate::resident_states::Session,
        input: FireFrame<'_>,
        contexts: &mut crate::attached_effect_context::Pool<crate::resident_states::Id>,
        random: &mut crate::audio_parameters::Random,
        subjective: bool,
    ) -> Result<crate::mecc_fire_primary::Output, String> {
        let FireFrame {
            frame,
            view,
            buttons,
            seconds,
            auxiliary_reset,
        } = input;
        self.preflight_route(self.view.source, view.number, subjective)?;
        let mut aim = crate::mecc_aim::Services {
            terrain: self.terrain,
            source: self.view.source,
            view: crate::mecc_aim::View {
                camera: self.view.camera,
                first_person: self.view.first_person,
                force_owner_angles: false,
                viewport_forward: Some(self.view.viewport_forward),
                owner_pitch: view.look_pitch,
            },
            other: &mut *self.other,
            sampled: None,
        };
        let mut effects = crate::mecc_shot::ContextServices {
            contexts,
            other: &mut aim,
        };
        crate::mecc_fire_primary::advance_local(
            session,
            self.factory,
            crate::mecc_fire_primary::Input {
                source: self.view.source,
                frame,
                buttons,
                seconds,
                authoritative: true,
                input_mode: self.view.input_mode,
                fire_disabled: self.view.fire_disabled,
                source_word174: 0,
                local_target_scan: true,
                auxiliary_reset,
                camera_origin: self.view.viewport_origin,
                counter: self.view.counter,
            },
            crate::mecc_fire_primary::LocalServices {
                resources: self.resources,
                terrain: self.terrain,
                camera: self.view.camera,
                viewport_origin: self.view.viewport_origin,
                viewport_forward: self.view.viewport_forward,
                vertex_epoch: self.view.vertex_epoch,
                force_vertices: self.view.force_vertices,
                headers: &mut crate::mecc_target_owned::OwnedHeaders,
            },
            random,
            &mut effects,
        )
    }
}
