//! Initial director allocation and first bound-clock reset. No playback cadence.
#[derive(Debug)]
pub struct Initial {
    pub counts: [u32; 3],
    pub cursor_offset: usize,
    pub seconds: f32,
    pub dt: f32,
    pub finished: bool,
    pub animation_bound: bool,
}
impl Initial {
    pub fn from_header(header: [u32; 6]) -> Result<Self, &'static str> {
        if header[0] != 3
            || !(24..=4 * 1024 * 1024).contains(&header[1])
            || header[2] > 32
            || header[3] > 32
            || header[4] > 4096
        {
            return Err("director header outside supported initialization budgets");
        }
        Ok(Self {
            counts: [header[2], header[3], header[4]],
            cursor_offset: 24,
            seconds: 0.,
            dt: 0.,
            finished: true,
            animation_bound: false,
        })
    }
    /// Prediction for the first clock call after animation binding. This does
    /// not execute binding or mark a director/camera ready. Raw dt is caller input.
    pub fn first_bound_reset(
        &self,
        timeline: [i32; 4],
        raw_dt: f32,
    ) -> Result<(crate::cinematic_clock::Step, i32), &'static str> {
        let step = crate::cinematic_clock::advance(crate::cinematic_clock::Input {
            seconds: self.seconds,
            raw_dt,
            finished: self.finished,
            sync: None,
        })?;
        Ok((
            step,
            crate::model_pose::seconds_tick(timeline, step.seconds)?,
        ))
    }
}

pub struct Prepared {
    pub initial: Initial,
    pub animation: crate::animation::Animation,
    pub dependencies: giants_formats::flick::DependencyMetadata,
    pub program: Result<giants_formats::flick::Program, String>,
    pub effect_requests: Result<Vec<crate::cinematic_fx::Request>, String>,
    pub binding_candidate: Result<CameraBinding, String>,
    /// Candidate at the controlled first bound reset, not a measured visible frame.
    pub reset_tick: i32,
    pub camera_candidate: Result<crate::animation::Camera, String>,
}
#[derive(Debug)]
pub struct CameraBinding {
    pub command_offset: usize,
    pub tree_slot: u32,
    pub tree: Option<String>,
    pub name: String,
    pub node_offset: usize,
}
impl CameraBinding {
    /// First declared camera request; preceding actor callbacks are not executed.
    fn first(
        bytes: &[u8],
        program: &giants_formats::flick::Program,
        animation: &crate::animation::Animation,
    ) -> Result<Self, String> {
        let command = program
            .commands
            .iter()
            .find(|c| c.opcode == 4)
            .ok_or("missing camera request")?;
        Self::at(bytes, program, animation, command.offset)
    }
    /// Resolve this request only. Playback must choose it from actual dispatch,
    /// rather than assuming the first request remains active for the whole film.
    pub fn at(
        bytes: &[u8],
        program: &giants_formats::flick::Program,
        animation: &crate::animation::Animation,
        command_offset: usize,
    ) -> Result<Self, String> {
        let command = program
            .commands
            .iter()
            .find(|c| c.offset == command_offset && c.opcode == 4)
            .ok_or("camera request offset is not command four")?;
        if command.words.len() != 3 || bytes.len() < 40 {
            return Err("truncated camera request/header".into());
        }
        let slot = command.words[0];
        if slot >= u32::from_le_bytes(bytes[8..12].try_into().unwrap()) {
            return Err("camera tree slot outside director declarations".into());
        }
        let declaration = program
            .commands
            .iter()
            .take_while(|c| c.offset < command.offset)
            .find(|c| c.opcode == 1 && c.words.first() == Some(&slot))
            .ok_or("camera tree slot has no prior declaration")?;
        let image = u32::from_le_bytes(bytes[36..40].try_into().unwrap()) as usize;
        if image > bytes.len() || program.program_end > image {
            return Err("camera string pool outside source".into());
        }
        if declaration.words.get(1).copied().map(|v| v as usize) != Some(image) {
            return Err("camera tree is not the prepared inline animation".into());
        }
        let tree = if command.words[1] == 0 {
            None
        } else {
            let at = command.words[1] as usize;
            if !(program.program_end..image).contains(&at) {
                return Err("camera tree string outside pool".into());
            }
            let raw = &bytes[at..image.min(at + 128)];
            let end = raw
                .iter()
                .position(|b| *b == 0)
                .ok_or("unterminated camera tree")?;
            if end == 0 || !raw[..end].is_ascii() {
                return Err("invalid camera tree name".into());
            }
            Some(String::from_utf8(raw[..end].to_vec()).unwrap())
        };
        let name_at = command.words[2] as usize;
        if !(program.program_end..image).contains(&name_at) || image > bytes.len() {
            return Err("camera node string outside pool".into());
        }
        let raw = &bytes[name_at..image.min(name_at + 128)];
        let end = raw
            .iter()
            .position(|b| *b == 0)
            .ok_or("unterminated camera node name")?;
        if end == 0 || !raw[..end].is_ascii() {
            return Err("invalid camera node name".into());
        }
        let name = String::from_utf8(raw[..end].to_vec()).unwrap();
        let node = animation
            .named_node(tree.as_deref(), &name)
            .map_err(|e| e.to_string())?;
        if node.kind != 1 {
            return Err("camera request resolves to a non-camera node".into());
        }
        Ok(Self {
            command_offset: command.offset,
            tree_slot: slot,
            tree,
            name,
            node_offset: node.offset,
        })
    }
}
impl Prepared {
    pub fn load(root: &std::path::Path, name: &str) -> Result<Self, String> {
        let bytes = crate::read_asset(crate::flick::sequence_path(root, name)?)
            .map_err(|e| e.to_string())?;
        let header = bytes.get(..24).ok_or("truncated director header")?;
        let header = std::array::from_fn(|i| {
            u32::from_le_bytes(header[i * 4..i * 4 + 4].try_into().unwrap())
        });
        if header[1] as usize != bytes.len() {
            return Err("director size differs from header".into());
        }
        let initial = Initial::from_header(header)?;
        let dependencies =
            giants_formats::flick::discover_dependencies(&bytes).map_err(|e| e.to_string())?;
        let effect_requests = crate::cinematic_fx::declarations(&bytes, &dependencies);
        let program = giants_formats::flick::parse(&bytes).map_err(|e| e.to_string());
        let animation = giants_formats::anm::inline_flk(&bytes).map_err(|e| e.to_string())?;
        let (_, reset_tick) = initial.first_bound_reset(animation.timeline, 0.)?;
        let binding_candidate = program
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|p| CameraBinding::first(&bytes, p, &animation));
        let camera_candidate =
            binding_candidate
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|binding| {
                    crate::animation::Evaluator {
                        animation: &animation,
                    }
                    .camera_named(binding.tree.as_deref(), &binding.name, reset_tick)
                    .map_err(|e| e.to_string())
                });
        Ok(Self {
            initial,
            animation,
            dependencies,
            program,
            effect_requests,
            binding_candidate,
            reset_tick,
            camera_candidate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initial_clock_is_unbound_and_first_reset_ignores_positive_dt() {
        let initial = Initial::from_header([3, 24, 1, 6, 0, 892]).unwrap();
        assert!(!initial.animation_bound);
        assert!(initial.finished);
        assert_eq!(initial.counts, [1, 6, 0]);
        assert_eq!(initial.cursor_offset, 24);
        for dt in [0., 0.001, 0.25, 1.] {
            let (step, tick) = initial
                .first_bound_reset([160, 42080, 160, 30], dt)
                .unwrap();
            assert_eq!((step.seconds, step.dt, tick), (0., 0., 160));
        }
        assert!(
            initial
                .first_bound_reset([0, 1, 160, 30], f32::NAN)
                .is_err()
        );
        assert!(Initial::from_header([3, 24, 1, 33, 0, 0]).is_err());
    }
}
