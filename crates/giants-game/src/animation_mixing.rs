//! Hierarchical current/backup frame selection with retained ANM node matrices.
//! Mutable procedural callbacks need separate services. Flagged references also
//! sample declared LOOKAT dependencies; the deformation profile stays bounded.
use crate::animation::{Evaluator, pose_matrix};
use giants_formats::{
    Error,
    anm::{Animation, read_word},
};
use std::collections::{BTreeMap, BTreeSet};

pub type Memo = BTreeMap<usize, (u32, [f32; 16])>;
pub type FlagMemo = BTreeMap<usize, (u32, crate::procedural_aim::Matrix)>;
pub type PoseMemo = BTreeMap<usize, (i32, crate::animation::Pose)>;
trait Value: Copy {
    fn sample(
        frame: Frame,
        animations: &[&Animation],
        epoch: u32,
        caches: &mut [BTreeMap<usize, (u32, Self)>],
        poses: &mut [PoseMemo],
    ) -> Result<crate::animation::Pose, Error>;
    fn local(pose: &crate::animation::Pose) -> Self;
    fn product(parent: Self, local: Self) -> Self;
}
impl Value for [f32; 16] {
    fn sample(
        frame: Frame,
        animations: &[&Animation],
        _: u32,
        _: &mut [Memo],
        _: &mut [PoseMemo],
    ) -> Result<crate::animation::Pose, Error> {
        let animation = animations
            .get(frame.animation)
            .ok_or(Error("mixed source animation absent"))?;
        let at = frame.node.ok_or(Error("mixed source node absent"))?;
        let source = animation.node(at)?;
        if source.callback != 0 || source.transform > 2 {
            return Err(Error(
                "mixed deformation requires ordinary nodes without callbacks",
            ));
        }
        Evaluator { animation }.affine(at, frame.tick)
    }
    fn local(pose: &crate::animation::Pose) -> Self {
        pose_matrix(pose)
    }
    fn product(parent: Self, local: Self) -> Self {
        std::array::from_fn(|i| {
            (0..4)
                .map(|k| f64::from(parent[i / 4 * 4 + k]) * f64::from(local[k * 4 + i % 4]))
                .sum::<f64>() as f32
        })
    }
}
impl Value for crate::procedural_aim::Matrix {
    fn sample(
        frame: Frame,
        animations: &[&Animation],
        epoch: u32,
        caches: &mut [FlagMemo],
        poses: &mut [PoseMemo],
    ) -> Result<crate::animation::Pose, Error> {
        let animation = animations
            .get(frame.animation)
            .ok_or(Error("mixed source animation absent"))?;
        let at = frame.node.ok_or(Error("mixed source node absent"))?;
        flagged_pose(
            animation,
            at,
            frame.tick,
            epoch,
            &mut caches[frame.animation],
            &mut poses[frame.animation],
            &mut vec![at],
        )
    }
    fn local(pose: &crate::animation::Pose) -> Self {
        Self {
            flags: pose.flags,
            values: pose_matrix(pose),
        }
    }
    fn product(parent: Self, local: Self) -> Self {
        let union = parent.flags | local.flags;
        let mut flags = union ^ ((union ^ parent.flags ^ local.flags) & 8);
        if parent.flags & 2 != 0 && local.flags & 1 != 0 {
            flags |= 4;
        }
        Self {
            flags,
            values: <[f32; 16] as Value>::product(parent.values, local.values),
        }
    }
}
fn flagged_pose(
    animation: &Animation,
    node: usize,
    tick: i32,
    epoch: u32,
    cache: &mut FlagMemo,
    poses: &mut PoseMemo,
    active: &mut Vec<usize>,
) -> Result<crate::animation::Pose, Error> {
    let source = animation.node(node)?;
    if source.callback != 0 || source.transform > 3 {
        return Err(Error("mixed reference node callback is not acquired"));
    }
    if let Some((saved, value)) = poses.get(&node)
        && *saved == tick
    {
        return Ok(value.clone());
    }
    let evaluator = Evaluator { animation };
    let pose = if source.transform != 3 {
        evaluator.affine(node, tick)?
    } else {
        let target = flagged_matrix(animation, source.target, tick, epoch, cache, poses, active)?;
        let parent = if source.parent != 0 {
            let m = flagged_matrix(animation, source.parent, tick, epoch, cache, poses, active)?;
            Some((m.flags, m.values))
        } else {
            None
        };
        evaluator.affine_with_look_at_matrices(node, tick, target.values, parent)?
    };
    poses.insert(node, (tick, pose.clone()));
    Ok(pose)
}
fn flagged_matrix(
    animation: &Animation,
    node: usize,
    tick: i32,
    epoch: u32,
    cache: &mut FlagMemo,
    poses: &mut PoseMemo,
    active: &mut Vec<usize>,
) -> Result<crate::procedural_aim::Matrix, Error> {
    if let Some((stamp, matrix)) = cache.get(&node)
        && *stamp == epoch
    {
        return Ok(*matrix);
    }
    if active.contains(&node) || active.len() >= 128 {
        return Err(Error("retained LOOKAT dependency cycle/depth"));
    }
    active.push(node);
    let source = animation.node(node)?;
    // The original single-frame dependency walker evaluates parents first.
    let parent = if source.parent != 0 {
        Some(flagged_matrix(
            animation,
            source.parent,
            tick,
            epoch,
            cache,
            poses,
            active,
        )?)
    } else {
        None
    };
    let pose = flagged_pose(animation, node, tick, epoch, cache, poses, active)?;
    let mut value = <crate::procedural_aim::Matrix as Value>::local(&pose);
    if let Some(parent) = parent {
        value = <crate::procedural_aim::Matrix as Value>::product(parent, value);
    }
    cache.insert(node, (epoch, value));
    active.pop();
    Ok(value)
}
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub animation: usize,
    pub node: Option<usize>,
    pub tick: i32,
    pub backup: bool,
    pub transition: Option<f32>,
}
fn choose(
    frames: &[Frame],
    animations: &[&Animation],
    backup: bool,
) -> Result<Option<Frame>, Error> {
    let mut fallback = None;
    for frame in frames {
        if !backup && frame.backup {
            break;
        }
        if frame.backup != backup {
            continue;
        }
        let Some(node) = frame.node else {
            continue;
        };
        let animation = animations
            .get(frame.animation)
            .ok_or(Error("mixed animation identity absent"))?;
        animation.node(node)?;
        if fallback.is_none() {
            fallback = Some(*frame);
        }
        if read_word(animation.bytes(), node + 8)? & 0x3f != 0 {
            return Ok(Some(*frame));
        }
    }
    Ok(fallback)
}
pub fn matrix(
    frames: &[Frame],
    animations: &[&Animation],
    epoch: u32,
    caches: &mut [Memo],
) -> Result<Option<[f32; 16]>, Error> {
    evaluate(frames, animations, epoch, caches, &mut [])
}
/// The same hierarchy traversal with the native matrix flags retained.
pub fn matrix_with_flags(
    frames: &[Frame],
    animations: &[&Animation],
    epoch: u32,
    caches: &mut [FlagMemo],
    poses: &mut [PoseMemo],
) -> Result<Option<crate::procedural_aim::Matrix>, Error> {
    if poses.len() != animations.len() {
        return Err(Error("mixed pose cache budget mismatch"));
    }
    evaluate(frames, animations, epoch, caches, poses)
}
fn evaluate<T: Value>(
    frames: &[Frame],
    animations: &[&Animation],
    epoch: u32,
    caches: &mut [BTreeMap<usize, (u32, T)>],
    poses: &mut [PoseMemo],
) -> Result<Option<T>, Error> {
    if frames.len() > 4 || animations.len() != caches.len() {
        return Err(Error("mixed frame/cache budget mismatch"));
    }
    let mut current = frames.to_vec();
    let mut pending = Vec::new();
    let mut seen = BTreeSet::new();
    let mut parent = loop {
        let Some(a) = choose(&current, animations, false)? else {
            break None;
        };
        let node = a.node.ok_or(Error("selected mixed node absent"))?;
        if let Some((stamp, value)) = caches[a.animation].get(&node)
            && *stamp == epoch
        {
            break Some(*value);
        }
        if !seen.insert((a.animation, node)) || pending.len() >= 128 {
            return Err(Error("mixed hierarchy cycle/depth"));
        }
        let weight = a
            .transition
            .ok_or(Error("current mixed frame has no transition"))?;
        let b = if weight != 0. {
            choose(&current, animations, true)?
        } else {
            None
        };
        pending.push((a, b));
        for frame in &mut current {
            if let Some(node) = frame.node {
                let animation = animations
                    .get(frame.animation)
                    .ok_or(Error("mixed parent animation absent"))?;
                let parent = animation.node(node)?.parent;
                frame.node = (parent != 0).then_some(parent);
            }
        }
    };
    for (a, b) in pending.into_iter().rev() {
        let node = a.node.ok_or(Error("pending mixed node absent"))?;
        let mut pose = T::sample(a, animations, epoch, caches, poses)?;
        if let Some(b) = b {
            let backup = T::sample(b, animations, epoch, caches, poses)?;
            // Native affine sampling returns the node's mutable local cache.
            // Sampling the same node again aliases and overwrites the first
            // input pointer before the blend call (callbacks are excluded).
            if a.animation == b.animation && a.node == b.node {
                pose = backup.clone();
            }
            pose =
                crate::pose_mixing::affine(&pose, &backup, a.transition.unwrap()).map_err(Error)?;
        }
        let mut value = T::local(&pose);
        if let Some(parent) = parent {
            value = T::product(parent, value);
        }
        caches[a.animation].insert(node, (epoch, value));
        parent = Some(value);
    }
    Ok(parent)
}
