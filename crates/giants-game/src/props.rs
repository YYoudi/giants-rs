//! First cinematic pub, selected through the demonstrated world mission slot.
//! Other scenery, non-flat ground and animated pub parts remain unsupported.
use std::path::Path;

pub struct Pub {
    pub source_offset: usize,
    pub model: String,
    pub mesh: giants_formats::gbs::Mesh,
    pub textures: Vec<crate::LogoTexture>,
    pub serialized_position: [f32; 3],
    pub matrix: [f32; 16],
    pub material_passes: Vec<crate::render_parts::Pass>,
}

pub fn load_first(
    root: &Path,
    world: &crate::WorldAsset,
) -> Result<Pub, Box<dyn std::error::Error>> {
    let plan = crate::instances::Plan::load_first(root, world)?;
    load_planned(&plan, world)
}
pub fn load_planned(
    plan: &crate::instances::Plan,
    world: &crate::WorldAsset,
) -> Result<Pub, Box<dyn std::error::Error>> {
    let instance = plan.unique_class(53)?;
    let input = instance.source.as_ref().map_err(Clone::clone)?;
    let position = input.position;
    // Measured static pub collision profile: zero ground offset, flat cell,
    // and the native strict <0.1 ground snap. Do not extrapolate to slopes.
    let t = &world.terrain;
    let cell = [
        (position[0] - t.origin[0]) / t.spacing,
        (position[1] - t.origin[1]) / t.spacing,
    ];
    if cell.iter().any(|v| !v.is_finite() || *v < 0.)
        || cell[0] >= (t.width - 1) as f32
        || cell[1] >= (t.height - 1) as f32
    {
        return Err("Pub outside terrain".into());
    }
    let [x, y] = cell.map(|v| v.floor() as usize);
    if x + 1 >= t.width || y + 1 >= t.height {
        return Err("Pub outside terrain".into());
    }
    let heights = [
        t.heights[y * t.width + x],
        t.heights[y * t.width + x + 1],
        t.heights[(y + 1) * t.width + x],
        t.heights[(y + 1) * t.width + x + 1],
    ];
    if heights.iter().any(|h| h.to_bits() != heights[0].to_bits()) {
        return Err("Pub ground outside measured flat snap profile".into());
    }
    let geometry = instance.geometry(
        0,
        crate::scene_resources::TextureEncoding::Rgb565,
        crate::instances::Adjustment::Contact {
            ground: heights[0],
            plane: [0., 0., 1., -heights[0]],
            platform: None,
            vertical_velocity: 0.,
            flags: 0,
            lock_z: false,
            precision: crate::arithmetic::Precision::Double53,
        },
    )?;
    if geometry
        .contact
        .as_ref()
        .ok_or("Pub contact missing")?
        .flags
        & crate::ground_contact::CONTACT_BIT
        == 0
    {
        return Err("Pub ground outside measured flat snap profile".into());
    }
    let source = geometry.model.ok_or("Pub model slot is empty")?;
    let matrix = geometry.matrix;
    let material_passes = source.material_passes();
    Ok(Pub {
        source_offset: input.source_offset,
        model: source.name,
        mesh: source.mesh,
        textures: source.textures,
        serialized_position: instance.serialized.position,
        matrix,
        material_passes,
    })
}

pub(crate) fn first_mission(
    root: &Path,
    world: &crate::WorldAsset,
) -> Result<giants_formats::mission::Mission, Box<dyn std::error::Error>> {
    // Inspection context: preserve the measured benchmark slot for multi-entry
    // worlds; a single declared slot is unambiguous. Not campaign selection.
    let single = world.config.scenarios.len() == 1;
    let scenario = world
        .config
        .scenarios
        .get(usize::from(!single))
        .ok_or("Cinematic inspection mission slot missing")?;
    if scenario.kind != 2
        || scenario.parameter != if single { 1 } else { 2 }
        || scenario.name.is_empty()
        || !scenario
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("Unsupported first cinematic mission selection".into());
    }
    let mission = giants_formats::mission::parse(&std::fs::read(
        root.join("Bin").join(format!("wm_{}.bin", scenario.name)),
    )?)?;
    Ok(mission)
}
