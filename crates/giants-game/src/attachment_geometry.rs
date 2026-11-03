//! Explicit static inspection geometry. Source selection and the measured skin
//! kernel are reused; native live animation/LOD/submission policy stays separate.
use crate::{
    attachment_resources::Asset, catalog_registry::Location, scene_resources::SceneResources,
};
pub struct Geometry {
    pub positions: Vec<[f32; 3]>,
    pub skin: Option<Location>,
    pub profile: &'static str,
}
/// Fresh-buffer inspection at the constructed actor's zero clock. No live
/// shared-buffer history or channel advancement is inferred by this adapter.
pub fn prepare_actor_initial(
    asset: &Asset,
    actor: &crate::actor_context::Prepared,
) -> Result<Geometry, String> {
    let model = actor
        .class
        .specification
        .as_ref()
        .ok_or("actor model table absent")?
        .models
        .first()
        .and_then(Option::as_ref)
        .ok_or("actor model zero absent")?;
    if !model.name.eq_ignore_ascii_case(&asset.model.name)
        || model.prepared.mesh.positions != asset.model.mesh.positions
        || model.prepared.mesh.indices != asset.model.mesh.indices
    {
        return Err("constructed actor and attachment geometry identities differ".into());
    }
    let (positions, profile) = match actor.position_update(0, 0.)? {
        crate::actor_context::PositionUpdate::Keep => (
            asset.model.mesh.positions.clone(),
            "constructed_actor_fresh_buffer_kept",
        ),
        crate::actor_context::PositionUpdate::Replace(positions) => {
            (positions, "constructed_actor_zero_clock")
        }
    };
    if positions.len() != asset.model.mesh.positions.len() {
        return Err("constructed actor position count differs from attachment mesh".into());
    }
    Ok(Geometry {
        positions,
        skin: model.skeleton_source.clone(),
        profile,
    })
}
pub fn prepare(
    resources: &SceneResources,
    asset: &Asset,
    tick: i32,
) -> Result<Geometry, Box<dyn std::error::Error>> {
    let member = format!("{}.skn", asset.model.name);
    if !resources
        .archives
        .members
        .contains_key(&member.to_ascii_lowercase())
    {
        return Ok(Geometry {
            positions: asset.model.mesh.positions.clone(),
            skin: None,
            profile: "source_gbs",
        });
    }
    let location = resources
        .archives
        .select_bin(&member, &resources.language)?
        .clone();
    let skin =
        giants_formats::skn::parse(&resources.archives.read_bin(&member, &resources.language)?)?;
    let animation = asset
        .animation
        .as_ref()
        .ok_or("skin geometry needs a selected animation")?;
    let tree = animation
        .trees
        .iter()
        .find(|t| t.offset == animation.default_tree)
        .ok_or("skin geometry default tree missing")?;
    let positions = crate::skinning::positions(&skin, animation, &tree.name, tick)?;
    if skin.objects.len() != asset.model.mesh.groups.len()
        || positions.len() != asset.model.mesh.positions.len()
    {
        return Err("attachment skin/model geometry dimensions differ".into());
    }
    Ok(Geometry {
        positions,
        skin: Some(location),
        profile: "static_skin_default_tree",
    })
}
