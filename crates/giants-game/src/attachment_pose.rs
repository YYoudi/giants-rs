//! Explicit initial-field camera-facing inspection. Live callbacks, visibility
//! and subsequent field writers are separate from the source binding matrix.
#[derive(Clone, Copy, Debug)]
pub struct InitialFacing {
    pub personality: u32,
    pub scale: f32,
}
impl InitialFacing {
    /// Initial fields of a standalone single-loop actor. This does
    /// not admit state callbacks, director channels or later field writers.
    pub fn project_constructed(
        program: &giants_formats::class_program::Program,
        actor: &crate::actor_context::Prepared,
        has_animation: bool,
    ) -> Result<Option<Self>, String> {
        let class = &actor.class;
        let ordinary = Self::project(class.declaration_class, program, has_animation);
        if ordinary.is_ok() {
            return ordinary.map_err(str::to_owned);
        }
        let initial = crate::actor_context::Prepared::class_initial(class.clone());
        if !has_animation
            || class.state != 0
            || class.declared_loop.is_none()
            || actor.inline.is_some()
            || actor.director_slot.is_some()
            || actor.channels != initial.channels
            || !actor.needs.is_empty()
            || program.records.iter().any(|r| {
                !matches!(
                    r.opcode,
                    1 | 0x13 | 0x14 | 0x16 | 0x1d | 0x22 | 0x36 | 0x17 | 0xff
                )
            })
        {
            return Err("camera-facing actor outside initial single-loop profile".into());
        }
        let specification = class
            .specification
            .as_ref()
            .ok_or("camera-facing actor specification absent")?;
        if specification
            .models
            .first()
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err("camera-facing actor model zero absent".into());
        }
        crate::effect_animation::Playback::new(initial)?
            .ok_or("camera-facing declared loop absent")?;
        let flags = crate::object_lists::DirectFlags::project(class.declaration_class, program);
        let scale = program.declared_scale().map_err(|e| e.to_string())?;
        if flags.personality != class.flags.personality
            || flags.active != class.flags.active
            || scale.to_bits() != class.scale.to_bits()
        {
            return Err("camera-facing class construction identity differs".into());
        }
        Ok(Some(Self {
            personality: flags.personality,
            scale,
        }))
    }
    pub fn project(
        class: i32,
        program: &giants_formats::class_program::Program,
        has_animation: bool,
    ) -> Result<Option<Self>, &'static str> {
        let flags = crate::object_lists::DirectFlags::project(class, program);
        if flags.personality & 0x84 == 0 {
            return Ok(None);
        }
        if has_animation
            || program
                .records
                .iter()
                .any(|r| !matches!(r.opcode, 1 | 0x14 | 0x16 | 0x1d | 0x22 | 0x36 | 0x17 | 0xff))
        {
            return Err("camera-facing initial fields outside checked direct-field capability");
        }
        Ok(Some(Self {
            personality: flags.personality,
            scale: program
                .declared_scale()
                .map_err(|_| "facing scale unavailable")?,
        }))
    }
    pub fn draw_matrix(
        self,
        binding: [f32; 16],
        view: [f32; 16],
    ) -> Result<[f32; 16], &'static str> {
        crate::camera_facing::draw_matrix(
            binding,
            self.personality,
            crate::camera_facing::Input {
                view,
                position: [binding[3], binding[7], binding[11]],
                scale: self.scale,
                angle: 0.,
            },
            crate::arithmetic::Precision::Double53,
        )
    }
    /// Wrapper mode1 reaches D3DCULL_CW (Bevy back-face culling). Personality4
    /// forces this mode even when the cached matrix carries the mirror bit.
    pub fn front(self, matrix_flags: u32) -> bool {
        self.personality & 4 == 0 && matrix_flags & 8 != 0
    }
}
