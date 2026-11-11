//! PC objective predicates, independent of a mission name or actor AI.
//! Prototype PS2 navigation: `mission_check_obj_proximity`. Attribution remains
//! probable: its resource gate agrees, but the PC uses a point/AABB fallback
//! where the prototype uses summed radii. No prototype ABI/layout is imported.
use crate::{mission_notice, volume_queries::Bounds};

/// Source declarations admitted to the stationary destination subset. Other
/// actor states need their live owner; their first placement is not a substitute
/// for current position/bounds. This prepares data, not campaign readiness.
pub struct Program {
    pub scenario_index: usize,
    pub required: i32,
    pub quota: Option<i32>,
    pub prefix: Vec<u8>,
    pub targets: Vec<(crate::declared_residents::Key, Target)>,
}
impl Program {
    pub fn prepare(
        packet: &crate::mission_loading::PreparedMission,
        plan: &crate::instances::Plan,
    ) -> Result<Self, String> {
        use giants_formats::mission::{ObjectRef, SlotWriteKind};
        let mission = packet.mission.as_ref().ok_or("objective mission absent")?;
        let world = packet.world.as_ref().ok_or("objective world absent")?;
        let index = packet
            .scenario_index
            .ok_or("objective scenario identity absent")?;
        let record = world
            .scenarios
            .get(index)
            .ok_or("objective record absent")?;
        if record.kind != 2 || record.name != packet.request.mission_name {
            return Err("objective record is not the selected source mission".into());
        }
        let quota = mission
            .slot_writes
            .iter()
            .filter_map(|w| match w.kind {
                SlotWriteKind::Parameter15(v) => Some(v),
                _ => None,
            })
            .next_back();
        let mut marks = std::collections::BTreeMap::<usize, u32>::new();
        for mark in &mission.objective_marks {
            let ObjectRef::Placement(i) = mark.object else {
                return Err("objective child requires its actual live owner".into());
            };
            *marks.entry(i).or_default() |= u32::from(mark.value);
        }
        let mut targets = Vec::new();
        for (i, mark) in marks {
            if mark == 0 {
                continue;
            }
            let source = plan
                .entries
                .get(i)
                .ok_or("objective placement absent")?
                .source
                .as_ref()
                .map_err(Clone::clone)?;
            let state = source
                .state_seed
                .as_ref()
                .map_err(Clone::clone)?
                .storage
                .state();
            if state != 94 || source.class != 1017 || !matches!(mark, 1 | 4) {
                return Err("objective requires an unbound live actor or predicate service".into());
            }
            if mark == 4 && quota.is_none() {
                return Err("resource destination requires its real mission slot quota".into());
            }
            let specification =
                crate::specification::Prepared::load_registered(plan.resources(), source.class)?;
            if !specification.needs.is_empty() {
                return Err("destination specification has unresolved resources".into());
            }
            let input =
                crate::actor_fields::Fields::load(plan.resources(), source.class, &specification)?;
            let mut fields = input
                .placed(crate::collider_init::DeclaredPose {
                    class: source.class as u32,
                    position: source.position,
                    yaw: source.yaw,
                    pitch_roll: source.pitch_roll,
                })
                .map_err(str::to_owned)?;
            fields
                .set_declared_scale(source.scale)
                .map_err(str::to_owned)?;
            fields.mark_mission_placement();
            let resources = plan
                .resources()
                .collision_resource_bounds(source.class, None)
                .map_err(|e| e.to_string())?;
            let bounds = crate::collision_bounds::uncached(
                Some(crate::collision_bounds::Models {
                    primary: resources.primary,
                    auxiliary: None,
                    slot: 254,
                }),
                crate::uncached_pose::Pose {
                    position: fields.current,
                    angles: fields.angles,
                    scale: fields.scale,
                },
                crate::arithmetic::Precision::Double53,
            )
            .map_err(str::to_owned)?;
            targets.push((
                crate::declared_residents::Key {
                    scope: crate::declared_residents::Scope::Mission,
                    index: i,
                },
                Target {
                    position: source.position,
                    bounds: Bounds {
                        min: bounds[..3].try_into().unwrap(),
                        max: bounds[3..].try_into().unwrap(),
                    },
                    response_flags: fields.response_flags | mark,
                },
            ));
        }
        if targets.is_empty() && quota.is_none() {
            return Err("no admitted objective producer in selected mission".into());
        }
        let producers = targets.len() + usize::from(quota.is_some_and(|v| v != -1));
        if record.parameter == 0 || producers != record.parameter as usize {
            return Err(
                "objective producer coverage does not match the source required count".into(),
            );
        }
        Ok(Self {
            scenario_index: index,
            required: record.parameter as i32,
            quota,
            prefix: record.name.as_bytes().to_vec(),
            targets,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub state: u8,
    pub position: [f32; 3],
    /// Actual state2 food or state3 resource; never the accumulated quota credit.
    pub resource: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub position: [f32; 3],
    /// Current source actor bounds, not a rendered mesh approximation.
    pub bounds: Bounds,
    pub response_flags: u32,
}

/// Acquired PC proximity predicate and one-shot mark consumption. The owning
/// mission supplies its real player, slot quota and suppression state. Returning
/// a lookup does not acknowledge display, cinematic playback or mission victory.
pub fn proximity(
    target: &mut Target,
    player: Option<Player>,
    quota: i32,
    progress: &mut mission_notice::State,
    context: mission_notice::Context<'_>,
) -> Result<Option<mission_notice::Lookup>, String> {
    if !context.gates.time.is_finite()
        || context.prefix.len() > 31
        || !context.prefix.is_ascii()
        || context.prefix.contains(&0)
    {
        return Err("objective requires a finite owned mission context".into());
    }
    if context.gates.suppress[0] != 0 {
        return Ok(None);
    }
    let Some(player) = player else {
        return Ok(None);
    };
    let kind = target.response_flags & 15;
    if kind != 1 && kind != 4 {
        return Ok(None);
    }
    if !target
        .position
        .into_iter()
        .chain(target.bounds.min)
        .chain(target.bounds.max)
        .chain(player.position)
        .chain([player.resource])
        .all(f32::is_finite)
    {
        return Err("objective position, resource or bounds outside finite profile".into());
    }
    if kind == 4
        && (!matches!(player.state, 2 | 3) || f64::from(player.resource) < f64::from(quota))
    {
        return Ok(None);
    }
    let delta = std::array::from_fn::<_, 3, _>(|i| {
        (f64::from(target.position[i]) - f64::from(player.position[i])) as f32
    });
    let wide = std::array::from_fn::<_, 3, _>(|i| {
        f64::from(target.position[i]) - f64::from(player.position[i])
    });
    let squared = wide.into_iter().fold(0., |sum, v| sum + v * v);
    if squared >= 100. {
        let reciprocal = 1.
            / delta
                .into_iter()
                .fold(0., |sum, v| sum + f64::from(v) * f64::from(v))
                .sqrt();
        let point = std::array::from_fn::<_, 3, _>(|i| {
            let direction = (f64::from(delta[i]) * reciprocal) as f32;
            (f64::from(player.position[i]) + f64::from(direction) * 10.) as f32
        });
        if !(0..3).all(|i| target.bounds.min[i] <= point[i] && point[i] <= target.bounds.max[i]) {
            return Ok(None);
        }
    }
    if context.gates.suppress.into_iter().any(|v| v != 0) || context.gates.time > 0. {
        return Ok(None);
    }
    progress.counter = progress.counter.wrapping_add(1);
    target.response_flags &= !15;
    let mut key = context.prefix.to_vec();
    key.extend_from_slice((progress.counter as i32).to_string().as_bytes());
    Ok(Some(mission_notice::Lookup { key }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn return_requires_carried_quota_and_consumes_only_its_own_mark_once() {
        let mut target = Target {
            position: [0.; 3],
            bounds: Bounds {
                min: [-20.; 3],
                max: [20.; 3],
            },
            response_flags: 0x20000004,
        };
        let mut progress = mission_notice::State {
            pending: 7,
            counter: 1,
        };
        let context = mission_notice::Context {
            prefix: b"mission",
            gates: mission_notice::Gates {
                suppress: [0; 3],
                time: 0.,
            },
        };
        let mut player = Player {
            state: 2,
            position: [0.; 3],
            resource: 19.,
        };
        assert!(
            proximity(&mut target, Some(player), 20, &mut progress, context)
                .unwrap()
                .is_none()
        );
        assert_eq!(target.response_flags, 0x20000004);
        player.resource = 20.;
        player.position = [40., 0., 0.];
        assert!(
            proximity(&mut target, Some(player), 20, &mut progress, context)
                .unwrap()
                .is_none()
        );
        player.position = [30., 0., 0.];
        assert_eq!(
            proximity(&mut target, Some(player), 20, &mut progress, context)
                .unwrap()
                .unwrap()
                .key,
            b"mission2"
        );
        assert_eq!(
            progress,
            mission_notice::State {
                pending: 7,
                counter: 2
            }
        );
        assert_eq!(target.response_flags, 0x20000000);
        assert!(
            proximity(&mut target, Some(player), 20, &mut progress, context)
                .unwrap()
                .is_none()
        );
    }
}
