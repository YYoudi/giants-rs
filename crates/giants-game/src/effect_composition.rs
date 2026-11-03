//! A bounded multi-object visual inspection profile. Immediate children remain
//! explicit unresolved work; this does not execute their projectile behavior.
use giants_formats::effect_program::{Program, Record};
#[derive(Clone, Debug)]
pub struct Immediate {
    pub reference: u32,
    pub name: String,
    pub operand: u32,
}
#[derive(Clone, Debug)]
pub struct Object {
    pub attached: bool,
    pub hidden: bool,
    pub timed: crate::timed_effect::Recipe,
    pub immediate: Vec<Immediate>,
}
pub fn project(program: &Program) -> Result<Vec<Object>, &'static str> {
    let mut records = program.records.as_slice();
    let mut objects = Vec::new();
    while records.len() > 1 {
        if objects.len() >= 64 {
            return Err("composite object budget");
        }
        let end = records
            .iter()
            .position(|r| r.opcode == 7)
            .ok_or("composite object has no end")?;
        let (block, tail) = records.split_at(end + 1);
        let hidden = block.len() >= 3 && block[block.len() - 2].opcode == 0x21;
        if block[..block.len() - usize::from(hidden) - 1]
            .iter()
            .any(|r| r.opcode == 0x21)
        {
            return Err("composite hidden marker outside final position");
        }
        let filtered = block
            .iter()
            .filter(|r| r.opcode != 0x21)
            .cloned()
            .collect::<Vec<_>>();
        let block = filtered.as_slice();
        let ops = block.iter().map(|r| r.opcode).collect::<Vec<_>>();
        let attached = matches!(ops.first(), Some(0x24));
        let mut immediate = Vec::new();
        let timed = if attached
            && ops.len() >= 4
            && ops[1] == 6
            && ops[ops.len() - 2..] == [4, 7]
            && ops[2..ops.len() - 2].iter().all(|op| *op == 0x10)
        {
            if word(&block[0], 9)? != 0 {
                return Err("composite attachment reference unsupported");
            }
            for child in &block[2..block.len() - 2] {
                let end = child
                    .bytes
                    .get(5..)
                    .ok_or("truncated composite child name")?
                    .iter()
                    .position(|b| *b == 0)
                    .ok_or("composite child name unterminated")?
                    + 5;
                if end == 5 || end + 5 != child.bytes.len() {
                    return Err("composite child operands invalid");
                }
                immediate.push(Immediate {
                    reference: word(child, 1)?,
                    name: String::from_utf8(child.bytes[5..end].to_vec())
                        .map_err(|_| "composite child name invalid")?,
                    operand: word(child, end + 1)?,
                });
            }
            let life = &block[block.len() - 2];
            crate::timed_effect::Recipe {
                class: word(&block[0], 5)? as i32,
                opaque: word(&block[0], 1)?,
                range: [1.; 2],
                sample_size: false,
                lifetime: f32::from_bits(word(life, 1)?),
                fade_in: f32::from_bits(word(life, 5)?),
                fade_out: f32::from_bits(word(life, 9)?),
                child: String::new(),
            }
        } else if ops == [3, 6, 0x1c, 4, 0x0d, 7] {
            // Reuse the measured size/lifetime/retirement decoder; creation is
            // still detached and is retained separately from that decoder.
            let mut bytes = vec![0x24];
            bytes.extend(0u32.to_le_bytes());
            bytes.extend(word(&block[0], 1)?.to_le_bytes());
            bytes.extend(0u32.to_le_bytes());
            for record in &block[1..] {
                bytes.extend(&record.bytes);
            }
            bytes.push(2);
            crate::timed_effect::Recipe::project(
                &giants_formats::effect_program::parse(&bytes)
                    .map_err(|_| "composite timed framing invalid")?,
            )?
        } else {
            return Err("composite object commands outside visual profile");
        };
        if timed.class < 0 {
            return Err("composite class outside signed profile");
        }
        let mut validation_seed = 1;
        timed.create(&mut validation_seed)?;
        objects.push(Object {
            attached,
            hidden,
            timed,
            immediate,
        });
        records = tail;
    }
    if objects.is_empty()
        || objects.len() == 1
            && !(objects[0].attached && objects[0].hidden && !objects[0].immediate.is_empty())
        || records.len() != 1
        || records[0].opcode != 2
    {
        return Err("composite profile requires checked terminated objects");
    }
    Ok(objects)
}
fn word(record: &Record, at: usize) -> Result<u32, &'static str> {
    Ok(u32::from_le_bytes(
        record
            .bytes
            .get(at..at + 4)
            .ok_or("truncated composite operand")?
            .try_into()
            .map_err(|_| "composite word invalid")?,
    ))
}
#[cfg(test)]
mod tests {
    #[test]
    fn single_hidden_support_requires_an_immediate_child_and_final_hidden_marker() {
        let mut bytes = vec![0x24];
        for w in [21u32, 900, 0] {
            bytes.extend(w.to_le_bytes());
        }
        bytes.extend([6, 0x10]);
        bytes.extend(0u32.to_le_bytes());
        bytes.extend(b"child\0");
        bytes.extend(0u32.to_le_bytes());
        bytes.push(4);
        for v in [1f32, 0., 1.] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend([0x21, 7, 2]);
        let p = giants_formats::effect_program::parse(&bytes).unwrap();
        let objects = super::project(&p).unwrap();
        assert_eq!(objects.len(), 1);
        assert!(objects[0].hidden);
        assert_eq!(objects[0].immediate.len(), 1);
        let mut visible = p.clone();
        visible.records.retain(|r| r.opcode != 0x21);
        assert!(super::project(&visible).is_err());
        let mut misplaced = p;
        let hidden = misplaced.records.remove(4);
        misplaced.records.insert(2, hidden);
        assert!(super::project(&misplaced).is_err());
    }
    #[test]
    fn detached_lifetime_and_unresolved_child_are_kept_separate() {
        let mut bytes = vec![0x24];
        for w in [240u32, 900, 0] {
            bytes.extend(w.to_le_bytes());
        }
        bytes.extend([6, 0x10]);
        bytes.extend(3u32.to_le_bytes());
        bytes.extend(b"synthetic_child\0");
        bytes.extend(0u32.to_le_bytes());
        bytes.push(4);
        for v in [1f32, 0., 1.] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend([7, 3]);
        bytes.extend(901u32.to_le_bytes());
        bytes.extend([6, 0x1c]);
        for v in [6f32, 0.01] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.push(4);
        for v in [0.05f32, 0., 0.001] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.push(0x0d);
        bytes.extend(b"retirement\0");
        bytes.extend([7, 2]);
        let p = giants_formats::effect_program::parse(&bytes).unwrap();
        let objects = super::project(&p).unwrap();
        assert!(objects[0].attached);
        assert!(!objects[1].attached);
        assert_eq!(objects[0].immediate[0].reference, 3);
        assert_eq!(objects[0].immediate[0].name, "synthetic_child");
        assert_eq!(objects[1].timed.child, "retirement");
        let mut seed = 1;
        objects[0].timed.create(&mut seed).unwrap();
        assert_eq!(seed, 1);
        objects[1].timed.create(&mut seed).unwrap();
        assert_ne!(seed, 1);
        let mut bad = p;
        bad.records[2].opcode = 0x1e;
        assert!(super::project(&bad).is_err());
    }
}
