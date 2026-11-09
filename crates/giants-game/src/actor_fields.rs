//! Initial class/header fields for the acquired ordinary actor profile.
//! Resource pointers are represented separately; this is not a complete actor,
//! state callback, dynamic-shadow renderer or mission readiness service.
use crate::{animation_channels::Channels, scene_resources::SceneResources, specification};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowDescriptor {
    /// Native label identifies the resource family; range interpretation and
    /// allocation of a rendered texture remain unimplemented.
    pub words: [u32; 7],
}
#[derive(Clone, Debug)]
pub struct Fields {
    pub class: i32,
    /// Owned header context. Class-group initialization supplies zero; ordered
    /// class commands can replace it. Constructor and child producers must
    /// preserve their acquired copy/override semantics.
    pub dispatch_context: u32,
    /// Thirty source words, with pointer slots19/29 normalized to zero. The
    /// gun list is owned below; string-bearing declarations remain refused.
    pub configuration: [u32; 30],
    pub scale: f32,
    pub object_flags: u32,
    /// Shared native status/response word: dirty bit31 and carrying bit23.
    pub response_flags: u32,
    /// Independent control word; it is not the response word above.
    pub control_flags: u32,
    pub action: u32,
    pub shadow_reciprocal: f32,
    pub shadow: Option<Box<ShadowDescriptor>>,
    /// Immutable source-ordered class list; not an acquired player inventory.
    pub gunlist: Arc<[u32]>,
    pub channels: Channels,
    pub position: [f32; 3],
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub angles: [f32; 3],
}
impl Fields {
    /// Common position-history preparation. Copies float bit patterns without
    /// changing flags, pose, animation or declaring a completed actor update.
    pub fn advance_position_history(&mut self) {
        self.previous = self.current;
        self.current = self.position;
    }

    pub fn load(
        resources: &SceneResources,
        class: i32,
        spec: &specification::Prepared,
    ) -> Result<Self, String> {
        Self::load_inner(resources, class, spec)
    }
    pub(crate) fn load_with_context(
        resources: &SceneResources,
        class: i32,
        spec: &specification::Prepared,
    ) -> Result<(Self, u32), String> {
        let fields = Self::load(resources, class, spec)?;
        let context = fields.dispatch_context;
        Ok((fields, context))
    }
    fn load_inner(
        resources: &SceneResources,
        class: i32,
        spec: &specification::Prepared,
    ) -> Result<Self, String> {
        let declaration = resources
            .classes
            .models
            .first_program(class)
            .ok_or("actor class program absent")?;
        let program = declaration.program.as_ref().map_err(Clone::clone)?;
        if class < 0 || spec.class != class || spec.class_catalog != declaration.catalog {
            return Err("actor fields require matching ordinary class resources".into());
        }
        let seed = crate::state_storage::Seed::from_program_for(
            program,
            crate::state_storage::Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )?;
        if !seed.diagnostics.is_empty() {
            return Err("actor state seed has diagnostics".into());
        }
        let mut out = Self {
            class,
            dispatch_context: 0,
            configuration: crate::class_template::initial_configuration(),
            scale: 1.,
            object_flags: 0,
            response_flags: 0,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: Arc::from([]),
            channels: crate::actor_context::initial_channels(None, false),
            position: [0.; 3],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        };
        let mut gunlist = vec![];
        let mut ended = false;
        for record in &program.records {
            let word = |at: usize| -> Result<u32, String> {
                Ok(u32::from_le_bytes(
                    record
                        .bytes
                        .get(at..at + 4)
                        .ok_or("actor field operand truncated")?
                        .try_into()
                        .unwrap(),
                ))
            };
            let float = |at| word(at).map(f32::from_bits);
            match record.opcode {
                1 => {
                    if word(1)? != spec.specification {
                        return Err("actor specification rebinding outside acquired profile".into());
                    }
                    if let Some(Some(model)) = spec.models.first() {
                        let height =
                            model.prepared.mesh.bounds().map_err(|e| e.to_string())?[5].max(1.);
                        out.configuration[23] = height.to_bits();
                        out.configuration[22] =
                            ((f64::from(height) * f64::from(0.8f32)) as f32).to_bits();
                        out.configuration[21] =
                            ((f64::from(height) * f64::from(0.2f32)) as f32).to_bits();
                    }
                }
                3 => {
                    let divisor = float(1)?;
                    if !divisor.is_finite() {
                        return Err("nonfinite dynamic shadow class input".into());
                    }
                    out.shadow_reciprocal = if divisor == 0. {
                        0.
                    } else {
                        (1.0 / f64::from(divisor)) as f32
                    };
                    out.shadow = Some(Box::new(ShadowDescriptor {
                        words: [0, 0, word(5)?, word(9)?, 5, 5, 0x0080_0020],
                    }));
                }
                4 => {
                    out.configuration[1] = word(1)?;
                    out.configuration[0] = word(5)?;
                    out.configuration[25] |= 1 << 16;
                }
                5 => out.configuration[4] = word(1)?,
                6 => out.configuration[2] = word(1)?,
                7 => out.configuration[3] = word(1)?,
                8 => {
                    out.configuration[6] = word(1)?;
                    out.configuration[5] = word(5)?;
                }
                9 => out.configuration[7] = word(1)?,
                10 => out.configuration[8] = word(1)?,
                11 => out.configuration[10] = word(1)?,
                0x0d => out.configuration[9] = word(1)?,
                0x13 => {
                    crate::animation_channels::select_primary(
                        &mut out.channels,
                        crate::animation_channels::Request {
                            slot: *record.bytes.get(1).ok_or("actor loop operand absent")?,
                            flags: 1,
                            speed_source: 0,
                            reference_speed: 0.,
                            blend_seconds: 0.,
                        },
                        crate::animation_channels::Blending::Disabled,
                    )?;
                }
                // Exact-D flag-only stores. Their ordered OR projection is
                // applied once below; none allocates a resource or payload.
                0x14 | 0x16 | 0x2a | 0x18 | 0x1b | 0x1d | 0x22 | 0x23 | 0x2f | 0x41 | 0x54
                | 0x52 | 0x55 | 0x7e => {}
                0x17 => out.scale = float(1)?,
                0x19 => out.configuration[11] = word(1)?,
                0x1a => {
                    if gunlist.len() == 16 {
                        return Err("actor gun list exceeds acquired profile".into());
                    }
                    gunlist.push(word(1)?);
                    out.configuration[18] = gunlist.len() as u32;
                }
                // These records only advance the source cursor in this D build.
                0x1c | 0x32 => {}
                // State allocation and payload attributes have their own owner.
                0x1e | 0x1f | 0x3c | 0x44 => {}
                0x21 => {
                    let center = float(1)?;
                    out.configuration[22] = center.to_bits();
                    out.configuration[23] = word(5)?;
                    out.configuration[21] = ((f64::from(center) * 0.5) as f32).to_bits();
                }
                0x29 => out.configuration[24] = word(1)?,
                0x3b => out.dispatch_context = word(1)?,
                0x15 | 0x25 | 0x26 | 0x31 | 0x36 | 0x39 | 0x4e | 0x4f | 0x50 | 0x69 => {
                    crate::class_template::apply_configuration_extension(
                        &mut out.configuration,
                        record,
                    )?;
                }
                0xff => {
                    ended = true;
                    break;
                }
                other => {
                    return Err(format!(
                        "actor class command {other:02x} outside acquired field profile"
                    ));
                }
            }
        }
        if !ended || !out.scale.is_finite() || out.scale <= 0. {
            return Err("actor field program or scale outside profile".into());
        }
        let flags = crate::object_lists::DirectFlags::project(class, program);
        out.object_flags = flags.active;
        out.configuration[14] = flags.personality;
        out.configuration[20] = u32::from(seed.storage.state());
        out.gunlist = Arc::from(gunlist);
        Ok(out)
    }
    /// Ordinary first/second field copies. Box descriptors are private to each
    /// copy; the immutable class gun list is shared. No callback is executed.
    pub fn instantiate(&self) -> Self {
        let mut fields = self.clone();
        fields.response_flags |= 0x8000_0000;
        if let Some(shadow) = &mut fields.shadow {
            shadow.words = [0, 0, shadow.words[2], shadow.words[3], 5, 5, 0x0080_0020];
        }
        fields
    }
    /// D +1c4 lies in the acquired block +190 at word13. Use current owned
    /// flags and configuration, never a class-name guess or visibility test.
    pub fn entry_contribution(&self) -> crate::mission_selection::EntryContribution {
        crate::mission_selection::EntryContribution {
            object_flags: self.object_flags,
            word_1c4: self.configuration[13],
        }
    }
    /// Mission command3/4 header stores after ordinary template/placement
    /// preparation, before the first object update. This marks no slot or
    /// mission ready and executes no callback or physical-stock allocation.
    pub fn mark_mission_placement(&mut self) {
        self.object_flags |= 0x400;
        self.response_flags |= 0x2000_0000;
    }
    pub fn words(&self) -> Vec<u32> {
        let mut words = self.configuration.to_vec();
        words.extend([
            self.class as u32,
            self.scale.to_bits(),
            self.object_flags,
            self.response_flags,
            self.control_flags,
            self.action,
            self.shadow_reciprocal.to_bits(),
            u32::from(self.shadow.is_some()),
        ]);
        words.extend(self.shadow.as_ref().map_or([0; 7], |s| s.words));
        words.push(self.gunlist.len() as u32);
        words.extend(self.gunlist.iter().copied());
        words.extend(self.channels.iter().flatten().copied());
        words.extend(
            self.position
                .into_iter()
                .chain(self.current)
                .chain(self.previous)
                .chain(self.angles)
                .map(f32::to_bits),
        );
        words
    }
    /// Direct declared scale store for the finite positive source profile.
    /// No transform, bounds, world modifier or callback is updated here.
    pub fn set_declared_scale(&mut self, scale: f32) -> Result<(), &'static str> {
        if !scale.is_finite() || scale <= 0. {
            return Err("declared actor scale outside finite positive profile");
        }
        self.scale = scale;
        Ok(())
    }
    /// Source-declared pose stores after the two copies. Ground adjustment,
    /// modifiers, location callbacks and later movement are separate services.
    pub fn placed(&self, pose: crate::collider_init::DeclaredPose) -> Result<Self, &'static str> {
        if pose.class != self.class as u32 {
            return Err("actor placement class differs");
        }
        if !pose
            .position
            .into_iter()
            .chain([pose.yaw])
            .chain(pose.pitch_roll.into_iter().flatten())
            .all(f32::is_finite)
        {
            return Err("actor placement is nonfinite");
        }
        let mut out = self.instantiate();
        out.position = pose.position;
        out.current = pose.position;
        // The placement block retains B8 from the template. It copies the
        // declared A0 location only into AC; pickup later advances B8 itself.
        out.angles[0] = pose.yaw;
        if let Some([pitch, roll]) = pose.pitch_roll {
            out.angles[1] = pitch;
            out.angles[2] = roll;
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn template() -> Fields {
        let mut configuration = crate::class_template::initial_configuration();
        configuration[18] = 3;
        Fields {
            class: 2,
            dispatch_context: 0x81ff_0246,
            configuration,
            scale: 0.6,
            object_flags: 0,
            response_flags: 0,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.01,
            shadow: Some(Box::new(ShadowDescriptor {
                words: [123, 456, 6, 6, 99, 100, 0x00800020],
            })),
            gunlist: Arc::from([21, 22, 21]),
            channels: crate::actor_context::initial_channels(Some(0), false),
            position: [0.; 3],
            current: [0.; 3],
            previous: [9., 8., 7.],
            angles: [0., 7., -8.],
        }
    }
    #[test]
    fn copies_share_immutable_lists_and_reset_private_shadow_state() {
        let source = template();
        let mut a = source.instantiate();
        let b = source.instantiate();
        assert!(Arc::ptr_eq(&source.gunlist, &a.gunlist));
        assert!(Arc::ptr_eq(&a.gunlist, &b.gunlist));
        assert_eq!(a.gunlist.as_ref(), [21, 22, 21]);
        assert!(!std::ptr::eq(
            a.shadow.as_deref().unwrap(),
            b.shadow.as_deref().unwrap()
        ));
        assert_eq!(
            a.shadow.as_deref().unwrap().words,
            [0, 0, 6, 6, 5, 5, 0x00800020]
        );
        a.shadow.as_deref_mut().unwrap().words[2] = 9;
        assert_eq!(b.shadow.as_deref().unwrap().words[2], 6);
        assert_eq!(source.shadow.as_deref().unwrap().words[0], 123);
        assert_eq!(a.response_flags, 0x80000000);
        assert_eq!(a.dispatch_context, source.dispatch_context);
        assert_eq!(b.dispatch_context, source.dispatch_context);
    }
    #[test]
    fn placed_actor_retains_older_history_and_inherits_omitted_angles() {
        let source = template();
        let pose = crate::collider_init::DeclaredPose {
            class: 2,
            position: [1., 2., 3.],
            yaw: 90.,
            pitch_roll: None,
        };
        let placed = source.placed(pose).unwrap();
        assert_eq!(placed.position, [1., 2., 3.]);
        assert_eq!(placed.current, placed.position);
        assert_eq!(placed.previous, [9., 8., 7.]);
        assert_eq!(placed.angles, [90., 7., -8.]);
        assert_eq!(placed.scale, 0.6);
        assert_eq!(placed.dispatch_context, source.dispatch_context);
        assert!(
            source
                .placed(crate::collider_init::DeclaredPose { class: 11, ..pose })
                .is_err()
        );
        assert!(
            source
                .placed(crate::collider_init::DeclaredPose {
                    yaw: f32::NAN,
                    ..pose
                })
                .is_err()
        );
    }
}
