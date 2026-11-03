//! Bounded detached bursts separated by source waits. The caller supplies each
//! context callback; this does not establish native frame cadence or ownership.
use giants_formats::{effect_image::Image, effect_program::Record};

#[derive(Clone, Debug)]
pub struct Phase {
    pub plan: crate::effect_burst::Plan,
    pub wait_after: Option<f32>,
}
#[derive(Clone, Debug)]
pub struct Sequence {
    pub phases: Vec<Phase>,
}
#[derive(Clone, Debug)]
pub struct Cursor {
    pub next: usize,
    pub remaining: Option<f32>,
}
impl Sequence {
    pub fn project(image: &Image, name: &str) -> Result<Self, &'static str> {
        let (_, entry) = image.find(name).ok_or("effect sequence absent")?;
        let program = image
            .program(entry.offset)
            .map_err(|_| "effect sequence framing invalid")?;
        let mut phases = Vec::new();
        let mut start = 0;
        let mut inside = false;
        for (i, record) in program.records.iter().enumerate() {
            match record.opcode {
                3 => inside = true,
                7 => inside = false,
                0x2b if start != 0 => return Err("sound selector after wait not checked"),
                0x0b | 2 => {
                    if inside || i == start || phases.len() >= 16 {
                        return Err("effect wait outside bounded context profile");
                    }
                    let wait_after = if record.opcode == 0x0b {
                        let value = f32::from_le_bytes(
                            record.bytes[1..5]
                                .try_into()
                                .map_err(|_| "wait truncated")?,
                        );
                        if !value.is_finite() || value < 0. {
                            return Err("effect wait outside nonnegative profile");
                        }
                        Some(value)
                    } else {
                        None
                    };
                    let mut body = program.records[start..i].to_vec();
                    body.push(Record {
                        offset: record.offset,
                        opcode: 2,
                        bytes: vec![2],
                    });
                    let plan = crate::effect_burst::Plan::from_records(image, &body, 1)?;
                    phases.push(Phase { plan, wait_after });
                    start = i + 1;
                }
                _ => {}
            }
        }
        if phases.len() < 2 || phases.last().is_none_or(|p| p.wait_after.is_some()) {
            return Err("effect sequence requires wait and terminal phase");
        }
        Ok(Self { phases })
    }
    /// Preparation of named children occurs when their phase activates. No RNG
    /// is consumed for future phases here.
    pub fn start(
        &self,
        seed: &mut u32,
    ) -> Result<(Vec<crate::effect_burst::Spawn>, Cursor), &'static str> {
        let phase = self.phases.first().ok_or("empty effect sequence")?;
        Ok((
            phase.plan.create(seed)?,
            Cursor {
                next: 1,
                remaining: phase.wait_after,
            },
        ))
    }
}
impl Cursor {
    /// A wait ending at zero resumes the interpreter. Positive rounded
    /// remainders stay pending; a subsequent wait replaces the overshoot.
    pub fn advance(
        &mut self,
        sequence: &Sequence,
        dt: f32,
        seed: &mut u32,
    ) -> Result<Option<Vec<crate::effect_burst::Spawn>>, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid effect context delta");
        }
        let Some(remaining) = &mut self.remaining else {
            return Ok(None);
        };
        *remaining -= dt;
        if *remaining > 0. {
            return Ok(None);
        }
        let phase = sequence
            .phases
            .get(self.next)
            .ok_or("effect phase absent")?;
        let spawns = phase.plan.create(seed)?;
        self.next += 1;
        self.remaining = phase.wait_after;
        Ok(Some(spawns))
    }
}
#[cfg(test)]
mod tests {
    fn image(waits: &[f32]) -> giants_formats::effect_image::Image {
        let mut code = Vec::new();
        for i in 0..=waits.len() {
            code.push(3);
            code.extend(900u32.to_le_bytes());
            code.extend([6, 8]);
            for value in [2f32, 8.] {
                code.extend(value.to_le_bytes());
            }
            code.push(4);
            for value in [1f32, 0., 0.5] {
                code.extend(value.to_le_bytes());
            }
            code.push(7);
            if let Some(wait) = waits.get(i) {
                code.push(0x0b);
                code.extend(wait.to_le_bytes());
            }
        }
        code.push(2);
        let mut payload = b"s\0".to_vec();
        payload.extend(6u32.to_le_bytes());
        payload.extend(code);
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend((payload.len() as u32).to_le_bytes());
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(payload);
        giants_formats::effect_image::parse(&bytes).unwrap()
    }
    #[test]
    fn exact_zero_resumes_but_positive_float_residual_waits() {
        let sequence = super::Sequence::project(&image(&[0.2]), "s").unwrap();
        let mut seed = 1;
        let (first, mut cursor) = sequence.start(&mut seed).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].visual.end_size, 8.);
        assert!(cursor.advance(&sequence, 0., &mut seed).unwrap().is_none());
        assert_eq!(
            cursor
                .advance(&sequence, 0.2, &mut seed)
                .unwrap()
                .unwrap()
                .len(),
            1
        );
        assert!(cursor.remaining.is_none());
        let (_, mut cursor) = sequence.start(&mut seed).unwrap();
        for _ in 0..4 {
            assert!(
                cursor
                    .advance(&sequence, 0.05, &mut seed)
                    .unwrap()
                    .is_none()
            );
        }
        assert!(cursor.remaining.unwrap() > 0.);
        assert!(
            cursor
                .advance(&sequence, 0.05, &mut seed)
                .unwrap()
                .is_some()
        );
    }
    #[test]
    fn invalid_wait_and_delta_fail_without_advancing() {
        for wait in [-0.1, f32::NAN, f32::INFINITY] {
            assert!(super::Sequence::project(&image(&[wait]), "s").is_err());
        }
        let sequence = super::Sequence::project(&image(&[0.2]), "s").unwrap();
        let mut seed = 1;
        let (_, mut cursor) = sequence.start(&mut seed).unwrap();
        for dt in [-0.1, f32::NAN] {
            assert!(cursor.advance(&sequence, dt, &mut seed).is_err());
            assert_eq!(cursor.remaining, Some(0.2));
            assert_eq!(cursor.next, 1);
        }
    }
}
