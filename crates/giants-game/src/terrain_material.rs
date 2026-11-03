//! Terrain preparation with supplied resident texture identities and backend
//! capabilities. This does not create GPU textures or select user settings.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Texture {
    /// Opaque nonzero identity, shared by aliases of the same resident texture.
    pub identity: u32,
    pub words: [u32; 8],
}

/// Unlit draw preparation consumes this renderer word after applying the
/// material. Its live producer is separate; it is not a presumed GPU capability.
pub fn unlit_secondary_color(program: u32, renderer_word: u32) -> Result<bool, &'static str> {
    if program & 1 != 0 {
        return Err("lit secondary-color preparation is outside this profile");
    }
    Ok(program & 2 != 0 && renderer_word & 0x8000 != 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Additional {
    Absent,
    /// Update the separately loaded descriptor in place.
    Separate([u32; 8]),
    /// New descriptor; word 1 refers to the base texture, which stays unchanged.
    Derived([u32; 8]),
}

/// Prepare one additional layer after texture lookup. A missing declaration
/// stays missing even when a base texture exists. Identity, not pixel/name
/// equality, determines whether a new wrapper is required.
pub fn prepare_additional(
    base: Option<Texture>,
    additional: Option<Texture>,
) -> Result<Additional, &'static str> {
    if base.into_iter().chain(additional).any(|t| t.identity == 0) {
        return Err("resident texture identities must be nonzero");
    }
    let Some(additional) = additional else {
        return Ok(Additional::Absent);
    };
    let mut words = additional.words;
    if let Some(base) = base.filter(|base| base.identity == additional.identity) {
        if base.words != additional.words {
            return Err("one resident identity has inconsistent descriptors");
        }
        // The original helper materializes format 8 when this bit is clear.
        // Its I/O and conversion are a preceding, still separate consumer.
        if words[5] == 8 && words[7] & 0x0008_0000 == 0 {
            return Err("source texture materialization is required before deriving terrain layer");
        }
        words[1] = base.identity;
        words[6] = 2;
        let flags = ((words[7] >> 16) as u16 | 0x410) & 0xfdfe;
        words[7] = (words[7] & 0xffff) | (u32::from(flags) << 16);
        Ok(Additional::Derived(words))
    } else {
        words[6] = 2;
        words[7] |= 0x0110_0000;
        Ok(Additional::Separate(words))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VertexPath {
    /// Capability bit 8 absent; payload consumers are a separate control.
    CapabilityFallback,
    Base,
    Additional,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawProfile {
    pub path: VertexPath,
    pub stride: u32,
    pub buffer_kind: u32,
    /// Raw selector written alongside the native vertex-buffer kind.
    pub selector_word: u32,
}

/// Original nine face classes map to three material slots. The caller must
/// supply the effective backend capability word and terrain option; neither
/// is inferred from unrelated object-bump registry settings.
pub fn draw_profiles(
    capabilities: u32,
    disable_additional: u32,
    additional_present: [bool; 3],
) -> [DrawProfile; 9] {
    std::array::from_fn(|class| {
        let slot = if class == 0 {
            0
        } else if class < 5 {
            1
        } else {
            2
        };
        if capabilities & 8 == 0 {
            DrawProfile {
                path: VertexPath::CapabilityFallback,
                stride: 24,
                buffer_kind: 0,
                selector_word: 0,
            }
        } else if disable_additional == 0 && additional_present[slot] {
            DrawProfile {
                path: VertexPath::Additional,
                stride: 20,
                buffer_kind: 4,
                selector_word: 1,
            }
        } else {
            DrawProfile {
                path: VertexPath::Base,
                stride: 16,
                buffer_kind: 3,
                selector_word: 1,
            }
        }
    })
}

/// Version-specific additions to the material packet. These are supplied
/// runtime inputs, not values inferred from the world file alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketPolicy {
    Debug,
    Gog {
        overlay_rgb: Option<u32>,
        auxiliary_texture: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketInput {
    pub base_program: u32,
    pub base_texture: u32,
    pub additional_texture: u32,
    pub disable_additional: u32,
    pub policy: PacketPolicy,
}

/// Prepare the next material in source order. The constant and unused tail
/// retain their incoming values; GOG's overlay accumulates into the constant.
/// Debug's setter copies only four words, while GOG's copies all six.
pub fn material_packet(input: PacketInput, previous: [u32; 6]) -> [u32; 6] {
    let mut packet = previous;
    if input.disable_additional == 0 && input.additional_texture != 0 {
        packet[0] = input.additional_texture;
        packet[1] = input.base_texture;
        packet[2] = (input.base_program & 0xffff8fff) | 0x68002;
        if let PacketPolicy::Gog {
            overlay_rgb,
            auxiliary_texture,
        } = input.policy
        {
            packet[4] = auxiliary_texture;
            packet[5] = 0;
            if let Some(rgb) = overlay_rgb {
                packet[2] = (input.base_program & 0xffff8fff) | 0x68300;
                packet[3] |= rgb & 0xffffff;
            }
        }
    } else {
        packet[0] = input.base_texture;
        packet[1] = 0;
        packet[2] = input.base_program;
    }
    packet
}

#[derive(Clone, Copy, Debug)]
pub struct VertexInput {
    pub position: [f32; 3],
    pub color: u32,
    /// Separate cell word consumed as diffuse on the additional-layer path.
    /// Its production and interpretation are separate from this packing step.
    pub additional_color: u32,
    pub source_flags: u32,
}

/// Pack the two generated-coordinate paths with the height modifier disabled.
/// `reduce_unmarked` selects the original update variant, rather than the cold
/// constructor variant; source flag bit31 exempts its color reduction.
pub fn vertex_packet(
    input: VertexInput,
    path: VertexPath,
    reduce_unmarked: bool,
) -> Result<Vec<u32>, &'static str> {
    if path == VertexPath::CapabilityFallback {
        return Err("terrain capability-fallback vertex consumer remains unverified");
    }
    if !input.position.iter().all(|v| v.is_finite()) {
        return Err("terrain vertex position must be finite");
    }
    let color = if reduce_unmarked && input.source_flags & 0x80000000 == 0 {
        (input.color & 0xff000000) | ((input.color >> 3) & 0x1f1f1f)
    } else {
        input.color
    };
    let mut packet = input.position.map(f32::to_bits).to_vec();
    if path == VertexPath::Additional {
        packet.push(input.additional_color);
    }
    packet.push(color);
    Ok(packet)
}
