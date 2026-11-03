//! Explicit static Mecc test target. It is neither mission placement nor AI.
use bevy::prelude::*;
use giants_game::{arithmetic::Precision, mecc_damage};
#[derive(Resource)]
pub struct Target {
    pub enabled: bool,
    pub health: mecc_damage::State,
    pub aim: [f32; 3],
    pub deaths: u32,
    parts: Vec<Vec<[f32; 3]>>,
    collider: giants_game::object_segment::Mesh,
}
impl Target {
    pub fn trace(&self, start: [f32; 3], end: [f32; 3]) -> Result<Option<[f32; 3]>, &'static str> {
        if !self.enabled || !self.health.alive() {
            return Ok(None);
        }
        self.collider.hit(start, end)
    }
    pub fn reset(&mut self) {
        self.health = mecc_damage::State::cold();
        self.deaths = 0;
    }
    pub fn impact(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        amount: f32,
    ) -> Result<Option<[f32; 3]>, &'static str> {
        if !self.enabled || !self.health.alive() {
            return Ok(None);
        }
        let Some(point) = self.collider.hit(start, end)? else {
            return Ok(None);
        };
        let hit = self.health.apply(mecc_damage::Hit {
            kind: 41,
            amount,
            globally_disabled: false,
            cutscene: false,
            allowed: true,
            shield_percent: 0.,
        })?;
        self.health = hit.state;
        self.deaths += u32::from(hit.died);
        println!(
            "mecc_dev_object_hit id={} amount={amount} damage={} limit={} died={} team_policy=development native_death_presentation=false",
            self.collider.id, self.health.damage, self.health.limit, hit.died
        );
        Ok(Some(point))
    }
}
#[derive(Component)]
struct TargetDraw;
pub(crate) fn load(
    actor: &crate::mecc_dev_scene::State,
    scene: &crate::world_scene::Scene,
) -> Result<Target, Box<dyn std::error::Error>> {
    let enabled = std::env::var("GIANTS_MECC_DEV_REPLAY")
        .ok()
        .is_none_or(|s| {
            s.starts_with("damage") || s.starts_with("player-") || s.starts_with("session-cycles")
        });
    let owner = giants_game::object_transform::euler(
        actor.start.position,
        [actor.initial_yaw.to_degrees(), 0., 0.],
        actor.scale,
    )?;
    let mut position =
        std::array::from_fn(|i| actor.start.position[i] - owner[i * 4 + 1] / actor.scale * 30.);
    let ground = giants_game::terrain_queries::StaticTerrainQuery::new(
        &scene.asset.terrain,
        1. / scene.asset.terrain.spacing,
        Precision::Double53,
    )?
    .sample([position[0], position[1]])?;
    if !ground.in_bounds {
        return Err("development target outside terrain".into());
    }
    let positions = actor.standing_positions();
    let low = positions.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
    position[2] = ground.height - low * actor.scale;
    let matrix = giants_game::object_transform::euler(
        position,
        [actor.initial_yaw.to_degrees() + 180., 0., 0.],
        actor.scale,
    )?;
    let pose = giants_game::collision_world::SourcePose::from_matrix(matrix)?;
    let posed = positions
        .iter()
        .map(|p| pose.transform(*p, Precision::Double53))
        .collect::<Result<Vec<_>, _>>()?;
    let parts = actor
        .body
        .mesh
        .subobjects
        .iter()
        .map(|part| {
            part.triangles
                .iter()
                .flatten()
                .map(|id| {
                    let p = posed[usize::from(actor.body.mesh.indices[usize::from(*id)])];
                    [p[0], p[2], -p[1]]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let triangles = parts
        .iter()
        .flat_map(|part| {
            part.as_chunks::<3>()
                .0
                .iter()
                .map(|face| std::array::from_fn(|i| [face[i][0], -face[i][2], face[i][1]]))
        })
        .collect::<Vec<_>>();
    let aim = std::array::from_fn(|i| {
        (posed.iter().map(|p| p[i]).fold(f32::INFINITY, f32::min)
            + posed.iter().map(|p| p[i]).fold(f32::NEG_INFINITY, f32::max))
            * 0.5
    });
    let collider = giants_game::object_segment::Mesh::new(1, triangles)?;
    println!(
        "mecc_dev_target enabled={enabled} class=2 placement=development position={position:?} aim={aim:?} geometry=standing_source_mesh ai=false"
    );
    Ok(Target {
        enabled,
        health: mecc_damage::State::cold(),
        aim,
        deaths: 0,
        parts,
        collider,
    })
}
pub fn install(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let target = load(
        app.world().resource::<crate::mecc_dev_scene::State>(),
        app.world().resource::<crate::world_scene::Scene>(),
    )?;
    app.insert_resource(target).add_systems(
        Update,
        update
            .after(crate::mecc_combat::CombatUpdate)
            .run_if(crate::mecc_session::playing),
    );
    Ok(())
}
pub(crate) fn setup(
    mut commands: Commands,
    state: Res<crate::mecc_dev_scene::State>,
    target: Res<Target>,
    templates: Query<
        (&Mesh3d, &MeshMaterial3d<crate::actor_scene::Material>),
        With<crate::mecc_dev_scene::BodyDraw>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !target.enabled {
        return;
    }
    for (part, source) in &state.draws {
        let material = templates
            .iter()
            .find(|(mesh, _)| mesh.0 == *source)
            .expect("prepared player body template")
            .1
            .0
            .clone();
        let mut mesh = meshes.get(source).expect("prepared body mesh").clone();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, target.parts[*part].clone());
        commands.spawn((
            TargetDraw,
            Name::new("Development Mecc target / no AI"),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material),
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
}
fn update(target: Res<Target>, mut draws: Query<&mut Visibility, With<TargetDraw>>) {
    for mut visibility in &mut draws {
        // Removal is feedback for the development target, not the native death animation.
        *visibility = if target.health.alive() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
