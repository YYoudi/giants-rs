//! Mission resource/identity binding, before admission to common world callbacks.
//! No population, clock, random stream or successful actor update is fabricated.
use giants_game::{
    attached_effect_context::Pool,
    declared_residents::{Key, Report, Scope},
    instances::Plan,
    resident_states::{Id, Session},
    vimp_actor::Factory,
};
use std::collections::BTreeMap;
use std::sync::Arc;
mod auxiliary_draw;
mod blood;
pub(super) mod collection;
pub(super) mod live;
mod sounds;
use live::Driver;

pub(super) struct State {
    driver: Driver,
    blood: blood::Blood,
    fx: giants_game::vimp_death_fx::Runtime,
    proxy: giants_game::vimp_death_proxy::Factory,
    chunk_cursor: i32,
    collection: Option<collection::Collection>,
    collection_catalog: giants_formats::gtext::Catalog,
    factory: Factory,
    globals: giants_formats::global_data::GlobalData,
    expected: Vec<(Key, i32)>,
    bound: Option<Binding>,
    geometry: Option<Arc<giants_game::scene_resources::ModelAsset>>,
    auxiliary_geometry: BTreeMap<i32, Arc<giants_game::scene_resources::ModelAsset>>,
    contact_resources: Option<giants_game::vimp_contacts::Resources>,
}

/// A copied submission, not a second simulation or shared pose buffer.
pub(crate) struct Draw {
    pub texture: Option<String>,
    pub class: i32,
    pub transparency: f32,
    pub id: DrawOwner,
    pub frame: u64,
    pub geometry: Arc<giants_game::scene_resources::ModelAsset>,
    pub positions: Vec<[f32; 3]>,
    pub center: [f32; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DrawOwner {
    BuildingEffect(u64),
    Actor(Id),
    Effect(giants_game::vimp_death_fx::Key),
}
struct Binding {
    ticket: u64,
    declarations: Vec<(Key, Id, i32)>,
    contexts: Pool<Id>,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct View {
    pub herds: usize,
    pub markers: usize,
    pub vimps: usize,
    pub effect_contexts: usize,
}
impl State {
    pub(super) fn blood_quads(&self) -> Vec<giants_game::blood_particles::Quad> {
        self.blood.particles.quads().collect()
    }
    pub fn prepare(root: &std::path::Path, plan: &Plan) -> Result<Option<Self>, String> {
        let mut expected: Vec<_> = plan
            .world_entries
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(p.declaration.object_type, 49 | 679))
            .map(|(index, p)| {
                (
                    Key {
                        scope: Scope::World,
                        index,
                    },
                    p.declaration.object_type as i32,
                )
            })
            .collect();
        expected.extend(plan.entries.iter().enumerate().filter_map(|(index, p)| {
            let class = p.source.as_ref().ok()?.class;
            matches!(class, 49 | 679).then_some((
                Key {
                    scope: Scope::Mission,
                    index,
                },
                class,
            ))
        }));
        if !expected.iter().any(|(_, class)| *class == 49) {
            return Ok(None);
        }
        let globals = giants_formats::global_data::parse(
            &std::fs::read(root.join("Bin/GData.gbn")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let factory = Factory::load_registered(plan.resources())?;
        Ok(Some(Self {
            driver: Driver::default(),
            blood: blood::Blood::new(&globals)?,
            fx: giants_game::vimp_death_fx::Runtime::new(root, 0, plan.resources())?,
            proxy: giants_game::vimp_death_proxy::Factory::load(plan.resources())?,
            chunk_cursor: 1117,
            collection: None,
            collection_catalog: giants_formats::gtext::Catalog::parse(
                &std::fs::read(root.join("Bin/GTextEnglish.bin")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?,
            factory,
            globals,
            expected,
            bound: None,
            geometry: None,
            auxiliary_geometry: BTreeMap::new(),
            contact_resources: None,
        }))
    }
    pub fn bind(
        &mut self,
        ticket: u64,
        report: &Report,
        session: &Session,
    ) -> Result<View, String> {
        if self.bound.is_some() {
            return Err(
                "NPC mission resources already bound; new entry requires fresh preparation".into(),
            );
        }
        // Validate actual source constants before exposing resources to a future
        // birth callback. This does not advance its clock or select a seed.
        for at in [0xbb8, 0xbbc, 0xbc0] {
            if !self
                .globals
                .scalar(at)
                .map_err(|e| e.to_string())?
                .is_finite()
            {
                return Err("NPC birth parameter is nonfinite".into());
            }
        }
        let mut declarations = Vec::with_capacity(self.expected.len());
        for &(key, class) in &self.expected {
            let record = report
                .records
                .iter()
                .find(|r| r.key == key)
                .ok_or("NPC declaration construction missing")?;
            let id = *record
                .result
                .as_ref()
                .map_err(|e| format!("NPC declaration construction refused: {e}"))?;
            Self::check(session, ticket, key, id, class)?;
            declarations.push((key, id, class));
        }
        self.fx.bind_ticket(ticket)?;
        self.bound = Some(Binding {
            ticket,
            declarations,
            contexts: self.factory.animation_contexts(ticket),
        });
        self.view(session, ticket)
    }
    fn check(session: &Session, ticket: u64, key: Key, id: Id, class: i32) -> Result<(), String> {
        let actor = session
            .get(id)
            .ok_or("NPC declaration owner absent or foreign")?;
        if id.ticket() != ticket
            || session.declaration(key) != Some(id)
            || actor.class != class
            || (class == 49 && session.herd_clock(id).is_none())
            || (class == 679 && session.herd_marker(id).is_none())
        {
            return Err("NPC declaration identity or source construction changed".into());
        }
        Ok(())
    }
    pub fn view(&self, session: &Session, ticket: u64) -> Result<View, String> {
        let b = self
            .bound
            .as_ref()
            .ok_or("NPC mission resources not bound")?;
        if b.ticket != ticket {
            return Err("NPC mission ticket differs".into());
        }
        for &(key, id, class) in &b.declarations {
            Self::check(session, ticket, key, id, class)?;
        }
        Ok(View {
            herds: b
                .declarations
                .iter()
                .filter(|(_, _, class)| *class == 49)
                .count(),
            markers: b
                .declarations
                .iter()
                .filter(|(_, _, class)| *class == 679)
                .count(),
            // Count for diagnostics only; never rediscover an owner from class.
            vimps: session
                .list(0)
                .into_iter()
                .filter(|&id| session.vimp(id).is_some())
                .count(),
            effect_contexts: b.contexts.active().len(),
        })
    }
    /// Render only actual completed common-node frames. Primary model zero is
    /// an explicit presentation policy; native world visibility/LOD is not
    /// established by this consumer. It never advances AI, ANM, clocks or RNG.
    pub fn draws(
        &mut self,
        session: &mut Session,
        ticket: u64,
        resources: &giants_game::scene_resources::SceneResources,
    ) -> Result<Vec<Draw>, String> {
        self.view(session, ticket)?;
        let ids: Vec<_> = session
            .list(0)
            .into_iter()
            .filter(|&id| session.vimp(id).is_some())
            .collect();
        let mut result = Vec::new();
        for id in ids {
            if !self.factory.owns(session, id) {
                return Err("Vimp draw resources belong to another constructor".into());
            }
            let actor = session.get(id).ok_or("Vimp draw owner retired")?;
            let vimp = session.vimp(id).unwrap();
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("Vimp draw fields absent")?;
            if !giants_game::draw_routes::admit(
                u32::from(actor.seed.storage.state()),
                fields.object_flags,
                0,
            )
            .ordinary
            {
                continue;
            }
            // A cold birth has no current pose. A completed isolated physics or
            // ANM phase is also insufficient to acknowledge a common callback.
            if !vimp.initialized || session.vimp_frames(id) == Some(0) {
                continue;
            }
            let completed = session.completed_vimp_frame(id)?;
            let affine = session.actor_current_affine(id)?;
            if affine.flags & 8 != 0 {
                return Err("Vimp draw mirrored culling consumer required".into());
            }
            let center = fields.position;
            let epoch = completed.animation_epoch;
            let spec = session.specification(id).unwrap().clone();
            let model = spec
                .models
                .first()
                .and_then(Option::as_ref)
                .ok_or("Vimp primary draw model absent")?;
            if self.geometry.is_none() {
                let geometry = resources
                    .model_slot_for_body(
                        8,
                        0,
                        giants_game::scene_resources::TextureEncoding::Rgb565OrArgb4444,
                    )
                    .map_err(|e| e.to_string())?
                    .ok_or("Vimp draw geometry absent")?;
                if geometry.name != model.name || geometry.mesh.normal_count != 0 {
                    return Err("Vimp draw geometry identity or unlit profile differs".into());
                }
                for part in &geometry.mesh.subobjects {
                    let surface = giants_game::body_pipeline::preview_surface(
                        giants_game::body_pipeline::Normals {
                            prepare: false,
                            context: false,
                        },
                        &part.material,
                        geometry
                            .alpha_textures
                            .contains(&part.material.texture.to_ascii_lowercase()),
                    )?;
                    if surface != giants_game::body_pipeline::Surface::Rgb {
                        return Err("Vimp draw requires the acquired RGB body material".into());
                    }
                }
                self.geometry = Some(Arc::new(geometry));
            }
            session.prepare_actor_positions(id, 0, epoch, false)?;
            let local = session
                .actor_model_positions(id, 0)?
                .ok_or("Vimp animated positions not produced")?;
            let matrix: [f32; 12] = affine.matrix[..12].try_into().unwrap();
            // Copy immediately: another Vimp can reuse the native-style shared
            // model buffer in this same render pass with a different pose.
            let positions = local
                .iter()
                .map(|&point| {
                    let p = giants_game::uncached_pose::transform(
                        matrix,
                        point,
                        giants_game::arithmetic::Precision::Double53,
                    )?;
                    Ok([p[0], p[2], -p[1]])
                })
                .collect::<Result<Vec<_>, String>>()?;
            result.push(Draw {
                texture: None,
                class: 8,
                transparency: 0.,
                id: DrawOwner::Actor(id),
                frame: completed.number,
                geometry: self.geometry.as_ref().unwrap().clone(),
                positions,
                center,
            });
        }
        result.extend(self.auxiliary_draws(session, resources)?);
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
