//! Logical hidden burst owners. No drawable instance or second RNG traversal.
use giants_game::{
    effect_burst::{Playback, Spawn},
    procedural_aim::Matrix,
};
use std::collections::{BTreeMap, BTreeSet};

pub type Key = (usize, usize, usize, usize); // command, group, phase, object
#[derive(Default)]
pub struct Owners(BTreeMap<Key, Playback>);

pub fn admitted(spawn: &Spawn) -> bool {
    spawn.hidden
        && spawn.parent.is_none()
        && !spawn.physics_enabled
        && !spawn.contact_requested
        && spawn.speed == 0.
        && spawn.spin == [0.; 3]
        && spawn.child_services.is_empty()
        && spawn.texture_animation.is_none()
        && spawn.sounds.iter().any(|s| s.attached)
}

impl Owners {
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn start(
        &mut self,
        prefix: (usize, usize, usize),
        spawns: &[Spawn],
        matrices: &[Matrix],
        classes: &BTreeSet<i32>,
    ) -> Result<(), String> {
        if spawns.len() != matrices.len() {
            return Err("hidden owner birth count differs".into());
        }
        for (object, (spawn, matrix)) in spawns.iter().zip(matrices).enumerate() {
            if !admitted(spawn) || !classes.contains(&spawn.class) {
                continue;
            }
            let key = (prefix.0, prefix.1, prefix.2, object);
            if self.0.contains_key(&key) {
                return Err("hidden owner identity reused without reset".into());
            }
            let owner = Playback::new(spawn.clone(), *matrix).map_err(str::to_owned)?;
            println!(
                "cinematic_burst_hidden_owner_created command={} group={} phase={} object={} class={} position_bits={:?} native_cadence_verified=false",
                key.0,
                key.1,
                key.2,
                key.3,
                spawn.class,
                owner.motion.position.map(f32::to_bits)
            );
            self.0.insert(key, owner);
        }
        Ok(())
    }
    pub fn advance(&mut self, dt: f32) -> Result<(), String> {
        for (key, owner) in &mut self.0 {
            let was_expired = owner.expired;
            owner.advance_free(dt).map_err(str::to_owned)?;
            if !was_expired && owner.expired {
                println!(
                    "cinematic_burst_hidden_owner_deleted command={} group={} phase={} object={} age_bits={} position_bits={:?} native_cadence_verified=false",
                    key.0,
                    key.1,
                    key.2,
                    key.3,
                    owner.visual.age.to_bits(),
                    owner.motion.position.map(f32::to_bits)
                );
            }
        }
        Ok(())
    }
    pub fn get(&self, key: Key) -> Option<crate::cinematic_effect_audio::OwnerState> {
        self.0
            .get(&key)
            .map(|p| crate::cinematic_effect_audio::OwnerState {
                position: p.motion.position,
                valid: !p.expired,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spawn() -> Spawn {
        Spawn {
            class: 900,
            parent: None,
            reference: 0,
            own_reference: 0,
            visual: giants_game::effect_visual::Configuration {
                lifetime: 2.,
                fade_in: 0.,
                fade_out_start: 2.,
                start_size: 1.,
                end_size: 1.,
            },
            height: 0.,
            hidden: true,
            spin: [0.; 3],
            direction_requested: false,
            speed: 0.,
            gravity_mass: 1.,
            physics_enabled: false,
            damping: [0.; 2],
            bounce: 0.,
            contact_requested: false,
            parameter_b0: 1f32.to_bits(),
            attribute: None,
            sounds: vec![giants_game::effect_burst::SoundNeed {
                id: 12,
                attached: true,
            }],
            texture_animation: None,
            size_configured: true,
            child_services: Vec::new(),
        }
    }
    fn matrix() -> Matrix {
        let mut values = [0.; 16];
        values[15] = 1.;
        values[3] = 7.;
        values[7] = -5.;
        values[11] = 3.;
        Matrix { flags: 16, values }
    }
    #[test]
    fn hidden_support_lives_without_draws_then_invalidates_and_resets() {
        let mut owners = Owners::default();
        let spawns = [spawn()];
        let classes = BTreeSet::from([900]);
        owners
            .start((10, usize::MAX, 0), &spawns, &[matrix()], &classes)
            .unwrap();
        let key = (10, usize::MAX, 0, 0);
        assert_eq!(owners.get(key).unwrap().position, [7., -5., 3.]);
        owners.advance(9.).unwrap(); // First positive callback does not age.
        assert!(owners.get(key).unwrap().valid);
        owners.advance(1.).unwrap();
        owners.advance(0.).unwrap();
        assert!(owners.get(key).unwrap().valid);
        owners.advance(1.).unwrap();
        assert!(!owners.get(key).unwrap().valid);
        assert!(
            owners
                .start((10, usize::MAX, 0), &spawns, &[matrix()], &classes)
                .is_err()
        );
        owners
            .start((10, 1, 0), &spawns, &[matrix()], &classes)
            .unwrap();
        assert!(owners.get((10, 1, 0, 0)).unwrap().valid);
        owners.clear();
        assert!(owners.get(key).is_none());
        owners
            .start((10, usize::MAX, 0), &spawns, &[matrix()], &classes)
            .unwrap();
        assert!(owners.get(key).unwrap().valid);
    }
    #[test]
    fn unresolved_classes_and_dynamic_hidden_supports_do_not_get_fake_owners() {
        let mut owners = Owners::default();
        owners
            .start((0, 0, 0), &[spawn()], &[matrix()], &BTreeSet::new())
            .unwrap();
        assert!(owners.get((0, 0, 0, 0)).is_none());
        let mut mobile = spawn();
        mobile.physics_enabled = true;
        owners
            .start((0, 0, 0), &[mobile], &[matrix()], &BTreeSet::from([900]))
            .unwrap();
        assert!(owners.get((0, 0, 0, 0)).is_none());
    }
}
