//! Copy an active canonical enemy-hit response into the actual partial player.
//! Prototype PS2 `mecc_shot` remains an unknown hurt-callback search route.
use super::*;
use giants_game::{
    controlled_mecc::{HitPose, HitResponse, HitState},
    resident_states::{Id, Session},
};

impl State {
    /// Health has already been changed by the canonical receiver. Applying
    /// damage again here would double it. Position/history, controls and forces
    /// stay with the real physics/input producer; this copies health/velocity.
    #[expect(
        dead_code,
        reason = "Actual ordinary hit host still requires source-backed world gates; no empty provider is installed"
    )]
    pub(crate) fn accept_enemy_hit_response(
        &mut self,
        session: &Session,
        expected_owner: Id,
        response: HitResponse,
    ) -> Result<(), String> {
        if self.owned_pose.is_some() || self.body.source.is_none() {
            return Err("enemy response requires the real ordinary partial Mecc body".into());
        }
        let after = response.validate_controller(
            session,
            expected_owner,
            HitPose {
                position: self.motion.position,
                angles: [self.yaw.to_degrees(), 0., 0.],
                scale: self.scale,
            },
            HitState {
                health: self.health,
                velocity: self.motion.velocity,
            },
        )?;
        self.health = after.health;
        self.motion.velocity = after.velocity;
        Ok(())
    }
}
