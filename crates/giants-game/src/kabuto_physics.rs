//! Kabuto's acquired post-prefix physical generation refresh and size tail.
//! Construction/parent links, world ticking and movement remain separate phases.
use crate::{actor_fields::Fields, kabuto_size};
use giants_formats::global_data::GlobalData;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Physics {
    words: [u32; 15],
}
impl Physics {
    pub(crate) fn from_owned_payload(payload: &[u8]) -> Self {
        Self {
            words: std::array::from_fn(|i| word(payload, 0xe4 + i * 4)),
        }
    }
    pub fn words(self) -> [u32; 15] {
        self.words
    }
}
pub struct Refresh<'a> {
    pub generation: u32,
    pub present: bool,
    pub globals: &'a GlobalData,
    pub size: Option<kabuto_size::Source<'a>>,
}
pub struct Prepared {
    pub fields: Fields,
    pub payload: Vec<u8>,
    pub radius: f32,
    pub physics: Option<Physics>,
    pub refreshed: bool,
}
fn word(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(p[at..at + 4].try_into().unwrap())
}
fn put(p: &mut [u8], at: usize, value: u32) {
    p[at..at + 4].copy_from_slice(&value.to_le_bytes())
}
/// The caller supplies the actual physical-presence token, not a payload pointer.
/// The initialized prefix bit is required. An unchanged generation deliberately
/// retains absent physics too; that result is not a gameplay-readiness signal.
pub fn prepare(
    fields: &Fields,
    payload: &[u8],
    radius: f32,
    context: Refresh<'_>,
) -> Result<Prepared, String> {
    if fields.class != 1 || payload.len() != 844 || word(payload, 0) & 1 == 0 {
        return Err("Kabuto physics requires the acquired debug post-initialization prefix".into());
    }
    let mut words = std::array::from_fn(|i| word(payload, 0xe4 + i * 4));
    if word(payload, 0x120) == context.generation {
        return Ok(Prepared {
            fields: fields.clone(),
            payload: payload.to_vec(),
            radius,
            physics: context.present.then_some(Physics { words }),
            refreshed: false,
        });
    }
    if context
        .size
        .as_ref()
        .is_some_and(|s| !std::ptr::eq(s.globals, context.globals))
    {
        return Err("Kabuto physical and size parameters require the same globals".into());
    }
    let g = |at| context.globals.scalar(at).map_err(|e| e.to_string());
    let gravity_mass = g(0x6b4)?;
    let mass = g(0x6b8)?;
    if mass <= 0. {
        return Err("Kabuto physical refresh requires positive mass".into());
    }
    let values = [gravity_mass, mass, g(0x5f8)?, g(0x604)?, g(0x608)?];
    if !context.present {
        words = [0; 15];
        crate::physical_initial::reset(&mut words);
    }
    for (index, value) in [1, 2, 10, 3, 12].into_iter().zip(values) {
        words[index] = value.to_bits();
    }
    words[13] = 0;
    words[14] = 0;
    words[0] |= 0x400;
    let ratio = ((f64::from(gravity_mass) / f64::from(mass)) * f64::from(-32.17405f32)) as f32;
    if !ratio.is_finite() {
        return Err("Kabuto physical ratio overflow".into());
    }
    let mut body = payload.to_vec();
    for (i, value) in words.into_iter().enumerate() {
        put(&mut body, 0xe4 + i * 4, value);
    }
    put(&mut body, 0x120, context.generation);
    put(&mut body, 0x144, 0);
    put(&mut body, 0x148, 0);
    put(&mut body, 0x14c, ratio.to_bits());
    let size = kabuto_size::prepare(fields, &body, radius, context.size)?;
    Ok(Prepared {
        fields: size.fields,
        payload: size.payload,
        radius: size.radius,
        physics: Some(Physics { words }),
        refreshed: true,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fields() -> Fields {
        Fields {
            class: 1,
            dispatch_context: 0,
            configuration: [0; 30],
            scale: 1.,
            object_flags: 0,
            response_flags: 0,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: Default::default(),
            channels: [[0; 9]; 4],
            position: [0.; 3],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        }
    }
    fn payload() -> Vec<u8> {
        let mut p = vec![0; 844];
        put(&mut p, 0, 1);
        put(&mut p, 0x120, 7);
        p
    }
    fn globals() -> GlobalData {
        GlobalData {
            words: vec![1f32.to_bits(); 0x700 / 4],
            array_words: vec![],
        }
    }
    #[test]
    fn unchanged_generation_retains_absence_and_does_not_read_unused_globals() {
        let f = fields();
        let p = payload();
        let g = GlobalData {
            words: vec![],
            array_words: vec![],
        };
        let out = prepare(
            &f,
            &p,
            3.,
            Refresh {
                generation: 7,
                present: false,
                globals: &g,
                size: None,
            },
        )
        .unwrap();
        assert!(!out.refreshed);
        assert!(out.physics.is_none());
        assert_eq!(out.payload, p);
        assert_eq!(out.radius, 3.);
    }
    #[test]
    fn fresh_reset_discards_inline_garbage_but_refresh_preserves_retained_words() {
        let f = fields();
        let mut p = payload();
        let g = globals();
        put(&mut p, 0xe4 + 5 * 4, 0xdeadbeef);
        put(&mut p, 0xe4 + 6 * 4, 0x12345678);
        let a = prepare(
            &f,
            &p,
            3.,
            Refresh {
                generation: 8,
                present: false,
                globals: &g,
                size: None,
            },
        )
        .unwrap();
        assert_eq!(a.physics.unwrap().words()[5], 0);
        assert_eq!(a.physics.unwrap().words()[4], 1f32.to_bits());
        let b = prepare(
            &f,
            &p,
            3.,
            Refresh {
                generation: 8,
                present: true,
                globals: &g,
                size: None,
            },
        )
        .unwrap();
        assert_eq!(b.physics.unwrap().words()[5], 0xdeadbeef);
        assert_eq!(b.physics.unwrap().words()[6], 0x12345678);
        assert_eq!(word(&b.payload, 0x120), 8);
        assert_eq!(b.physics.unwrap().words()[0] & 0x400, 0x400);
        assert_eq!(word(&p, 0x120), 7);
    }
    #[test]
    fn invalid_mass_refuses_without_mutating_the_caller() {
        let f = fields();
        let p = payload();
        let mut g = globals();
        g.words[0x6b8 / 4] = 0;
        assert!(
            prepare(
                &f,
                &p,
                3.,
                Refresh {
                    generation: 8,
                    present: false,
                    globals: &g,
                    size: None
                }
            )
            .is_err()
        );
        assert_eq!(p, payload());
    }
}
