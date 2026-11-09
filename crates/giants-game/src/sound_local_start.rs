//! Acquired debug 2D-start caller policy. Descriptor preparation, instance
//! ownership, stopping and actual creation are required services, not no-ops.

#[derive(Clone, Copy)]
pub struct Instance<D, O> {
    pub descriptor: D,
    pub owner: Option<O>,
    pub flags: u32,
}

pub trait Services {
    type Id: Copy + Eq;
    type Descriptor: Copy + Eq;
    type Owner: Copy + Eq;
    fn prepare(&mut self, descriptor: Self::Descriptor) -> Result<(), String>;
    fn first(&self) -> Result<Option<Self::Id>, String>;
    fn next(&self, id: Self::Id) -> Result<Option<Self::Id>, String>;
    fn instance(&self, id: Self::Id) -> Result<Instance<Self::Descriptor, Self::Owner>, String>;
    fn stop_descriptor(&mut self, descriptor: Self::Descriptor) -> Result<(), String>;
    fn stop(&mut self, id: Self::Id) -> Result<(), String>;
    fn create(&mut self, descriptor: Self::Descriptor) -> Result<Self::Id, String>;
    /// Retain the actual new instance and OR its owner/update marks. This service
    /// is not called for reuse, a forced start, or an absent owner.
    fn attach(&mut self, id: Self::Id, owner: Self::Owner) -> Result<(), String>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Started<I> {
    pub id: I,
    pub reused: bool,
}

pub fn start<S: Services>(
    services: &mut S,
    descriptor: S::Descriptor,
    flags: u32,
    forced: bool,
    owner: Option<S::Owner>,
) -> Result<Started<S::Id>, String> {
    services.prepare(descriptor)?;
    if forced {
        services.stop_descriptor(descriptor)?;
        return services
            .create(descriptor)
            .map(|id| Started { id, reused: false });
    }
    if flags & 0x20 != 0
        && let Some(owner) = owner
    {
        let mut current = services.first()?;
        let mut seen = Vec::new();
        while let Some(id) = current {
            if seen.contains(&id) || seen.len() >= 4096 {
                return Err("local sound instance list cycle or traversal budget".into());
            }
            seen.push(id);
            let instance = services.instance(id)?;
            if instance.descriptor == descriptor
                && instance.owner == Some(owner)
                && instance.flags & 0xc0000000 == 0
            {
                if flags & 0x80 == 0 {
                    return Ok(Started { id, reused: true });
                }
                services.stop(id)?;
            }
            current = services.next(id)?;
        }
    }
    let id = services.create(descriptor)?;
    if let Some(owner) = owner {
        services.attach(id, owner)?;
    }
    Ok(Started { id, reused: false })
}

/// Actual descriptor distance preparation before even a reused voice. This
/// consumes no RNG. Invalid numeric input is an independent-runtime refusal.
pub fn prepare_distances(words: &mut [u32; 6]) -> Result<(), String> {
    let mut minimum = f32::from_bits(words[4]);
    let mut maximum = f32::from_bits(words[5]);
    if !minimum.is_finite() || !maximum.is_finite() {
        return Err("local descriptor distances must be finite".into());
    }
    if minimum == 0. {
        minimum = 10.;
    }
    if maximum == 0. {
        maximum = (f64::from(minimum) * 15.) as f32;
    }
    if !maximum.is_finite() {
        return Err("local descriptor distance preparation overflow".into());
    }
    words[4] = minimum.to_bits();
    words[5] = maximum.to_bits();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_zero_defaults_are_idempotent_and_preserve_negative_values() {
        let mut words = [0, 0, 0, 0, (-0f32).to_bits(), 0];
        prepare_distances(&mut words).unwrap();
        assert_eq!(&words[4..], &[10f32.to_bits(), 150f32.to_bits()]);
        let prepared = words;
        prepare_distances(&mut words).unwrap();
        assert_eq!(words, prepared);
        words[4] = (-1f32).to_bits();
        words[5] = (-2f32).to_bits();
        prepare_distances(&mut words).unwrap();
        assert_eq!(&words[4..], &[(-1f32).to_bits(), (-2f32).to_bits()]);
    }
    struct Pool {
        nodes: Vec<Instance<u32, u32>>,
        stopped: Vec<usize>,
        attach_failure: bool,
    }
    impl Services for Pool {
        type Id = usize;
        type Descriptor = u32;
        type Owner = u32;
        fn prepare(&mut self, _: u32) -> Result<(), String> {
            Ok(())
        }
        fn first(&self) -> Result<Option<usize>, String> {
            Ok((!self.nodes.is_empty()).then_some(0))
        }
        fn next(&self, id: usize) -> Result<Option<usize>, String> {
            Ok((id + 1 < self.nodes.len()).then_some(id + 1))
        }
        fn instance(&self, id: usize) -> Result<Instance<u32, u32>, String> {
            Ok(self.nodes[id])
        }
        fn stop_descriptor(&mut self, _: u32) -> Result<(), String> {
            Err("required descriptor stop".into())
        }
        fn stop(&mut self, id: usize) -> Result<(), String> {
            self.nodes[id].flags |= 0x80000000;
            self.stopped.push(id);
            Err("late stop failure".into())
        }
        fn create(&mut self, descriptor: u32) -> Result<usize, String> {
            let id = self.nodes.len();
            self.nodes.push(Instance {
                descriptor,
                owner: None,
                flags: 1,
            });
            Ok(id)
        }
        fn attach(&mut self, id: usize, owner: u32) -> Result<(), String> {
            self.nodes[id].owner = Some(owner);
            if self.attach_failure {
                Err("late owner binding failure".into())
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn required_service_failures_preserve_completed_instance_effects() {
        let mut pool = Pool {
            nodes: vec![Instance {
                descriptor: 9,
                owner: Some(7),
                flags: 1,
            }],
            stopped: Vec::new(),
            attach_failure: false,
        };
        assert!(start(&mut pool, 9, 0xa0, false, Some(7)).is_err());
        assert_eq!(pool.stopped, [0]);
        assert_eq!(pool.nodes.len(), 1);
        assert_eq!(pool.nodes[0].flags, 0x80000001);
        pool.attach_failure = true;
        assert!(start(&mut pool, 9, 0x20, false, Some(7)).is_err());
        assert_eq!(pool.nodes.len(), 2);
        assert_eq!(pool.nodes[1].owner, Some(7));
        assert!(start(&mut pool, 9, 0, true, Some(7)).is_err());
        assert_eq!(pool.nodes.len(), 2);
    }
}
