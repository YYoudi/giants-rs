//! Source-bounded projectile flight and optional requested static-terrain impact.
//! Object contacts, attached services and native world scheduling remain upstream.
#[derive(Clone, Debug)]
pub struct AttachedChild {
    pub reference: u32,
    pub child_reference: u32,
    pub name: String,
}
#[derive(Clone, Debug)]
pub struct Recipe {
    pub class: i32,
    pub speed: f32,
    pub gravity_mass: f32,
    pub contact_requested: bool,
    pub attached_children: Vec<AttachedChild>,
    pub retirement_child: String,
    pub visual: Option<crate::effect_visual::Configuration>,
    pub tracer: Option<Tracer>,
}
pub use crate::effect_tracer::Configuration as Tracer;
impl Recipe {
    /// Initial loop1/state78 body with source collision kind7. Static contact
    /// must be bound separately when the effect program requests it.
    pub fn validate_class(
        &self,
        p: &giants_formats::class_program::Program,
    ) -> Result<(), &'static str> {
        let ops = p.records.iter().map(|r| r.opcode).collect::<Vec<_>>();
        if !(ops == [1, 0x13, 0x36, 0x1e, 0xff]
            || self.visual.is_some() && ops == [1, 0x13, 0x1b, 0x1e, 0x36, 0xff])
            || p.records.iter().any(|r| {
                r.opcode == 0x13 && r.bytes.get(1) != Some(&1)
                    || r.opcode == 0x36 && r.bytes.get(1) != Some(&7)
                    || r.opcode == 0x1e && r.bytes.get(1) != Some(&78)
            })
        {
            return Err("projectile class outside checked loop1/kind7/state78 profile");
        }
        Ok(())
    }
    pub fn project(p: &giants_formats::effect_program::Program) -> Result<Self, &'static str> {
        let r = &p.records;
        let ops = r.iter().map(|r| r.opcode).collect::<Vec<_>>();
        if ops == [3, 6, 0x11, 4, 0x20, 0x22, 0x25, 0x26, 0xd, 7, 2] {
            return Self::finite_traced(p);
        }
        if ops.len() < 8
            || ops[..4] != [3, 6, 0x11, 0x0e]
            || ops[ops.len() - 4..] != [0x15, 0x0d, 7, 2]
            || ops[4..ops.len() - 4].iter().any(|op| *op != 0x0a)
        {
            return Err("projectile commands outside free-flight inspection profile");
        }
        let word =
            |r: &giants_formats::effect_program::Record, at: usize| -> Result<u32, &'static str> {
                Ok(u32::from_le_bytes(
                    r.bytes
                        .get(at..at + 4)
                        .ok_or("truncated projectile operand")?
                        .try_into()
                        .map_err(|_| "invalid projectile word")?,
                ))
            };
        let mut attached_children = Vec::new();
        for r in &r[4..r.len() - 4] {
            let tail = r.bytes.get(5..).ok_or("projectile child name absent")?;
            let end = tail
                .iter()
                .position(|b| *b == 0)
                .ok_or("projectile child name unterminated")?
                + 5;
            if end == 5 || end + 5 != r.bytes.len() {
                return Err("projectile child framing invalid");
            }
            attached_children.push(AttachedChild {
                reference: word(r, 1)?,
                child_reference: word(r, end + 1)?,
                name: String::from_utf8(r.bytes[5..end].to_vec())
                    .map_err(|_| "invalid projectile child name")?,
            });
        }
        let name = &r[r.len() - 3].bytes;
        if name.last() != Some(&0) {
            return Err("projectile retirement name unterminated");
        }
        let retirement_child = String::from_utf8(
            name.get(
                1..name
                    .len()
                    .checked_sub(1)
                    .ok_or("projectile retirement name absent")?,
            )
            .ok_or("projectile retirement name truncated")?
            .to_vec(),
        )
        .map_err(|_| "invalid projectile retirement name")?;
        let recipe = Self {
            class: word(&r[0], 1)? as i32,
            speed: f32::from_bits(word(&r[2], 1)?),
            gravity_mass: f32::from_bits(word(&r[3], 1)?),
            contact_requested: true,
            attached_children,
            retirement_child,
            visual: None,
            tracer: None,
        };
        if recipe.class < 0
            || !recipe.speed.is_finite()
            || !recipe.gravity_mass.is_finite()
            || recipe.retirement_child.is_empty()
        {
            return Err("invalid projectile recipe");
        }
        Ok(recipe)
    }
    fn finite_traced(p: &giants_formats::effect_program::Program) -> Result<Self, &'static str> {
        let word = |i: usize, at: usize| -> Result<u32, &'static str> {
            Ok(u32::from_le_bytes(
                p.records[i]
                    .bytes
                    .get(at..at + 4)
                    .ok_or("finite projectile operand truncated")?
                    .try_into()
                    .map_err(|_| "finite projectile word invalid")?,
            ))
        };
        let value = |i, at| word(i, at).map(f32::from_bits);
        let visual = crate::effect_visual::Configuration {
            lifetime: value(3, 1)?,
            fade_in: value(3, 5)?,
            fade_out_start: value(3, 9)?,
            start_size: 1.,
            end_size: 1.,
        };
        crate::effect_visual::Visual::new(visual)?;
        let name = &p.records[8].bytes;
        if name.len() < 3 || name.last() != Some(&0) || name[1..name.len() - 1].contains(&0) {
            return Err("finite projectile retirement invalid");
        }
        let tracer = Tracer {
            colors: [
                [value(4, 1)?, value(4, 5)?, value(4, 9)?],
                [value(4, 13)?, value(4, 17)?, value(4, 21)?],
            ],
            lifetime: value(5, 1)?,
            fade_start: value(5, 5)?,
            half_width: value(6, 1)?,
            min_interval: value(7, 1)?,
        };
        if tracer.fade_start >= tracer.lifetime
            || tracer
                .colors
                .iter()
                .flatten()
                .chain([&tracer.lifetime, &tracer.fade_start])
                .chain([&tracer.half_width, &tracer.min_interval])
                .any(|v| !v.is_finite())
            || !tracer
                .configured_words()
                .iter()
                .all(|w| f32::from_bits(*w).is_finite())
        {
            return Err("nonfinite projectile tracer configuration");
        }
        let recipe = Self {
            class: word(0, 1)? as i32,
            speed: value(2, 1)?,
            gravity_mass: 1.,
            contact_requested: false,
            attached_children: Vec::new(),
            retirement_child: std::str::from_utf8(&name[1..name.len() - 1])
                .map_err(|_| "projectile retirement name invalid")?
                .into(),
            visual: Some(visual),
            tracer: Some(tracer),
        };
        if recipe.class < 0 || !recipe.speed.is_finite() {
            return Err("invalid finite projectile profile");
        }
        Ok(recipe)
    }
}
pub struct Playback {
    pub recipe: Recipe,
    pub motion: crate::mecc_motion::Motion,
    pub matrix: Option<crate::procedural_aim::Matrix>,
    pub callbacks: u64,
    pub visual: Option<crate::effect_visual::Visual>,
    pub expired: bool,
    pub contact: Option<crate::effect_contact::State>,
    object_flags: u32,
}
impl Playback {
    pub fn new(recipe: Recipe) -> Self {
        Self {
            recipe,
            motion: crate::mecc_motion::Motion {
                position: [0.; 3],
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                force: [0.; 3],
                mass: 1.,
                speed_limit: 0.,
                force_time_limit: 0.,
                stored_dt: 0.,
            },
            matrix: None,
            callbacks: 0,
            visual: None,
            expired: false,
            contact: None,
            object_flags: 0,
        }
    }
    pub fn reset(&mut self) {
        self.matrix = None;
        self.callbacks = 0;
        self.visual = None;
        self.expired = false;
        self.motion.position = [0.; 3];
        self.motion.velocity = [0.; 3];
        self.motion.previous_velocity = [0.; 3];
        self.motion.force = [0.; 3];
        self.motion.stored_dt = 0.;
        self.motion.force_time_limit = 0.;
        if let Some(contact) = &mut self.contact {
            contact.reset();
        }
    }
    pub fn request(&mut self, matrix: crate::procedural_aim::Matrix) -> Result<(), &'static str> {
        if self.matrix.is_some() {
            return Err("projectile already detached");
        }
        initial_velocity(matrix, self.recipe.speed)?;
        let visual = self
            .recipe
            .visual
            .map(crate::effect_visual::Visual::new)
            .transpose()?;
        self.motion.position = [matrix.values[3], matrix.values[7], matrix.values[11]];
        self.matrix = Some(matrix);
        self.visual = visual;
        Ok(())
    }
    pub fn configure_class(
        &mut self,
        program: &giants_formats::class_program::Program,
    ) -> Result<(), &'static str> {
        if self.matrix.is_some() {
            return Err("projectile class must bind before birth");
        }
        self.recipe.validate_class(program)?;
        self.object_flags =
            crate::object_lists::DirectFlags::project(self.recipe.class, program).active;
        Ok(())
    }
    /// Diagnostic contact-free flight only. Contact/trail/retirement services
    /// remain pending, with no invented timeout or impact acknowledgement.
    pub fn advance_free(&mut self, dt: f32) -> Result<bool, &'static str> {
        if self.contact.is_some() {
            return Err("bound projectile contact requires a terrain query");
        }
        self.advance_with_query(dt, |_| Err("projectile ground query absent"))
    }
    pub fn bind_static_contact(
        &mut self,
        seed: crate::effect_contact::Seed,
    ) -> Result<(), &'static str> {
        if self.matrix.is_some() || self.contact.is_some() {
            return Err("projectile contact must bind once before birth");
        }
        self.contact = Some(seed.instantiate_projectile(&self.recipe)?);
        Ok(())
    }
    pub fn advance_static(
        &mut self,
        dt: f32,
        query: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<bool, &'static str> {
        if self.contact.is_none() {
            return Err("projectile static contact is not bound");
        }
        self.advance_with_query(dt, query)
    }
    fn advance_with_query(
        &mut self,
        dt: f32,
        query: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<bool, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid projectile delta");
        }
        let Some(matrix) = self.matrix else {
            return Ok(false);
        };
        if dt == 0. || self.expired {
            return Ok(false);
        }
        if let Some(visual) = &mut self.visual {
            match visual.advance(dt)? {
                crate::effect_visual::Step::FirstFrame => {
                    self.callbacks += 1;
                    return Ok(true);
                }
                crate::effect_visual::Step::Expired => {
                    self.expired = true;
                    self.callbacks += 1;
                    return Ok(true);
                }
                crate::effect_visual::Step::VisibleState => {}
            }
        }
        if self.callbacks > 0 {
            if self.callbacks == 1 {
                self.motion.velocity = initial_velocity(matrix, self.recipe.speed)?;
            }
            let previous = self.motion.position;
            self.motion = crate::mecc_motion::integrate(
                self.motion,
                dt,
                false,
                false,
                crate::arithmetic::Precision::Double53,
            )?;
            if let Some(contact) = &mut self.contact {
                contact.resolve_projectile(&mut self.motion, previous, &self.recipe, query)?;
            }
            let gravity = crate::mecc_gravity::apply(
                crate::mecc_gravity::Input {
                    force: self.motion.force,
                    velocity: self.motion.velocity,
                    normal: self
                        .contact
                        .as_ref()
                        .map_or([0.; 3], |c| [c.plane[0], c.plane[1], c.plane[2]]),
                    gravity_mass: self.recipe.gravity_mass,
                    mass: 1.,
                    friction_limit: 0.,
                    damping_xy: 0.,
                    damping_z: 0.,
                    force_time_limit: self.motion.force_time_limit,
                    object_flags: self.object_flags,
                    physics_flags: self.contact.as_ref().map_or(0, |c| c.flags),
                    project_contact: self.contact.is_some(),
                },
                crate::arithmetic::Precision::Double53,
            )?;
            self.motion.force = gravity.force;
            self.motion.force_time_limit = gravity.force_time_limit;
            if self.contact.as_ref().is_some_and(|c| c.flags & 4 != 0) {
                self.expired = true;
            }
        }
        self.callbacks += 1;
        Ok(true)
    }
    pub fn world_matrix(&self) -> Option<crate::procedural_aim::Matrix> {
        if self.expired {
            return None;
        }
        self.matrix.map(|mut m| {
            for (i, v) in [3, 7, 11].into_iter().zip(self.motion.position) {
                m.values[i] = v;
            }
            m
        })
    }
}
pub fn initial_velocity(
    matrix: crate::procedural_aim::Matrix,
    speed: f32,
) -> Result<[f32; 3], &'static str> {
    if !speed.is_finite()
        || !matrix.values.iter().all(|v| v.is_finite())
        || matrix.flags & 0x20 != 0
    {
        return Err("projectile velocity outside finite affine profile");
    }
    let vector = [-matrix.values[1], -matrix.values[5], -matrix.values[9]];
    let [x, y, z] = vector.map(f64::from);
    let squared = (x * x + z * z) + y * y;
    let normalized = if squared < f64::from(0.0001_f32) {
        [0.; 3]
    } else {
        let inverse = 1. / squared.sqrt();
        vector.map(|v| (f64::from(v) * inverse) as f32)
    };
    Ok(normalized.map(|v| (f64::from(v) * f64::from(speed)) as f32))
}
#[cfg(test)]
mod tests {
    #[test]
    fn requested_contact_requires_backend_retires_once_and_resets_without_losing_binding() {
        let program = giants_formats::class_program::parse(&[0x1e, 78, 0x36, 7, 0xff]).unwrap();
        let profile = crate::effect_contact::Seed::from_projectile_class(&program).unwrap();
        let mut recipe = super::Recipe {
            class: 900,
            speed: 0.,
            gravity_mass: 0.,
            contact_requested: false,
            attached_children: vec![],
            retirement_child: "synthetic_child".into(),
            visual: None,
            tracer: None,
        };
        assert!(profile.instantiate_projectile(&recipe).is_err());
        recipe.contact_requested = true;
        let mut playback = super::Playback::new(recipe);
        playback.bind_static_contact(profile).unwrap();
        assert!(playback.advance_free(0.01).is_err());
        playback
            .request(crate::procedural_aim::Matrix::identity())
            .unwrap();
        assert!(
            playback
                .advance_static(0.01, |_| Err("first callback queried terrain"))
                .unwrap()
        );
        assert!(!playback.expired);
        playback
            .advance_static(0.01, |_| {
                Ok(crate::physical_ground::Ground {
                    plane: [0., 0., 1., -10.],
                    height: 10.,
                    key: [1, 0],
                })
            })
            .unwrap();
        assert!(playback.expired);
        assert!(playback.world_matrix().is_none());
        let terminal = playback.motion.position;
        assert!(
            !playback
                .advance_static(100., |_| Err("expired body queried terrain"))
                .unwrap()
        );
        assert_eq!(playback.motion.position, terminal);
        playback.reset();
        assert!(!playback.expired);
        assert_eq!(playback.contact.as_ref().unwrap().flags, 0);
        playback
            .request(crate::procedural_aim::Matrix::identity())
            .unwrap();
        playback
            .advance_static(0.01, |_| Err("first callback queried terrain"))
            .unwrap();
        assert!(
            playback
                .advance_static(0.01, |_| Err("terrain unavailable"))
                .is_err()
        );
    }
    #[test]
    fn finite_source_lifetime_stops_flight_without_faking_retirement() {
        let mut bytes = vec![3];
        bytes.extend(900u32.to_le_bytes());
        bytes.extend([6, 0x11]);
        bytes.extend(90f32.to_le_bytes());
        bytes.push(4);
        for v in [0.03f32, 0., 0.03] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.push(0x20);
        for v in [0.4f32, 0.8, 0.9, 0.3, 0.7, 0.8] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.push(0x22);
        for v in [1f32, 0.001] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.push(0x25);
        bytes.extend(1.2f32.to_le_bytes());
        bytes.push(0x26);
        bytes.extend(0.0001f32.to_le_bytes());
        bytes.push(0xd);
        bytes.extend(b"pending_retirement\0");
        bytes.extend([7, 2]);
        let recipe =
            super::Recipe::project(&giants_formats::effect_program::parse(&bytes).unwrap())
                .unwrap();
        assert!(recipe.tracer.is_some());
        assert_eq!(recipe.retirement_child, "pending_retirement");
        let mut p = super::Playback::new(recipe);
        p.request(crate::procedural_aim::Matrix::identity())
            .unwrap();
        p.advance_free(0.01).unwrap();
        assert_eq!(p.motion.position, [0.; 3]);
        assert_eq!(p.visual.as_ref().unwrap().age, 0.);
        p.advance_free(0.01).unwrap();
        let last = p.motion.position;
        assert!(last[1] < 0.);
        p.advance_free(0.02).unwrap();
        assert!(p.expired);
        assert!(p.world_matrix().is_none());
        assert_eq!(p.motion.position, last);
        assert!(!p.advance_free(1.).unwrap());
        assert_eq!(p.motion.position, last);
        p.reset();
        assert!(!p.expired);
        assert!(p.visual.is_none());
        assert!(p.matrix.is_none());
        p.request(crate::procedural_aim::Matrix::identity())
            .unwrap();
        p.advance_free(0.01).unwrap();
        assert!(!p.expired);
    }
    #[test]
    fn forward_axis_preserves_speed_sign_and_small_direction_cutoff() {
        let mut m = crate::procedural_aim::Matrix::identity();
        assert_eq!(super::initial_velocity(m, 100.).unwrap(), [-0., -100., -0.]);
        assert_eq!(super::initial_velocity(m, -100.).unwrap(), [0., 100., 0.]);
        m.values[5] = 0.001;
        assert_eq!(super::initial_velocity(m, 100.).unwrap(), [0.; 3]);
        assert!(super::initial_velocity(m, f32::NAN).is_err());
    }
    #[test]
    fn source_services_survive_projection_and_detachment_has_no_timeout() {
        let mut bytes = vec![3];
        bytes.extend(900u32.to_le_bytes());
        bytes.extend([6, 0x11]);
        bytes.extend(100f32.to_le_bytes());
        bytes.push(0x0e);
        bytes.extend(2f32.to_le_bytes());
        bytes.push(0x0a);
        bytes.extend(2u32.to_le_bytes());
        bytes.extend(b"synthetic_trail\0");
        bytes.extend(0u32.to_le_bytes());
        bytes.extend([0x15, 0x0d]);
        bytes.extend(b"synthetic_impact\0");
        bytes.extend([7, 2]);
        let program = giants_formats::effect_program::parse(&bytes).unwrap();
        let recipe = super::Recipe::project(&program).unwrap();
        assert_eq!(recipe.attached_children[0].reference, 2);
        assert_eq!(recipe.attached_children[0].name, "synthetic_trail");
        assert_eq!(recipe.retirement_child, "synthetic_impact");
        let mut playback = super::Playback::new(recipe);
        let birth = crate::procedural_aim::Matrix::identity();
        assert!(!playback.advance_free(0.01).unwrap());
        playback.request(birth).unwrap();
        assert!(playback.request(birth).is_err());
        assert!(!playback.advance_free(0.).unwrap());
        assert_eq!(playback.callbacks, 0);
        playback.advance_free(0.01).unwrap();
        assert_eq!(playback.motion.position, [0.; 3]);
        playback.advance_free(0.01).unwrap();
        assert!(playback.motion.position[1] < 0.);
        assert_eq!(playback.matrix.unwrap().values, birth.values);
        for _ in 0..200 {
            playback.advance_free(0.01).unwrap();
        }
        assert!(playback.world_matrix().is_some());
        playback.reset();
        assert!(playback.world_matrix().is_none());
        assert_eq!(playback.callbacks, 0);
        let mut bad = program;
        bad.records[3].opcode = 4;
        assert!(super::Recipe::project(&bad).is_err());
    }
}
