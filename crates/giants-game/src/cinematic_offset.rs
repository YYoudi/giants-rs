//! FLK actor-local offset fields. Short records preserve earlier components;
//! angles use the source ordering, distinct from ordinary object Euler inputs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Offset {
    pub position: [f32; 3],
    pub angles: [f32; 3],
    pub enabled: bool,
}
impl Offset {
    pub fn apply(&mut self, words: &[u32]) -> Result<(), &'static str> {
        if !matches!(words.len(), 3 | 4 | 5 | 7) {
            return Err("unsupported actor offset record");
        }
        let values: Vec<f32> = words[..words.len() - 1]
            .iter()
            .copied()
            .map(f32::from_bits)
            .collect();
        if !values.iter().all(|v| v.is_finite()) {
            return Err("non-finite actor offset field");
        }
        let mut next = *self;
        next.position[..2].copy_from_slice(&values[..2]);
        if words.len() >= 4 {
            next.position[2] = values[2];
        }
        if words.len() >= 5 {
            next.angles[2] = values[3];
        }
        if words.len() == 7 {
            next.angles[1] = values[4];
            next.angles[0] = values[5];
        }
        next.enabled = true;
        next.matrix(1.)?;
        *self = next;
        Ok(())
    }
    pub fn matrix(self, scale: f32) -> Result<([f32; 16], u32), &'static str> {
        if !scale.is_finite() || scale <= 0. {
            return Err("invalid actor offset scale");
        }
        if !self.enabled {
            return crate::attachments::unparented_matrix(scale)
                .map_err(|_| "invalid actor offset scale");
        }
        let local = crate::object_transform::euler(
            self.position,
            [self.angles[2], self.angles[1], self.angles[0]],
            1.,
        )?;
        let identity = crate::attachments::unparented_matrix(1.)
            .map_err(|_| "invalid identity")?
            .0;
        let scale = crate::attachments::unparented_matrix(scale)
            .map_err(|_| "invalid actor offset scale")?
            .0;
        // Preserve both native stores, including signed zeros from the first
        // identity/local multiplication, before multiplying the class scale.
        let matrix =
            crate::attachments::multiply(crate::attachments::multiply(identity, local), scale);
        Ok((matrix, 0x13))
    }
}
