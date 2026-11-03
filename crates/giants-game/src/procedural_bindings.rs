//! Measured procedural registrations from the common scene initializer.
//! This is compiled engine policy, independent of cinematic sequence names.
use crate::procedural_aim::Axes;
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub node: &'static str,
    pub axes: Axes,
}
/// Registered profiles with unresolved relative-axis or special callbacks fail
/// explicitly. Other specs have no binding in this measured initializer.
pub fn bindings(spec: u32, slot: u8) -> Result<Vec<Binding>, &'static str> {
    let value = match spec {
        59 | 1071 | 1070 | 1196 | 1197 | 1195 | 1194 if slot == 1 => {
            Some(("center", [0., -1., 0.], [0., 0., 1.]))
        }
        584 if matches!(slot, 1 | 11 | 12) => Some(("center", [0., 0., 1.], [-1., 0., 0.])),
        624 if matches!(slot, 1 | 11 | 12) => Some(("seatbone", [0., -1., 0.], [0., 0., 1.])),
        2 | 133 | 1 | 101 | 102 => {
            return Err(
                "registered procedural profile requires unimplemented rebasing or special callback",
            );
        }
        _ => None,
    };
    Ok(value
        .into_iter()
        .map(|(node, forward, up)| Binding {
            node,
            axes: Axes {
                forward,
                up: Some(up),
            },
        })
        .collect())
}
