//! Acquired normal geometry-entry submission packet. Source state producers,
//! registry/world scheduling and an actual GPU backend remain separate.
use crate::{
    animation::simd_trig,
    audio_parameters::Random,
    kabuto_geometry::{Geometry, Vertex},
    procedural_aim::{Matrix, basis},
};

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub enabled: bool,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub scale: [f32; 3],
    /// Source scalar at entry+28; gameplay interpretation remains unassigned.
    pub weight: f32,
    pub flags: u32,
    pub state_word: u32,
}
pub struct Submission {
    transform: Matrix,
    material_flags: u32,
    material_word: u32,
    state_word: u32,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}
pub enum Command<'a> {
    Transform(Matrix),
    Mode(u32),
    State(u32),
    /// Select the registry's generated material; no original pointer is stored.
    Material {
        flags: u32,
        word: u32,
    },
    /// Map for this count and copy all eight source words per vertex.
    Vertices(&'a [Vertex]),
    Unmap(u32),
    Indices(&'a [u16]),
}
impl Submission {
    /// Consume the packet. Completed backend calls and the already consumed RNG
    /// draw are not rolled back on a late failure; the packet cannot be replayed.
    pub fn emit<E>(self, mut backend: impl FnMut(Command<'_>) -> Result<(), E>) -> Result<(), E> {
        backend(Command::Transform(self.transform))?;
        backend(Command::Mode(1))?;
        backend(Command::State(self.state_word))?;
        backend(Command::Material {
            flags: self.material_flags,
            word: self.material_word,
        })?;
        backend(Command::Vertices(&self.vertices))?;
        backend(Command::Unmap(0))?;
        backend(Command::Indices(&self.indices))
    }
}
pub fn prepare(
    state: State,
    geometry: &Geometry,
    random: &mut Random,
) -> Result<Option<Submission>, String> {
    // Disabled entries touch neither source values, buffers, backend nor RNG.
    if !state.enabled {
        return Ok(None);
    }
    if !state
        .position
        .into_iter()
        .chain(state.direction)
        .chain(state.scale)
        .chain([state.weight])
        .all(f32::is_finite)
        || geometry.vertices.len() > u32::MAX as usize
        || geometry.indices.len() > u32::MAX as usize
        || geometry
            .indices
            .iter()
            .any(|&i| usize::from(i) >= geometry.vertices.len())
    {
        return Err("Kabuto submission outside finite owned source profile".into());
    }
    let integer = f64::from(state.weight) * 128.;
    if integer < f64::from(i32::MIN) || integer > f64::from(i32::MAX) {
        return Err("Kabuto submission scalar outside acquired integer profile".into());
    }
    let mut base = basis(state.direction, [0., 0., 1.]).map_err(str::to_owned)?;
    for i in 0..3 {
        base.values[i * 4 + 3] = state.position[i];
    }
    let mut scale = Matrix::identity();
    scale.flags = 2 | if state.scale.into_iter().filter(|&v| v < 0.).count() % 2 != 0 {
        8
    } else {
        0
    };
    for i in 0..3 {
        scale.values[i * 5] = state.scale[i];
    }
    let mut next = *random;
    let angle =
        (f64::from(next.next_15()) * f64::from(std::f32::consts::TAU) * (1. / 32768.)) as f32;
    let sine = simd_trig(angle, false);
    let cosine = simd_trig(angle, true);
    let mut rotation = Matrix::identity();
    rotation.flags = 1;
    rotation.values[0] = cosine;
    rotation.values[1] = -sine;
    rotation.values[4] = sine;
    rotation.values[5] = cosine;
    let composed = base
        .mul(scale.mul(rotation).map_err(str::to_owned)?)
        .map_err(str::to_owned)?;
    let transform = Matrix {
        flags: (composed.flags & !0x30)
            | ((composed.flags & 0x10) << 1)
            | ((composed.flags & 0x20) >> 1),
        values: std::array::from_fn(|i| composed.values[(i % 4) * 4 + i / 4]),
    };
    let result = Submission {
        transform,
        material_flags: state.flags | 0x21104,
        material_word: ((integer.trunc() as i32 as u32) | 0xffffff00) << 16,
        state_word: state.state_word,
        vertices: geometry.vertices.clone(),
        indices: geometry.indices.clone(),
    };
    *random = next;
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        State {
            enabled: true,
            position: [4., -5., 6.],
            direction: [0., 1., 0.],
            scale: [-2., 3., 4.],
            weight: 0.5,
            flags: 0x40000000,
            state_word: 73,
        }
    }

    #[test]
    fn disabled_entries_bypass_invalid_values_and_buffers_without_rng() {
        let geometry = Geometry {
            vertices: vec![],
            indices: vec![9],
        };
        let mut random = Random(17);
        let input = State {
            enabled: false,
            direction: [f32::NAN; 3],
            ..state()
        };
        assert!(prepare(input, &geometry, &mut random).unwrap().is_none());
        assert_eq!(random.0, 17);
    }

    #[test]
    fn invalid_owned_profiles_refuse_before_rng_publication() {
        let geometry = crate::kabuto_geometry::prepare(3, 2, 1.).unwrap();
        let mut random = Random(29);
        for input in [
            State {
                direction: [0.; 3],
                ..state()
            },
            State {
                direction: [0., 0., 1.],
                ..state()
            },
            State {
                weight: f32::MAX,
                ..state()
            },
            State {
                position: [f32::INFINITY; 3],
                ..state()
            },
        ] {
            assert!(prepare(input, &geometry, &mut random).is_err());
            assert_eq!(random.0, 29);
        }
        let invalid = Geometry {
            vertices: vec![],
            indices: vec![0],
        };
        assert!(prepare(state(), &invalid, &mut random).is_err());
        assert_eq!(random.0, 29);
    }

    #[test]
    fn emission_preserves_order_owned_buffers_words_and_one_rng_draw() {
        let mut geometry = crate::kabuto_geometry::prepare(3, 2, 1.).unwrap();
        let expected_vertices = geometry.vertices.clone();
        let expected_indices = geometry.indices.clone();
        let mut random = Random(43);
        let mut expected_random = random;
        expected_random.next_15();
        let packet = prepare(state(), &geometry, &mut random).unwrap().unwrap();
        assert_eq!(random.0, expected_random.0);
        geometry.vertices.clear();
        geometry.indices.clear();
        let mut order = Vec::new();
        packet
            .emit::<()>(|command| {
                order.push(match command {
                    Command::Transform(matrix) => {
                        assert_eq!(&matrix.values[12..15], &[4., -5., 6.]);
                        assert_ne!(matrix.flags & 8, 0);
                        "transform"
                    }
                    Command::Mode(n) => {
                        assert_eq!(n, 1);
                        "mode"
                    }
                    Command::State(n) => {
                        assert_eq!(n, 73);
                        "state"
                    }
                    Command::Material { flags, word } => {
                        assert_eq!(flags, 0x40021104);
                        assert_eq!(word, 0xff400000);
                        "material"
                    }
                    Command::Vertices(v) => {
                        assert_eq!(v, expected_vertices);
                        "vertices"
                    }
                    Command::Unmap(n) => {
                        assert_eq!(n, 0);
                        "unmap"
                    }
                    Command::Indices(v) => {
                        assert_eq!(v, expected_indices);
                        "indices"
                    }
                });
                Ok(())
            })
            .unwrap();
        assert_eq!(
            order,
            [
                "transform",
                "mode",
                "state",
                "material",
                "vertices",
                "unmap",
                "indices"
            ]
        );
    }

    #[test]
    fn each_backend_failure_keeps_completed_prefix_and_consumed_rng() {
        let geometry = crate::kabuto_geometry::prepare(3, 2, 1.).unwrap();
        for failed in 0..7 {
            let mut random = Random(61);
            let mut expected = random;
            expected.next_15();
            let packet = prepare(state(), &geometry, &mut random).unwrap().unwrap();
            let mut calls = 0;
            let result = packet.emit(|_| {
                let index = calls;
                calls += 1;
                if index == failed { Err(index) } else { Ok(()) }
            });
            assert_eq!(result, Err(failed));
            assert_eq!(calls, failed + 1);
            assert_eq!(random.0, expected.0);
        }
    }
}
