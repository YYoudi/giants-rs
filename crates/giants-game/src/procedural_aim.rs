//! Procedural target orientation with explicit current state and matrices.
//! Node attachment, target simulation and general reference-axis rebasing remain separate.
use crate::animation::{Pose, cross, matrix_quat, normalize, pose_matrix, quat_mul};
#[derive(Clone, Copy, Debug)]
pub struct Matrix {
    pub flags: u32,
    pub values: [f32; 16],
}
#[derive(Clone, Copy, Debug)]
pub struct Axes {
    pub forward: [f32; 3],
    pub up: Option<[f32; 3]>,
}
impl Matrix {
    pub fn identity() -> Self {
        Self {
            flags: 0,
            values: [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
        }
    }
    pub(crate) fn mul(self, b: Self) -> Result<Self, &'static str> {
        if (self.flags | b.flags) & 0x20 != 0 {
            return Err("projective aiming matrix unsupported");
        }
        let mut out = Self::identity();
        for r in 0..3 {
            for c in 0..3 {
                out.values[r * 4 + c] = if self.flags & 15 == 0 {
                    b.values[r * 4 + c]
                } else if b.flags & 15 == 0 {
                    self.values[r * 4 + c]
                } else {
                    let a = &self.values[r * 4..];
                    ((d(a[0]) * d(b.values[c]) + d(a[2]) * d(b.values[8 + c]))
                        + d(a[1]) * d(b.values[4 + c])) as f32
                };
            }
            let a = &self.values[r * 4..];
            out.values[r * 4 + 3] = if b.flags & 16 == 0 {
                a[3]
            } else {
                (((d(a[1]) * d(b.values[7]) + d(a[0]) * d(b.values[3]))
                    + d(a[2]) * d(b.values[11]))
                    + d(a[3])) as f32
            };
        }
        out.flags = (self.flags | b.flags) & !8 | ((self.flags ^ b.flags) & 8);
        if self.flags & 2 != 0 && b.flags & 1 != 0 {
            out.flags |= 4;
        }
        Ok(out)
    }
    pub(crate) fn inverse(self) -> Result<Self, &'static str> {
        if self.flags & 0x20 != 0 {
            return Err("projective aiming inverse unsupported");
        }
        let mut out = Self::identity();
        out.flags = self.flags;
        let a = self.values.map(d);
        if self.flags & 15 != 0 {
            if self.flags & 6 == 0 {
                for r in 0..3 {
                    for c in 0..3 {
                        out.values[r * 4 + c] = self.values[c * 4 + r];
                    }
                }
            } else {
                let co = a[10] * a[5] - a[6] * a[9];
                let det = (co * a[0] - (a[10] * a[4] - a[8] * a[6]) * a[1])
                    + (a[4] * a[9] - a[8] * a[5]) * a[2];
                if !det.is_finite() || det == 0. {
                    return Err("singular aiming matrix");
                }
                let inv = 1. / det;
                for (i, v) in [
                    (0, co),
                    (1, a[9] * a[2] - a[10] * a[1]),
                    (2, a[6] * a[1] - a[2] * a[5]),
                    (4, a[8] * a[6] - a[10] * a[4]),
                    (5, a[10] * a[0] - a[8] * a[2]),
                    (6, a[4] * a[2] - a[6] * a[0]),
                    (8, a[4] * a[9] - a[8] * a[5]),
                    (9, a[8] * a[1] - a[0] * a[9]),
                    (10, a[0] * a[5] - a[4] * a[1]),
                ] {
                    out.values[i] = (v * inv) as f32;
                }
            }
        }
        if self.flags & 16 != 0 {
            for r in 0..3 {
                let m = &out.values[r * 4..];
                out.values[r * 4 + 3] =
                    -(((d(m[2]) * a[11] + d(m[0]) * a[3]) + d(m[1]) * a[7]) as f32);
            }
        }
        Ok(out)
    }
    pub(crate) fn transform(self, v: [f32; 4]) -> [f32; 3] {
        std::array::from_fn(|r| {
            let m = &self.values[r * 4..];
            if self.flags & 11 == 0 {
                if self.flags & 16 != 0 {
                    (d(v[r]) + d(v[3]) * d(m[3])) as f32
                } else {
                    v[r]
                }
            } else {
                let [x, y, z, w] = v.map(d);
                let [a, b, c, t] = [m[0], m[1], m[2], m[3]].map(d);
                if self.flags & 16 != 0 {
                    match r {
                        0 => (((w * t + z * c) + y * b) + x * a) as f32,
                        1 => (((x * a + z * c) + y * b) + w * t) as f32,
                        _ => (((y * b + x * a) + w * t) + z * c) as f32,
                    }
                } else {
                    match r {
                        0 => ((z * c + y * b) + x * a) as f32,
                        1 => ((x * a + z * c) + y * b) as f32,
                        _ => ((y * b + x * a) + z * c) as f32,
                    }
                }
            }
        })
    }
    /// Acquired Double53 point-array kernel. Its arithmetic order differs from
    /// the homogeneous aiming kernel; retain both source profiles explicitly.
    pub(crate) fn transform_point(self, point: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|r| {
            let m = &self.values[r * 4..];
            if self.flags & 11 == 0 {
                if self.flags & 16 != 0 {
                    (d(point[r]) + d(m[3])) as f32
                } else {
                    point[r]
                }
            } else {
                let [x, y, z] = point.map(d);
                let [a, b, c] = [m[0], m[1], m[2]].map(d);
                let sum = if r == 2 {
                    (z * c + y * b) + x * a
                } else {
                    (y * b + x * a) + z * c
                };
                if self.flags & 16 != 0 {
                    (sum + d(m[3])) as f32
                } else {
                    sum as f32
                }
            }
        })
    }
}
fn d(v: f32) -> f64 {
    f64::from(v)
}
pub fn local_matrix(pose: &Pose, parent: Option<Matrix>) -> Result<Matrix, &'static str> {
    let local = Matrix {
        flags: pose.flags,
        values: pose_matrix(pose),
    };
    if let Some(parent) = parent {
        parent.mul(local)
    } else {
        Ok(local)
    }
}
pub(crate) fn basis(forward: [f32; 3], up: [f32; 3]) -> Result<Matrix, &'static str> {
    let right = normalize(cross(up, forward)).map_err(|_| "degenerate aiming axes")?;
    let forward = normalize(forward).map_err(|_| "degenerate aiming forward")?;
    let up = cross(forward, right);
    let mut m = Matrix::identity();
    m.flags = 17;
    for r in 0..3 {
        m.values[r * 4] = right[r];
        m.values[r * 4 + 1] = up[r];
        m.values[r * 4 + 2] = forward[r];
    }
    Ok(m)
}
/// `None` represents absent owner/state; state bit1 suppresses the callback.
/// Axes are the current configured vectors; relative-node axis rebasing must
/// be resolved before calling this kernel.
pub fn apply(
    pose: &Pose,
    world: Matrix,
    parent: Option<Matrix>,
    target: Option<([f32; 4], u32)>,
    forward: [f32; 3],
    up: Option<[f32; 3]>,
) -> Result<Pose, &'static str> {
    let Some((target, flags)) = target else {
        return Ok(pose.clone());
    };
    if flags & 1 != 0 {
        return Ok(pose.clone());
    }
    if !pose
        .position
        .into_iter()
        .chain(pose.rotation)
        .chain(pose.scale)
        .chain(world.values)
        .chain(target)
        .chain(forward)
        .chain(up.into_iter().flatten())
        .all(f32::is_finite)
    {
        return Err("non-finite aiming input");
    }
    let node = local_matrix(pose, parent)?;
    let inverse = world.mul(node)?.inverse()?;
    let direction = normalize(inverse.transform(target)).map_err(|_| "degenerate aiming target")?;
    let rotation = if let Some(up) = up {
        let world_up =
            normalize(inverse.transform([0., 0., 1., 0.])).map_err(|_| "degenerate aiming up")?;
        let correction = basis(direction, world_up)?.mul(basis(forward, up)?.inverse()?)?;
        matrix_quat(std::array::from_fn(|r| {
            std::array::from_fn(|c| correction.values[r * 4 + c])
        }))
        .map_err(|_| "invalid aiming quaternion")?
    } else {
        let cross = cross(forward, direction);
        let dot = (d(forward[0]) * d(direction[0]) + d(forward[1]) * d(direction[1]))
            + d(forward[2]) * d(direction[2]);
        let w = d(((dot + 1.) * 0.5) as f32).sqrt();
        if !w.is_finite() || w == 0. {
            return Err("antiparallel aiming vectors unsupported");
        }
        let inv = 1. / (w + w);
        [
            (d(cross[0]) * inv) as f32,
            (d(cross[1]) * inv) as f32,
            (d(cross[2]) * inv) as f32,
            w as f32,
        ]
    };
    let mut out = pose.clone();
    out.rotation = quat_mul(pose.rotation, rotation);
    out.flags |= 1;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pose() -> Pose {
        Pose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
            flags: 0,
        }
    }
    #[test]
    fn disabled_callback_copies_without_evaluating_invalid_inputs() {
        let mut p = pose();
        p.rotation[0] = f32::from_bits(0x7fc01234);
        for target in [None, Some(([f32::NAN; 4], 1))] {
            let out = apply(
                &p,
                Matrix {
                    flags: 0x20,
                    values: [f32::NAN; 16],
                },
                None,
                target,
                [f32::NAN; 3],
                None,
            )
            .unwrap();
            assert_eq!(out.rotation.map(f32::to_bits), p.rotation.map(f32::to_bits));
            assert_eq!(out.flags, p.flags);
        }
    }
    #[test]
    fn unsupported_active_profiles_fail_explicitly() {
        let target = Some(([0., 1., 0., 1.], 0));
        assert!(
            apply(
                &pose(),
                Matrix {
                    flags: 0x20,
                    ..Matrix::identity()
                },
                None,
                target,
                [0., -1., 0.],
                None
            )
            .is_err()
        );
        assert!(
            apply(
                &pose(),
                Matrix {
                    flags: 2,
                    values: [0.; 16]
                },
                None,
                target,
                [0., -1., 0.],
                None
            )
            .is_err()
        );
        assert!(
            apply(
                &pose(),
                Matrix::identity(),
                None,
                target,
                [0., -1., 0.],
                None
            )
            .is_err()
        );
    }
}
