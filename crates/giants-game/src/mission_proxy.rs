//! Mission34 orchestration. Creation and player callbacks require a real
//! backend; reading declarations or testing supplied buffers is not readiness.
use crate::mission_slot::SLOT_SIZE;

pub const CLASS: u32 = 1076;
pub const FLAGS: u32 = 0x8000_0000;

/// Caller-owned actor storage and mandatory runtime services. No default
/// successful creation, initialization or attachment is supplied here.
pub trait Backend {
    type Actor: Copy + Eq;
    fn current(&self) -> Option<Self::Actor>;
    fn residents(&self) -> Result<Vec<Self::Actor>, String>;
    fn bytes(&self, actor: Self::Actor) -> Result<&[u8], String>;
    fn bytes_mut(&mut self, actor: Self::Actor) -> Result<&mut [u8], String>;
    fn create(&mut self, class: u32, flags: u32) -> Result<Self::Actor, String>;
    fn initialize_player(&mut self, player: Self::Actor) -> Result<(), String>;
    fn attach(&mut self, player: Self::Actor, proxy: Self::Actor) -> Result<(), String>;
}

fn word(data: &[u8], offset: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
            .ok_or("mission34 actor storage truncated")?
            .try_into()
            .unwrap(),
    ))
}

/// Execute the acquired command body through explicit service boundaries.
/// The actor's state word selects the branch; no current actor is fabricated.
/// A service error propagates and prevents the final slot flag. Earlier world
/// mutations may already have occurred and require caller-owned cancellation.
pub fn apply(slot: &mut [u8], backend: &mut impl Backend) -> Result<(), String> {
    if slot.len() < SLOT_SIZE {
        return Err("mission34 slot storage truncated".into());
    }
    if let Some(player) = backend.current()
        && word(backend.bytes(player)?, 0x1e0)? == 3
    {
        let mut retiring = Vec::new();
        for actor in backend.residents()? {
            let data = backend.bytes(actor)?;
            if word(data, 4)? == CLASS {
                word(data, 0xd8)?;
                retiring.push(actor);
            }
        }
        for actor in retiring {
            let data = backend.bytes_mut(actor)?;
            let flags = word(data, 0xd8)? | 0x8000;
            data.get_mut(0xd8..0xdc)
                .ok_or("mission34 mutable resident storage truncated")?
                .copy_from_slice(&flags.to_le_bytes());
        }
        let proxy = backend.create(CLASS, FLAGS)?;
        let player = backend
            .current()
            .ok_or("mission34 player disappeared during creation")?;
        // Snapshot after creation: the native constructor is a service boundary
        // and may have changed the supplied world's current actor storage.
        let source = backend.bytes(player)?;
        let position: [u8; 12] = source
            .get(0xa0..0xac)
            .ok_or("mission34 player position truncated")?
            .try_into()
            .unwrap();
        let pose: [u8; 68] = source
            .get(0x294..0x2d8)
            .ok_or("mission34 player pose truncated")?
            .try_into()
            .unwrap();
        let parameter = word(source, 0xc8)?;
        let data = backend.bytes_mut(proxy)?;
        if data.len() < 0x2d8 {
            return Err("mission34 created actor storage truncated".into());
        }
        let flags = word(data, 0xd8)? | 0x400;
        data[0xa0..0xac].copy_from_slice(&position);
        data[0x294..0x2d8].copy_from_slice(&pose);
        data[0xc8..0xcc].copy_from_slice(&parameter.to_le_bytes());
        data[0xfc..0x100].copy_from_slice(&1u32.to_le_bytes());
        data[0xd8..0xdc].copy_from_slice(&flags.to_le_bytes());
        backend.initialize_player(player)?;
        let player = backend
            .current()
            .ok_or("mission34 player disappeared during initialization")?;
        backend.attach(player, proxy)?;
    }
    slot[0x110..0x114].copy_from_slice(&1u32.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Controlled {
        data: [Vec<u8>; 3],
        current: Option<usize>,
        fail: usize,
        calls: Vec<&'static str>,
        attached: Option<(usize, usize)>,
    }
    impl Backend for Controlled {
        type Actor = usize;
        fn current(&self) -> Option<usize> {
            self.current
        }
        fn residents(&self) -> Result<Vec<usize>, String> {
            Ok(Vec::new())
        }
        fn bytes(&self, id: usize) -> Result<&[u8], String> {
            Ok(&self.data[id])
        }
        fn bytes_mut(&mut self, id: usize) -> Result<&mut [u8], String> {
            Ok(&mut self.data[id])
        }
        fn create(&mut self, _: u32, _: u32) -> Result<usize, String> {
            self.calls.push("create");
            if self.fail == 0 {
                return Err("creation failed".into());
            }
            Ok(1)
        }
        fn initialize_player(&mut self, _: usize) -> Result<(), String> {
            self.calls.push("initialize");
            if self.fail == 1 {
                return Err("initialization failed".into());
            }
            self.current = Some(2);
            Ok(())
        }
        fn attach(&mut self, player: usize, proxy: usize) -> Result<(), String> {
            self.calls.push("attach");
            if self.fail == 2 {
                return Err("attachment failed".into());
            }
            self.attached = Some((player, proxy));
            Ok(())
        }
    }
    fn context(fail: usize) -> Controlled {
        let mut c = Controlled {
            data: std::array::from_fn(|_| vec![0; 0x304]),
            current: Some(0),
            fail,
            calls: Vec::new(),
            attached: None,
        };
        c.data[0][0x1e0..0x1e4].copy_from_slice(&3u32.to_le_bytes());
        c
    }
    #[test]
    fn missing_service_success_does_not_acknowledge_the_slot() {
        for fail in 0..3 {
            let mut slot = [0x55; SLOT_SIZE];
            let before = slot;
            let mut backend = context(fail);
            assert!(apply(&mut slot, &mut backend).is_err());
            assert_eq!(slot, before);
            assert_eq!(backend.calls, ["create", "initialize", "attach"][..=fail]);
            assert!(backend.attached.is_none());
        }
    }
    #[test]
    fn callback_can_change_current_actor_before_attachment() {
        let mut backend = context(usize::MAX);
        let mut slot = [0; SLOT_SIZE];
        apply(&mut slot, &mut backend).unwrap();
        assert_eq!(backend.attached, Some((2, 1)));
        assert_eq!(backend.calls, ["create", "initialize", "attach"]);
        assert_eq!(&slot[0x110..0x114], &1u32.to_le_bytes());
    }
}
