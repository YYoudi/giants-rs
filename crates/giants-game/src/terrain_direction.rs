//! Direction words from terrain topology and an explicitly supplied context
//! direction/normal-component lower bound. Context selection and GPU use are separate.
use crate::body_pipeline::inverse_prepared_length;
use giants_formats::gti::Terrain;

pub struct Prepared {
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<u32>,
}

/// Use the world-declared floor and the already selected context getter output.
/// The original terrain caller negates this vector. Its cache construction,
/// readiness and live selection are not inferred here.
pub fn prepare_declared(
    terrain: &Terrain,
    config: &giants_formats::world::WorldConfig,
    context_direction: [f32; 3],
) -> Result<Prepared, &'static str> {
    prepare(
        terrain,
        context_direction.map(|v| -v),
        config
            .terrain_direction_floor
            .ok_or("terrain direction floor has no declared or validated default")?,
    )
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    let a = a.map(f64::from);
    let b = b.map(f64::from);
    [
        (a[1] * b[2] - a[2] * b[1]) as f32,
        (a[2] * b[0] - a[0] * b[2]) as f32,
        (a[0] * b[1] - a[1] * b[0]) as f32,
    ]
}

fn normalize(v: [f32; 3], reverse: bool) -> Result<[f32; 3], &'static str> {
    let [x, y, z] = v.map(f64::from);
    let squared = if reverse {
        ((z * z + y * y) + x * x) as f32
    } else {
        ((x * x + y * y) + z * z) as f32
    };
    let inverse = inverse_prepared_length(squared)?;
    let result = v.map(|c| (f64::from(c) * inverse) as f32);
    if !result.iter().all(|c| c.is_finite()) {
        return Err("terrain normal outside finite arithmetic profile");
    }
    Ok(result)
}

/// Each corner's rotated triangle is evaluated separately before its float
/// accumulator store. Sharing one pre-rounded face cross changes this order.
pub fn prepare(
    terrain: &Terrain,
    direction: [f32; 3],
    normal_floor: f32,
) -> Result<Prepared, &'static str> {
    let count = terrain
        .width
        .checked_mul(terrain.height)
        .filter(|n| *n > 0 && *n <= 262144)
        .ok_or("terrain direction grid outside budget")?;
    if terrain.width < 2
        || terrain.height < 2
        || terrain.heights.len() != count
        || terrain.tags.len() != count
        || !direction.iter().all(|v| v.is_finite())
        || !normal_floor.is_finite()
        || !(-1. ..=1.).contains(&normal_floor)
    {
        return Err("invalid terrain direction inputs");
    }
    let positions = (0..count)
        .map(|i| terrain.position(i).map_err(|_| "invalid terrain position"))
        .collect::<Result<Vec<_>, _>>()?;
    if !positions.iter().flatten().all(|v| v.is_finite()) {
        return Err("non-finite terrain direction position");
    }
    let mut normals = vec![[0f32; 3]; count];
    for (index, tag) in terrain.tags.iter().copied().enumerate() {
        let orders: &[[usize; 3]] = match tag {
            0 => continue,
            1 => &[[0, 1, 2]],
            2 => &[[3, 2, 1]],
            3 => &[[1, 3, 0]],
            4 | 7 => &[[2, 0, 3]],
            5 => &[[2, 0, 3], [1, 3, 0]],
            6 => &[[0, 1, 2], [3, 2, 1]],
            _ => return Err("unsupported terrain direction topology"),
        };
        if index % terrain.width == terrain.width - 1 || index / terrain.width == terrain.height - 1
        {
            return Err("terrain direction face crosses grid boundary");
        }
        let corners = [
            index,
            index + 1,
            index + terrain.width,
            index + terrain.width + 1,
        ];
        for order in orders {
            for shift in 0..3 {
                let [a, b, c] = std::array::from_fn(|j| corners[order[(j + shift) % 3]]);
                if [a, b, c].iter().any(|i| terrain.heights[*i] == -4096.) {
                    return Err("terrain direction face references sentinel height");
                }
                let pa = positions[a].map(f64::from);
                let pb = positions[b].map(f64::from);
                let pc = positions[c].map(f64::from);
                let u: [f64; 3] = std::array::from_fn(|i| pb[i] - pa[i]);
                let v: [f64; 3] = std::array::from_fn(|i| pc[i] - pa[i]);
                let face = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                for (accumulator, value) in normals[a].iter_mut().zip(face) {
                    *accumulator = (f64::from(*accumulator) + value) as f32;
                }
            }
        }
    }
    let direction = direction.map(f64::from);
    let mut colors = Vec::with_capacity(count);
    for normal in &mut normals {
        *normal = normalize(*normal, false)?;
        let a = cross(*normal, [1., 0., 0.]);
        let b = normalize(cross(a, *normal), true)?;
        let a = normalize(a, true)?;
        let dot = |v: [f32; 3]| {
            let [x, y, z] = v.map(f64::from);
            (z * direction[2] + y * direction[1]) + x * direction[0]
        };
        let u = dot(b);
        let [x, y, z] = normal.map(f64::from);
        let n = ((direction[0] * x + direction[2] * z) + direction[1] * y) as f32;
        let v = dot(a) as f32;
        let squared = ((f64::from(v) * f64::from(v) + f64::from(n) * f64::from(n)) + u * u) as f32;
        let inverse = f64::from(inverse_prepared_length(squared)? as f32);
        let u = f64::from((u * inverse) as f32);
        let v = f64::from((f64::from(v) * inverse) as f32);
        let n = (f64::from(n) * inverse).max(f64::from(normal_floor));
        let encode = |component: f64| {
            let word = ((component + 1.) * 127.5).trunc();
            if (0. ..=255.).contains(&word) {
                Ok(word as u32)
            } else {
                Err("terrain direction color outside byte profile")
            }
        };
        colors.push((encode(u)? << 16) | (encode(n)? << 8) | encode(v)?);
    }
    Ok(Prepared { normals, colors })
}
