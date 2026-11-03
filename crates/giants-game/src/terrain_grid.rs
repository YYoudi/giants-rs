//! Native post-decode terrain preparation. The raw format reader stays distinct.
use giants_formats::{Error, gti::Terrain};

pub struct Prepared {
    pub terrain: Terrain,
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub face_classes: Vec<u8>,
    pub packed_tags: Vec<u16>,
    /// First/last cell whose height is not the native sentinel; empty = width/-1.
    pub row_spans: Vec<[i32; 2]>,
}

pub fn prepare(mut terrain: Terrain, thresholds: [f32; 2]) -> Result<Prepared, Error> {
    let count = terrain
        .width
        .checked_mul(terrain.height)
        .ok_or(Error("terrain grid overflow"))?;
    if !matches!(terrain.source_flags, 2 | 3)
        || terrain.width < 2
        || terrain.height < 2
        || count > 262144
        || terrain.heights.len() != count
        || terrain.tags.len() != count
        || terrain.colors.len() != count
        || !terrain.spacing.is_finite()
        || terrain.spacing <= 0.
        || !thresholds.iter().all(|v| v.is_finite())
        || terrain.tags.iter().any(|t| *t > 7)
        || !terrain.heights.iter().all(|v| v.is_finite())
    {
        return Err(Error("terrain postprocess outside decoded RGB profile"));
    }
    let row_spans = terrain
        .heights
        .chunks_exact(terrain.width)
        .map(|row| {
            let valid = |v: &f32| v.to_bits() != (-4096f32).to_bits();
            [
                row.iter()
                    .position(valid)
                    .map_or(terrain.width as i32, |i| i as i32),
                row.iter().rposition(valid).map_or(-1, |i| i as i32),
            ]
        })
        .collect();
    let positions = (0..count)
        .map(|i| terrain.position(i))
        .collect::<Result<Vec<_>, _>>()?;
    for tag in &mut terrain.tags {
        if *tag == 7 {
            *tag = 4;
        }
    }
    let triangles = terrain.triangles()?;
    let face_classes = triangles
        .iter()
        .map(|triangle| {
            giants_formats::gti::face_class_with_thresholds(
                triangle.map(|i| positions[i as usize]),
                thresholds,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut next = 0;
    let packed_tags = terrain
        .tags
        .iter()
        .map(|tag| {
            if *tag == 0 {
                return 0;
            }
            let first = face_classes[next];
            next += 1;
            let second = if matches!(*tag, 5 | 6) {
                let value = face_classes[next];
                next += 1;
                value
            } else {
                0
            };
            u16::from(*tag) | ((u16::from(first) + 9 * u16::from(second)) << 3)
        })
        .collect();
    Ok(Prepared {
        terrain,
        positions,
        triangles,
        face_classes,
        packed_tags,
        row_spans,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{arithmetic::Precision, terrain_queries::StaticTerrainQuery};

    fn source(flags: u8) -> Terrain {
        Terrain {
            source_flags: flags,
            width: 2,
            height: 2,
            origin: [0., 0.],
            spacing: 1.,
            height_bounds: [0., 10.],
            fallback_texture: String::new(),
            heights: vec![0., 0., 0., 10.],
            colors: vec![0xff123456; 4],
            tags: vec![7, 0, 0, 0],
            consumed_bytes: 0,
            opaque_tail: vec![],
        }
    }

    #[test]
    fn loaded_seven_becomes_contact_surface_without_changing_render_faces() {
        let raw = source(2);
        let triangles = raw.triangles().unwrap();
        fn query(t: &Terrain) -> crate::terrain_queries::GroundSample {
            StaticTerrainQuery::new(t, 1., Precision::Double53)
                .unwrap()
                .sample([0.25, 0.75])
                .unwrap()
        }
        assert_eq!(raw.tags[0], 7);
        assert_eq!(query(&raw).triangle, 0);
        let prepared = prepare(raw, giants_formats::gti::DEFAULT_SLOPE_THRESHOLDS).unwrap();
        assert_eq!(prepared.terrain.tags[0], 4);
        assert_eq!(prepared.triangles, triangles);
        assert_ne!(query(&prepared.terrain).triangle, 0);
        assert!((query(&prepared.terrain).height - 2.5).abs() < 0.00001);
        assert_eq!(prepared.packed_tags[0] & 7, 4);
        assert_eq!(
            prepared.packed_tags[0] >> 3,
            u16::from(prepared.face_classes[0])
        );
    }

    #[test]
    fn row_spans_use_height_even_when_cell_has_no_face() {
        let mut t = source(3);
        t.tags.fill(0);
        t.heights = vec![-4096., -4096., -4096., 0.];
        let p = prepare(t, [0.6, 0.5]).unwrap();
        assert_eq!(p.row_spans, [[2, -1], [1, 1]]);
        assert!(p.triangles.is_empty());
        assert!(p.packed_tags.iter().all(|v| *v == 0));
    }

    #[test]
    fn both_rgb_profiles_prepare_identically_and_invalid_input_is_refused() {
        let a = prepare(source(2), [0.6, 0.5]).unwrap();
        let b = prepare(source(3), [0.6, 0.5]).unwrap();
        assert_eq!(a.packed_tags, b.packed_tags);
        assert_eq!(a.row_spans, b.row_spans);
        assert!(prepare(source(1), [0.6, 0.5]).is_err());
        assert!(prepare(source(2), [f32::NAN, 0.5]).is_err());
        let mut t = source(3);
        t.tags[3] = 7;
        assert!(prepare(t, [0.6, 0.5]).is_err());
    }
}
