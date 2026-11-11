//! Shared last-noise publication. Emission callers and mission clock are separate
//! producers; publishing does not play sound or decide an actor's threat response.
#[derive(Clone, Copy, Debug)]
pub struct Noise<I> {
    pub owner: Option<I>,
    pub seconds: f32,
    pub position: [f32; 3],
}
impl<I> Default for Noise<I> {
    fn default() -> Self {
        Self {
            owner: None,
            seconds: 0.,
            position: [0.; 3],
        }
    }
}
impl<I> Noise<I> {
    /// Word-preserving publication of the current owner, clock and actor position.
    /// Callers must validate ownership; no pointer identity or coordinate conversion
    /// is manufactured here. The latest emission replaces the previous one.
    pub fn publish(&mut self, owner: I, seconds: f32, position: [f32; 3]) {
        self.owner = Some(owner);
        self.seconds = seconds;
        self.position = position;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn last_emission_retains_exact_words_and_owner() {
        let mut noise = Noise::default();
        assert_eq!(noise.owner, None);
        let words = [0x80000000, 0x7fc01234, 0xff800000, 0x00000001];
        noise.publish(
            7_u64,
            f32::from_bits(words[0]),
            <[u32; 3]>::try_from(&words[1..])
                .unwrap()
                .map(f32::from_bits),
        );
        assert_eq!(noise.owner, Some(7));
        assert_eq!(noise.seconds.to_bits(), words[0]);
        assert_eq!(noise.position.map(f32::to_bits), words[1..]);
        noise.publish(8, 2., [1., 2., 3.]);
        assert_eq!(
            (noise.owner, noise.seconds, noise.position),
            (Some(8), 2., [1., 2., 3.])
        );
    }
}
