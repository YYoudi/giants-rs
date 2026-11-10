use super::*;
use crate::vimp_wander::{put, word};

fn prepared() -> (
    Session,
    Id,
    crate::vimp_actor::Factory,
    giants_formats::global_data::GlobalData,
    Arc<giants_formats::effect_image::Image>,
    crate::audio_parameters::Random,
) {
    let (mut s, f, g, e, children, mut random) = super::vimp_history_tests::source_births(true);
    let id = children[0];
    s.begin_vimp_wander_offline(
        id,
        crate::vimp_wander::Tick {
            global_delta: 0.1,
            control_factor: 0.5,
            skip_heading: false,
            slow_areas: &[],
            now: 2.,
            noise_time: 0.,
            noise_position: [50000., 50000., 0.],
        },
        &f,
        &g,
        &mut random,
    )
    .unwrap();
    s.advance_vimp_physical_offline(id, &f, &g, |_| {
        Ok(crate::physical_ground::Ground {
            height: 7.25,
            plane: [0., 0., 1., -7.25],
            key: [0, 0],
        })
    })
    .unwrap();
    assert_ne!(s.vimp_physical(id).unwrap().model_rates[1], 0.);
    (s, id, f, g, e, random)
}
#[test]
#[ignore = "requires GIANTS_NPC_ROOT; actual sealed physical/ANM ownership with declared post-physical gait/contact inputs"]
fn source_vimp_locomotion_animation_consumes_actual_resources_and_physical_phase() {
    for (gait, physical, slot) in [(1, 0, 3), (2, 0, 4), (2, 0x40, 72)] {
        let (mut s, id, f, g, e, mut random) = prepared();
        let a = s.residents.get_mut(&id.serial).unwrap();
        let b = a.seed.storage.data_mut().unwrap();
        put(b, 0x74, gait);
        let flags = word(b, 0);
        put(b, 0, (flags & !0x40) | physical);
        let mut contexts = crate::attached_effect_context::Pool::new(660, Some(e));
        let mut stamp = crate::animation_epoch::Stamp::default();
        s.advance_vimp_animation_offline(
            id,
            &f,
            &g,
            crate::vimp_animation::Services {
                blending: crate::animation_channels::Blending::Disabled,
                contexts: &mut contexts,
                random: &mut random,
                stamp: &mut stamp,
            },
        )
        .unwrap();
        let a = s.get(id).unwrap();
        assert_eq!(a.channels.unwrap()[0][0], slot);
        assert_eq!(
            a.channels,
            Some(a.construction_fields.as_ref().unwrap().channels)
        );
        // ANM consumes its own phase; only the complete frame may close wander.
        assert!(a.vimp.unwrap().wander_pending);
        assert!(a.vimp_physical.as_ref().unwrap().view.animation_consumed);
        assert!(a.vimp_physical.as_ref().unwrap().view.failure.is_none());
        assert_eq!(a.vimp_frames, 0);
        let before = a.channels;
        assert!(
            s.advance_vimp_animation_offline(
                id,
                &f,
                &g,
                crate::vimp_animation::Services {
                    blending: crate::animation_channels::Blending::Disabled,
                    contexts: &mut contexts,
                    random: &mut random,
                    stamp: &mut stamp,
                },
            )
            .is_err()
        );
        assert_eq!(s.get(id).unwrap().channels, before);
    }
}
