//! Native platform surface selection over caller-prepared world geometry.
//!
//! Footprint membership and world-space triangles must be supplied by the caller.
//! This does not build native object footprints, choose a model, or resolve motion.
use crate::{arithmetic::Precision, terrain_queries::HEIGHT_FLOOR};

pub const MIN_PLATFORM_NORMAL_Z: f32 = 0.64278;

pub struct PreparedPlatform<'a> {
    pub id: u32,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub bounds_xy: [[f32; 2]; 2],
    /// Result of the separate platform footprint query for this point.
    /// Native byte-footprint bit 1 is distinct from solid-occupancy bit 0.
    pub footprint_hit: bool,
    /// Ordered world-space triangles after source model selection/transformation.
    pub triangles: &'a [[[f32; 3]; 3]],
}
#[derive(Clone, Copy, Debug)]
pub struct PlatformSample {
    pub height: f32,
    pub object: Option<u32>,
}

pub fn sample_prepared(
    point: [f32; 3],
    objects: &[PreparedPlatform<'_>],
    precision: Precision,
) -> Result<PlatformSample, &'static str> {
    if !point.iter().all(|v| v.is_finite()) {
        return Err("non-finite platform point");
    }
    let r = |x| precision.round(x);
    let mut result = PlatformSample {
        height: HEIGHT_FLOOR,
        object: None,
    };
    for object in objects {
        if object.id == 0
            || !object.bounds_xy.iter().flatten().all(|v| v.is_finite())
            || object.bounds_xy[0][0] > object.bounds_xy[1][0]
            || object.bounds_xy[0][1] > object.bounds_xy[1][1]
        {
            return Err("invalid prepared platform identity/bounds");
        }
        if object.collision_flags & 8 == 0
            || object.collision_flags & 0x800 != 0
            || object.object_flags & 0x4000_c000 != 0
            || r(f64::from(point[0]) - 0.5) > f64::from(object.bounds_xy[1][0])
            || r(f64::from(point[1]) - 0.5) > f64::from(object.bounds_xy[1][1])
            || f64::from(object.bounds_xy[0][0]) > r(f64::from(point[0]) + 0.5)
            || f64::from(object.bounds_xy[0][1]) > r(f64::from(point[1]) + 0.5)
            || !object.footprint_hit
        {
            continue;
        }
        for triangle in object.triangles {
            if !triangle.iter().flatten().all(|v| v.is_finite()) {
                return Err("non-finite prepared platform triangle");
            }
            let normal = platform_normal(*triangle, precision)?;
            if normal[2] < MIN_PLATFORM_NORMAL_Z {
                continue;
            }
            let [a, _, _] = *triangle;
            let d = (-r(r(r(f64::from(normal[0]) * f64::from(a[0]))
                + r(f64::from(normal[1]) * f64::from(a[1])))
                + r(f64::from(normal[2]) * f64::from(a[2])))) as f32;
            let distance = r(r(r(r(f64::from(normal[1]) * f64::from(point[1]))
                + r(f64::from(normal[0]) * f64::from(point[0])))
                + r(f64::from(normal[2]) * f64::from(point[2])))
                + f64::from(d));
            if !distance.is_finite() {
                return Err("platform distance overflow");
            }
            // Upper limit uses the wide register; lower limit reloads its f32 spill.
            let stored_distance = distance as f32;
            if distance > 10. || stored_distance < -1. {
                continue;
            }
            let height =
                r(f64::from(point[2]) - r(f64::from(stored_distance) / f64::from(normal[2])));
            if !height.is_finite() {
                return Err("platform height overflow");
            }
            // Compare before the height's float32 spill; equality replaces the marker.
            if height < f64::from(result.height) {
                continue;
            }
            if inside_xy(point, *triangle, precision) {
                result.height = height as f32;
                result.object = Some(object.id);
            }
        }
    }
    Ok(result)
}

pub(crate) fn platform_normal(
    t: [[f32; 3]; 3],
    precision: Precision,
) -> Result<[f32; 3], &'static str> {
    let r = |x| precision.round(x);
    let [a, b, c] = t;
    let uy = r(f64::from(b[1]) - f64::from(a[1]));
    let uz = r(f64::from(b[2]) - f64::from(a[2]));
    let vy = f64::from(r(f64::from(c[1]) - f64::from(a[1])) as f32);
    let vz = r(f64::from(c[2]) - f64::from(a[2]));
    let vx = f64::from(r(f64::from(c[0]) - f64::from(a[0])) as f32);
    let ux = f64::from(r(f64::from(b[0]) - f64::from(a[0])) as f32);
    let n = [
        r(r(vz * uy) - r(uz * vy)),
        r(r(vx * uz) - r(ux * vz)),
        r(r(ux * vy) - r(vx * uy)),
    ]
    .map(|v| f64::from(v as f32));
    let squared = r(r(r(n[0] * n[0]) + r(n[2] * n[2])) + r(n[1] * n[1]));
    if !squared.is_finite() {
        return Err("platform normal overflow");
    }
    if squared < f64::from(0.0001f32) {
        return Ok([0.; 3]);
    }
    let inverse = r(1. / r(squared.sqrt()));
    let normal = n.map(|v| r(inverse * v) as f32);
    if !normal.iter().all(|v| v.is_finite()) {
        return Err("platform normal overflow");
    }
    Ok(normal)
}
fn inside_xy(point: [f32; 3], t: [[f32; 3]; 3], precision: Precision) -> bool {
    let r = |x| precision.round(x);
    let q = t.map(|v| {
        [
            r(f64::from(v[0]) - f64::from(point[0])) as f32,
            r(f64::from(v[1]) - f64::from(point[1])) as f32,
        ]
    });
    let mut inside = false;
    for i in 0..3 {
        let [x, y] = q[i].map(f64::from);
        let [nx, ny] = q[(i + 1) % 3].map(f64::from);
        if (y >= 0.) != (ny >= 0.)
            && ((x >= 0. && nx >= 0.)
                || ((x >= 0. || nx >= 0.) && r(r(r(nx - x) * y) / r(ny - y)) < x))
        {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn highest_valid_surface_and_equal_height_keep_last_marker() {
        let face = [[[0., 0., 0.], [10., 0., 0.], [0., 10., 0.]]];
        let make = |id| PreparedPlatform {
            id,
            object_flags: 0,
            collision_flags: 8,
            bounds_xy: [[0., 0.], [10., 10.]],
            footprint_hit: true,
            triangles: &face,
        };
        let sample =
            sample_prepared([1., 1., 1.], &[make(1), make(2)], Precision::Double53).unwrap();
        assert_eq!(sample.object, Some(2));
        assert_eq!(sample.height, 0.);
        let mut disabled = make(3);
        disabled.object_flags = 0x4000;
        assert!(
            sample_prepared([1., 1., 1.], &[disabled], Precision::Double53)
                .unwrap()
                .object
                .is_none()
        );
    }
    #[test]
    fn prepared_platform_feeds_native_final_contact_stage() {
        use crate::ground_contact::{self, CONTACT_BIT, Input};
        let faces = [[[0., 0., 3.], [10., 0., 3.], [0., 10., 3.]]];
        let platform = PreparedPlatform {
            id: 1,
            object_flags: 0,
            collision_flags: 8,
            bounds_xy: [[0., 0.], [10., 10.]],
            footprint_hit: true,
            triangles: &faces,
        };
        let sample = sample_prepared([1., 1., 3.05], &[platform], Precision::Double53).unwrap();
        let contact = ground_contact::resolve(
            Input {
                z: 3.05,
                vertical_velocity: -1.,
                flags: 0,
                lock_z: false,
                ground: 0.,
                plane: [0., 0., 1., 0.],
                vertical_offset: 0.,
                platform: sample.object.map(|_| sample.height),
            },
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(contact.z, 3.);
        assert_ne!(contact.flags & CONTACT_BIT, 0);
        assert_eq!(contact.plane, [0., 0., 1., -3.]);
    }
    #[test]
    fn degenerate_and_reverse_faces_are_not_platforms() {
        for face in [[[0.; 3]; 3], [[0., 0., 0.], [0., 10., 0.], [10., 0., 0.]]] {
            let faces = [face];
            let object = PreparedPlatform {
                id: 1,
                object_flags: 0,
                collision_flags: 8,
                bounds_xy: [[-10., -10.], [10., 10.]],
                footprint_hit: true,
                triangles: &faces,
            };
            assert!(
                sample_prepared([1., 1., 1.], &[object], Precision::Double53)
                    .unwrap()
                    .object
                    .is_none()
            );
        }
    }
}
