//! Per-source-actor render permission, applied after all scene updates and
//! before Bevy visibility propagation. Source attachment parents are not ECS
//! visibility parents; each rendered part retains its own actor slot.
use bevy::prelude::*;

#[derive(Component)]
pub struct ActorSlot(pub usize);

pub fn declared_slots(
    program: &giants_formats::flick::Program,
) -> impl Iterator<Item = usize> + '_ {
    program
        .commands
        .iter()
        .filter(|command| command.opcode == 2)
        .map(|command| command.words[2] as usize)
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct EntityCount {
    pub mesh_entities: usize,
    pub local_render_enabled: usize,
    pub gpu_prepared_entities: usize,
}
pub fn capture_counts(
    declared: impl IntoIterator<Item = usize>,
    observed: impl IntoIterator<Item = (usize, bool, bool)>,
) -> std::collections::BTreeMap<usize, EntityCount> {
    let mut rows = declared
        .into_iter()
        .map(|slot| (slot, EntityCount::default()))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (slot, enabled, prepared) in observed {
        let row = rows.entry(slot).or_default();
        row.mesh_entities += 1;
        row.local_render_enabled += usize::from(enabled);
        row.gpu_prepared_entities += usize::from(prepared);
    }
    rows
}

pub fn update(
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut actors: Query<(Entity, Ref<ActorSlot>, &mut Visibility)>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    for (entity, slot, mut visibility) in &mut actors {
        let Some(enabled) = cinematic.actor_visible(slot.0) else {
            error!(
                "Cinematic visibility actor slot outside timeline: {}",
                slot.0
            );
            exit.write(bevy::app::AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        };
        let transparent = cinematic.actor_transparency(slot.0) == Some(1.);
        let visible = cinematic.actor_render_visible(slot.0) == Some(true);
        let next = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != next || slot.is_added() {
            *visibility = next;
            println!(
                "cinematic_actor_visibility sequence={} director={} tick={} actor_slot={} entity={:?} enabled={} transparent={} visible={} initial={} declaration_reached={}",
                cinematic.sequence(),
                cinematic.director_index(),
                cinematic.tick(),
                slot.0,
                entity,
                enabled,
                transparent,
                visible,
                slot.is_added(),
                cinematic.actor_requested(slot.0) == Some(true)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_declaration_uses_actor_slot_not_the_name_pointer() {
        let program = giants_formats::flick::Program {
            commands: vec![giants_formats::flick::Command {
                offset: 88,
                opcode: 2,
                words: vec![0, 792, 2, 216],
                label: None,
            }],
            next: None,
            program_end: 112,
        };
        assert_eq!(declared_slots(&program).collect::<Vec<_>>(), vec![2]);
    }
    #[test]
    fn capture_separates_entity_presence_from_permission_and_gpu_preparation() {
        let rows = capture_counts(
            [0, 1, 2],
            [(0, true, true), (0, false, false), (1, true, false)],
        );
        assert_eq!(
            rows[&0],
            EntityCount {
                mesh_entities: 2,
                local_render_enabled: 1,
                gpu_prepared_entities: 1
            }
        );
        assert_eq!(rows[&1].mesh_entities, 1);
        assert_eq!(rows[&1].gpu_prepared_entities, 0);
        assert_eq!(rows[&2], EntityCount::default());
        // A tagged entity outside the declaration set stays reported, not
        // silently attributed to some other declared actor.
        assert_eq!(
            capture_counts([0], [(7, false, false)])[&7].mesh_entities,
            1
        );
    }
}
