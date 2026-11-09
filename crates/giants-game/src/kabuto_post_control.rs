//! Acquired guard between control forces and resource timers. Required lifecycle
//! and external services remain explicit; a requested global tail is not complete.
#[derive(Clone, Copy, Debug)]
pub struct Actor {
    pub heading: f32,
    pub flags: u32,
    pub damage: f32,
    pub limit: f32,
    pub state: u32,
    pub buttons: u32,
    pub physics_flags: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notification {
    Sound(u32),
    Growth(u8),
}
pub trait World {
    fn local_owner(&mut self) -> Result<bool, String>;
    fn timer(&mut self) -> Result<f32, String>;
    fn global_seconds(&mut self) -> Result<f32, String>;
    fn store_timer(&mut self, value: f32) -> Result<(), String>;
    fn mode(&mut self) -> Result<u32, String>;
    fn start_sound(&mut self, id: u32, actor: &mut Actor, payload: &mut [u8])
    -> Result<(), String>;
    fn notifications(&mut self) -> Result<bool, String>;
    fn notify(
        &mut self,
        notification: Notification,
        actor: &mut Actor,
        payload: &mut [u8],
    ) -> Result<(), String>;
    fn reinitialize(
        &mut self,
        class: u32,
        actor: &mut Actor,
        payload: &mut [u8],
    ) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    ResourceTimers,
    PrimaryExit,
    GlobalTailRequired,
}
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub forward: [f32; 3],
    pub outcome: Outcome,
}
fn scalar(p: &[u8], at: usize) -> Result<f32, String> {
    finite(f32::from_le_bytes(p[at..at + 4].try_into().unwrap()))
}
fn finite(value: f32) -> Result<f32, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Kabuto post-control scalar outside finite profile".into())
    }
}
/// Retain prefix, timer and provider effects on error. The enclosing owner must
/// quarantine replay. Neither success nor either exit acknowledges a whole frame.
pub fn advance(actor: &mut Actor, p: &mut [u8], w: &mut impl World) -> Result<View, String> {
    if p.len() != 844 || !actor.heading.is_finite() || actor.heading.abs() > 1_000_000. {
        return Err("Kabuto post-control requires its acquired payload and heading".into());
    }
    let (sine, cosine) = crate::object_transform::sincos(actor.heading);
    let forward = [sine as f32, -cosine as f32, 0.];
    // This store precedes the guard and every external service.
    actor.flags |= 0x40;
    let damage = finite(actor.damage)?;
    let limit = finite(actor.limit)?;
    if damage < limit {
        return Ok(View {
            forward,
            outcome: Outcome::ResourceTimers,
        });
    }
    if w.local_owner()? {
        let mut timer = f64::from(finite(w.timer()?)?);
        if actor.flags & 0x80000 != 0 || actor.physics_flags & 0x68 != 0 {
            timer += f64::from(finite(w.global_seconds()?)?);
            // Keep the wide value for the comparison, even though the shared
            // timer has already been stored at single precision.
            w.store_timer(finite(timer as f32)?)?;
        }
        if w.mode()? == 0 {
            if timer > 5. {
                return Ok(View {
                    forward,
                    outcome: Outcome::GlobalTailRequired,
                });
            }
        } else if timer > 2. && actor.buttons & 4 != 0 {
            let size = scalar(p, 0x158)?;
            let low = crate::arithmetic::integer_low_word(f64::from(size) - 1.) as u8 as i8;
            let target = low.max(0) as u8;
            let target_scalar = f32::from(target);
            if size != target_scalar {
                w.start_sound(0x6e4, actor, p)?;
                if w.notifications()? {
                    w.notify(Notification::Sound(0x6e4), actor, p)?;
                }
                p[0x158..0x15c].copy_from_slice(&target_scalar.to_le_bytes());
                if w.notifications()? {
                    w.notify(Notification::Growth(target), actor, p)?;
                }
            }
            let class = if actor.state == 1 {
                u32::from_le_bytes(p[0x348..0x34c].try_into().unwrap())
            } else {
                1
            };
            w.reinitialize(class, actor, p)?;
        }
    }
    Ok(View {
        forward,
        outcome: if actor.state == 1 {
            Outcome::ResourceTimers
        } else {
            Outcome::PrimaryExit
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Services {
        timer: f32,
        fail_sound: bool,
        calls: Vec<u32>,
    }
    impl World for Services {
        fn local_owner(&mut self) -> Result<bool, String> {
            Ok(true)
        }
        fn timer(&mut self) -> Result<f32, String> {
            Ok(self.timer)
        }
        fn global_seconds(&mut self) -> Result<f32, String> {
            Ok(0.25)
        }
        fn store_timer(&mut self, value: f32) -> Result<(), String> {
            self.timer = value;
            Ok(())
        }
        fn mode(&mut self) -> Result<u32, String> {
            Ok(1)
        }
        fn start_sound(&mut self, id: u32, a: &mut Actor, p: &mut [u8]) -> Result<(), String> {
            assert_eq!(id, 0x6e4);
            self.calls.push(0);
            p[0x128..0x12c].copy_from_slice(&0.5f32.to_le_bytes());
            if self.fail_sound {
                a.state = 23;
                Err("sound unavailable".into())
            } else {
                Ok(())
            }
        }
        fn notifications(&mut self) -> Result<bool, String> {
            self.calls.push(1);
            Ok(true)
        }
        fn notify(&mut self, n: Notification, _: &mut Actor, p: &mut [u8]) -> Result<(), String> {
            match n {
                Notification::Sound(0x6e4) => {
                    assert_eq!(scalar(p, 0x158)?, 2.5);
                    self.calls.push(2);
                }
                Notification::Growth(1) => {
                    assert_eq!(scalar(p, 0x158)?, 1.);
                    self.calls.push(3);
                }
                _ => panic!("unexpected notification"),
            }
            Ok(())
        }
        fn reinitialize(&mut self, class: u32, a: &mut Actor, _: &mut [u8]) -> Result<(), String> {
            assert_eq!(class, 17);
            self.calls.push(4);
            a.state = 23;
            Err("lifecycle service unavailable".into())
        }
    }
    fn run(fail_sound: bool) -> (Actor, Vec<u8>, Services) {
        let mut actor = Actor {
            heading: 0.,
            flags: 0,
            damage: 100.,
            limit: 100.,
            state: 1,
            buttons: 4,
            physics_flags: 8,
        };
        let mut payload = vec![0; 844];
        payload[0x158..0x15c].copy_from_slice(&2.5f32.to_le_bytes());
        payload[0x348..0x34c].copy_from_slice(&17u32.to_le_bytes());
        let mut services = Services {
            timer: 3.,
            fail_sound,
            calls: vec![],
        };
        assert!(advance(&mut actor, &mut payload, &mut services).is_err());
        assert_eq!(actor.flags, 0x40);
        assert_eq!(actor.state, 23);
        assert_eq!(services.timer, 3.25);
        assert_eq!(scalar(&payload, 0x128).unwrap(), 0.5);
        (actor, payload, services)
    }
    #[test]
    fn failed_sound_retains_prefix_and_provider_effects_before_target_store() {
        let (_, p, w) = run(true);
        assert_eq!(scalar(&p, 0x158).unwrap(), 2.5);
        assert_eq!(w.calls, [0]);
    }
    #[test]
    fn failed_reinitialization_retains_target_and_ordered_notifications() {
        let (_, p, w) = run(false);
        assert_eq!(scalar(&p, 0x158).unwrap(), 1.);
        assert_eq!(w.calls, [0, 1, 2, 1, 3, 4]);
    }
}
