//! Debug animation descriptors and source SKN-to-ANM bindings.
//! Geometry descriptors and arithmetic precision are explicit inputs. Preparing
//! these tables does not start animation, register an actor or draw a model.
use crate::arithmetic::Precision;
use giants_formats::{
    Error,
    anm::{Animation, read_word},
    skn::Skin,
};

pub enum Model<'a> {
    Empty,
    Unskinned,
    Skinned(&'a Skin),
}
#[derive(Debug)]
pub struct Cluster {
    pub source: usize,
    pub node: Option<usize>,
}
#[derive(Debug)]
pub struct Object {
    pub source: usize,
    pub vertex_count: usize,
    pub clusters: Vec<Cluster>,
}
#[derive(Debug)]
pub struct Binding {
    pub tree: usize,
    pub objects: Vec<Object>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Need {
    MissingSkin(usize),
    MissingNode { model: usize, cluster: usize },
}
#[derive(Debug)]
pub struct Template {
    pub duration: f32,
    pub start_tick: i32,
    pub flags: u32,
    pub bindings: Vec<Option<Binding>>,
    pub needs: Vec<Need>,
}

/// Original descriptor arithmetic: reciprocal FPS, divide by ticks per frame,
/// multiply by the signed span, then store binary32. No playback cadence input.
pub fn duration(timeline: [i32; 4], precision: Precision) -> Result<f32, Error> {
    let [start, end, ticks, fps] = timeline;
    let span = end
        .checked_sub(start)
        .ok_or(Error("animation span overflow"))?;
    if span < 0 || ticks <= 0 || fps <= 0 {
        return Err(Error("invalid animation descriptor timeline"));
    }
    let reciprocal = precision.round(1. / f64::from(fps));
    let step = precision.round(reciprocal / f64::from(ticks));
    Ok(precision.round(f64::from(span) * step) as f32)
}

impl Template {
    /// Actor construction uses the requested inline tree, leaves unskinned
    /// models unbound, and continues until the first absent model. This differs
    /// from the specification descriptor's first-model routing below.
    pub fn prepare_inline(
        animation: &Animation,
        models: &[Model<'_>],
        tree_name: Option<&str>,
        precision: Precision,
    ) -> Result<Self, Error> {
        if models.len() > 10 {
            return Err(Error("inline animation model budget"));
        }
        let mut out = Self {
            duration: duration(animation.timeline, precision)?,
            start_tick: animation.timeline[0],
            flags: 0,
            bindings: (0..models.len()).map(|_| None).collect(),
            needs: Vec::new(),
        };
        for (slot, model) in models.iter().enumerate() {
            let skin = match model {
                Model::Empty => break,
                Model::Unskinned => continue,
                Model::Skinned(skin) => skin,
            };
            let tree = if let Some(name) = tree_name {
                if name.is_empty() || !name.is_ascii() {
                    return Err(Error("invalid inline animation tree name"));
                }
                let dictionary = read_word(animation.bytes(), animation.root + 32)? as usize;
                if read_word(animation.bytes(), dictionary + 8)? & 2 == 0 {
                    return Err(Error("unsupported inline tree comparison"));
                }
                animation
                    .trees
                    .iter()
                    .find(|t| t.name.eq_ignore_ascii_case(name))
            } else {
                animation
                    .trees
                    .iter()
                    .find(|t| t.offset == animation.default_tree)
            }
            .ok_or(Error("missing inline animation tree"))?;
            if read_word(animation.bytes(), tree.offset + 8)? & 2 == 0 {
                return Err(Error("unsupported inline node comparison"));
            }
            out.bindings[slot] = Some(bind(skin, tree, slot, &mut out.needs));
        }
        Ok(out)
    }

    pub fn prepare(
        animation: &Animation,
        models: &[Model<'_>],
        precision: Precision,
    ) -> Result<Self, Error> {
        if models.is_empty() || models.len() > 10 || matches!(models[0], Model::Empty) {
            return Err(Error(
                "animation requires a first model and at most ten slots",
            ));
        }
        let mut out = Self {
            duration: duration(animation.timeline, precision)?,
            start_tick: animation.timeline[0],
            flags: u32::from(matches!(models[0], Model::Unskinned)),
            bindings: (0..models.len()).map(|_| None).collect(),
            needs: Vec::new(),
        };
        // The original constructor selects its route using the first model.
        if out.flags == 1 {
            return Ok(out);
        }
        let tree = animation
            .trees
            .iter()
            .find(|t| t.offset == animation.default_tree)
            .ok_or(Error("missing animation default tree"))?;
        if read_word(animation.bytes(), tree.offset + 8)? & 2 == 0 {
            return Err(Error("unsupported animation binding name comparison"));
        }
        for (slot, model) in models.iter().enumerate() {
            let skin = match model {
                Model::Empty => break,
                Model::Unskinned => {
                    out.needs.push(Need::MissingSkin(slot));
                    break;
                }
                Model::Skinned(skin) => skin,
            };
            out.bindings[slot] = Some(bind(skin, tree, slot, &mut out.needs));
        }
        Ok(out)
    }
}

fn bind(
    skin: &Skin,
    tree: &giants_formats::anm::Tree,
    slot: usize,
    needs: &mut Vec<Need>,
) -> Binding {
    Binding {
        tree: tree.offset,
        objects: skin
            .objects
            .iter()
            .map(|object| Object {
                source: object.offset,
                vertex_count: object.vertex_count,
                clusters: object
                    .clusters
                    .iter()
                    .map(|cluster| {
                        let node = tree
                            .entries
                            .iter()
                            .find(|(name, _)| name.eq_ignore_ascii_case(&cluster.bone))
                            .map(|(_, node)| *node);
                        if node.is_none() {
                            needs.push(Need::MissingNode {
                                model: slot,
                                cluster: cluster.offset,
                            });
                        }
                        Cluster {
                            source: cluster.offset,
                            node,
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_static_timeline_is_a_descriptor_but_invalid_rates_are_not() {
        assert_eq!(
            duration([160, 160, 160, 30], Precision::Double53).unwrap(),
            0.
        );
        assert!(duration([0, 1, 0, 30], Precision::Double53).is_err());
        assert!(duration([i32::MIN, i32::MAX, 1, 1], Precision::Double53).is_err());
    }
}
