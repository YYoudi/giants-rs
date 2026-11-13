//! Copy an active canonical enemy-hit response into the actual partial player.
//! Prototype PS2 `mecc_shot` remains an unknown hurt-callback search route.
use super::*;
use giants_game::{
    controlled_mecc::{HitPose, HitResponse, HitState},
    resident_states::{Id, Session},
};

pub(crate) struct EnemyReceiver<'a> {
    pose: HitPose,
    health: &'a mut giants_game::mecc_damage::State,
    velocity: &'a mut [f32; 3],
}
pub(crate) struct EnemyFrame<'a> {
    pub camera: [f32; 3],
    pub objects: &'a [giants_game::actor_environment::Collider],
    pub receiver: EnemyReceiver<'a>,
}
impl State {
    /// Borrow the actual controller's health/velocity independently of its
    /// immutable environment. No copied controller or copied collision world.
    pub(crate) fn enemy_hit_frame(&mut self) -> Result<EnemyFrame<'_>, String> {
        if self.owned_pose.is_some() || self.body.source.is_none() {
            return Err("enemy response requires the real ordinary partial Mecc body".into());
        }
        Ok(EnemyFrame {
            camera: self.camera().1,
            objects: &self.environment_objects,
            receiver: EnemyReceiver {
                pose: HitPose {
                    position: self.motion.position,
                    angles: [self.yaw.to_degrees(), 0., 0.],
                    scale: self.scale,
                },
                health: &mut self.health,
                velocity: &mut self.motion.velocity,
            },
        })
    }
}
impl EnemyReceiver<'_> {
    /// Health has already been changed by the canonical receiver. Applying
    /// damage again here would double it. Position/history, controls and forces
    /// stay with the real physics/input producer; this copies health/velocity.
    pub(crate) fn accept_enemy_hit_response(
        &mut self,
        session: &Session,
        expected_owner: Id,
        response: HitResponse,
    ) -> Result<(), String> {
        let after = response.validate_controller(
            session,
            expected_owner,
            self.pose,
            HitState {
                health: *self.health,
                velocity: *self.velocity,
            },
        )?;
        *self.health = after.health;
        *self.velocity = after.velocity;
        Ok(())
    }
}
