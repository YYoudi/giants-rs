//! Measured footprint selection and circular row construction.
//! Object fields, building-query result and arithmetic policy are explicit inputs.
//! This does not infer collider eligibility/fields from a render mesh.
use crate::arithmetic::Precision;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Plan {
    Existing = 0,
    Absent = 1,
    Bounds1 = 2,
    Pair2 = 3,
    Raster3 = 4,
    Packed4 = 5,
    Circle5 = 6,
}
pub struct Input {
    pub existing: bool,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub object_kind: u32,
    /// EAX of the separate building query, not a reconstructed energy policy.
    pub building_query: u32,
    /// Ground-plane normal Z from the actual header, not a supplied global gate.
    pub zero_gate: f32,
    pub z: f32,
    pub floor: f32,
    pub ground_offset: f32,
}
pub fn select(input: Input, precision: Precision) -> Result<Plan, &'static str> {
    if input.existing {
        return Ok(Plan::Existing);
    }
    if input.object_flags & 0x4000_c000 != 0
        || input.collision_flags & (0x800 | 0x40_0000) != 0
        || (matches!(input.object_kind, 74 | 83) && input.building_query != 0)
    {
        return Ok(Plan::Absent);
    }
    let height = if input.object_flags & 0x8_0000 != 0 {
        if !input.ground_offset.is_finite() {
            return Err("non-finite ground offset");
        }
        f64::from(input.ground_offset)
    } else {
        if !input.zero_gate.is_finite() || !input.z.is_finite() || !input.floor.is_finite() {
            return Err("non-finite footprint gate");
        }
        if input.zero_gate == 0. {
            return Ok(Plan::Absent);
        }
        precision.round(f64::from(input.z) - f64::from(input.floor))
    };
    if height > 4. {
        return Ok(Plan::Absent);
    }
    Ok(if input.collision_flags & 0x8_0000 != 0 {
        Plan::Bounds1
    } else if input.collision_flags & 0x10_0000 != 0 {
        Plan::Pair2
    } else if input.collision_flags & 8 != 0 {
        Plan::Raster3
    } else if input.collision_flags & 0x1000 != 0 {
        Plan::Packed4
    } else {
        Plan::Circle5
    })
}
/// Serialized profile 5. Membership semantics are a separate unimplemented reader.
/// Width 256 deliberately encodes as byte zero, matching the measured writer.
pub fn circle(
    center: [f32; 2],
    radius: f32,
    precision: Precision,
) -> Result<Vec<u8>, &'static str> {
    if !center.into_iter().chain([radius]).all(f32::is_finite) || radius < 0. {
        return Err("invalid circular footprint input");
    }
    let r = |x| precision.round(x);
    let cell = |value: f64| -> Result<i32, &'static str> {
        let v = r(r(value * f64::from(0.2_f32)) + 0.5).floor();
        if !v.is_finite() || v < i16::MIN as f64 || v > i16::MAX as f64 {
            return Err("circular footprint cell outside bounded header");
        }
        Ok(v as i32)
    };
    let [x, y] = center.map(f64::from);
    let radius = f64::from(radius);
    let bounds = [
        cell(r(x - radius))?,
        cell(r(y - radius))?,
        cell(r(x + radius))?,
        cell(r(y + radius))?,
    ];
    let rows = bounds[3] - bounds[1] + 1;
    if rows <= 0 || rows * 2 + 10 > 600 {
        return Err("circular footprint row budget");
    }
    let mut bytes = vec![5];
    for v in bounds {
        bytes.extend((v as i16).to_le_bytes());
    }
    let mut previous = bounds[0];
    for row in bounds[1]..=bounds[3] {
        let distance = r(y - r(f64::from(row) * 5.)).abs();
        let half = if distance < radius {
            r(r(r(radius * radius) - r(distance * distance)).sqrt()) as f32
        } else {
            0.
        };
        let first = cell(r(x - f64::from(half)))?;
        let last = cell(r(x + f64::from(half)))?;
        let delta = first - previous;
        let width_minus_one = last - first;
        if !(-127..=127).contains(&delta) || !(0..=255).contains(&width_minus_one) {
            return Err("circular footprint span outside measured writer budget");
        }
        bytes.push(delta as i8 as u8);
        bytes.push((width_minus_one as u8).wrapping_add(1));
        previous = first;
    }
    bytes.push(0x80);
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires private exact-PC selector controls"]
    fn native_debug_selector_preserves_platform_and_building_gates() {
        let text = std::fs::read_to_string(
            std::env::var_os("GIANTS_PLATFORM_SELECTOR_CONTROLS")
                .expect("private selector controls"),
        )
        .unwrap();
        let mut count = 0;
        for line in text.lines() {
            let p = line.split_whitespace().collect::<Vec<_>>();
            let w = |i| u32::from_str_radix(p[i], 16).unwrap();
            let input = Input {
                existing: w(1) != 0,
                object_flags: w(2),
                collision_flags: w(3),
                object_kind: w(4),
                building_query: w(5),
                zero_gate: f32::from_bits(w(6)),
                z: f32::from_bits(w(7)),
                floor: f32::from_bits(w(8)),
                ground_offset: f32::from_bits(w(9)),
            };
            let precision = match p[0] {
                "24" => Precision::Single24,
                "53" => Precision::Double53,
                _ => panic!(),
            };
            assert_eq!(
                select(input, precision).unwrap() as u32,
                p[10].parse::<u32>().unwrap(),
                "native selector case {count}"
            );
            count += 1;
        }
        assert_eq!(count, 1024);
    }
    #[test]
    fn circle_retains_row_deltas_and_refuses_overflow() {
        let value = circle([0., 0.], 5., Precision::Double53).unwrap();
        assert_eq!(
            value,
            [5, 255, 255, 255, 255, 1, 0, 1, 0, 1, 1, 255, 3, 1, 1, 128]
        );
        assert!(circle([f32::INFINITY, 0.], 1., Precision::Double53).is_err());
        assert!(circle([0., 0.], -1., Precision::Double53).is_err());
        assert!(circle([0., 0.], 1000., Precision::Double53).is_err());
    }
}
