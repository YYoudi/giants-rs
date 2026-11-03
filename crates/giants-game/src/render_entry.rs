//! Main object-list renderer entry after upstream transforms, before actual draw
//! preparation. Query answers are explicit, never inferred from source placement.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub has_spec: bool,
    pub active_flags: u32,
    pub opacity_bits: u32,
    pub local_player: bool,
    pub draw_local_player: bool,
    pub state: u32,
    pub focal_self: bool,
    pub focal_parent: bool,
    pub focal_owner: bool,
    pub personality_flags: u32,
    pub auxiliary_disabled: bool,
}
#[derive(Clone, Copy, Debug)]
/// Selected model presence from LOD selection. Prior culling flags stay in Input.
pub struct RangeAnswer {
    pub selected_model_present: bool,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Answers {
    pub effect_suppressed: Option<bool>,
    pub range: Option<RangeAnswer>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Query {
    Effect,
    Range,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skip {
    MissingSpec,
    Inactive,
    OpacityOne,
    LocalPlayer,
    Effect,
    RangeHidden,
    MissingModel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Related,
    General { auxiliary: bool },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Need(Query),
    Skip(Skip),
    Dispatch(Route),
}
/// Re-evaluate after supplying a requested answer. A dispatch means the native
/// entry wrapper is reached, not that any submesh is visible or submitted.
pub fn evaluate(input: Input, answers: Answers) -> Decision {
    if !input.has_spec {
        return Decision::Skip(Skip::MissingSpec);
    }
    if input.active_flags & 0x4000c000 != 0 {
        return Decision::Skip(Skip::Inactive);
    }
    if input.opacity_bits == 0x3f800000 {
        return Decision::Skip(Skip::OpacityOne);
    }
    if input.local_player && !input.draw_local_player {
        return Decision::Skip(Skip::LocalPlayer);
    }
    if input.state == 78 {
        match answers.effect_suppressed {
            None => return Decision::Need(Query::Effect),
            Some(true) => return Decision::Skip(Skip::Effect),
            Some(false) => {}
        }
    }
    if input.focal_self || input.focal_parent {
        return Decision::Dispatch(Route::Related);
    }
    if input.state == 90 && input.focal_owner {
        return Decision::Dispatch(Route::Related);
    }
    let Some(range) = answers.range else {
        return Decision::Need(Query::Range);
    };
    if input.active_flags & 0x80 != 0 {
        return Decision::Skip(Skip::RangeHidden);
    }
    if !range.selected_model_present {
        return Decision::Skip(Skip::MissingModel);
    }
    Decision::Dispatch(Route::General {
        auxiliary: input.personality_flags & 0x40000040 == 0
            && input.active_flags & 0x10 == 0
            && !input.auxiliary_disabled,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unresolved_queries_and_exact_opacity_bits_do_not_invent_visibility() {
        let mut input = Input {
            has_spec: true,
            active_flags: 0,
            opacity_bits: 0x3f800001,
            local_player: false,
            draw_local_player: false,
            state: 78,
            focal_self: false,
            focal_parent: false,
            focal_owner: false,
            personality_flags: 0,
            auxiliary_disabled: false,
        };
        assert_eq!(
            evaluate(input, Answers::default()),
            Decision::Need(Query::Effect)
        );
        input.opacity_bits = 0x3f800000;
        assert_eq!(
            evaluate(input, Answers::default()),
            Decision::Skip(Skip::OpacityOne)
        );
        input.opacity_bits = 0;
        input.focal_self = true;
        assert_eq!(
            evaluate(
                input,
                Answers {
                    effect_suppressed: Some(false),
                    ..Default::default()
                }
            ),
            Decision::Dispatch(Route::Related)
        );
    }
}
