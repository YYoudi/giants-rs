//! Attached wounds and shared particles advance in the mission component.
//! Rendering reads quads only; it cannot age a wound or consume actor RNG.
use super::*;
use giants_game::{audio_parameters::Random, blood_particles, blood_wounds, vimp_wander::Tick};

pub(super) struct Blood {
    pub particles: blood_particles::Pool,
    pub wounds: blood_wounds::Pool<Id, String>,
}
impl Blood {
    pub fn new(globals: &giants_formats::global_data::GlobalData) -> Result<Self, String> {
        Ok(Self {
            particles: blood_particles::Pool::new(blood_particles::Configuration::from_globals(
                globals,
            )?)?,
            wounds: blood_wounds::Pool::new(blood_wounds::Configuration::from_globals(globals)?)?,
        })
    }
    pub fn advance(
        &mut self,
        session: &Session,
        tick: Tick<'_>,
        world: &giants_game::WorldAsset,
        random: &mut Random,
    ) -> Result<(), String> {
        self.wounds.step(
            tick.global_delta,
            &mut Owners { session, tick },
            &mut self.particles,
            random,
        )?;
        let ground = giants_game::terrain_queries::StaticTerrainQuery::new(
            &world.terrain,
            1. / world.terrain.spacing,
            giants_game::arithmetic::Precision::Double53,
        )?;
        self.particles.update(
            random,
            |position| {
                Ok(giants_game::object_time::seconds(
                    tick.global_delta,
                    None,
                    Some(position),
                    tick.slow_areas,
                ) as f32)
            },
            |position| Ok(ground.sample([position[0], position[1]])?.height),
        )?;
        self.particles.purge();
        Ok(())
    }
}
struct Owners<'a> {
    session: &'a Session,
    tick: Tick<'a>,
}
impl blood_wounds::Scene<Id, String> for Owners<'_> {
    fn owner(&mut self, id: Id) -> Result<blood_wounds::Owner<String>, String> {
        let fields = self
            .session
            .get(id)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("wound owner absent")?;
        let context = self
            .session
            .vimp(id)
            .ok_or("wound Vimp owner replaced")?
            .context;
        Ok(blood_wounds::Owner {
            seconds: giants_game::object_time::seconds(
                self.tick.global_delta,
                Some(giants_game::object_time::Sample {
                    owner: context,
                    position: fields.position,
                }),
                None,
                self.tick.slow_areas,
            ) as f32,
            retired: fields.object_flags & 0x8000 != 0,
            position: fields.position,
            pose: self.session.vimp_wound_pose(id)?,
            joints: self.session.vimp_hit_point_joints(id)?,
        })
    }
}
