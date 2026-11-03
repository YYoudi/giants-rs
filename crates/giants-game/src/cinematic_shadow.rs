//! Source Flick34 shadow declarations. Texture production and terrain projection
//! are separate consumers; retaining a declaration does not allocate an image.
use giants_formats::flick::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub command_offset: usize,
    pub actor_slot: usize,
    /// Native descriptor words, before the texture consumer interprets them.
    pub texture_words: [u32; 2],
    /// Original float words; no finite, ordered or nondegenerate bounds implied.
    pub bounds_words: [[u32; 3]; 2],
}

impl Declaration {
    pub fn decode(command: &Command) -> Result<Option<Self>, &'static str> {
        if command.opcode != 34 {
            return Ok(None);
        }
        let words: [u32; 7] = command
            .words
            .as_slice()
            .try_into()
            .map_err(|_| "shadow declaration requires seven words")?;
        if (words[0] as i32) < 0 {
            return Ok(None);
        }
        let actor_slot = words[0] as usize;
        if actor_slot >= 32 {
            return Err("shadow actor outside bounded director profile");
        }
        Ok(Some(Self {
            command_offset: command.offset,
            actor_slot,
            texture_words: [words[1], words[2]],
            bounds_words: [[words[3], words[4], 0], [words[5], words[6], 0]],
        }))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Declarations {
    actors: [Option<Declaration>; 32],
}

#[derive(Clone, Copy, Debug)]
pub struct Mapping {
    pub world_to_texture: crate::procedural_aim::Matrix,
    /// Four transformed plane corners, before grid clipping or terrain height.
    pub footprint: [[f32; 3]; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct SilhouetteCamera {
    /// Engine row-major inverse frame, before the renderer's handedness change.
    pub view_from_world: crate::procedural_aim::Matrix,
    /// Native renderer row-vector view, with its third column sign reversed.
    pub renderer_view: [f32; 16],
    /// Native renderer row-vector projection words. Depth is flattened to zero.
    pub projection: [f32; 16],
    pub viewport: [u32; 4],
    pub aspect: f32,
}

impl SilhouetteCamera {
    /// Uses the supplied, allocated surface extent, not an assumed allocation
    /// from declaration words. One black border pixel surrounds the silhouette.
    pub fn prepare(
        frame: crate::procedural_aim::Matrix,
        bounds: [f32; 4],
        extent: [u32; 2],
    ) -> Result<Self, &'static str> {
        let [width, height] = extent;
        if width <= 2 || height <= 2 || width > 4096 || height > 4096 {
            return Err("silhouette surface outside bounded viewport profile");
        }
        validate_mapping_inputs(frame, bounds)?;
        let view_from_world = frame.inverse()?;
        let renderer_view = std::array::from_fn(|i| {
            let value = view_from_world.values[(i % 4) * 4 + i / 4];
            if i % 4 == 2 { -value } else { value }
        });
        let sx = (2. / (f64::from(bounds[2]) - f64::from(bounds[0]))) as f32;
        let sy = (2. / (f64::from(bounds[3]) - f64::from(bounds[1]))) as f32;
        let tx = ((f64::from(bounds[0]) + f64::from(bounds[2])) * f64::from(sx) * -0.5) as f32;
        let ty = ((f64::from(bounds[1]) + f64::from(bounds[3])) * f64::from(sy) * -0.5) as f32;
        let aspect = (f64::from(sx) / f64::from(sy)) as f32;
        let projection = [
            sx, 0., 0., 0., 0., sy, 0., 0., 0., 0., 0., 0., tx, ty, 0., 1.,
        ];
        if projection
            .iter()
            .chain(&view_from_world.values)
            .any(|v| !v.is_finite())
            || !aspect.is_finite()
        {
            return Err("silhouette camera overflow");
        }
        Ok(Self {
            view_from_world,
            renderer_view,
            projection,
            viewport: [1, 1, width - 2, height - 2],
            aspect,
        })
    }
}

fn validate_mapping_inputs(
    frame: crate::procedural_aim::Matrix,
    bounds: [f32; 4],
) -> Result<(), &'static str> {
    if frame.flags & !31 != 0
        || frame.values[12..] != [0., 0., 0., 1.]
        || frame.values.iter().chain(&bounds).any(|v| !v.is_finite())
        || bounds[2] <= bounds[0]
        || bounds[3] <= bounds[1]
    {
        return Err("shadow mapping requires finite affine frame and ordered plane bounds");
    }
    Ok(())
}

/// Native Flick shadow frame from the already evaluated actor matrix. Actor
/// animation/attachment evaluation must happen first; no actor-origin guess.
pub fn projector_frame(
    actor: crate::procedural_aim::Matrix,
) -> Result<crate::procedural_aim::Matrix, &'static str> {
    use crate::animation::{cross, normalize};
    use crate::procedural_aim::Matrix;
    if actor.flags & !31 != 0
        || actor.values[12..] != [0., 0., 0., 1.]
        || actor.values.iter().any(|v| !v.is_finite())
    {
        return Err("shadow frame requires finite evaluated affine actor matrix");
    }
    let mut forward = actor.transform([0., 1., 0., 0.]);
    forward[2] = 0.;
    let forward = normalize(forward).map_err(|_| "degenerate horizontal shadow direction")?;
    let up = [0., 0., 1.];
    let right = cross(forward, up);
    Ok(Matrix {
        flags: 17,
        values: [
            right[0],
            forward[0],
            up[0],
            actor.values[3],
            right[1],
            forward[1],
            up[1],
            actor.values[7],
            right[2],
            forward[2],
            up[2],
            actor.values[11],
            0.,
            0.,
            0.,
            1.,
        ],
    })
}

impl Mapping {
    /// Double53 affine preparation from an explicitly supplied projection frame.
    /// Frame selection, silhouette drawing and the terrain mesh remain separate.
    pub fn prepare(
        world_from_shadow: crate::procedural_aim::Matrix,
        bounds: [f32; 4],
    ) -> Result<Self, &'static str> {
        use crate::procedural_aim::Matrix;
        validate_mapping_inputs(world_from_shadow, bounds)?;
        let mut translation = Matrix::identity();
        translation.flags = 16;
        translation.values[3] = -bounds[0];
        translation.values[7] = -bounds[3];
        let mut scale = Matrix::identity();
        scale.flags = 10;
        scale.values[0] = (1. / (f64::from(bounds[2]) - f64::from(bounds[0]))) as f32;
        scale.values[5] = (-1. / (f64::from(bounds[3]) - f64::from(bounds[1]))) as f32;
        scale.values[10] = 0.;
        let world_to_texture = scale.mul(translation.mul(world_from_shadow.inverse()?)?)?;
        let footprint = [
            [bounds[0], bounds[1]],
            [bounds[2], bounds[1]],
            [bounds[2], bounds[3]],
            [bounds[0], bounds[3]],
        ]
        .map(|[x, y]| world_from_shadow.transform([x, y, 0., 1.]));
        if world_to_texture
            .values
            .iter()
            .chain(footprint.iter().flatten())
            .any(|v| !v.is_finite())
        {
            return Err("shadow mapping overflow");
        }
        Ok(Self {
            world_to_texture,
            footprint,
        })
    }
}

impl Declarations {
    /// The caller supplies the native global suppression policy. Suppression
    /// skips new writes; it does not remove a previously declared shadow.
    pub fn apply(
        &mut self,
        command: &Command,
        suppressed: bool,
    ) -> Result<Option<Declaration>, &'static str> {
        if command.opcode != 34 || suppressed {
            return Ok(None);
        }
        let Some(declaration) = Declaration::decode(command)? else {
            return Ok(None);
        };
        self.actors[declaration.actor_slot] = Some(declaration);
        Ok(Some(declaration))
    }

    pub fn actor(&self, slot: usize) -> Option<Declaration> {
        self.actors.get(slot).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(slot: u32, offset: usize) -> Command {
        Command {
            offset,
            opcode: 34,
            words: vec![slot, 6, 5, 0x80000000, 0x7fc00001, 1, u32::MAX],
            label: None,
        }
    }

    #[test]
    fn replacement_is_actor_local_and_suppression_keeps_previous_declarations() {
        let mut declarations = Declarations::default();
        let first = declarations.apply(&command(2, 40), false).unwrap();
        declarations.apply(&command(3, 80), false).unwrap();
        assert_eq!(declarations.actor(2), first);
        assert_eq!(declarations.actor(3).unwrap().command_offset, 80);
        assert_eq!(declarations.apply(&command(2, 120), true), Ok(None));
        assert_eq!(declarations.actor(2), first);
        declarations.apply(&command(2, 160), false).unwrap();
        assert_eq!(declarations.actor(2).unwrap().command_offset, 160);
        assert_eq!(
            declarations.actor(2).unwrap().bounds_words,
            [[0x80000000, 0x7fc00001, 0], [1, u32::MAX, 0]]
        );
        assert_eq!(declarations.actor(3).unwrap().command_offset, 80);
        assert_eq!(Declarations::default().actor(2), None);
    }

    #[test]
    fn negative_actor_is_skipped_and_invalid_positive_slot_is_not_remapped() {
        let mut declarations = Declarations::default();
        assert_eq!(declarations.apply(&command(u32::MAX, 0), false), Ok(None));
        assert!(declarations.apply(&command(32, 0), false).is_err());
        assert_eq!(declarations.apply(&command(32, 0), true), Ok(None));
        assert_eq!(declarations.actor(32), None);
    }

    #[test]
    fn mapping_rejects_degenerate_bounds_and_non_affine_or_singular_frames() {
        use crate::procedural_aim::Matrix;
        let identity = Matrix::identity();
        assert!(Mapping::prepare(identity, [0., 0., 0., 1.]).is_err());
        assert!(Mapping::prepare(identity, [0., 0., 1., f32::NAN]).is_err());
        let mut frame = identity;
        frame.flags = 32;
        assert!(Mapping::prepare(frame, [0., 0., 1., 1.]).is_err());
        frame.flags = 2;
        frame.values[0] = 0.;
        assert!(Mapping::prepare(frame, [0., 0., 1., 1.]).is_err());
        frame = identity;
        frame.values[15] = 0.;
        assert!(Mapping::prepare(frame, [0., 0., 1., 1.]).is_err());
    }

    #[test]
    fn projector_rejects_vertical_direction_and_invalid_evaluated_inputs() {
        use crate::procedural_aim::Matrix;
        let mut actor = Matrix::identity();
        actor.flags = 1;
        actor.values[5] = 0.;
        actor.values[9] = 1.;
        assert!(projector_frame(actor).is_err());
        actor = Matrix::identity();
        actor.values[3] = f32::NAN;
        assert!(projector_frame(actor).is_err());
        actor = Matrix::identity();
        actor.flags = 32;
        assert!(projector_frame(actor).is_err());
        actor = Matrix::identity();
        actor.values[15] = 0.;
        assert!(projector_frame(actor).is_err());
    }

    #[test]
    fn silhouette_rejects_empty_interior_and_unrepresentable_projection() {
        use crate::procedural_aim::Matrix;
        let frame = Matrix::identity();
        assert!(SilhouetteCamera::prepare(frame, [-1., -1., 1., 1.], [2, 32]).is_err());
        assert!(SilhouetteCamera::prepare(frame, [-1., -1., 1., 1.], [64, 4097]).is_err());
        assert!(SilhouetteCamera::prepare(frame, [0., 0., 0., 1.], [64, 32]).is_err());
        assert!(
            SilhouetteCamera::prepare(frame, [0., 0., f32::from_bits(1), 1.], [64, 32]).is_err()
        );
    }
}
