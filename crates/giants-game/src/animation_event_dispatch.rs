//! Event-consumer decisions and requests, with explicit clock/material/pose inputs.
//! Requests do not execute contact services, audio or an effect allocator.
use crate::animation_events::Event;
use giants_formats::Error;

#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub previous: f32,
    pub current: f32,
    pub reverse: bool,
    pub wrapped: bool,
}
impl Window {
    /// Classify the supplied interval using half the declared duration. This
    /// does not advance an animation or choose the clock which supplies times.
    pub fn classify(previous: f32, current: f32, duration: f32) -> Result<Self, Error> {
        if !previous.is_finite() || !current.is_finite() || !duration.is_finite() || duration <= 0.
        {
            return Err(Error("invalid event interval"));
        }
        let delta = f64::from(current) - f64::from(previous);
        let wrapped = delta.abs() > f64::from(duration) * 0.5;
        Ok(Self {
            previous,
            current,
            reverse: (delta < 0.) ^ wrapped,
            wrapped,
        })
    }
    fn crosses(self, time: f32) -> bool {
        match (self.reverse, self.wrapped) {
            (false, false) => self.previous < time && self.current >= time,
            (true, false) => self.previous > time && self.current <= time,
            (false, true) => self.previous < time || self.current >= time,
            (true, true) => self.previous > time || self.current <= time,
        }
    }
}

pub struct Context<'a> {
    pub wet: bool,
    pub material: Option<&'a str>,
    pub position: [f32; 3],
    pub world: [f32; 12],
    pub references: &'a [[f32; 3]],
    pub contact_scalar: f32,
    pub cinematic: bool,
}
#[derive(Debug)]
pub enum Anchor {
    Position([f32; 3]),
    OwnerReference(i32),
}
#[derive(Debug)]
pub enum Request {
    Primary(u32),
    ContactBegin(bool),
    ContactFinish {
        kind: u32,
        scalar: f32,
    },
    Auxiliary {
        channel: u8,
        selector: u32,
    },
    Effect {
        flags: u32,
        target: Option<u32>,
        anchor: Anchor,
    },
}
#[derive(Debug)]
pub struct Dispatch {
    pub fired: bool,
    pub choice: Option<usize>,
    pub requests: Vec<Request>,
    pub configuration_override: Option<[f32; 2]>,
}
#[derive(Clone, Debug)]
pub struct State {
    pub source_offset: usize,
    pub kind: u32,
    pub time: f32,
    pub parameters: [u32; 6],
}
impl From<&Event> for State {
    fn from(event: &Event) -> Self {
        Self {
            source_offset: event.source_offset,
            kind: event.kind,
            time: event.time,
            parameters: event.parameters,
        }
    }
}

fn choice(context: &Context<'_>) -> usize {
    if context.wet {
        return 1;
    }
    let Some(name) = context.material else {
        return 0;
    };
    if ["B_smartbrick", "D_B_smartbrick", "s_stony2", "s_stony"]
        .iter()
        .any(|s| name.eq_ignore_ascii_case(s))
    {
        3
    } else if ["Wood_tile", "Wood_rings", "sm_thatch"]
        .iter()
        .any(|s| name.eq_ignore_ascii_case(s))
    {
        2
    } else if [
        "Metal",
        "D_metal",
        "rusted_metal",
        "Rusted_metal_stripe",
        "floor1_tile",
    ]
    .iter()
    .any(|s| name.eq_ignore_ascii_case(s))
    {
        4
    } else {
        0
    }
}

fn reference_position(index: i32, context: &Context<'_>) -> [f32; 3] {
    let local = if index as u8 & 0x80 != 0 {
        [0.; 3]
    } else if index < 0 || index as usize >= context.references.len() {
        return context.position;
    } else {
        context.references[index as usize]
    };
    std::array::from_fn(|r| {
        let m = &context.world[r * 4..];
        (((f64::from(m[2]) * f64::from(local[2]) + f64::from(m[0]) * f64::from(local[0]))
            + f64::from(m[1]) * f64::from(local[1]))
            + f64::from(m[3])) as f32
    })
}

impl State {
    pub fn dispatch(&mut self, window: Window, context: &Context<'_>) -> Result<Dispatch, Error> {
        if !self.time.is_finite()
            || !window.previous.is_finite()
            || !window.current.is_finite()
            || context.material.is_some_and(|s| !s.is_ascii())
            || !context
                .position
                .iter()
                .chain(&context.world)
                .chain(context.references.iter().flatten())
                .all(|v| v.is_finite())
            || !context.contact_scalar.is_finite()
        {
            return Err(Error("unsupported event dispatch input"));
        }
        let before = self.kind;
        if window.wrapped {
            self.kind ^= 0x8000;
        }
        let mut result = Dispatch {
            fired: false,
            choice: None,
            requests: Vec::new(),
            configuration_override: None,
        };
        if (before & 2 != 0 && before & 0x8000 != 0)
            || (before & 2 == 0 && before & 4 != 0 && before & 0x8000 == 0)
            || !window.crosses(self.time)
        {
            return Ok(result);
        }
        result.fired = true;
        let choice = choice(context);
        result.choice = Some(choice);
        let selector = self.parameters[choice];
        if before & 1 != 0 {
            result.requests.push(Request::Primary(selector));
        }
        if before & 0x10008 != 0 {
            if context.cinematic {
                result.configuration_override = Some([5., 2.]);
            }
            let alternate = before & 0x10000 != 0;
            result.requests.push(Request::ContactBegin(alternate));
            result.requests.push(Request::ContactFinish {
                kind: if alternate { 9 } else { 8 },
                scalar: context.contact_scalar,
            });
        }
        for (mask, channel) in [(0x10, 0), (0x20, 1)] {
            if before & mask != 0 {
                result
                    .requests
                    .push(Request::Auxiliary { channel, selector });
            }
        }
        let target = (selector != u32::MAX).then_some(selector);
        if before & 0x40 != 0 {
            result.requests.push(Request::Effect {
                flags: 0x18,
                target,
                anchor: Anchor::Position(reference_position(self.parameters[5] as i32, context)),
            });
        }
        if before & 0x80 != 0 {
            result.requests.push(Request::Effect {
                flags: 0x14,
                target,
                anchor: Anchor::OwnerReference(self.parameters[5] as i32),
            });
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_intervals_are_not_replaced_with_a_default_clock() {
        assert!(Window::classify(0., 1., 0.).is_err());
        assert!(Window::classify(f32::NAN, 1., 2.).is_err());
        let w = Window::classify(0., 1., 2.).unwrap();
        assert!(!w.wrapped && !w.reverse);
        let w = Window::classify(1.9, 0.1, 2.).unwrap();
        assert!(w.wrapped && !w.reverse);
    }
}
