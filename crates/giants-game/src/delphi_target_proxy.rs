//! Acquired current-target proxy shape. This prepares a matrix only; direct
//! actor construction, retargeting, lifetime, render routing and world scheduling
//! require separate owned services. It does not imply an ordinary actor affine.
use crate::procedural_aim::Matrix;

/// Source-backed direct class constructor. Unlike an attached/positioned effect,
/// this route has no FX cursor and does not initialize a physical block.
pub struct Template(crate::effect_actor::Template);
impl Template {
    pub fn load(resources: &crate::scene_resources::SceneResources) -> Result<Self, String> {
        Ok(Self(crate::effect_actor::Template::load(resources, 0x3d4)?))
    }
    pub fn construct(
        &self,
        session: &mut crate::resident_states::Session,
    ) -> Result<crate::resident_states::Id, String> {
        let t = &self.0;
        session.construct_delphi_target_proxy(crate::resident_states::Construction {
            class: 0x3d4,
            seed: &t.seed,
            bindings: &t.bindings,
            flags: crate::object_lists::DirectFlags {
                active: t.fields.object_flags,
                personality: t.fields.configuration[14],
            },
            specification: t.specification.clone(),
            state_resources: t.states.clone(),
            fields: t.fields.instantiate(),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub position: [f32; 3],
    pub height: f32,
    pub scale: f32,
    pub radius: f32,
    /// Actual current physical block flags, distinct from actor header flags.
    pub physical_flags: Option<u32>,
}
pub fn matrix(shape: Shape, factor: f32, half: f32) -> Result<Matrix, String> {
    if !shape
        .position
        .into_iter()
        .chain([shape.height, shape.scale, shape.radius, factor, half])
        .all(f32::is_finite)
    {
        return Err("Delphi target proxy outside acquired finite profile".into());
    }
    let half_height = (f64::from(shape.height) * f64::from(shape.scale)) * f64::from(half);
    // The offset is stored, while the original retains the wide half-height
    // for the subsequent scale multiplication.
    let offset = if shape.physical_flags.is_some_and(|flags| flags & 0x200 != 0) {
        0.
    } else {
        half_height as f32
    };
    let radial = (f64::from(shape.radius) * f64::from(factor)) as f32;
    let vertical = (half_height * f64::from(factor)) as f32;
    let center_z = (f64::from(offset) + f64::from(shape.position[2])) as f32;
    if ![offset, radial, vertical, center_z]
        .into_iter()
        .all(f32::is_finite)
    {
        return Err("Delphi target proxy matrix overflow".into());
    }
    let axes = [radial, radial, vertical];
    let mut scale = Matrix::identity();
    scale.flags = 2 | if axes.into_iter().filter(|v| *v < 0.).count() % 2 != 0 {
        8
    } else {
        0
    };
    for (at, value) in [0, 5, 10].into_iter().zip(axes) {
        scale.values[at] = value;
    }
    let mut translation = Matrix::identity();
    translation.flags = 0x10;
    for (at, value) in [3, 7, 11]
        .into_iter()
        .zip([shape.position[0], shape.position[1], center_z])
    {
        translation.values[at] = value;
    }
    translation.mul(scale).map_err(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT; silent cold direct proxy resource control"]
    fn source_delphi_proxy_is_direct_zero_state_without_effect_origin() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English")
                .unwrap();
        let template = Template::load(&resources).unwrap();
        let mut session = crate::resident_states::Session::new(983);
        let ids: Vec<_> = (0..3)
            .map(|_| template.construct(&mut session).unwrap())
            .collect();
        assert_eq!(session.list(0), ids);
        for id in &ids {
            let actor = session.get(*id).unwrap();
            assert_eq!(actor.class, 0x3d4);
            assert_eq!(actor.seed.storage.state(), 78);
            assert_eq!(actor.seed.storage.data().unwrap(), &[0; 268]);
            assert_eq!(actor.constructor_mask, 0);
            assert!(actor.attachment.is_none());
            let fields = actor.construction_fields.as_ref().unwrap();
            assert_eq!(fields.object_flags, 0x10);
            assert_eq!(fields.configuration[14], 0x840);
            assert_eq!(fields.position, [0.; 3]);
            assert!(session.actor_current_affine(*id).is_err());
        }
        for list in 1..15 {
            assert!(session.list(list).is_empty());
        }
        session.clear();
        assert!(session.list(0).is_empty());
        assert!(ids.into_iter().all(|id| session.get(id).is_none()));
        let next = template.construct(&mut session).unwrap();
        assert!(session.get(next).is_some());
    }
}
