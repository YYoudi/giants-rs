//! Shared deferred-entry keys, cache preparation and reference queue ordering.
use crate::arithmetic::Precision;

/// Only the demonstrated direct name-priority declaration is projected here.
/// Runtime extent/cache flags are separate current fields.
pub fn declared_name_flags(opcodes: &[u8]) -> u32 {
    if opcodes.contains(&0x55) { 0x10 } else { 0 }
}
pub fn centroid(
    mesh: &giants_formats::gbs::Mesh,
    part: usize,
    precision: Precision,
) -> Result<[f32; 3], String> {
    let part = mesh.subobjects.get(part).ok_or("sort part outside model")?;
    let end = part
        .vertex_start
        .checked_add(part.vertex_count)
        .ok_or("sort range overflow")?;
    let indices = mesh
        .indices
        .get(part.vertex_start..end)
        .ok_or("sort range outside vertices")?;
    if indices.is_empty() {
        return Err("empty sort centroid".into());
    }
    let mut sum = [0f32; 3];
    for index in indices {
        let point = mesh
            .positions
            .get(usize::from(*index))
            .ok_or("sort position outside model")?;
        for axis in 0..3 {
            sum[axis] += point[axis];
        }
    }
    let result = sum.map(|v| precision.round(f64::from(v) / indices.len() as f64) as f32);
    if !result.into_iter().all(f32::is_finite) {
        return Err("sort centroid overflow".into());
    }
    Ok(result)
}
/// Preserve the observed metric: X and Y both update slot zero; slot one stays
/// zero. Extrema start at zero, including for entirely negative geometry.
pub fn extent_metric(points: impl IntoIterator<Item = [f32; 3]>) -> [f32; 3] {
    let mut metric = [0.; 3];
    for point in points {
        if point[0] > metric[0] {
            metric[0] = point[0];
        }
        if point[1] > metric[0] {
            metric[0] = point[1];
        }
        if point[2] > metric[2] {
            metric[2] = point[2];
        }
    }
    metric
}
pub fn needs_extent_cache(flags: u32) -> bool {
    flags & 0x10 == 0 && flags & 4 != 0 && flags & 8 == 0
}
/// Shared model point updates and the spec's cache bit are explicit mutations.
pub fn prepare_extent_cache(
    flags: &mut u32,
    points: &mut [[f32; 3]],
    metrics: &[[f32; 3]],
) -> Result<(), String> {
    if !needs_extent_cache(*flags) {
        return Ok(());
    }
    if points.len() != metrics.len() {
        return Err("sort metric count mismatch".into());
    }
    points.copy_from_slice(metrics);
    *flags |= 8;
    Ok(())
}
#[derive(Clone, Copy, Debug)]
pub enum Transform {
    Cached([f32; 16]),
    /// Current rotation/scale basis supplied by the caller. Euler construction
    /// itself is not performed or inferred by the key kernel.
    ComputedBasis([f32; 9]),
}
pub fn world_point(
    point: [f32; 3],
    position: [f32; 3],
    transform: Transform,
    precision: Precision,
) -> Result<[f32; 3], String> {
    let (matrix, order) = match transform {
        Transform::Cached(matrix) => (matrix, [2, 0, 1]),
        Transform::ComputedBasis(basis) => {
            let matrix = std::array::from_fn(|i| {
                if i < 12 {
                    if i % 4 == 3 {
                        position[i / 4]
                    } else {
                        basis[i / 4 * 3 + i % 4]
                    }
                } else {
                    f32::from(i == 15)
                }
            });
            (matrix, [2, 1, 0])
        }
    };
    if !point
        .into_iter()
        .chain(matrix[..12].iter().copied())
        .all(f32::is_finite)
    {
        return Err("non-finite sort transform".into());
    }
    let r = |v| precision.round(v);
    let result = std::array::from_fn(|row| {
        let product = |axis| r(f64::from(point[axis]) * f64::from(matrix[row * 4 + axis]));
        r(
            r(r(product(order[0]) + product(order[1])) + product(order[2]))
                + f64::from(matrix[row * 4 + 3]),
        ) as f32
    });
    if !result.into_iter().all(f32::is_finite) {
        return Err("sort world-point overflow".into());
    }
    Ok(result)
}
pub fn depth(point: [f32; 3], clip_w: [f32; 4], precision: Precision) -> Result<f32, String> {
    if !point.into_iter().chain(clip_w).all(f32::is_finite) {
        return Err("non-finite sort depth input".into());
    }
    let r = |v| precision.round(v);
    let product = |axis| r(f64::from(point[axis]) * f64::from(clip_w[axis]));
    let value = r(r(r(product(0) + product(2)) + product(1)) + f64::from(clip_w[3])) as f32;
    if !value.is_finite() {
        return Err("sort depth overflow".into());
    }
    Ok(value)
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub spec_flags: u32,
    pub first_name_byte: u8,
    pub part_point: [f32; 3],
    pub position: [f32; 3],
    pub scale: f32,
    pub transform: Transform,
    pub clip_w: [f32; 4],
}
pub fn key(input: Input, precision: Precision) -> Result<f32, String> {
    let value = if input.spec_flags & 0x10 != 0 {
        let priority = i32::from(input.first_name_byte as i8) - 65;
        precision
            .round(f64::from(depth(input.position, input.clip_w, precision)?) - f64::from(priority))
            as f32
    } else if input.spec_flags & 4 != 0 {
        if input.spec_flags & 8 == 0 {
            return Err("sort extent cache not prepared".into());
        }
        if !input.part_point[0].is_finite() || !input.scale.is_finite() {
            return Err("non-finite sort metric".into());
        }
        let offset = precision.round(f64::from(input.part_point[0]) * f64::from(input.scale));
        precision.round(f64::from(depth(input.position, input.clip_w, precision)?) - offset) as f32
    } else {
        depth(
            world_point(input.part_point, input.position, input.transform, precision)?,
            input.clip_w,
            precision,
        )?
    };
    if !value.is_finite() {
        return Err("sort key overflow".into());
    }
    Ok(value)
}
/// Raw comparator semantics, including unordered inputs returning +1. This is
/// deliberately not an Ordering suitable for Rust's standard sort routines.
pub fn compare(a: f32, b: f32) -> i32 {
    if a > b {
        -1
    } else if a == b {
        0
    } else {
        1
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub object: u32,
    pub part: i32,
    pub depth: f32,
}
/// Match the bounded reference queue's pivot, partition and small-range swaps.
/// Equal depths are not made stable, and NaN payload bits remain untouched.
pub fn sort(entries: &mut [Entry]) -> Result<(), String> {
    if entries.len() > crate::render_parts::QUEUE_CAPACITY {
        return Err("sort queue capacity exceeded".into());
    }
    if entries.len() < 2 {
        return Ok(());
    }
    let mut ranges = vec![(0, entries.len() - 1)];
    while let Some((lo, hi)) = ranges.pop() {
        if hi - lo < 8 {
            for end in (lo + 1..=hi).rev() {
                let mut last = lo;
                for i in lo + 1..=end {
                    if compare(entries[i].depth, entries[last].depth) > 0 {
                        last = i;
                    }
                }
                entries.swap(last, end);
            }
            continue;
        }
        entries.swap(lo, lo + (hi - lo).div_ceil(2));
        let (mut left, mut right) = (lo, hi + 1);
        loop {
            left += 1;
            while left <= hi && compare(entries[left].depth, entries[lo].depth) <= 0 {
                left += 1;
            }
            right -= 1;
            while right > lo && compare(entries[right].depth, entries[lo].depth) >= 0 {
                right -= 1;
            }
            if left > right {
                break;
            }
            entries.swap(left, right);
        }
        entries.swap(lo, right);
        if lo + 1 < right {
            ranges.push((lo, right - 1));
        }
        if left < hi {
            ranges.push((left, hi));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_key_order_follows_both_reference_size_paths() {
        for (n, expected) in [
            (8, vec![1, 2, 3, 4, 5, 6, 7, 0]),
            (9, vec![4, 1, 2, 3, 0, 5, 6, 7, 8]),
        ] {
            let mut e: Vec<_> = (0..n)
                .map(|i| Entry {
                    object: i,
                    part: 0,
                    depth: 1.,
                })
                .collect();
            sort(&mut e).unwrap();
            assert_eq!(e.iter().map(|e| e.object).collect::<Vec<_>>(), expected);
        }
        assert_eq!(compare(f32::NAN, 1.), 1);
        assert_eq!(compare(1., f32::NAN), 1);
    }
    #[test]
    fn extent_cache_is_shared_mutation_and_name_mode_takes_priority() {
        assert_eq!(extent_metric([[-3., 7., -1.], [4., -2., 5.]]), [7., 0., 5.]);
        let mut flags = 4;
        let mut points = [[9.; 3]; 2];
        prepare_extent_cache(&mut flags, &mut points, &[[1.; 3], [2.; 3]]).unwrap();
        assert_eq!(flags, 12);
        assert_eq!(points, [[1.; 3], [2.; 3]]);
        prepare_extent_cache(&mut flags, &mut points, &[]).unwrap();
        let mut flags = 0x14;
        prepare_extent_cache(&mut flags, &mut points, &[]).unwrap();
        assert_eq!(flags, 0x14);
    }
}
