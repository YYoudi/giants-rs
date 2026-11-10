//! Acquired pistol draw action table, shared by the registered Mecc class.
//! Selection, ANM scheduling and held-object ownership belong to the caller.
use crate::animation_event_dispatch::{Context, Request, State, Window};

/// Keep this with the class event bank, not a fresh copy for each actor/frame.
/// Native selection patches the weapon arguments without resetting event marks.
#[derive(Clone, Debug)]
pub struct Actions {
    events: [State; 3],
    failed: bool,
}
impl Actions {
    pub fn new(catalog: &giants_formats::weapons::Catalog) -> Result<Self, String> {
        let event = |kind, time| State {
            source_offset: 0,
            kind,
            time,
            parameters: [0; 6],
        };
        let mut result = Self {
            // Retain table order: the later equip action precedes the earlier
            // remove action when a supplied interval crosses both at once.
            events: [event(0, 0.), event(0x20, 23. / 30.), event(0x4020, 0.2)],
            failed: false,
        };
        result.bind(catalog)?;
        Ok(result)
    }
    pub fn bind(&mut self, catalog: &giants_formats::weapons::Catalog) -> Result<(), String> {
        if self.failed {
            return Err("pistol draw actions are quarantined".into());
        }
        let left = crate::held_weapon::wield_class(catalog, 1, false)?;
        let right = crate::held_weapon::wield_class(catalog, 1, true)?;
        if left != 0 || right != 21 {
            return Err("pistol draw outside acquired weapon classes".into());
        }
        self.events[0].parameters[0] = left as u32;
        self.events[1].parameters[0] = right as u32;
        Ok(())
    }
    pub fn event_kinds(&self) -> [u32; 3] {
        self.events.each_ref().map(|event| event.kind)
    }
    /// `context.material == None` must mean an observed absence of material,
    /// not a missing producer. Wet/material selection still affects arguments.
    /// Execute each replacement immediately in table order, preserving completed
    /// effects and event marks on failure. The provider must own the actual actor.
    pub fn dispatch(
        &mut self,
        window: Window,
        context: &Context<'_>,
        mut replace: impl FnMut(u8, i32) -> Result<(), String>,
    ) -> Result<(), String> {
        if self.failed {
            return Err("pistol draw actions are quarantined".into());
        }
        let result = (|| {
            for event in &mut self.events {
                for request in event
                    .dispatch(window, context)
                    .map_err(|e| e.to_string())?
                    .requests
                {
                    let Request::Auxiliary { channel, selector } = request else {
                        return Err("pistol draw action has an unsupported service".into());
                    };
                    replace(channel, selector as i32)?;
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_replacement_retains_marks_and_prevents_replay() {
        let mut actions = Actions {
            events: std::array::from_fn(|i| State {
                source_offset: 0,
                kind: 0x20,
                time: 0.2,
                parameters: [i as u32; 6],
            }),
            failed: false,
        };
        let context = Context {
            wet: false,
            material: None,
            position: [0.; 3],
            world: None,
            references: &[],
            contact_scalar: 0.,
            cinematic: false,
        };
        let window = Window {
            previous: 0.,
            current: 1.,
            reverse: false,
            wrapped: true,
        };
        let mut completed = Vec::new();
        assert!(
            actions
                .dispatch(window, &context, |_, class| {
                    if class == 1 {
                        return Err("actual replacement failed".into());
                    }
                    completed.push(class);
                    Ok(())
                })
                .is_err()
        );
        assert_eq!(completed, [0]);
        assert_eq!(actions.event_kinds(), [0x8020, 0x8020, 0x20]);
        assert!(
            actions
                .dispatch(window, &context, |_, _| panic!("replayed"))
                .is_err()
        );
    }
}
