//! Acquired reset body with required owned services. No Session admission,
//! constructor, animation backend or reference lifetime is supplied here.
use crate::{actor_fields::Fields, animation_channels::Request, procedural_aim::Matrix};

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub position: [f32; 3],
    pub heading: f32,
}
/// All getters and direct setters refer to the same current owned actors.
/// Construction must return a distinct owned actor, with its actual resources.
/// Used payload, descriptor and reference storage must stay alive throughout
/// this operation. Setters publish only the named fields, without callbacks.
pub trait Services<K> {
    fn fields(&self, actor: K) -> Result<&Fields, String>;
    fn fields_mut(&mut self, actor: K) -> Result<&mut Fields, String>;
    fn mode_is_four(&mut self) -> Result<bool, String>;
    fn birth_pose(&self, source: K) -> Result<Pose, String>;
    fn point_class(&self, point: K) -> Result<u32, String>;
    fn point_offset(&self, point: K) -> Result<Pose, String>;
    fn construct(&mut self, class: u32, parameter: u32) -> Result<K, String>;
    /// Actual position writer, including changed-position footprint invalidation
    /// before stores, and current/previous position history afterward.
    fn position(&mut self, actor: K, position: [f32; 3]) -> Result<(), String>;
    fn prepare_transform(&mut self, actor: K) -> Result<(), String>;
    fn select_animation(&mut self, actor: K, request: Request) -> Result<(), String>;
    fn step_animation(&mut self, actor: K, seconds: f32) -> Result<(), String>;
    /// Current cached affine and optional current root reference, resolved after
    /// animation. An absent reference is a proven descriptor result.
    fn matrices(&self, actor: K) -> Result<(Matrix, Option<Matrix>), String>;
    /// Publish the acquired global-cache stores through its real owner.
    fn reset_global_cache(&mut self) -> Result<(), String>;
    fn zero_velocity(&mut self, actor: K) -> Result<(), String>;
    /// Independent behavior word; it is not an alias of Fields control/response flags.
    fn set_behavior(&mut self, source: K, value: u32) -> Result<(), String>;
    fn set_preferred(&mut self, source: K, point: K) -> Result<(), String>;
    fn bind_attachment(
        &mut self,
        child: K,
        parent: K,
        child_reference: u32,
        parent_reference: u32,
    ) -> Result<(), String>;
}

fn select(slot: u8) -> Request {
    Request {
        slot,
        flags: 0x20,
        speed_source: 0,
        reference_speed: 0.,
        blend_seconds: 0.,
    }
}
fn history(fields: &mut Fields) {
    fields.previous = fields.current;
    fields.current = fields.position;
}
fn difference(a: f32, b: f32) -> Result<f32, String> {
    let value = (f64::from(a) - f64::from(b)) as f32;
    if a.is_finite() && b.is_finite() && value.is_finite() {
        Ok(value)
    } else {
        Err("reset offset outside acquired finite arithmetic".into())
    }
}

/// Execute once against current ownership. Errors retain already published
/// stores and service effects; callers must quarantine a failed operation,
/// rather than replaying construction or animation from the beginning.
/// This does not acknowledge a primary phase or finish an actor frame.
pub fn execute<K: Copy>(
    source: K,
    point: K,
    services: &mut impl Services<K>,
) -> Result<(), String> {
    services.fields_mut(source)?.control_flags &= !0x100;
    if services.mode_is_four()? {
        let class = services.point_class(point)?;
        if class != 0 {
            let created = services.construct(class, 0)?;
            let point_position = services.fields(point)?.position;
            let offset = services.point_offset(point)?;
            let mut position = [0.; 3];
            for i in 0..3 {
                position[i] = difference(point_position[i], offset.position[i])?;
            }
            services.position(created, position)?;
            let heading = difference(
                services.fields(point)?.angles[0],
                services.point_offset(point)?.heading,
            )?;
            services.fields_mut(created)?.angles[0] = heading;
            services.prepare_transform(created)?;
            services.select_animation(created, select(1))?;
            services.step_animation(created, 0.05)?;
            let (cached, reference) = services.matrices(created)?;
            let matrix = if let Some(reference) = reference {
                cached.mul(reference)?
            } else {
                cached
            };
            if matrix.flags & 0x20 != 0 || !matrix.values.iter().all(|v| v.is_finite()) {
                return Err("reset matrix outside acquired finite affine profile".into());
            }
            let m = matrix.values;
            let heading = (crate::effect_orientation::angle_wide(-m[5], -m[1]) + 90.) as f32;
            services.position(source, [m[3], m[7], m[11]])?;
            services.fields_mut(source)?.angles[0] = heading;
            services.fields_mut(created)?.object_flags |= 0x8000;
        } else {
            let point_fields = services.fields(point)?;
            let heading = point_fields.angles[0];
            let mut position = point_fields.position;
            let x = (f64::from(position[0]) + 40.) as f32;
            if !position[0].is_finite() || !x.is_finite() {
                return Err("reset position offset outside acquired finite arithmetic".into());
            }
            position[0] = x;
            let fields = services.fields_mut(source)?;
            fields.angles[0] = heading;
            fields.position = position;
            history(fields);
        }
    } else {
        let pose = services.birth_pose(source)?;
        let fields = services.fields_mut(source)?;
        fields.angles[0] = pose.heading;
        fields.position = pose.position;
        history(fields);
        services.reset_global_cache()?;
    }
    services.fields_mut(source)?.scale = 2.;
    services.zero_velocity(source)?;
    services.fields_mut(source)?.action = 10;
    services.set_preferred(source, point)?;
    services.set_behavior(source, 0x33)?;
    let context = services.fields(point)?.dispatch_context;
    let fields = services.fields_mut(source)?;
    fields.dispatch_context = context;
    fields.control_flags |= 8;
    services.select_animation(source, select(101))?;
    if services.mode_is_four()? && services.fields(source)?.control_flags & 0x80 == 0 {
        let class = if services.fields(point)?.dispatch_context == 2 {
            951
        } else {
            952
        };
        let child = services.construct(class, 0)?;
        let flags = services.fields(child)?.object_flags | 0x400000;
        services.bind_attachment(child, source, 0, 2)?;
        services.fields_mut(child)?.object_flags = flags;
        services.fields_mut(source)?.control_flags |= 0x80;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Owner {
        fields: Fields,
        behavior: u32,
        preferred: Option<u32>,
    }
    impl Services<u32> for Owner {
        fn fields(&self, _: u32) -> Result<&Fields, String> {
            Ok(&self.fields)
        }
        fn fields_mut(&mut self, _: u32) -> Result<&mut Fields, String> {
            Ok(&mut self.fields)
        }
        fn mode_is_four(&mut self) -> Result<bool, String> {
            Ok(false)
        }
        fn birth_pose(&self, _: u32) -> Result<Pose, String> {
            Ok(Pose {
                position: [1., 2., 3.],
                heading: 40.,
            })
        }
        fn point_class(&self, _: u32) -> Result<u32, String> {
            unreachable!()
        }
        fn point_offset(&self, _: u32) -> Result<Pose, String> {
            unreachable!()
        }
        fn construct(&mut self, _: u32, _: u32) -> Result<u32, String> {
            unreachable!()
        }
        fn position(&mut self, _: u32, _: [f32; 3]) -> Result<(), String> {
            unreachable!()
        }
        fn prepare_transform(&mut self, _: u32) -> Result<(), String> {
            unreachable!()
        }
        fn select_animation(&mut self, _: u32, request: Request) -> Result<(), String> {
            assert_eq!(request.slot, 101);
            // A real backend sees the published header before reporting failure.
            assert_eq!(self.fields.response_flags, 0x80000100);
            assert_eq!(self.fields.control_flags, 8);
            assert_eq!(self.behavior, 0x33);
            assert_eq!(self.preferred, Some(2));
            Err("declared animation failure".into())
        }
        fn step_animation(&mut self, _: u32, _: f32) -> Result<(), String> {
            unreachable!()
        }
        fn matrices(&self, _: u32) -> Result<(Matrix, Option<Matrix>), String> {
            unreachable!()
        }
        fn reset_global_cache(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn zero_velocity(&mut self, _: u32) -> Result<(), String> {
            Ok(())
        }
        fn set_behavior(&mut self, _: u32, value: u32) -> Result<(), String> {
            self.behavior = value;
            Ok(())
        }
        fn set_preferred(&mut self, _: u32, point: u32) -> Result<(), String> {
            self.preferred = Some(point);
            Ok(())
        }
        fn bind_attachment(&mut self, _: u32, _: u32, _: u32, _: u32) -> Result<(), String> {
            unreachable!()
        }
    }
    #[test]
    fn late_failure_preserves_separate_response_control_and_behavior_owners() {
        let mut owner = Owner {
            fields: Fields {
                class: 0,
                dispatch_context: 0,
                configuration: [0; 30],
                scale: 1.,
                object_flags: 0,
                response_flags: 0x80000100,
                control_flags: 0x100,
                action: 0,
                shadow_reciprocal: 0.,
                shadow: None,
                gunlist: Default::default(),
                channels: [[0; 9]; 4],
                position: [0.; 3],
                current: [0.; 3],
                previous: [0.; 3],
                angles: [0.; 3],
            },
            behavior: 77,
            preferred: None,
        };
        assert!(execute(1, 2, &mut owner).is_err());
        assert_eq!(owner.fields.response_flags, 0x80000100);
        assert_eq!((owner.fields.control_flags, owner.behavior), (8, 0x33));
    }
}
