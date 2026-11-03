//! Acquired message storage and age selection. Rendering, input modes and the
//! mission's live clock remain consumers; queue acceptance is not display proof.
pub const CAPACITY: usize = 25;
pub const LINE_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queue {
    lines: [[u8; LINE_BYTES]; CAPACITY],
    time_bits: [u32; CAPACITY],
    routes: [u32; CAPACITY],
}
impl Default for Queue {
    fn default() -> Self {
        Self {
            lines: [[0; LINE_BYTES]; CAPACITY],
            time_bits: [0; CAPACITY],
            routes: [0; CAPACITY],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub first: usize,
    pub last: Option<usize>,
    /// Native flag written when a recent/history range exists and alternate is off.
    pub indicator: bool,
}
impl Queue {
    /// Diagnostic serialization preserving all padding, timestamps and routes.
    pub fn snapshot(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(CAPACITY * (LINE_BYTES + 8));
        for line in &self.lines {
            bytes.extend_from_slice(line);
        }
        for word in self.time_bits.iter().chain(&self.routes) {
            bytes.extend(word.to_le_bytes());
        }
        bytes
    }
    /// Restore only a bounded serialized state, not a save-game format.
    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != CAPACITY * (LINE_BYTES + 8) {
            return Err("message storage size");
        }
        let mut result = Self::default();
        for (i, line) in result.lines.iter_mut().enumerate() {
            line.copy_from_slice(&bytes[i * LINE_BYTES..(i + 1) * LINE_BYTES]);
            if line[LINE_BYTES - 1] != 0 {
                return Err("unterminated message storage");
            }
        }
        for (i, word) in result
            .time_bits
            .iter_mut()
            .chain(&mut result.routes)
            .enumerate()
        {
            let at = CAPACITY * LINE_BYTES + i * 4;
            *word = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        }
        Ok(result)
    }
    /// The native writer accepts one C string. Empty/null becomes a space;
    /// nonempty text is truncated/padded to 255 bytes with a final terminator.
    /// Finite supplied clock only; nonfinite post times are outside this profile.
    pub fn post(&mut self, text: Option<&[u8]>, route: u32, now: f32) -> Result<(), &'static str> {
        if !now.is_finite() {
            return Err("nonfinite message post clock");
        }
        let slot = match self.lines.iter().position(|line| line[0] == 0) {
            Some(slot) => slot,
            None => {
                self.lines.copy_within(1..CAPACITY, 0);
                self.time_bits.copy_within(1..CAPACITY, 0);
                self.routes.copy_within(1..CAPACITY, 0);
                self.lines[CAPACITY - 1][0] = 0;
                self.time_bits[CAPACITY - 1] = 0;
                self.routes[CAPACITY - 1] = 0;
                CAPACITY - 1
            }
        };
        let text = text.unwrap_or_default();
        let count = text
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(text.len())
            .min(255);
        if count == 0 {
            // Native empty path writes two bytes only; keep old tail padding.
            self.lines[slot][..2].copy_from_slice(b" \0");
        } else {
            self.lines[slot].fill(0);
            self.lines[slot][..count].copy_from_slice(&text[..count]);
        }
        self.lines[slot][255] = 0;
        self.time_bits[slot] = now.to_bits();
        self.routes[slot] = route;
        Ok(())
    }
    pub fn post_notice(
        &mut self,
        notice: &crate::mission_notice::Notice,
        now: f32,
    ) -> Result<(), String> {
        if notice.text.contains(&0) {
            return Err("notice contains an interior terminator".into());
        }
        self.post(Some(&notice.text), notice.route, now)
            .map_err(str::to_owned)
    }
    /// Native C-locale CPU selector, with its supplied display-mode flags.
    /// Expiry hides rows; it does not delete history or free a storage slot.
    pub fn select(&self, now: f32, history: bool, alternate: bool) -> Selection {
        if history {
            return Selection {
                first: 0,
                last: Some(24),
                indicator: !alternate,
            };
        }
        // x87 evaluates the subtraction before storing any f32 intermediate.
        let age = |i: usize| f64::from(now) - f64::from(f32::from_bits(self.time_bits[i]));
        let newest = (0..CAPACITY)
            .rev()
            .find(|&i| self.lines[i][0] != 0 && age(i) < 6.);
        let Some(newest) = newest else {
            return Selection {
                first: 0,
                last: None,
                indicator: false,
            };
        };
        let mut first = newest.saturating_sub(4);
        let mut last = Some((first + 4).min(24));
        let candidates = first..=last.unwrap();
        for i in candidates {
            if self.lines[i][0] == 0 {
                last = i.checked_sub(1);
                break;
            }
            if age(i) >= 6. {
                first += 1;
            }
        }
        Selection {
            first,
            last,
            indicator: !alternate,
        }
    }
    pub fn line(&self, index: usize) -> Option<(&[u8], u32)> {
        let line = self.lines.get(index)?;
        if line[0] == 0 {
            return None;
        }
        let end = line.iter().position(|&b| b == 0).unwrap();
        Some((&line[..end], self.routes[index]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overflow_shifts_and_expiry_preserves_history() {
        let mut queue = Queue::default();
        for i in 0..30 {
            queue
                .post(Some(format!("line{i}").as_bytes()), i, i as f32)
                .unwrap();
        }
        assert_eq!(queue.line(0), Some((b"line5".as_slice(), 5)));
        assert_eq!(queue.line(24), Some((b"line29".as_slice(), 29)));
        assert_eq!(
            queue.select(35., false, false),
            Selection {
                first: 0,
                last: None,
                indicator: false
            }
        );
        assert_eq!(queue.select(35., true, false).last, Some(24));
        assert_eq!(queue.line(24).unwrap().0, b"line29");
    }
    #[test]
    fn blank_truncation_and_strict_six_second_boundary() {
        let mut queue = Queue::default();
        queue.post(None, 6, 1.).unwrap();
        queue.post(Some(&[0xe9; 300]), 4, 2.).unwrap();
        assert_eq!(queue.line(0), Some((b" ".as_slice(), 6)));
        assert_eq!(queue.line(1).unwrap().0.len(), 255);
        assert_eq!(queue.select(7., false, false).first, 1);
        assert_eq!(queue.select(8., false, false).last, None);
        assert_eq!(queue, Queue::from_snapshot(&queue.snapshot()).unwrap());
        let before = queue.clone();
        assert!(queue.post(None, 6, f32::NAN).is_err());
        assert_eq!(queue, before);
    }
    #[test]
    fn lossless_delivery_accepts_actual_queue_once_without_display_claim() {
        use crate::mission_notice::{Context, Delivery, Gates, State};
        let mut state = State {
            pending: 1,
            counter: 0,
        };
        let mut delivery = Delivery::default();
        let mut queue = Queue::default();
        let mut text = giants_formats::gtext::TextTable::new();
        text.insert(b"mission1".to_vec(), b"synthetic notice".to_vec());
        let catalog = giants_formats::gtext::Catalog::from_table(text).unwrap();
        delivery
            .advance(
                &mut state,
                Some(Context {
                    prefix: b"Mission",
                    gates: Gates {
                        suppress: [0; 3],
                        time: 0.,
                    },
                }),
                |key| Ok(catalog.get(key).map(<[u8]>::to_vec)),
                |notice| queue.post_notice(notice, 3.),
            )
            .unwrap();
        assert_eq!(queue.line(0), Some((b"synthetic notice".as_slice(), 6)));
        delivery
            .advance(
                &mut state,
                None,
                |_| panic!("duplicate lookup"),
                |_| panic!("duplicate post"),
            )
            .unwrap();
        assert!(queue.line(1).is_none());
    }
}
