//! State storage lifecycle and explicit class-command projection.
//! Behavior callbacks and other state attribute commands remain unexecuted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    Debug,
    /// Compatibility default for the retained unversioned GOG projections.
    #[default]
    Gog,
}
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub bytes: usize,
    pub size_callback: bool,
    pub destructor: bool,
}
pub fn layout(state: u8) -> Option<Layout> {
    layout_for(Profile::Gog, state)
}
/// Payload size and callback presence, independently checked on each reference.
/// Equal sizes do not establish equal field offsets or callback behavior.
pub fn layout_for(profile: Profile, state: u8) -> Option<Layout> {
    if state > 106 {
        return None;
    }
    let mut bytes = match state {
        1 => 856,
        2 => 544,
        3 => 4100,
        4 => 100,
        5 => 36,
        6 => 36,
        15 => 108,
        17 => 52,
        18 => 200,
        21 => 216,
        23 => 88,
        24 => 28,
        25 => 16,
        28 => 4,
        29 => 96,
        32 => 1504,
        36 => 8,
        37 => 32,
        38 => 8,
        39 => 44,
        42 => 88,
        43 => 36,
        44 => 76,
        47 => 4,
        49 => 8,
        51 => 576,
        52 => 16,
        53 => 40,
        54 => 12,
        55 => 20,
        57 => 36,
        58 => 700,
        60 => 76,
        74 => 24,
        75 => 200,
        76 => 112,
        77 => 504,
        78 => 268,
        79 => 24,
        82 => 1504,
        83 => 24,
        84 => 4,
        87 => 4,
        88 => 16,
        89 => 4,
        90 => 144,
        92 => 112,
        96 => 96,
        97 => 44,
        99 => 3560,
        100 => 20,
        101 => 16,
        102 => 328,
        103 => 36,
        104 => 12,
        105 => 32,
        106 => 18984,
        _ => 0,
    };
    if profile == Profile::Debug {
        bytes = match state {
            1 => 844,
            2 => 524,
            3 => 4096,
            32 | 82 => 1500,
            37 => 24,
            44 => 68,
            74 | 83 => 20,
            77 => 500,
            _ => bytes,
        };
    }
    Some(Layout {
        bytes,
        size_callback: matches!(
            state,
            1 | 2
                | 3
                | 4
                | 5
                | 6
                | 15
                | 17
                | 18
                | 21
                | 23
                | 24
                | 25
                | 28
                | 29
                | 32
                | 36
                | 37
                | 38
                | 39
                | 42
                | 43
                | 44
                | 47
                | 49
                | 51
                | 52
                | 53
                | 54
                | 55
                | 57
                | 58
                | 60
                | 64
                | 74
                | 75
                | 76
                | 77
                | 78
                | 79
                | 82
                | 83
                | 84
                | 87
                | 88
                | 89
                | 90
                | 92
                | 96
                | 97
                | 99
                | 100
                | 101
                | 102
                | 103
                | 104
                | 105
                | 106
        ),
        destructor: matches!(
            state,
            1 | 3
                | 17
                | 21
                | 24
                | 38
                | 51
                | 58
                | 75
                | 76
                | 77
                | 78
                | 90
                | 92
                | 96
                | 99
                | 104
                | 105
                | 106
        ),
    })
}
#[derive(Debug, Default)]
pub struct Storage {
    profile: Profile,
    state: u8,
    data: Option<Vec<u8>>,
}
#[derive(Clone, Copy, Debug)]
pub struct Transition {
    pub released: bool,
    pub destructor_called: bool,
    pub size_callback: bool,
    pub allocated: usize,
}
impl Storage {
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            ..Default::default()
        }
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn state(&self) -> u8 {
        self.state
    }
    pub fn data_mut(&mut self) -> Option<&mut [u8]> {
        self.data.as_deref_mut()
    }
    pub fn data(&self) -> Option<&[u8]> {
        self.data.as_deref()
    }
    /// Copy payload bytes into already allocated matching storage. Never changes
    /// state, allocates, releases, or runs a behavior/destructor callback.
    pub fn copy_payload_from(&mut self, source: &Self) -> Result<usize, String> {
        if self.profile != source.profile {
            return Err("state payload copy requires the same explicit reference profile".into());
        }
        if self.state != source.state {
            return Ok(0);
        }
        let (Some(destination), Some(source)) = (self.data.as_mut(), source.data.as_ref()) else {
            return Ok(0);
        };
        let bytes = layout_for(self.profile, self.state).map_or(0, |v| v.bytes);
        if destination.len() != bytes || source.len() != bytes {
            return Err("state copy storage does not match descriptor size".into());
        }
        destination.copy_from_slice(source);
        Ok(bytes)
    }
    pub fn transition(&mut self, state: u8) -> Result<Transition, String> {
        self.transition_with(state, |id, _| {
            Err(format!("state {id} destructor is not implemented"))
        })
    }
    /// The supplied destructor runs before release. Missing destructor behavior
    /// is an error; a known layout does not implement its behavior callbacks.
    pub fn transition_with<F>(&mut self, state: u8, mut destroy: F) -> Result<Transition, String>
    where
        F: FnMut(u8, &mut [u8]) -> Result<(), String>,
    {
        let released = self.data.is_some();
        let destructor_called =
            released && layout_for(self.profile, self.state).is_some_and(|v| v.destructor);
        if destructor_called {
            destroy(self.state, self.data.as_mut().unwrap())?;
        }
        self.data = None;
        self.state = state;
        let target = layout_for(self.profile, state);
        let allocated = target.map_or(0, |v| v.bytes);
        if allocated > 0 {
            self.data = Some(vec![0; allocated]);
        }
        Ok(Transition {
            released,
            destructor_called,
            size_callback: target.is_some_and(|v| v.size_callback),
            allocated,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceKey {
    Spec(u32),
    Effect(String),
}
#[derive(Clone, Debug)]
pub struct Reference {
    pub offset: usize,
    pub key: ReferenceKey,
}
#[derive(Debug)]
pub struct Seed {
    pub storage: Storage,
    pub transitions: Vec<Transition>,
    /// Commands not applied to state storage. Object-level effects are separate;
    /// a command with partially handled effects can also remain here.
    pub unexecuted: Vec<u8>,
    pub diagnostics: Vec<String>,
    pub unresolved: Vec<Reference>,
}
impl Seed {
    /// Allocate an independent payload, then copy a prepared class projection.
    /// Typed reference keys remain references to loading-time resources; cloning
    /// their descriptors does not instantiate the resource or a nested object.
    pub fn instantiate(&self) -> Result<Self, String> {
        let mut storage = Storage::new(self.storage.profile());
        storage.transition(self.storage.state())?;
        storage.copy_payload_from(&self.storage)?;
        Ok(Self {
            storage,
            transitions: self.transitions.clone(),
            unexecuted: self.unexecuted.clone(),
            diagnostics: self.diagnostics.clone(),
            unresolved: self.unresolved.clone(),
        })
    }
    pub fn from_program(program: &giants_formats::class_program::Program) -> Result<Self, String> {
        Self::from_program_with_precision(program, crate::arithmetic::Precision::Double53)
    }
    pub fn from_program_with_precision(
        program: &giants_formats::class_program::Program,
        precision: crate::arithmetic::Precision,
    ) -> Result<Self, String> {
        Self::from_program_for(program, Profile::Gog, precision)
    }
    /// The profile selects measured allocation, raw inline and attribute writes.
    /// Resource keys remain unresolved; object effects stay separate.
    /// Object commands remain unexecuted, as with the older projection.
    pub fn from_program_for(
        program: &giants_formats::class_program::Program,
        profile: Profile,
        precision: crate::arithmetic::Precision,
    ) -> Result<Self, String> {
        let mut seed = Self {
            storage: Storage::new(profile),
            transitions: vec![],
            unexecuted: vec![],
            diagnostics: vec![],
            unresolved: vec![],
        };
        for record in &program.records {
            match record.opcode {
                0x1e => {
                    if layout_for(profile, record.bytes[1]).is_none() {
                        seed.diagnostics.push(
                            "state id has no known descriptor; behavior remains unsupported".into(),
                        );
                    }
                    seed.transitions
                        .push(seed.storage.transition(record.bytes[1])?);
                    seed.unresolved.clear();
                }
                0x44 => {
                    let payload = &record.bytes[5..];
                    if let Some(data) = seed.storage.data.as_mut() {
                        if payload.len() > data.len() {
                            return Err("inline class words exceed state storage".into());
                        }
                        data[..payload.len()].copy_from_slice(payload);
                        seed.unresolved.retain(|r| r.offset >= payload.len());
                    } else {
                        seed.diagnostics
                            .push("inline state words have no allocated storage".into());
                    }
                }
                0xff => {}
                opcode => {
                    if !seed.apply_attribute(record, precision)? {
                        seed.unexecuted.push(opcode);
                    }
                }
            }
        }
        Ok(seed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_sizes_do_not_allow_cross_reference_payload_copies() {
        let mut source = Storage::new(Profile::Debug);
        source.transition(18).unwrap();
        source.data_mut().unwrap().fill(7);
        let mut destination = Storage::new(Profile::Gog);
        destination.transition(18).unwrap();
        destination.data_mut().unwrap().fill(0xa5);
        assert!(destination.copy_payload_from(&source).is_err());
        assert_eq!(destination.data().unwrap(), &[0xa5; 200]);
        assert_eq!(source.data().unwrap(), &[7; 200]);
    }
    #[test]
    fn debug_seeds_preserve_profile_and_keep_gog_offsets_separate() {
        let program = giants_formats::class_program::parse(&[
            0x1e, 2, 0x1f, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 0xff,
        ])
        .unwrap();
        let seed = Seed::from_program_for(
            &program,
            Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )
        .unwrap();
        assert_eq!(seed.storage.data().unwrap().len(), 524);
        assert!(seed.unexecuted.is_empty());
        assert!(seed.diagnostics.is_empty());
        let gog = Seed::from_program(&program).unwrap();
        assert_eq!(seed.unresolved[0].offset, 0x70);
        assert_eq!(gog.unresolved[0].offset, 0x6c);
        assert_eq!(seed.storage.data().unwrap()[0x6c..0x70], [0; 4]);
        let instance = seed.instantiate().unwrap();
        assert_eq!(instance.storage.profile(), Profile::Debug);
        assert_eq!(instance.storage.data(), seed.storage.data());
        assert_eq!(instance.unexecuted, seed.unexecuted);
        let mut oversized = vec![0x1e, 2, 0x44];
        oversized.extend_from_slice(&136u32.to_le_bytes());
        oversized.extend_from_slice(&[1; 544]);
        oversized.push(0xff);
        let program = giants_formats::class_program::parse(&oversized).unwrap();
        assert!(
            Seed::from_program_for(
                &program,
                Profile::Debug,
                crate::arithmetic::Precision::Double53
            )
            .is_err()
        );
        assert_eq!(
            Seed::from_program(&program)
                .unwrap()
                .storage
                .data()
                .unwrap(),
            &[1; 544]
        );
    }
    #[test]
    fn seed_keeps_deferred_attributes_and_checks_inline_bounds() {
        let program = giants_formats::class_program::parse(&[
            0x1e, 28, 0x44, 1, 0, 0, 0, 1, 2, 3, 4, 0x13, 1, 0xff,
        ])
        .unwrap();
        let seed = Seed::from_program(&program).unwrap();
        assert_eq!(seed.storage.data().unwrap(), [1, 2, 3, 4]);
        assert_eq!(seed.unexecuted, [0x13]);
        let oversized = giants_formats::class_program::parse(&[
            0x1e, 28, 0x44, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff,
        ])
        .unwrap();
        assert!(Seed::from_program(&oversized).is_err());
        let unknown = giants_formats::class_program::parse(&[0x1e, 255, 0xff]).unwrap();
        assert!(!Seed::from_program(&unknown).unwrap().diagnostics.is_empty());
    }
    #[test]
    fn lifecycle_requires_destructor_and_zeroes_new_storage() {
        let mut state = Storage::default();
        assert_eq!(state.transition(1).unwrap().allocated, 856);
        state.data.as_mut().unwrap()[0] = 99;
        assert!(state.transition(74).is_err());
        assert_eq!(state.state(), 1);
        assert_eq!(state.data().unwrap()[0], 99);
        let mut calls = 0;
        let result = state
            .transition_with(74, |id, data| {
                assert_eq!(id, 1);
                assert_eq!(data[0], 99);
                calls += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(calls, 1);
        assert!(result.released && result.destructor_called);
        assert_eq!(state.data().unwrap(), vec![0; 24]);
        let zero = state.transition(64).unwrap();
        assert!(zero.released && zero.size_callback);
        assert!(state.data().is_none());
        assert!(layout(255).is_none());
        assert_eq!(state.transition(255).unwrap().allocated, 0);
    }
}
