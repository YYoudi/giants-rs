//! World companion ABX bounds: ordered specification and animation overrides.
use crate::{Error, slice, u32_at};
#[derive(Clone, Debug)]
pub struct Record {
    pub spec: u32,
    pub primary: [f32; 6],
    pub animations: Vec<(u8, [f32; 6])>,
}
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub records: Vec<Record>,
}
#[derive(Clone, Debug)]
pub struct Resolved {
    pub primary: [f32; 6],
    pub animation: Option<[f32; 6]>,
}
impl Table {
    /// Resolve a declared wrapper after global model alias overrides have run.
    pub fn animation_bounds(&self, spec: u32, slot: u8, primary: [f32; 6]) -> [f32; 6] {
        let mut bounds = [0.; 6];
        for record in self.records.iter().filter(|r| r.spec == spec) {
            for (at, value) in &record.animations {
                if *at == slot {
                    bounds = *value;
                }
            }
        }
        if (0..3).all(|i| bounds[i] == bounds[i + 3]) {
            primary
        } else {
            bounds
        }
    }
    /// Isolated unaliased specification control. Production resolves shared model
    /// identities before calling animation_bounds. Retain slots not overwritten.
    /// The caller establishes whether a wrapper exists; absent slots stay absent.
    /// Degenerate animation extents inherit the final primary bounds.
    pub fn resolve(&self, spec: u32, primary: [f32; 6], animation: Option<u8>) -> Resolved {
        let mut out = Resolved {
            primary,
            animation: animation.map(|_| [0.; 6]),
        };
        for record in self.records.iter().filter(|r| r.spec == spec) {
            out.primary = record.primary;
            if let Some(slot) = animation {
                for (at, bounds) in &record.animations {
                    if *at == slot {
                        out.animation = Some(*bounds);
                    }
                }
            }
        }
        if let Some(bounds) = out.animation.as_mut()
            && (0..3).all(|i| bounds[i] == bounds[i + 3])
        {
            *bounds = out.primary;
        }
        out
    }
}
pub fn parse(data: &[u8]) -> Result<Table, Error> {
    if data.len() > 16 * 1024 * 1024 || u32_at(data, 0)? != 0x1a0002dd {
        return Err(Error("world bounds header or budget"));
    }
    let mut at = 4;
    let mut records = Vec::new();
    let word = |at: &mut usize| -> Result<u32, Error> {
        let v = u32_at(data, *at)?;
        *at += 4;
        Ok(v)
    };
    let bounds = |at: &mut usize| -> Result<[f32; 6], Error> {
        let raw = slice(data, *at, 24)?;
        *at += 24;
        let v =
            std::array::from_fn(|i| f32::from_le_bytes(raw[i * 4..i * 4 + 4].try_into().unwrap()));
        if !v.iter().all(|v| v.is_finite()) {
            return Err(Error("non-finite world bounds"));
        }
        Ok(v)
    };
    while at < data.len() {
        if records.len() >= 65536 {
            return Err(Error("world bounds record budget"));
        }
        let spec = word(&mut at)?;
        if spec == u32::MAX {
            if at != data.len() {
                return Err(Error("world bounds trailing bytes"));
            }
            break;
        }
        let primary = bounds(&mut at)?;
        let mut animations = Vec::new();
        loop {
            let slot = word(&mut at)?;
            if slot == u32::MAX {
                break;
            }
            if slot >= 254 || animations.len() >= 65536 {
                return Err(Error("world bounds animation slot or budget"));
            }
            animations.push((slot as u8, bounds(&mut at)?));
        }
        records.push(Record {
            spec,
            primary,
            animations,
        });
    }
    Ok(Table { records })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_overrides_retain_other_slots_and_fallback_after_last_record() {
        let t = Table {
            records: vec![
                Record {
                    spec: 7,
                    primary: [-1.; 6],
                    animations: vec![(1, [2.; 6]), (2, [-3., -2., -1., 4., 5., 6.])],
                },
                Record {
                    spec: 7,
                    primary: [-6., -5., -4., 3., 2., 1.],
                    animations: vec![],
                },
            ],
        };
        assert_eq!(
            t.resolve(7, [0.; 6], Some(1)).animation,
            Some([-6., -5., -4., 3., 2., 1.])
        );
        assert_eq!(
            t.resolve(7, [0.; 6], Some(2)).animation,
            Some([-3., -2., -1., 4., 5., 6.])
        );
        assert_eq!(t.resolve(7, [0.; 6], None).animation, None);
    }
    #[test]
    fn eof_is_valid_only_between_complete_records() {
        let mut b = 0x1a0002ddu32.to_le_bytes().to_vec();
        b.extend(7u32.to_le_bytes());
        b.extend([0; 24]);
        b.extend(u32::MAX.to_le_bytes());
        assert_eq!(parse(&b).unwrap().records.len(), 1);
        assert!(parse(&b[..b.len() - 4]).is_err());
        b.extend(u32::MAX.to_le_bytes());
        assert!(parse(&b).is_ok());
        b.push(0);
        assert!(parse(&b).is_err());
    }
}
