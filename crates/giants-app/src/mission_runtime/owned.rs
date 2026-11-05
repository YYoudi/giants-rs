//! Experimental mission-owned primary driver. Real terrain/colliders and audio
//! are required inputs; no provider is acknowledged by a silent substitute.
use super::{Active, ActorId, Runtime};
use giants_game::{resident_states::Session, scene_resources::SceneResources};
use std::{path::Path, sync::Arc};

pub(crate) struct Input<'a> {
    pub root: &'a Path,
    pub number: u64,
    pub commands: giants_game::player_input_channels::Commands,
    pub seconds: f32,
    pub terrain: &'a giants_formats::gti::Terrain,
    pub objects: &'a [giants_game::player_world_collision::Object],
    pub materials: &'a std::collections::BTreeMap<(u32, u32), String>,
    pub first_person: bool,
}
pub(super) struct Driver {
    ticket: u64,
    actor: ActorId,
    number: u64,
    failure: Option<String>,
    globals: giants_formats::global_data::GlobalData,
    catalog: giants_formats::weapons::Catalog,
    wield: giants_game::mecc_wield::Cache,
    contexts: giants_game::attached_effect_context::Pool<ActorId>,
    stamp: giants_game::animation_epoch::Stamp,
    sequences: giants_game::mecc_jet_owned::Pool,
    actor_key: Option<u32>,
    children: giants_game::mecc_jet_presentation::Children,
    weapon_pose: giants_game::mecc_wield_pose::Producer,
}
impl Driver {
    fn prepare(active: &mut Active, input: &Input<'_>) -> Result<Self, String> {
        if input.number != 1 || active.controlled_mecc_owner.is_some() {
            return Err("owned Mecc must start before any development motion publication".into());
        }
        let root = input.root;
        let materials = giants_game::mecc_contact_materials::Catalog::from_prepared(
            input.objects,
            input.materials.clone(),
        )?;
        let player = active.payload.player.as_ref().map_err(Clone::clone)?;
        let actor = active.resident_states.unique_class(player.start.kind)?;
        let source = Arc::clone(active.payload.plan.resources());
        let globals = giants_formats::global_data::parse(
            &std::fs::read(root.join("Bin/GData.gbn")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let catalog = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin")).map_err(|e| e.to_string())?,
            giants_formats::weapons::Profile::Debug,
        )
        .map_err(|e| e.to_string())?;
        let specification = active
            .resident_states
            .specification(actor)
            .ok_or("owned Mecc specification absent")?
            .clone();
        let physics = giants_game::mecc_physical_cycle::Resources::load(&source, specification)?;
        active
            .resident_states
            .prepare_mecc_physics(actor, 1, &globals, &catalog)?;
        active
            .resident_states
            .prepare_mecc_physical_cycle(actor, physics)?;
        active
            .resident_states
            .bind_mecc_contact_materials(materials)?;
        if active.resident_states.mecc_countdown_world().is_none() {
            // Explicit fresh-runtime policy, not a claim about native level reset.
            active.resident_states.prepare_mecc_countdown_world(0.)?;
        }
        let children =
            giants_game::mecc_jet_presentation::Children::new(&active.resident_states, actor)?;
        Ok(Self::new(
            active.ticket,
            actor,
            source,
            globals,
            catalog,
            children,
        ))
    }
    fn new(
        ticket: u64,
        actor: ActorId,
        source: Arc<SceneResources>,
        globals: giants_formats::global_data::GlobalData,
        catalog: giants_formats::weapons::Catalog,
        children: giants_game::mecc_jet_presentation::Children,
    ) -> Self {
        Self {
            ticket,
            actor,
            number: 0,
            failure: None,
            globals,
            catalog,
            wield: giants_game::mecc_wield::Cache::new(source.clone()),
            contexts: giants_game::attached_effect_context::Pool::from_active_resources(
                ticket, &source,
            ),
            stamp: giants_game::animation_epoch::Stamp(0),
            sequences: giants_game::mecc_jet_owned::Pool::new(ticket),
            actor_key: None,
            children,
            weapon_pose: giants_game::mecc_wield_pose::Producer::new(actor),
        }
    }
    fn advance(
        &mut self,
        session: &mut Session,
        random: &mut giants_game::audio_parameters::Random,
        input: Input<'_>,
        backend: &mut dyn giants_game::audio_sequence::Backend<Handle = u32>,
    ) -> Result<giants_game::mecc_presentation::Frame, String> {
        if self.failure.is_some()
            || self.number.checked_add(1) != Some(input.number)
            || input.commands.buttons & !0xc00 != 0
        {
            return Err(
                "owned Mecc frame is failed, stale or requires unacquired button callbacks".into(),
            );
        }
        let query = giants_game::terrain_queries::StaticTerrainQuery::new(
            input.terrain,
            1. / input.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?;
        if self.actor_key.is_none() {
            let mut key = u32::MAX;
            while input.objects.iter().any(|o| o.fields.id == key) {
                key = key
                    .checked_sub(1)
                    .filter(|k| *k != 0)
                    .ok_or("controlled collision identity exhausted")?;
            }
            self.actor_key = Some(key);
        }
        // Explicit inspection ordering: sound pass before the owned actor pass.
        // Complete native mode admission and world timing remain separate.
        if let Err(error) = self.sequences.update(
            input.number,
            input.seconds,
            false,
            giants_game::arithmetic::Precision::Double53,
            backend,
        ) {
            self.failure = Some(error.clone());
            return Err(error);
        }
        let mut wield = giants_game::mecc_wield::Factory::from_cache(&mut self.wield);
        session.begin_mecc_frame_history(self.actor, input.number)?;
        let result = session.advance_mecc_world_offline(
            self.actor,
            giants_game::mecc_physical_cycle::Environment {
                number: input.number,
                generation: 1,
                globals: &self.globals,
                catalog: &self.catalog,
                global_seconds: input.seconds,
                areas: &[],
                controls: [input.commands.axes[2], input.commands.axes[3]],
                facing_pitch: 0.,
                orientation: Some(giants_game::mecc_physical_cycle::Orientation {
                    controls: [input.commands.axes[0], input.commands.axes[1]],
                    control_factor: 1.,
                    skip_heading: false,
                    controlled_actor: true,
                }),
                actor_key: self.actor_key.unwrap(),
                objects: input.objects,
            },
            Some(self.actor),
            |point| {
                giants_game::physical_ground::Ground::from_sample(
                    query.sample(point)?,
                    input.terrain.width,
                )
            },
            giants_game::mecc_primary::Services {
                fire: None,
                animation: giants_game::mecc_animation_phase::Services {
                    contexts: &mut self.contexts,
                    random,
                    stamp: &mut self.stamp,
                    blending: giants_game::animation_channels::Blending::Enabled,
                },
                jet: giants_game::mecc_jet_owned::Services {
                    wield: &mut wield,
                    sequences: &mut self.sequences,
                    backend,
                    input: giants_game::mecc_jet_owned::Input {
                        buttons: input.commands.buttons,
                        audio_key: self.ticket,
                        ..Default::default()
                    },
                },
                collection: None,
            },
        );
        let result = result.and_then(|projection| {
            if projection.outcome != giants_game::actor_world_tick::Outcome::UpdatedByPrimary {
                return Err("owned Mecc needs an unacquired inactive/admission service".into());
            }
            self.number = input.number;
            let mut frame = giants_game::mecc_presentation::sample(
                session,
                self.actor,
                input.number,
                input.first_person,
            )?;
            frame.children =
                self.children
                    .advance(session, input.number, input.seconds, frame.stamp)?;
            frame.world_weapon =
                self.weapon_pose
                    .advance(session, input.number, input.seconds, frame.stamp)?;
            if !input.first_person
                && let Some(weapon) = &frame.world_weapon
            {
                frame.world_aim = Some(giants_game::mecc_aim::resolve(
                    &query,
                    giants_game::mecc_aim::View {
                        camera: giants_game::first_person::Camera {
                            position: frame.camera.eye,
                            angles: [frame.angles[0], frame.look_pitch, 0.],
                        },
                        first_person: false,
                        force_owner_angles: false,
                        viewport_forward: None,
                        owner_pitch: frame.look_pitch,
                    },
                    frame.angles[0],
                    weapon.muzzle,
                    giants_game::mecc_aim::NORMAL_PISTOL_RANGE,
                )?);
            }
            Ok(frame)
        });
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
}
impl Runtime {
    pub(crate) fn owned_jet_models(
        &self,
        ticket: u64,
    ) -> Result<Vec<giants_game::scene_resources::ModelAsset>, String> {
        let active = self
            .active
            .as_ref()
            .filter(|v| v.ticket == ticket)
            .ok_or("jet render ticket absent")?;
        let source = active.payload.plan.resources();
        [84, 85, 781]
            .into_iter()
            .map(|class| {
                source
                    .model_slot_for_body(
                        class,
                        0,
                        giants_game::scene_resources::TextureEncoding::Rgb565OrArgb4444,
                    )
                    .map_err(|e| e.to_string())?
                    .ok_or("jet model absent".into())
            })
            .collect()
    }
    pub(crate) fn advance_owned_mecc(
        &mut self,
        ticket: u64,
        input: Input<'_>,
        backend: &mut dyn giants_game::audio_sequence::Backend<Handle = u32>,
    ) -> Result<giants_game::mecc_presentation::Frame, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("owned Mecc mission ticket absent")?;
        if active.controlled_mecc_owner.is_some() {
            return Err("development motion already owns this mission's Mecc".into());
        }
        if active.owned_mecc.is_none() {
            active.owned_mecc = Some(Driver::prepare(active, &input)?);
        }
        active.owned_mecc.as_mut().unwrap().advance(
            &mut active.resident_states,
            &mut active.variant_random,
            input,
            backend,
        )
    }
}
