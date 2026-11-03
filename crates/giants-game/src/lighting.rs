//! Declared directional lights. Missing secondary slots contribute no light in
//! the renderer; their native enable/reset behavior still needs visual control.
pub use giants_formats::world::Fog;
use giants_formats::{Error, world::WorldConfig};
pub fn load_first(
    root: &std::path::Path,
) -> Result<(Vec<Directional>, Fog), Box<dyn std::error::Error>> {
    let config =
        giants_formats::world::parse_world(&std::fs::read(root.join("Bin/w_Story4.bin"))?)?;
    Ok((first_world(&config)?, config.fog))
}

pub struct Directional {
    pub variant: u8,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub color: [f32; 3],
}
/// Initial unbound primary-light cache direction. This is the static world
/// declaration profile; auto-assigned slots, parents and live callbacks remain
/// separate. The terrain caller negates this getter output once more.
pub fn terrain_context_direction(config: &WorldConfig) -> Result<[f32; 3], Error> {
    initial_primary_direction(&config.lights)
}
fn initial_primary_direction(lights: &[giants_formats::world::Light]) -> Result<[f32; 3], Error> {
    if lights.iter().any(|light| light.variant.is_none()) {
        return Err(Error(
            "automatic light slot selection is outside terrain initialization",
        ));
    }
    let mut selected = lights.iter().filter(|light| light.variant == Some(1));
    let light = selected
        .next()
        .ok_or(Error("terrain primary light missing"))?;
    if selected.next().is_some() {
        return Err(Error(
            "multiple primary lights require live update ordering",
        ));
    }
    let matrix =
        crate::object_transform::euler(light.position, light.angles, light.scale).map_err(Error)?;
    Ok([-matrix[1], -matrix[5], -matrix[9]])
}

pub fn first_world(config: &WorldConfig) -> Result<Vec<Directional>, Error> {
    declared(&config.lights)
}
fn declared(lights: &[giants_formats::world::Light]) -> Result<Vec<Directional>, Error> {
    let mut result = Vec::new();
    for light in lights {
        let Some(variant @ 1..=3) = light.variant else {
            return Err(Error("unmeasured directional light variant"));
        };
        let color = light
            .color
            .ok_or(Error("directional light color missing"))?;
        if color.iter().any(|v| *v < 0.) {
            return Err(Error("negative directional light color"));
        }
        let matrix = crate::object_transform::euler(light.position, light.angles, light.scale)
            .map_err(Error)?;
        result.push(Directional {
            variant,
            position: light.position,
            direction: [-matrix[1], -matrix[5], -matrix[9]],
            color,
        });
    }
    result.sort_by_key(|light| light.variant);
    if result.first().map(|l| l.variant) != Some(1) {
        return Err(Error("primary directional light missing"));
    }
    if result
        .windows(2)
        .any(|pair| pair[0].variant == pair[1].variant)
    {
        return Err(Error("duplicate directional light variant"));
    }
    Ok(result)
}

/// Stable source slots, including holes. No direction/color is invented for
/// an absent declaration, and primary suppression always addresses slot1.
pub fn shader_slots(lights: &[Directional]) -> [Option<&Directional>; 3] {
    std::array::from_fn(|slot| {
        lights
            .iter()
            .find(|light| usize::from(light.variant) == slot + 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn light(variant: u8) -> giants_formats::world::Light {
        giants_formats::world::Light {
            source_offset: 0,
            position: [1., 2., 3.],
            angles: [0.; 3],
            scale: 1.,
            variant: Some(variant),
            color: Some([0.2, 0.3, 0.4]),
        }
    }
    #[test]
    fn declared_secondary_holes_preserve_primary_and_slot_identity() {
        for variants in [vec![1], vec![2, 1], vec![3, 1], vec![3, 2, 1]] {
            let inputs: Vec<_> = variants.iter().copied().map(light).collect();
            let output = declared(&inputs).unwrap();
            assert_eq!(output.len(), inputs.len());
            assert_eq!(output[0].variant, 1);
            for (slot, output) in shader_slots(&output).iter().enumerate() {
                assert_eq!(output.is_some(), variants.contains(&((slot + 1) as u8)));
                if let Some(output) = output {
                    assert_eq!(output.variant, (slot + 1) as u8);
                    assert_eq!(output.color, [0.2, 0.3, 0.4]);
                }
            }
        }
    }
    #[test]
    fn malformed_or_ambiguous_light_declarations_are_rejected() {
        for variants in [vec![], vec![2], vec![1, 1], vec![1, 2, 2], vec![1, 4]] {
            let inputs: Vec<_> = variants.into_iter().map(light).collect();
            assert!(declared(&inputs).is_err());
        }
        let mut missing = light(1);
        missing.color = None;
        assert!(declared(&[missing]).is_err());
    }
}

#[cfg(test)]
mod terrain_tests {
    use super::*;
    fn light(variant: Option<u8>) -> giants_formats::world::Light {
        giants_formats::world::Light {
            source_offset: 0,
            position: [0.; 3],
            angles: [0.; 3],
            scale: 1.,
            variant,
            color: Some([1.; 3]),
        }
    }
    #[test]
    fn primary_direction_is_unscaled_by_color_or_translation() {
        let mut a = light(Some(1));
        let expected = initial_primary_direction(&[light(Some(2)), light(Some(1))]).unwrap();
        a.position = [10., -30., 50.];
        a.color = Some([0.2, 0.7, 0.4]);
        assert_eq!(initial_primary_direction(&[a]).unwrap(), expected);
        assert_eq!(expected, [0., -1., -0.]);
    }
    #[test]
    fn missing_automatic_and_duplicate_primary_slots_are_refused() {
        assert!(initial_primary_direction(&[]).is_err());
        assert!(initial_primary_direction(&[light(Some(2))]).is_err());
        assert!(initial_primary_direction(&[light(None), light(Some(1))]).is_err());
        assert!(initial_primary_direction(&[light(Some(1)), light(Some(1))]).is_err());
    }
}
