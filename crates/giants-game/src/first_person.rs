//! Debug subjective-player placement with supplied previous/current cameras.
//! Camera update, debug editing, actor construction and drawing remain external.

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: [f32; 3],
    /// Source yaw, pitch and roll, in degrees.
    pub angles: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub position: [f32; 3],
    pub angles: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct AnimationInput {
    pub primary: u32,
    pub channel_flags: u32,
    pub response: u32,
    pub secondary_flags: u32,
    pub motion: [f32; 3],
    /// Result of the icon-index service, not an assumed weapon number.
    pub icon_index: usize,
    pub weapon: i32,
    pub previous_weapon: i32,
    pub inventory_present: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Select,
    User,
    Restart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationCommand {
    pub method: Method,
    pub slot: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationUpdate {
    pub command: Option<AnimationCommand>,
    pub previous_weapon: i32,
    pub invalidate_primary: bool,
}

/// Normal Mecc branch; this selects a request, not a pose or service success.
/// The debug binary's tables are independent of the available clip descriptors:
/// a requested but absent slot must still be refused by the resource consumer.
pub fn animation(input: AnimationInput) -> Result<AnimationUpdate, &'static str> {
    let AnimationInput {
        primary,
        channel_flags,
        response,
        secondary_flags,
        motion,
        icon_index,
        weapon,
        previous_weapon,
        inventory_present,
    } = input;
    if icon_index >= 4 || !motion.into_iter().all(f32::is_finite) {
        return Err("subjective animation outside normal Mecc profile");
    }
    let stand = [20, 23, 22, 21];
    let change = [24, 27, 26, 25];
    let run = [32, 35, 34, 33];
    let fire = [28, 31, 30, 29];
    let mut update = AnimationUpdate {
        command: None,
        previous_weapon,
        invalidate_primary: false,
    };
    let command = if response & 0x40000 != 0 {
        Some((Method::Select, 36))
    } else if primary == 36 {
        Some((
            Method::User,
            if channel_flags & 2 != 0 {
                stand[icon_index]
            } else {
                primary
            },
        ))
    } else if response & 0x4000 != 0 {
        Some((Method::Select, fire[icon_index]))
    } else if motion.into_iter().any(|v| v != 0.) {
        Some((Method::User, run[icon_index]))
    } else if change.contains(&primary) || secondary_flags & 0x30 != 0 && inventory_present {
        if change.contains(&primary) {
            if channel_flags & 2 == 0 {
                Some((Method::User, primary))
            } else if channel_flags & 0x10 != 0 {
                Some((Method::User, stand[icon_index]))
            } else if weapon == previous_weapon {
                None
            } else {
                update.invalidate_primary = true;
                Some((Method::Restart, change[icon_index]))
            }
        } else {
            update.previous_weapon = weapon;
            Some((Method::Select, change[icon_index]))
        }
    } else if run.contains(&primary) && channel_flags & 2 == 0 {
        Some((Method::User, primary))
    } else {
        Some((Method::User, stand[icon_index]))
    };
    update.command = command.map(|(method, slot)| AnimationCommand { method, slot });
    Ok(update)
}

fn normalized(angle: f32) -> f32 {
    if angle < 0. {
        (f64::from(angle) % 360. + 360.) as f32
    } else if angle >= 360. {
        (f64::from(angle) % 360.) as f32
    } else {
        angle
    }
}

/// The native placement smooths camera changes, then applies the authored
/// forward/side/up offset. This is not an arbitrary camera-space translation.
/// Arithmetic profile is binary64 x87 intermediates and explicit binary32 stores.
pub fn place(previous: Camera, current: Camera, offset: [f32; 3]) -> Result<Pose, &'static str> {
    if !previous
        .position
        .into_iter()
        .chain(current.position)
        .chain(previous.angles)
        .chain(current.angles)
        .chain(offset)
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
    {
        return Err("subjective placement outside bounded finite profile");
    }
    let mut angles = [0.; 3];
    for (index, angle) in angles.iter_mut().enumerate() {
        let mut delta = f64::from(normalized(previous.angles[index]))
            - f64::from(normalized(current.angles[index]));
        if delta > 180. {
            delta -= 360.;
        }
        if delta < -180. {
            delta += 360.;
        }
        let nearest = (f64::from(current.angles[index]) + delta) as f32;
        *angle = ((f64::from(current.angles[index]) * 5. + f64::from(nearest))
            * f64::from(0.16666667_f32)) as f32;
    }
    let delta = std::array::from_fn::<_, 3, _>(|i| {
        (f64::from(previous.position[i]) - f64::from(current.position[i])) as f32
    });
    let [x, y, z] = delta.map(f64::from);
    let length_squared = (x * x + y * y) + z * z;
    let origin = if length_squared > f64::from(0.10890001_f32) {
        let reciprocal = 1. / length_squared.sqrt();
        std::array::from_fn(|i| {
            let unit = (f64::from(delta[i]) * reciprocal) as f32;
            (f64::from(unit) * f64::from(0.33_f32) + f64::from(current.position[i])) as f32
        })
    } else {
        previous.position
    };
    let (sy, cy) = crate::object_transform::sincos(angles[0]);
    let (sp, cp) = crate::object_transform::sincos(angles[1]);
    let forward = [(cp * sy) as f32, (-cp * cy) as f32, sp as f32];
    let up = [(-sp * sy) as f32, (sp * cy) as f32, cp as f32];
    // Keep the original stored-vector cross product: replacing it with exact
    // sine/cosine identities would discard the binary32 rounding residuals.
    let side = [
        (cp * f64::from(forward[1]) - f64::from(up[1]) * sp) as f32,
        (f64::from(up[0]) * sp - cp * f64::from(forward[0])) as f32,
        (f64::from(up[1]) * f64::from(forward[0]) - f64::from(up[0]) * f64::from(forward[1]))
            as f32,
    ];
    let position = std::array::from_fn(|i| {
        (((f64::from(forward[i]) * f64::from(offset[0])
            + f64::from(side[i]) * f64::from(offset[1]))
            + f64::from(up[i]) * f64::from(offset[2]))
            + f64::from(origin[i])) as f32
    });
    Ok(Pose { position, angles })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_camera_values_are_refused() {
        let good = Camera {
            position: [0.; 3],
            angles: [0.; 3],
        };
        assert!(
            place(
                good,
                Camera {
                    angles: [f32::NAN, 0., 0.],
                    ..good
                },
                [0.; 3]
            )
            .is_err()
        );
        assert!(place(good, good, [f32::INFINITY; 3]).is_err());
        assert!(
            place(
                Camera {
                    position: [1e8, 0., 0.],
                    ..good
                },
                good,
                [0.; 3]
            )
            .is_err()
        );
    }
    #[test]
    fn unsupported_weapon_index_is_not_silently_mapped_to_the_pistol() {
        assert!(
            animation(AnimationInput {
                primary: 20,
                channel_flags: 0,
                response: 0,
                secondary_flags: 0,
                motion: [0.; 3],
                icon_index: 4,
                weapon: 9,
                previous_weapon: 9,
                inventory_present: false
            })
            .is_err()
        );
    }
}
