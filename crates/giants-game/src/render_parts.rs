//! General model-part routing after model selection. A deferred part is not
//! necessarily alpha blended by the GPU; raster/material setup is separate.
pub const QUEUE_CAPACITY: usize = 1000;

#[derive(Clone, Copy, Debug)]
pub struct Object {
    pub opacity: f32,
    pub personality: u32,
    pub secondary_flags: u32,
    pub node_filter: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Part {
    /// Native material scalar at +0x98, sourced from GBS `falloff` in the
    /// observed unmodified models. It is separate from the GBS `blend` scalar.
    pub routing_value: f32,
    pub has_texture: bool,
    /// Prepared descriptor format, after texture preparation has completed.
    pub texture_format: Option<i32>,
    pub hidden: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    Opaque,
    ObjectOrMaterial,
    TextureFormat,
}
impl Pass {
    pub fn mask(self) -> u8 {
        match self {
            Self::Opaque => 1,
            Self::ObjectOrMaterial => 2,
            Self::TextureFormat => 4,
        }
    }
}
/// This is the prepared descriptor's internal format, not TGA/DDS flags or a
/// graphics API format number. Negative formats are outside the checked domain.
pub fn format_requires_alpha(format: i32) -> Result<bool, &'static str> {
    if format < 0 {
        return Err("negative prepared texture format");
    }
    Ok(matches!(format, 0 | 3 | 4 | 6))
}
pub fn part_pass(object: Object, routing_value: f32, texture_alpha: bool) -> Pass {
    let pass = material_pass(routing_value, texture_alpha);
    if pass == Pass::Opaque
        && (object.opacity != 0.
            || object.personality & 0x800000 != 0
            || object.secondary_flags & 0x40000 != 0)
    {
        Pass::ObjectOrMaterial
    } else {
        pass
    }
}
/// Material-only category. The caller must establish the absence of object
/// modifiers before using this as a complete part category.
pub fn material_pass(routing_value: f32, texture_alpha: bool) -> Pass {
    if texture_alpha {
        Pass::TextureFormat
    } else if routing_value != 0. {
        Pass::ObjectOrMaterial
    } else {
        Pass::Opaque
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Query {
    NodeVisibility(usize),
    PreparedTextureFormat(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Need(Query),
    Unsupported(&'static str),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Queue { entry: usize, part: i32 },
    Draw { mask: u8, part: i32 },
    Special,
    Overflow,
}
#[derive(Debug)]
pub struct Plan {
    pub passes: Vec<Option<Pass>>,
    pub actions: Vec<Action>,
    pub queue_len: usize,
}
/// Plan over resolved snapshots only. No callback, texture allocation, query,
/// queue mutation or GPU call occurs here. Missing answers remain explicit.
pub fn plan(object: Object, parts: &[Part], queue_len: usize) -> Result<Plan, Error> {
    if parts.len() > i32::MAX as usize || queue_len > i32::MAX as usize {
        return Err(Error::Unsupported(
            "part or queue count outside signed native domain",
        ));
    }
    let mut result = Plan {
        passes: Vec::with_capacity(parts.len()),
        actions: Vec::new(),
        queue_len,
    };
    let mut opaque = false;
    for (index, part) in parts.iter().enumerate() {
        if object.node_filter {
            match part.hidden {
                None => return Err(Error::Need(Query::NodeVisibility(index))),
                Some(true) => {
                    result.passes.push(None);
                    continue;
                }
                Some(false) => {}
            }
        }
        let texture_alpha = if part.has_texture {
            format_requires_alpha(
                part.texture_format
                    .ok_or(Error::Need(Query::PreparedTextureFormat(index)))?,
            )
            .map_err(Error::Unsupported)?
        } else {
            false
        };
        let pass = part_pass(object, part.routing_value, texture_alpha);
        result.passes.push(Some(pass));
        if pass == Pass::Opaque {
            opaque = true;
        } else {
            result.defer(object.secondary_flags, index as i32);
        }
    }
    if object.secondary_flags & 0x2700 != 0 {
        result.defer(object.secondary_flags, -1);
    }
    if opaque {
        result.actions.push(Action::Draw { mask: 1, part: -1 });
    }
    Ok(result)
}
impl Plan {
    fn defer(&mut self, flags: u32, part: i32) {
        if flags & 0x200000 == 0 && self.queue_len < QUEUE_CAPACITY {
            self.actions.push(Action::Queue {
                entry: self.queue_len,
                part,
            });
            self.queue_len += 1;
        } else {
            self.actions.push(if part == -1 {
                Action::Special
            } else {
                Action::Draw { mask: 6, part }
            });
            if flags & 0x200000 == 0 {
                self.actions.push(Action::Overflow);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_saturation_keeps_later_parts_special_and_opaque_actions_in_order() {
        let object = Object {
            opacity: 0.,
            personality: 0,
            secondary_flags: 0x100,
            node_filter: false,
        };
        let parts = [0., 1., 1.].map(|routing_value| Part {
            routing_value,
            has_texture: false,
            texture_format: None,
            hidden: None,
        });
        let p = plan(object, &parts, 999).unwrap();
        assert_eq!(p.queue_len, 1000);
        assert_eq!(
            p.actions,
            vec![
                Action::Queue {
                    entry: 999,
                    part: 1
                },
                Action::Draw { mask: 6, part: 2 },
                Action::Overflow,
                Action::Special,
                Action::Overflow,
                Action::Draw { mask: 1, part: -1 }
            ]
        );
    }
    #[test]
    fn unknown_required_queries_stay_explicit_and_unused_answers_are_ignored() {
        let mut object = Object {
            opacity: 0.,
            personality: 0,
            secondary_flags: 0,
            node_filter: true,
        };
        let mut part = Part {
            routing_value: 1.,
            has_texture: true,
            texture_format: None,
            hidden: None,
        };
        assert_eq!(
            plan(object, &[part], 0).unwrap_err(),
            Error::Need(Query::NodeVisibility(0))
        );
        part.hidden = Some(true);
        assert!(plan(object, &[part], 0).unwrap().actions.is_empty());
        object.node_filter = false;
        assert_eq!(
            plan(object, &[part], 0).unwrap_err(),
            Error::Need(Query::PreparedTextureFormat(0))
        );
        part.has_texture = false;
        part.routing_value = f32::NAN;
        assert_eq!(
            plan(object, &[part], 0).unwrap().passes,
            vec![Some(Pass::ObjectOrMaterial)]
        );
        part.has_texture = true;
        part.texture_format = Some(-1);
        assert_eq!(
            plan(object, &[part], 0).unwrap_err(),
            Error::Unsupported("negative prepared texture format")
        );
    }
}
