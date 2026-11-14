//! Static terrain contact for source-requested state78 kind6, kind7 and bounded kind8 bodies.
//! Object volumes, world forces, sound and impact services remain upstream.
use crate::{arithmetic::Precision, physical_ground};

#[derive(Clone, Copy, Debug)]
pub struct Seed {
    kind: u8,
    class_scale: f32,
    ground_offset: f32,
}
impl Seed {
    pub fn from_class(
        program: &giants_formats::class_program::Program,
    ) -> Result<Self, &'static str> {
        Self::from_kind(program, 6)
    }
    pub fn from_projectile_class(
        program: &giants_formats::class_program::Program,
    ) -> Result<Self, &'static str> {
        Self::from_kind(program, 7)
    }
    /// Positional rigid debris: the source kind8 constructor uses the same
    /// physical defaults. Other scale/attachment profiles remain unbound.
    pub fn from_rigid_class(
        program: &giants_formats::class_program::Program,
    ) -> Result<Self, &'static str> {
        let seed = Self::from_kind(program, 8)?;
        if seed.class_scale != 1. {
            return Err("rigid contact inherited scale profile unavailable");
        }
        Ok(seed)
    }
    pub fn rigid(self) -> bool {
        self.kind == 8
    }
    fn from_kind(
        program: &giants_formats::class_program::Program,
        kind: u8,
    ) -> Result<Self, &'static str> {
        if program.records.iter().any(|r| {
            !matches!(
                r.opcode,
                1 | 0x13 | 0x17 | 0x1b | 0x1d | 0x1e | 0x21 | 0x22 | 0x36 | 0xff
            )
        }) || crate::state_storage::Seed::from_program(program)
            .map_err(|_| "effect contact class state unavailable")?
            .storage
            .state()
            != 78
            || program
                .records
                .iter()
                .rev()
                .find(|r| r.opcode == 0x36)
                .and_then(|r| r.bytes.get(1))
                != Some(&kind)
        {
            return Err("effect contact class outside requested state78 collision profile");
        }
        Ok(Self {
            kind,
            class_scale: program
                .declared_scale()
                .map_err(|_| "effect contact class scale unavailable")?,
            ground_offset: program
                .declared_ground_offset()
                .map_err(|_| "effect contact ground offset unavailable")?,
        })
    }
    pub fn instantiate(self, spawn: &crate::effect_burst::Spawn) -> Result<State, &'static str> {
        if !matches!(self.kind, 6 | 8)
            || (self.kind == 8 && !matches!(spawn.attribute, None | Some(1)))
            || !spawn.contact_requested
            || !spawn.physics_enabled
        {
            return Err("effect contact requires source request and movement gates");
        }
        if !spawn.size_configured && spawn.visual.start_size.to_bits() != self.class_scale.to_bits()
        {
            return Err("effect contact inherited class scale outside supplied burst profile");
        }
        Ok(self.state())
    }
    pub fn instantiate_projectile(
        self,
        recipe: &crate::effect_projectile::Recipe,
    ) -> Result<State, &'static str> {
        if self.kind != 7 || !recipe.contact_requested || self.class_scale != 1. {
            return Err("projectile contact requires requested unit state78/kind7 profile");
        }
        Ok(self.state())
    }
    fn state(self) -> State {
        State {
            seed: self,
            flags: 0,
            floor: 0.,
            plane: [0.; 4],
            steps: 0,
            responses: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct State {
    seed: Seed,
    pub flags: u32,
    pub floor: f32,
    pub plane: [f32; 4],
    pub steps: u64,
    pub responses: u64,
}
impl State {
    pub fn reset(&mut self) {
        *self = self.seed.state();
    }
    pub fn resolve(
        &mut self,
        motion: &mut crate::mecc_motion::Motion,
        previous: [f32; 3],
        scale: f32,
        spawn: &crate::effect_burst::Spawn,
        query: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
    ) -> Result<(), &'static str> {
        self.resolve_motion(
            motion,
            previous,
            scale,
            spawn.gravity_mass,
            spawn.bounce,
            query,
        )
    }
    pub fn resolve_projectile(
        &mut self,
        motion: &mut crate::mecc_motion::Motion,
        previous: [f32; 3],
        recipe: &crate::effect_projectile::Recipe,
        query: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
    ) -> Result<(), &'static str> {
        self.resolve_motion(motion, previous, 1., recipe.gravity_mass, 0., query)
    }
    fn resolve_motion(
        &mut self,
        motion: &mut crate::mecc_motion::Motion,
        previous: [f32; 3],
        scale: f32,
        gravity_mass: f32,
        restitution: f32,
        query: impl FnMut([f32; 2]) -> Result<physical_ground::Ground, &'static str>,
    ) -> Result<(), &'static str> {
        let out = physical_ground::resolve(
            physical_ground::Input {
                position: motion.position,
                previous,
                velocity: motion.velocity,
                scale,
                ground_offset: self.seed.ground_offset,
                // The effect constructor overrides the template's centre field.
                center_offset: 50.,
                gravity_mass,
                restitution,
                retention: 1.,
                force_z: motion.force[2],
                flags: self.flags,
                floor: self.floor,
                plane: self.plane,
            },
            Precision::Double53,
            query,
        )?;
        motion.position = out.position;
        motion.velocity = out.velocity;
        motion.force[2] = out.force_z;
        self.flags = out.flags;
        self.floor = out.floor;
        self.plane = out.plane;
        self.steps += 1;
        self.responses += u64::from(out.flags & 0x44 != 0);
        Ok(())
    }
}
