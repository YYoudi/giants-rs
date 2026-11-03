//! Collision-consumed fields across personality then active-data copies.
//! This is not whole-object construction, resource duplication or callbacks.
#[derive(Clone, Copy, Debug)]
pub struct Fields {
    pub class: u32,
    /// Independent resource identity, not a native address.
    pub model: u32,
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub scale: f32,
    pub angles: [f32; 3],
    pub object_flags: u32,
    pub response_flags: u32,
    pub collision_flags: u32,
    pub ground_offset: f32,
    pub center_offset: f32,
    pub height: f32,
    pub packed_parameters: u32,
    pub radius_limit: f32,
    pub radius_override: f32,
    pub radius: f32,
    pub floor: f32,
    pub plane: [f32; 4],
    pub bounds: [f32; 6],
    pub model_vertical_speed: f32,
    pub velocity: [f32; 3],
    pub hit: u32,
    pub support: u32,
    pub physical: bool,
    pub animation_slot: u32,
}
impl Fields {
    pub fn zeroed() -> Self {
        Self::from_words(&[0; 41]).expect("fixed collider field layout")
    }
    /// Cold class command projection for the bounded rigid-field profile.
    /// Resource binding and state payloads remain independent. No callback or
    /// post-command initialization is represented by this method.
    pub fn cold_declaration(
        resources: &crate::scene_resources::SceneResources,
        class: i32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let definition = resources
            .classes
            .models
            .first_program(class)
            .ok_or("collider class absent")?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let binding = resources
            .classes
            .models
            .first_selection(class)
            .ok_or("collider spec absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let mut out = Self::zeroed();
        out.class = class as u32;
        out.scale = program.declared_scale()?;
        out.animation_slot = 254;
        out.ground_offset = program.declared_ground_offset()?;
        let flags = crate::object_lists::DirectFlags::project(class, program);
        out.object_flags = flags.active;
        out.collision_flags = flags.personality;
        out.center_offset = 1.;
        out.height = 1.;
        for record in &program.records {
            let word = |at| -> Result<u32, &'static str> {
                Ok(u32::from_le_bytes(
                    record
                        .bytes
                        .get(at..at + 4)
                        .ok_or("collider declaration word")?
                        .try_into()
                        .unwrap(),
                ))
            };
            match record.opcode {
                1 => {
                    if word(1)? != binding.spec_id {
                        return Err("collider rebind requires ordered resource preparation".into());
                    }
                    out.model = binding.spec_id;
                    if let Some((_, mesh)) = resources.model_mesh_slot(class, 0)? {
                        out.height = mesh.bounds()?[5].max(1.);
                        out.center_offset = out.height * 0.8;
                    }
                }
                0x21 => {
                    out.center_offset = f32::from_bits(word(1)?);
                    out.height = f32::from_bits(word(5)?);
                }
                0x26 => {
                    out.packed_parameters =
                        (out.packed_parameters & 0x00ff_ffff) | (u32::from(record.bytes[1]) << 24);
                }
                0x36 => {
                    out.packed_parameters =
                        (out.packed_parameters & 0xff00_ffff) | (u32::from(record.bytes[1]) << 16)
                }
                0x39 => {
                    out.packed_parameters =
                        (out.packed_parameters & 0xffff_00ff) | ((word(1)? & 0xff) << 8)
                }
                0x13 => out.animation_slot = u32::from(record.bytes[1]),
                0x17 | 0x1b | 0x1d | 0x1e | 0x23 | 0x25 | 0x2a | 0x31 | 0x4f | 0x50 | 0xff => {}
                _ => return Err("cold collider command outside checked field profile".into()),
            }
        }
        let radius =
            crate::collision_radius::project(crate::collision_radius::Fields::cold(), program)?;
        out.radius_limit = radius.limit;
        out.radius_override = radius.radius_override;
        Ok(out)
    }
    /// Copy only this collision projection of the two original operations.
    /// Radius, floor/plane, bounds, velocities, hit/support and physics presence
    /// retain the destination's values. Later preparation must establish them.
    pub fn copy_from(&mut self, source: &Self) {
        self.class = source.class;
        self.animation_slot = source.animation_slot;
        self.model = source.model;
        self.current = source.current;
        self.previous = source.previous;
        self.scale = source.scale;
        self.angles = source.angles;
        self.object_flags = source.object_flags;
        self.response_flags = source.response_flags | 0x8000_0000;
        self.collision_flags = source.collision_flags;
        self.ground_offset = source.ground_offset;
        self.center_offset = source.center_offset;
        self.height = source.height;
        self.packed_parameters = source.packed_parameters;
        self.radius_limit = source.radius_limit;
        self.radius_override = source.radius_override;
    }
    /// Bit-preserving control serialization, independent of the native header.
    pub fn words(self) -> Vec<u32> {
        let mut w = vec![self.class, self.model];
        w.extend(
            self.current
                .into_iter()
                .chain(self.previous)
                .chain([self.scale])
                .chain(self.angles)
                .map(f32::to_bits),
        );
        w.extend([self.object_flags, self.response_flags, self.collision_flags]);
        w.extend([self.ground_offset, self.center_offset, self.height].map(f32::to_bits));
        w.push(self.packed_parameters);
        w.extend(
            [
                self.radius_limit,
                self.radius_override,
                self.radius,
                self.floor,
            ]
            .map(f32::to_bits),
        );
        w.extend(
            self.plane
                .into_iter()
                .chain(self.bounds)
                .chain([self.model_vertical_speed])
                .chain(self.velocity)
                .map(f32::to_bits),
        );
        w.extend([
            self.hit,
            self.support,
            u32::from(self.physical),
            self.animation_slot,
        ]);
        w
    }
    pub fn from_words(w: &[u32]) -> Result<Self, &'static str> {
        if w.len() != 41 {
            return Err("collider field control length");
        }
        let f = |i| f32::from_bits(w[i]);
        Ok(Self {
            class: w[0],
            model: w[1],
            current: [f(2), f(3), f(4)],
            previous: [f(5), f(6), f(7)],
            scale: f(8),
            angles: [f(9), f(10), f(11)],
            object_flags: w[12],
            response_flags: w[13],
            collision_flags: w[14],
            ground_offset: f(15),
            center_offset: f(16),
            height: f(17),
            packed_parameters: w[18],
            radius_limit: f(19),
            radius_override: f(20),
            radius: f(21),
            floor: f(22),
            plane: [f(23), f(24), f(25), f(26)],
            bounds: [f(27), f(28), f(29), f(30), f(31), f(32)],
            model_vertical_speed: f(33),
            velocity: [f(34), f(35), f(36)],
            hit: w[37],
            support: w[38],
            physical: w[39] != 0,
            animation_slot: w[40],
        })
    }
}
