//! Bind an impact point to a current animation joint, then retain that joint
//! and its local point as the animation changes. Frame production is upstream.
use crate::procedural_aim::Matrix;

pub struct Joint<K> {
    pub key: K,
    pub matrix: Matrix,
}
#[derive(Clone, Debug)]
pub struct Binding<K> {
    key: Option<K>,
    local: [f32; 3],
    position: [f32; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    Bound,
    Followed,
    /// A real lookup in the supplied complete current joint view missed the
    /// retained key. The native caller retains the previous object-space point.
    RetainedMissingJoint,
}
impl<K: Clone + Eq> Binding<K> {
    pub fn new(object_point: [f32; 3]) -> Result<Self, &'static str> {
        if !object_point.into_iter().all(f32::is_finite) {
            return Err("skinned impact point is nonfinite");
        }
        Ok(Self {
            key: None,
            local: object_point,
            position: object_point,
        })
    }
    pub fn position(&self) -> [f32; 3] {
        self.position
    }
    pub fn key(&self) -> Option<&K> {
        self.key.as_ref()
    }
    pub fn advance(&mut self, joints: &[Joint<K>]) -> Result<Update, &'static str> {
        if joints.is_empty() {
            return Err("skinned hit point requires a current joint view");
        }
        for (i, joint) in joints.iter().enumerate() {
            if joint.matrix.flags & !31 != 0
                || !joint.matrix.values.into_iter().all(f32::is_finite)
                || joint.matrix.values[12..] != [0., 0., 0., 1.]
            {
                return Err("skinned hit point requires finite current affine joints");
            }
            if joints[..i].iter().any(|other| other.key == joint.key) {
                return Err("duplicate joint identity in skinned point view");
            }
        }
        let fresh = self.key.is_none();
        if fresh {
            let mut best: Option<(usize, [f32; 3], f32)> = None;
            for (index, joint) in joints.iter().enumerate() {
                let inverse = Matrix {
                    flags: joint.matrix.flags,
                    values: crate::animation::inverse_affine(
                        joint.matrix.values,
                        joint.matrix.flags,
                    )
                    .map_err(|_| "skinned point joint inverse unavailable")?,
                };
                let local = inverse.transform_point(self.position);
                let length = ((f64::from(local[0]) * f64::from(local[0])
                    + f64::from(local[1]) * f64::from(local[1]))
                    + f64::from(local[2]) * f64::from(local[2]))
                    as f32;
                if !local.into_iter().all(f32::is_finite) || !length.is_finite() {
                    return Err("skinned point binding overflow");
                }
                if best
                    .as_ref()
                    .is_none_or(|(_, _, distance)| length < *distance)
                {
                    best = Some((index, local, length));
                }
            }
            let (index, local, _) = best.unwrap();
            self.key = Some(joints[index].key.clone());
            self.local = local;
        }
        let Some(joint) = joints
            .iter()
            .find(|joint| Some(&joint.key) == self.key.as_ref())
        else {
            return Ok(Update::RetainedMissingJoint);
        };
        let position = joint.matrix.transform_point(self.local);
        if !position.into_iter().all(f32::is_finite) {
            return Err("skinned point deformation overflow");
        }
        self.position = position;
        Ok(if fresh {
            Update::Bound
        } else {
            Update::Followed
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn joint(key: u32, x: f32, scale: f32) -> Joint<u32> {
        let mut matrix = Matrix::identity();
        matrix.flags = 16 | if scale != 1. { 2 } else { 0 };
        for i in [0, 5, 10] {
            matrix.values[i] = scale;
        }
        matrix.values[3] = x;
        Joint { key, matrix }
    }
    #[test]
    fn binds_in_joint_local_space_then_follows_the_same_joint() {
        let mut point = Binding::new([2., 0., 0.]).unwrap();
        assert_eq!(
            point
                .advance(&[joint(1, 0., 1.), joint(2, 8., 4.)])
                .unwrap(),
            Update::Bound
        );
        assert_eq!(point.key(), Some(&2));
        point
            .advance(&[joint(1, 2., 1.), joint(2, 12., 4.)])
            .unwrap();
        assert_eq!(point.position(), [6., 0., 0.]);
        assert_eq!(
            point.advance(&[joint(1, 2., 1.)]).unwrap(),
            Update::RetainedMissingJoint
        );
        assert_eq!(point.position(), [6., 0., 0.]);
    }
}
