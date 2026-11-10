//! Acquired debug Kabuto size-dependent parameter refresh. Growth decisions,
//! first-primary initialization and live movement are separate phases.
use crate::{actor_fields::Fields, arithmetic::Precision, collision_radius};
use giants_formats::global_data::GlobalData;
use giants_formats::world_bounds::Resolved;
/// Typed class/resource boundary; no native pointer is read from the payload.
pub struct Source<'a> {
    pub globals: &'a GlobalData,
    pub class_scale: f32,
    pub bounds: Resolved,
}
/// Independently owned candidate. The caller publishes it only after the rest of
/// its character phase succeeds; this service does not admit or tick an actor.
pub struct Prepared {
    pub fields: Fields,
    pub payload: Vec<u8>,
    pub radius: f32,
}
/// Acquired camera helper: the flag chooses the stored current gain; otherwise
/// the native route returns one. No growth, initialization or mode is changed.
pub fn camera_gain(payload: &[u8]) -> Result<f32, String> {
    if payload.len() != 844 {
        return Err("Kabuto camera gain requires its debug payload".into());
    }
    let word = |at: usize| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
    let gain = if word(0) & 1 != 0 {
        f32::from_bits(word(0x1d0))
    } else {
        1.
    };
    if !gain.is_finite() || gain <= 0. {
        return Err("Kabuto camera gain outside acquired positive profile".into());
    }
    Ok(gain)
}
/// Full acquired debug size refresh, using Double53 arithmetic. `None` follows
/// the absent-class boundary: only the size multiplier is stored. This does not
/// choose a growth target, advance current size, or initialize physical state.
pub fn prepare(
    fields: &Fields,
    payload: &[u8],
    radius: f32,
    source: Option<Source<'_>>,
) -> Result<Prepared, String> {
    if fields.class != 1 || payload.len() != 844 {
        return Err("Kabuto size refresh requires the debug class-1 payload".into());
    }
    let mut next = fields.clone();
    let mut body: Vec<u32> = payload
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    let x = f32::from_bits(body[0x15c / 4]) as f64;
    if !x.is_finite() {
        return Err("nonfinite current Kabuto size".into());
    }
    let gain = (x + 1.) * 0.25;
    let store = |body: &mut [u32], at: usize, value: f64| -> Result<(), String> {
        let value = value as f32;
        if !value.is_finite() {
            return Err("Kabuto size parameter overflow".into());
        }
        body[at / 4] = value.to_bits();
        Ok(())
    };
    store(&mut body, 0x1d0, gain)?;
    let Some(Source {
        globals,
        class_scale,
        bounds,
    }) = source
    else {
        return Ok(Prepared {
            fields: next,
            payload: body.into_iter().flat_map(u32::to_le_bytes).collect(),
            radius,
        });
    };
    let target = f32::from_bits(body[0x158 / 4]);
    if !target.is_finite() || !class_scale.is_finite() || class_scale <= 0. {
        return Err("Kabuto size requires finite target and positive class scale".into());
    }
    let g = |o| globals.scalar(o).map(f64::from).map_err(|e| e.to_string());
    let curve = |a: [usize; 4]| -> Result<f64, String> {
        let p = [g(a[0])?, g(a[1])?, g(a[2])?, g(a[3])?];
        let v = if x <= 1. {
            (1. - x) * p[0] + x * p[1]
        } else if x <= 2. {
            (2. - x) * p[1] + (x - 1.) * p[2]
        } else {
            (x - 2.) * p[3] + (3. - x) * p[2]
        };
        Ok(v)
    };
    for (dst, src) in [
        (0x164, [0x9f4, 0x1334, 0x1338, 0x133c]),
        (0x168, [0x9f8, 0x134c, 0x1350, 0x1354]),
        (0x16c, [0x9fc, 0x1340, 0x1344, 0x1348]),
        (0x170, [0xa00, 0x1358, 0x135c, 0x1360]),
        (0x1a0, [0x11b4, 0x11b8, 0x11bc, 0x11c0]),
        (0x1a4, [0x11c4, 0x11c8, 0x11cc, 0x11d0]),
        (0x1a8, [0x11d4, 0x11d8, 0x11dc, 0x11e0]),
        (0x1ac, [0x11e4, 0x11e8, 0x11ec, 0x11f0]),
        (0x1b0, [0x12b8, 0x12bc, 0x12c0, 0x12c4]),
        (0x1b4, [0x15a8, 0x15ac, 0x15b0, 0x15b4]),
        (0x1dc, [0x1238, 0x1304, 0x1308, 0x130c]),
        (0x1e0, [0x123c, 0x1310, 0x1314, 0x1318]),
        (0x1e4, [0x1240, 0x12f8, 0x12fc, 0x1300]),
        (0x1e8, [0x1244, 0x1328, 0x132c, 0x1330]),
        (0x1ec, [0x1248, 0x131c, 0x1320, 0x1324]),
        (0x330, [0x14a0, 0x14a4, 0x14a8, 0x14ac]),
    ] {
        store(&mut body, dst, curve(src)?)?;
    }
    let configuration12 = curve([0x13d0, 0x13d4, 0x13d8, 0x13dc])? as f32;
    if !configuration12.is_finite() {
        return Err("Kabuto size configuration overflow".into());
    }
    next.configuration[12] = configuration12.to_bits();
    for (dst, src) in [
        (0x174, 0x9d4),
        (0x178, 0x9d8),
        (0x17c, 0x9dc),
        (0x1d4, 0x1230),
        (0x1d8, 0x1234),
        (0x1f0, 0x12c8),
        (0x1f4, 0x12cc),
        (0x1f8, 0x12d4),
        (0x1fc, 0x12d0),
        (0x200, 0x12dc),
        (0x204, 0x12d8),
        (0x208, 0x12e0),
        (0x20c, 0x12e4),
        (0x210, 0x12e8),
        (0x214, 0x12ec),
        (0x218, 0x12f0),
        (0x21c, 0x12f4),
    ] {
        store(&mut body, dst, g(src)?)?;
    }
    for (dst, src) in [
        (0x180, 0x1220),
        (0x184, 0x1224),
        (0x188, 0x1228),
        (0x18c, 0x122c),
        (0x190, 0x118c),
        (0x194, 0x1190),
        (0x1b8, 0x3a8),
        (0x1bc, 0x3ac),
        (0x1c0, 0x3b0),
        (0x1c4, 0x3b4),
    ] {
        store(&mut body, dst, g(src)? * gain)?;
    }
    let q = x * f64::from(1f32 / 3.);
    store(&mut body, 0x198, g(0x15e0)? * q + g(0x15d8)? * (1. - q))?;
    for (dst, factor) in [(0x19c, 20.), (0x1c8, 200.), (0x1cc, 2500.)] {
        store(&mut body, dst, gain * factor)?;
    }
    // Unlike other curves, scale is multiplied by its declaration before storing.
    let a = [g(0x10c4)?, g(0x10c8)?, g(0x10cc)?, g(0x10d0)?];
    let value = if x <= 1. {
        (1. - x) * a[0] + x * a[1]
    } else if x <= 2. {
        (x - 1.) * a[2] + (2. - x) * a[1]
    } else {
        (x - 2.) * a[3] + (3. - x) * a[2]
    };
    let scale = (value * f64::from(class_scale)) as f32;
    next.scale = scale;
    let radius = collision_radius::calculate(
        collision_radius::Input {
            primary: bounds.primary,
            auxiliary: bounds.animation,
            scale,
            radius_override: f32::from_bits(next.configuration[27]),
            limit: f32::from_bits(next.configuration[26]),
        },
        Precision::Double53,
    )?;
    let category = if target < 1. {
        0
    } else if target < 2. {
        1
    } else if target < 3. {
        2
    } else {
        3
    };
    body[0x334 / 4] = 0x10 << category;
    body[0x338 / 4] = 1 << category;
    body[0x33c / 4] = 0x100 << category;
    Ok(Prepared {
        fields: next,
        payload: body.into_iter().flat_map(u32::to_le_bytes).collect(),
        radius,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fields() -> Fields {
        Fields {
            class: 1,
            dispatch_context: 7,
            configuration: [0; 30],
            scale: 2.,
            object_flags: 0,
            response_flags: 0x80000000,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: Default::default(),
            channels: [[0; 9]; 4],
            position: [0.; 3],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        }
    }

    fn globals() -> GlobalData {
        GlobalData {
            words: vec![1f32.to_bits(); 0x1600 / 4],
            array_words: vec![],
        }
    }
    fn payload(target: f32, current: f32) -> Vec<u8> {
        let mut p = vec![0; 844];
        p[0x158..0x15c].copy_from_slice(&target.to_le_bytes());
        p[0x15c..0x160].copy_from_slice(&current.to_le_bytes());
        p
    }
    fn source(globals: &GlobalData) -> Source<'_> {
        Source {
            globals,
            class_scale: 2.,
            bounds: Resolved {
                primary: [-1., -1., -1., 1., 1., 1.],
                animation: None,
            },
        }
    }
    #[test]
    fn size_refresh_keeps_target_and_current_distinct_and_preserves_unrelated_state() {
        let g = globals();
        let mut f = fields();
        f.configuration[26] = 0;
        f.configuration[27] = 0;
        let mut p = payload(2., 0.5);
        p[0x20..0x24].copy_from_slice(&0xdeadbeefu32.to_le_bytes());
        let prepared = prepare(&f, &p, 3., Some(source(&g))).unwrap();
        assert_eq!(
            u32::from_le_bytes(prepared.payload[0x338..0x33c].try_into().unwrap()),
            4
        );
        assert_eq!(
            f32::from_le_bytes(prepared.payload[0x1d0..0x1d4].try_into().unwrap()),
            0.375
        );
        assert_eq!(prepared.payload[0x158..0x160], p[0x158..0x160]);
        assert_eq!(prepared.payload[0x20..0x24], p[0x20..0x24]);
        assert_eq!(prepared.fields.channels, f.channels);
        assert_eq!(prepared.fields.position, f.position);
        assert_eq!(prepared.fields.scale, 2.);
        assert!(prepared.radius > 0.);
    }
    #[test]
    fn absent_class_only_updates_multiplier_without_reading_target_or_bounds() {
        let f = fields();
        let p = payload(f32::NAN, 2.);
        let prepared = prepare(&f, &p, 7., None).unwrap();
        let mut expected = p.clone();
        expected[0x1d0..0x1d4].copy_from_slice(&0.75f32.to_le_bytes());
        assert_eq!(prepared.payload, expected);
        assert_eq!(format!("{:?}", prepared.fields), format!("{:?}", f));
        assert_eq!(prepared.radius, 7.);
    }
    #[test]
    fn late_failure_missing_data_and_overflow_leave_caller_inputs_unchanged() {
        let f = fields();
        let p = payload(0., 1.5);
        let before = format!("{f:?}");
        let original = p.clone();
        let mut g = globals();
        g.words[0x14ac / 4] = f32::NAN.to_bits();
        assert!(prepare(&f, &p, 7., Some(source(&g))).is_err());
        g.words.clear();
        assert!(prepare(&f, &p, 7., Some(source(&g))).is_err());
        let mut g = globals();
        g.words[0x10cc / 4] = f32::MAX.to_bits();
        let mut context = source(&g);
        context.class_scale = f32::MAX;
        assert!(prepare(&f, &p, 7., Some(context)).is_err());
        assert_eq!(p, original);
        assert_eq!(format!("{f:?}"), before);
        assert!(prepare(&f, &p[..840], 7., None).is_err());
    }
}
