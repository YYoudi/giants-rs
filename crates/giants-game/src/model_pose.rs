//! Initial source poses: intact health or declared-loop benchmark warmup.
//! Damage, later AI, procedural aiming and general animation are not simulated.
use crate::{model_registry::SelectionSpec, scene_resources::SceneResources};
use giants_formats::{anm::Animation, gbs::Mesh};

#[derive(Clone, Debug)]
pub struct InitialPose {
    pub name: String,
    pub tree: String,
    pub timeline: [i32; 4],
    pub seconds: f32,
    pub tick: i32,
    pub profile: &'static str,
    pub unresolved_auxiliary_links: usize,
}
pub const BENCHMARK_INITIAL_STEP: f32 = 1. / 60.;

/// Ordered class command13 selects a loop without resetting its time field.
/// This projects a zero-initialized class template, not current animation state.
pub fn declared_loop(
    program: &giants_formats::class_program::Program,
) -> Result<Option<u8>, &'static str> {
    let mut result = None;
    for r in &program.records {
        if r.opcode == 0x13 {
            let slot = *r.bytes.get(1).ok_or("truncated loop declaration")?;
            if slot == 0xfe {
                return Err("loop declaration uses native error sentinel");
            }
            result = Some(slot);
        }
    }
    Ok(result)
}
/// Checked initial state60 branch: no attached actor and no cut/fall bit.
/// Its callback preserves the declared channel; later tree behavior is separate.
pub fn idle_tree_channel(
    program: &giants_formats::class_program::Program,
    storage: &crate::state_storage::Storage,
    attached_actor: bool,
) -> Result<u8, &'static str> {
    let data = storage.data().ok_or("idle tree state storage missing")?;
    if storage.state() != 60 || data.len() != 0x4c || attached_actor {
        return Err("tree outside unbound initial state60 profile");
    }
    let flags = u32::from_le_bytes(data[0x24..0x28].try_into().unwrap());
    if flags & 4 != 0 {
        return Err("cut/falling tree callback remains unresolved");
    }
    declared_loop(program)?.ok_or("idle tree has no declared channel")
}
/// Source class construction before AI/object callbacks. The declared channel
/// is not a current animation state and never certifies live visibility.
pub fn constructed_actor_channel(
    program: &giants_formats::class_program::Program,
    class_state: u8,
    instance_state: u8,
) -> Result<u8, &'static str> {
    if !matches!(class_state, 23 | 38 | 51 | 105) || class_state != instance_state {
        return Err("constructed actor state outside checked initial profile");
    }
    declared_loop(program)?.ok_or("constructed actor has no declared channel")
}
pub fn seconds_tick(timeline: [i32; 4], seconds: f32) -> Result<i32, &'static str> {
    let rate = timeline[2]
        .checked_mul(timeline[3])
        .ok_or("animation rate overflow")?;
    if rate <= 0 || !seconds.is_finite() || seconds < 0. {
        return Err("invalid initial animation seconds");
    }
    let offset = (f64::from(rate) * f64::from(seconds)).trunc();
    if offset > f64::from(i32::MAX) {
        return Err("initial animation tick overflow");
    }
    timeline[0]
        .checked_add(offset as i32)
        .ok_or("initial animation tick overflow")
}
pub fn loop_warmup_time(timeline: [i32; 4], step: f32) -> Result<(f32, i32), &'static str> {
    let (duration, _) = intact_time(timeline)?;
    if !step.is_finite() || step <= 0. || step >= duration {
        return Err("initial loop wrap or nonpositive step outside measured profile");
    }
    Ok((step, seconds_tick(timeline, step)?))
}
/// The descriptor stores duration in binary32; applying it truncates the
/// promoted product, then adds the source start tick. An intact pose therefore
/// need not land exactly on the final serialized tick.
pub fn intact_time(timeline: [i32; 4]) -> Result<(f32, i32), &'static str> {
    let [start, end, ticks, fps] = timeline;
    let rate = ticks.checked_mul(fps).ok_or("animation rate overflow")?;
    let span = end.checked_sub(start).ok_or("animation span overflow")?;
    if rate <= 0 || span <= 0 {
        return Err("invalid intact animation timeline");
    }
    let seconds = (f64::from(span) / f64::from(rate)) as f32;
    let offset = (f64::from(rate) * f64::from(seconds)).trunc();
    if offset < 0. || offset > f64::from(i32::MAX) {
        return Err("intact animation tick overflow");
    }
    Ok((
        seconds,
        start
            .checked_add(offset as i32)
            .ok_or("animation tick overflow")?,
    ))
}
/// Use the shared debug descriptor arithmetic, with its binary32 boundary.
pub fn normal_intact_time(timeline: [i32; 4]) -> Result<(f32, i32), String> {
    let seconds =
        crate::animation_template::duration(timeline, crate::arithmetic::Precision::Double53)
            .map_err(|e| e.to_string())?;
    Ok((seconds, seconds_tick(timeline, seconds)?))
}
pub struct Prepared {
    pub pose: InitialPose,
    animation: Animation,
}
impl Prepared {
    /// Declared base channel at its source start, before live state82 callbacks.
    /// This is an explicit construction preview, not normal animation timing.
    pub fn load_base_preview(
        resources: &SceneResources,
        class: i32,
        selection: &SelectionSpec,
    ) -> Result<Self, String> {
        let definition = resources
            .classes
            .models
            .first_program(class)
            .ok_or("base class missing")?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        if resources
            .class_state_seed(class)
            .map_err(|e| e.to_string())?
            .storage
            .state()
            != 82
        {
            return Err("base preview outside state82".into());
        }
        let channel = declared_loop(program)?.ok_or("base declared channel missing")?;
        let channels = selection
            .animations
            .iter()
            .filter(|a| a.slot == channel)
            .collect::<Vec<_>>();
        if channels.len() != 1 {
            return Err("base channel missing or ambiguous".into());
        }
        let name = channels[0].name.clone();
        let animation = resources.animation(&name).map_err(|e| e.to_string())?;
        if animation.trees.len() != 1 || animation.trees[0].offset != animation.default_tree {
            return Err("base preview outside single default tree".into());
        }
        let pose = InitialPose {
            name,
            tree: animation.trees[0].name.clone(),
            timeline: animation.timeline,
            seconds: 0.,
            tick: animation.timeline[0],
            profile: "source_base_declared_start",
            unresolved_auxiliary_links: animation.unresolved_links().len(),
        };
        Ok(Self { pose, animation })
    }
    /// Declared class pose at the cinematic terrain-query stage. The native
    /// state27 initializer does not select another channel; gravity, local time
    /// and the following state transition are separate from this preview.
    pub fn load_ground_preview(
        resources: &SceneResources,
        class: i32,
        selection: &SelectionSpec,
    ) -> Result<Self, String> {
        if resources
            .class_state_seed(class)
            .map_err(|e| e.to_string())?
            .storage
            .state()
            != 27
        {
            return Err("ground pose outside initial class state27 profile".into());
        }
        let mut prepared = Self::load_normal_initial(resources, 0, class, selection)?;
        prepared.pose.profile = "source_ground_declared_start";
        Ok(prepared)
    }
    /// State18 selectors8/9 choose the acquired stationary Smartie clips. This
    /// is their selected source start pose, not live Smartie AI or blend timing.
    /// Prototype PS2 search key: `smartie_move`; exact PC attribution unknown.
    pub fn load_stationary_smartie(
        resources: &SceneResources,
        class: i32,
        selection: &SelectionSpec,
        selector: Option<u8>,
    ) -> Result<Self, String> {
        if resources
            .class_state_seed(class)
            .map_err(|e| e.to_string())?
            .storage
            .state()
            != 18
        {
            return Err("stationary Smartie projection requires state18".into());
        }
        let slot = match selector {
            Some(8) => 0x82,
            Some(9) => 0x83,
            _ => return Err("Smartie selector requires its live animation service".into()),
        };
        Self::normal_slot(
            resources,
            18,
            selection,
            slot,
            "source_stationary_smartie_selected_start",
            false,
        )
    }
    /// Measured initial visual projection, separate from ongoing AI callbacks.
    pub fn load_normal_initial(
        resources: &SceneResources,
        state: u8,
        class: i32,
        selection: &SelectionSpec,
    ) -> Result<Self, String> {
        let (slot, profile, intact) = match state {
            61 | 98 | 101 | 102 => (1, "debug_normal_intact", true),
            23 | 38 | 51 | 105 => {
                let definition = resources
                    .classes
                    .models
                    .first_program(class)
                    .ok_or("class program missing")?;
                let program = definition.program.as_ref().map_err(Clone::clone)?;
                let seed = resources
                    .class_state_seed(class)
                    .map_err(|e| e.to_string())?;
                (
                    constructed_actor_channel(program, seed.storage.state(), state)?,
                    "debug_constructed_actor_start",
                    false,
                )
            }
            60 => {
                let definition = resources
                    .classes
                    .models
                    .first_program(class)
                    .ok_or("class program missing")?;
                let program = definition.program.as_ref().map_err(Clone::clone)?;
                let seed = resources
                    .class_state_seed(class)
                    .map_err(|e| e.to_string())?;
                (
                    idle_tree_channel(program, &seed.storage, false)?,
                    "debug_initial_idle_tree",
                    false,
                )
            }
            0 => {
                let definition = resources
                    .classes
                    .models
                    .first_program(class)
                    .ok_or("class program missing")?;
                let program = definition.program.as_ref().map_err(Clone::clone)?;
                (
                    declared_loop(program)?.ok_or("no declared initial channel")?,
                    "debug_normal_declared_start",
                    false,
                )
            }
            _ => {
                return Err(format!(
                    "normal initial animation for state {state} is unverified"
                ));
            }
        };
        Self::normal_slot(resources, state, selection, slot, profile, intact)
    }
    fn normal_slot(
        resources: &SceneResources,
        state: u8,
        selection: &SelectionSpec,
        slot: u8,
        profile: &'static str,
        intact: bool,
    ) -> Result<Self, String> {
        let bindings = selection
            .animations
            .iter()
            .filter(|a| a.slot == slot)
            .collect::<Vec<_>>();
        if bindings.len() != 1 {
            return Err("normal initial slot missing or ambiguous".into());
        }
        let name = bindings[0].name.clone();
        let animation = (if state == 60 {
            resources.animation_for_bindings(&name)
        } else {
            resources.animation(&name)
        })
        .map_err(|e| format!("animation {name}: {e}"))?;
        if animation.trees.len() != 1 || animation.trees[0].offset != animation.default_tree {
            return Err("normal initial animation outside single-default-tree profile".into());
        }
        let (seconds, tick) = if intact {
            normal_intact_time(animation.timeline)
        } else {
            Ok((0., animation.timeline[0]))
        }?;
        let pose = InitialPose {
            name,
            tree: animation.trees[0].name.clone(),
            timeline: animation.timeline,
            seconds,
            tick,
            profile,
            unresolved_auxiliary_links: animation.unresolved_links().len(),
        };
        Ok(Self { pose, animation })
    }
    pub fn load(
        resources: &SceneResources,
        state: u8,
        class: i32,
        selection: &SelectionSpec,
    ) -> Result<Self, String> {
        if matches!(state, 0 | 23 | 38 | 51 | 60 | 105) {
            return Self::load_normal_initial(resources, state, class, selection);
        }
        let (slot, profile) = match state {
            61 | 101 => (1, "intact_health"),
            44 => {
                let definition = resources
                    .classes
                    .models
                    .first_program(class)
                    .ok_or("class program missing")?;
                let program = definition.program.as_ref().map_err(Clone::clone)?;
                (
                    declared_loop(program)?.ok_or("state44 has no declared loop")?,
                    "declared_loop_warmup",
                )
            }
            _ => return Err(format!("animation callback for state {state} pending")),
        };
        let bindings: Vec<_> = selection
            .animations
            .iter()
            .filter(|a| a.slot == slot)
            .collect();
        if bindings.len() != 1 {
            return Err(format!(
                "missing or ambiguous initial animation slot {slot}"
            ));
        }
        let name = bindings[0].name.clone();
        let animation = resources.animation(&name).map_err(|e| e.to_string())?;
        if animation.trees.len() != 1 || animation.trees[0].offset != animation.default_tree {
            return Err(
                "health animation tree selection outside measured single-default profile".into(),
            );
        }
        let (seconds, tick) = if state == 44 {
            loop_warmup_time(animation.timeline, BENCHMARK_INITIAL_STEP)
        } else {
            intact_time(animation.timeline)
        }
        .map_err(String::from)?;
        let pose = InitialPose {
            name,
            tree: animation.trees[0].name.clone(),
            timeline: animation.timeline,
            seconds,
            tick,
            profile,
            unresolved_auxiliary_links: 0,
        };
        Ok(Self { pose, animation })
    }
    pub fn positions(
        &self,
        resources: &SceneResources,
        model_name: &str,
        mesh: &Mesh,
    ) -> Result<Vec<[f32; 3]>, String> {
        let skin_name = format!("{}.skn", model_name.to_ascii_lowercase());
        if !resources.archives.members.contains_key(&skin_name) {
            // The native deformation entry exits before animation/LOD selection
            // for an unskinned first model, independently of the pose clock.
            // Object/part transforms remain separate. A malformed present SKN
            // must still fail below rather than taking this optional-skin route.
            let descriptor = crate::animation_template::Template::prepare(
                &self.animation,
                &[crate::animation_template::Model::Unskinned],
                crate::arithmetic::Precision::Double53,
            )
            .map_err(|e| e.to_string())?;
            if descriptor.flags != 1 || !descriptor.needs.is_empty() {
                return Err("unskinned descriptor preparation disagrees".into());
            }
            return Ok(mesh.positions.clone());
        }
        let raw = resources
            .archives
            .read_bin(&format!("{model_name}.skn"), &resources.language)
            .map_err(|e| format!("skin {model_name}: {e}"))?;
        let skin = giants_formats::skn::parse(&raw).map_err(|e| e.to_string())?;
        if skin.objects.len() != mesh.groups.len()
            || skin
                .objects
                .iter()
                .zip(&mesh.groups)
                .any(|(object, group)| object.vertex_count != group[1] as usize)
        {
            return Err("skin/model group correspondence outside measured profile".into());
        }
        let positions =
            crate::skinning::positions(&skin, &self.animation, &self.pose.tree, self.pose.tick)
                .map_err(|e| e.to_string())?;
        if positions.len() != mesh.positions.len() {
            return Err("skin/model vertex count mismatch".into());
        }
        Ok(positions)
    }
}

#[cfg(test)]
mod idle_tree_tests {
    use super::*;
    #[test]
    fn idle_channel_comes_from_class_and_rejects_changed_tree_contexts() {
        let program = giants_formats::class_program::parse(&[0x13, 3, 0xff]).unwrap();
        let mut storage = crate::state_storage::Storage::default();
        storage.transition(60).unwrap();
        assert_eq!(idle_tree_channel(&program, &storage, false), Ok(3));
        assert!(idle_tree_channel(&program, &storage, true).is_err());
        storage.data_mut().unwrap()[0x24] = 4;
        assert!(idle_tree_channel(&program, &storage, false).is_err());
        storage.transition(0).unwrap();
        assert!(idle_tree_channel(&program, &storage, false).is_err());
    }
    #[test]
    fn constructed_actor_retains_source_channel_and_rejects_state_changes() {
        let program = giants_formats::class_program::parse(&[0x13, 80, 0xff]).unwrap();
        for state in [23, 38, 51] {
            assert_eq!(constructed_actor_channel(&program, state, state), Ok(80));
            assert!(constructed_actor_channel(&program, state, 0).is_err());
        }
        assert!(constructed_actor_channel(&program, 60, 60).is_err());
        let missing = giants_formats::class_program::parse(&[0xff]).unwrap();
        assert!(constructed_actor_channel(&missing, 51, 51).is_err());
    }
}
