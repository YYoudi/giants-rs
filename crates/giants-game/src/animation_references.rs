//! Source-declared reference slots with lazy positive-only ANM node caching.
//! A resolved node is not an evaluated matrix or an attached runtime object.
use giants_formats::{
    Error,
    anm::{Animation, read_word},
    object_spec::Program,
};

#[derive(Debug)]
pub struct References {
    pub names: Vec<Option<String>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Found(usize),
    Cached(usize),
    Missing,
    Undeclared,
    Outside,
}
impl Resolution {
    pub fn node(self) -> Option<usize> {
        match self {
            Self::Found(node) | Self::Cached(node) => Some(node),
            _ => None,
        }
    }
}
pub struct Binding<'a> {
    references: &'a References,
    animation: &'a Animation,
    cache: Vec<Option<usize>>,
}
pub struct EvaluatedReference {
    pub resolution: Resolution,
    pub matrix: Option<crate::procedural_aim::Matrix>,
}
impl EvaluatedReference {
    /// Initial object-reference table for an immutable class-loop descriptor.
    /// The model binder writes identity for a declared name absent from ANM.
    /// This is a consumer rule; the getter's missing result remains distinct.
    /// Descriptor changes and accumulated procedural reference poses are not
    /// covered by this initial-table profile.
    pub fn initial_object_matrix(self) -> Result<crate::procedural_aim::Matrix, Error> {
        match (self.resolution, self.matrix) {
            (Resolution::Found(_) | Resolution::Cached(_), Some(matrix)) => Ok(matrix),
            (Resolution::Missing, None) => Ok(crate::procedural_aim::Matrix::identity()),
            _ => Err(Error("reference outside initialized object table profile")),
        }
    }
}
/// Source hierarchy matrix, before world placement or procedural actor callbacks.
pub fn node_matrix(
    animation: &Animation,
    node: usize,
    tick: i32,
) -> Result<crate::procedural_aim::Matrix, Error> {
    let evaluator = crate::animation::Evaluator { animation };
    Ok(crate::procedural_aim::Matrix {
        flags: evaluator.matrix_flags(node, tick)?,
        values: evaluator.matrix(node, tick)?,
    })
}
impl References {
    pub fn from_program(program: &Program) -> Result<Self, Error> {
        Ok(Self {
            names: program.reference_names()?,
        })
    }
    /// Each binding keeps its own cache tied to an immutable source animation.
    pub fn bind<'a>(&'a self, animation: &'a Animation) -> Result<Binding<'a>, Error> {
        let tree = animation
            .trees
            .iter()
            .find(|t| t.offset == animation.default_tree)
            .ok_or(Error("missing reference default tree"))?;
        if read_word(animation.bytes(), tree.offset + 8)? & 2 == 0 {
            return Err(Error("unsupported reference name comparison"));
        }
        Ok(Binding {
            references: self,
            animation,
            cache: vec![None; self.names.len()],
        })
    }
    pub(crate) fn resolve_cached(
        &self,
        animation: &Animation,
        cache: &mut [Option<usize>],
        slot: usize,
    ) -> Result<Resolution, Error> {
        if cache.len() != self.names.len() {
            return Err(Error("reference node cache length changed"));
        }
        let Some(name) = self.names.get(slot) else {
            return Ok(Resolution::Outside);
        };
        let Some(name) = name else {
            return Ok(Resolution::Undeclared);
        };
        if let Some(node) = cache[slot] {
            return Ok(Resolution::Cached(node));
        }
        let tree = animation
            .trees
            .iter()
            .find(|t| t.offset == animation.default_tree)
            .ok_or(Error("missing reference default tree"))?;
        if read_word(animation.bytes(), tree.offset + 8)? & 2 == 0 {
            return Err(Error("unsupported reference name comparison"));
        }
        let Some((_, node)) = tree
            .entries
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
        else {
            return Ok(Resolution::Missing);
        };
        animation.node(*node)?;
        cache[slot] = Some(*node);
        Ok(Resolution::Found(*node))
    }
}
impl Binding<'_> {
    pub fn evaluate(&mut self, slot: usize, tick: i32) -> Result<EvaluatedReference, Error> {
        let resolution = self.resolve(slot)?;
        let matrix = resolution
            .node()
            .map(|node| node_matrix(self.animation, node, tick))
            .transpose()?;
        Ok(EvaluatedReference { resolution, matrix })
    }
    pub fn resolve(&mut self, slot: usize) -> Result<Resolution, Error> {
        self.references
            .resolve_cached(self.animation, &mut self.cache, slot)
    }
}

#[cfg(test)]
mod tests {
    use super::{EvaluatedReference, Resolution};
    #[test]
    fn declared_missing_initializes_identity_but_invalid_slots_still_fail() {
        let matrix = EvaluatedReference {
            resolution: Resolution::Missing,
            matrix: None,
        }
        .initial_object_matrix()
        .unwrap();
        assert_eq!(matrix.flags, 0);
        assert_eq!(
            matrix.values,
            crate::procedural_aim::Matrix::identity().values
        );
        for resolution in [
            Resolution::Outside,
            Resolution::Undeclared,
            Resolution::Found(7),
        ] {
            assert!(
                EvaluatedReference {
                    resolution,
                    matrix: None
                }
                .initial_object_matrix()
                .is_err()
            );
        }
    }
    #[test]
    fn resolved_initial_table_keeps_exact_matrix_and_flags() {
        let mut matrix = crate::procedural_aim::Matrix::identity();
        matrix.flags = 17;
        matrix.values[3] = 42.;
        for resolution in [Resolution::Found(7), Resolution::Cached(7)] {
            let actual = EvaluatedReference {
                resolution,
                matrix: Some(matrix),
            }
            .initial_object_matrix()
            .unwrap();
            assert_eq!(actual.flags, 17);
            assert_eq!(actual.values, matrix.values);
        }
        assert!(
            EvaluatedReference {
                resolution: Resolution::Missing,
                matrix: Some(matrix)
            }
            .initial_object_matrix()
            .is_err()
        );
    }
}
