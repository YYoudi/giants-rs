//! Source text-block presentation before font measurement and glyph drawing.
//! A block is selected by the director cursor and replayed at render boundaries.
use crate::arithmetic::Precision;
use giants_formats::flick::Program;

#[derive(Clone, Debug)]
pub struct Frame {
    pub command_offset: usize,
    /// Translation lookup input; some source blocks contain literal multiline
    /// text rather than a localization key. Preserve its bytes and newlines.
    pub key: String,
    /// Exact source render-info words before the font-layout boundary:
    /// alignment xy, scales xy, position xy, rate, phase, character phase/step,
    /// opacity rate/value and packed RGB.
    pub words: [u32; 13],
}

impl Frame {
    /// Exact-key lookup using the existing decoded GText table. A missing entry
    /// preserves the source literal. Native table matching policy is separate.
    pub fn resolve<'a>(
        &'a self,
        table: &'a std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> &'a [u8] {
        table
            .get(self.key.as_bytes())
            .map_or(self.key.as_bytes(), Vec::as_slice)
    }
}

#[derive(Clone, Debug, Default)]
pub struct State {
    block: Option<usize>,
    words: [u32; 13],
}

impl State {
    pub fn block(&self) -> Option<usize> {
        self.block
    }

    /// Command14 resets the source text transport, opacity and default color.
    /// Other render-info fields are supplied by the selected block, not guessed.
    pub fn select(&mut self, offset: usize, program: &Program) -> Result<(), &'static str> {
        let block = program
            .commands
            .iter()
            .position(|c| c.offset == offset)
            .ok_or("text block command absent")?;
        if program.commands[block].opcode != 14 {
            return Err("text selection requires command14");
        }
        self.block = Some(offset);
        self.words[7] = 0;
        self.words[10] = 0;
        self.words[11] = 1f32.to_bits();
        self.words[12] = 0xffff;
        Ok(())
    }

    /// One-text blocks are the measured source profile. Multiple text commands
    /// need real font metrics because layout mutates position/alignment between
    /// calls. They remain explicitly refused here.
    pub fn render(
        &mut self,
        program: &Program,
        dt: f32,
        precision: Precision,
    ) -> Result<Option<Frame>, &'static str> {
        if !dt.is_finite() || !(0. ..=1.).contains(&dt) {
            return Err("text render delta outside profile");
        }
        let Some(offset) = self.block else {
            return Ok(None);
        };
        let start = program
            .commands
            .iter()
            .position(|c| c.offset == offset)
            .ok_or("selected text block absent")?;
        let mut words = self.words;
        let mut frame = None;
        let mut supplied = [false; 6];
        for command in &program.commands[start..] {
            let word = |index: usize| {
                command
                    .words
                    .get(index)
                    .copied()
                    .ok_or("text command word absent")
            };
            let scalar = |index: usize| -> Result<f32, &'static str> {
                let value = f32::from_bits(word(index)?);
                if !value.is_finite() {
                    return Err("text scalar outside finite profile");
                }
                Ok(value)
            };
            match command.opcode {
                14 if command.offset == offset => words[8] = words[7],
                14 => return Err("nested text block outside profile"),
                15 => {
                    if frame.is_none() {
                        return Err("empty text block needs font lifecycle policy");
                    }
                    words[7] = (precision.round(
                        precision.round(f64::from(dt) * f64::from(f32::from_bits(words[6])))
                            + f64::from(f32::from_bits(words[7])),
                    ) as f32)
                        .to_bits();
                    // Store the transport separately from the per-character
                    // cursor consumed by the actual layout service.
                    let opacity = precision.round(
                        f64::from(f32::from_bits(words[11]))
                            - precision.round(f64::from(dt) * f64::from(f32::from_bits(words[10]))),
                    );
                    words[11] = (opacity as f32).to_bits();
                    words[8] = words[7];
                    self.words = words;
                    return Ok(frame);
                }
                16 => {
                    if frame.is_some() {
                        return Err("multiple text calls require font layout metrics");
                    }
                    if !supplied.into_iter().all(|value| value) {
                        return Err("text block requires inherited font layout fields");
                    }
                    frame = Some(Frame {
                        command_offset: command.offset,
                        key: command.label.clone().ok_or("text key absent")?,
                        words,
                    });
                }
                17..=20 => {
                    let index = match command.opcode {
                        17 => 4,
                        18 => 5,
                        19 => 2,
                        _ => 3,
                    };
                    scalar(0)?;
                    words[index] = word(0)?;
                    supplied[index] = true;
                }
                21 => {
                    for i in 0..2 {
                        scalar(i)?;
                        words[i] = word(i)?;
                        supplied[i] = true;
                    }
                }
                22 => {
                    let duration = scalar(0)?;
                    if f32::from_bits(words[8]) >= 0. {
                        let rate = if duration == 0. {
                            1000.
                        } else {
                            precision.round(1. / f64::from(duration)) as f32
                        };
                        if !rate.is_finite() {
                            return Err("text rate overflow");
                        }
                        words[6] = rate.to_bits();
                    }
                }
                23 => {
                    scalar(0)?;
                    words[9] = word(0)?;
                }
                24 => {
                    words[8] = (precision
                        .round(f64::from(f32::from_bits(words[8])) + f64::from(scalar(0)?))
                        as f32)
                        .to_bits();
                }
                41 => {
                    let mut rgb = 0;
                    for i in 0..3 {
                        let channel = scalar(i)?;
                        if !(0. ..=1.).contains(&channel) {
                            return Err("text color outside unit profile");
                        }
                        rgb = (rgb << 8) | (precision.round(f64::from(channel) * 255.) as u32);
                    }
                    words[12] = rgb;
                }
                5 => return Err("text block missing command15"),
                // The text cursor ignores director/audio/actor commands.
                _ => {}
            }
        }
        Err("text block missing command15")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use giants_formats::flick::{Command, Program};

    fn program() -> Program {
        let mut commands = Vec::new();
        for (opcode, words, label) in [
            (14, vec![], None),
            (17, vec![0.5f32.to_bits()], None),
            (18, vec![0.94f32.to_bits()], None),
            (19, vec![0.04f32.to_bits()], None),
            (20, vec![0.04f32.to_bits()], None),
            (21, vec![0.5f32.to_bits(), 0.5f32.to_bits()], None),
            (22, vec![1f32.to_bits()], None),
            (23, vec![0], None),
            (16, vec![500], Some("fixture_key".to_owned())),
            (15, vec![], None),
            (5, vec![], None),
        ] {
            let offset = commands
                .last()
                .map_or(24, |c: &Command| c.offset + 8 + c.words.len() * 4);
            commands.push(Command {
                offset,
                opcode,
                words,
                label,
            });
        }
        Program {
            commands,
            next: None,
            program_end: 500,
        }
    }

    #[test]
    fn text_transport_persists_until_another_block_and_new_directors_start_empty() {
        let p = program();
        let mut state = State::default();
        assert!(
            state
                .render(&p, 0.1, Precision::Double53)
                .unwrap()
                .is_none()
        );
        state.select(24, &p).unwrap();
        let first = state
            .render(&p, 0.25, Precision::Double53)
            .unwrap()
            .unwrap();
        let second = state
            .render(&p, 0.25, Precision::Double53)
            .unwrap()
            .unwrap();
        assert_eq!(first.words[7], 0);
        assert_eq!(second.words[7], 0.25f32.to_bits());
        state.select(24, &p).unwrap();
        assert_eq!(
            state
                .render(&p, 0., Precision::Double53)
                .unwrap()
                .unwrap()
                .words[7],
            0
        );
        assert!(
            State::default()
                .render(&p, 0., Precision::Double53)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn director_cursor_selects_the_block_once_and_reset_discards_its_transport() {
        let p = program();
        let mut timeline = crate::flick::Timeline::new(p, 160).unwrap();
        assert_eq!(timeline.text_block(), None);
        timeline.step(160).unwrap();
        assert_eq!(timeline.text_block(), Some(24));
        timeline.render_text(0.25, Precision::Double53).unwrap();
        // Reaching command5 again does not dispatch command14 again.
        timeline.step(320).unwrap();
        let frame = timeline
            .render_text(0.25, Precision::Double53)
            .unwrap()
            .unwrap();
        assert_eq!(frame.words[7], 0.25f32.to_bits());
        timeline.reset();
        assert_eq!(timeline.text_block(), None);
    }

    #[test]
    fn refusal_does_not_commit_partial_transport_or_select_an_unrelated_command() {
        let mut p = program();
        let mut state = State::default();
        state.select(24, &p).unwrap();
        let before = state.words;
        assert!(state.render(&p, f32::NAN, Precision::Double53).is_err());
        assert_eq!(state.words, before);
        let offset = p.commands[1].offset;
        assert!(state.select(offset, &p).is_err());
        assert_eq!(state.block(), Some(24));
        let mut second = p.commands[8].clone();
        second.offset += 1;
        p.commands.insert(9, second);
        assert!(state.render(&p, 0.25, Precision::Double53).is_err());
        assert_eq!(state.words, before);
    }

    #[test]
    fn literal_multiline_fallback_and_translation_keep_the_source_bytes() {
        let frame = Frame {
            command_offset: 24,
            key: "line one\nline two".into(),
            words: [0; 13],
        };
        let mut table = std::collections::BTreeMap::new();
        assert_eq!(frame.resolve(&table), b"line one\nline two");
        table.insert(frame.key.as_bytes().to_vec(), b"translated\xe9".to_vec());
        assert_eq!(frame.resolve(&table), b"translated\xe9");
    }
}
