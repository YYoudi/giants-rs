//! Bounded serialized ANM graphs. Only measured track payloads are decoded.
use crate::{Error, slice, u32_at};
use std::collections::{BTreeMap, BTreeSet};
pub fn read_word(data: &[u8], at: usize) -> Result<u32, Error> {
    u32_at(data, at)
}
pub fn read_bytes(data: &[u8], at: usize, size: usize) -> Result<&[u8], Error> {
    slice(data, at, size)
}

#[derive(Clone, Debug)]
pub struct Node {
    pub offset: usize,
    pub kind: u16,
    pub transform: u16,
    pub parent: usize,
    pub target: usize,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub flags: u32,
    pub tracks: [usize; 3],
    pub callback: u32,
    pub camera: Option<(f32, usize)>,
}
#[derive(Debug)]
pub struct Tree {
    pub name: String,
    pub offset: usize,
    pub entries: Vec<(String, usize)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnresolvedLink {
    pub node: usize,
    pub target: usize,
    pub look_at: bool,
}
#[derive(Debug)]
pub struct Animation {
    data: Vec<u8>,
    pub root: usize,
    pub timeline: [i32; 4],
    pub default_tree: usize,
    /// Serialized metadata can exceed the number of NAM-addressable nodes.
    /// No offsets are invented for unnamed/unrepresented entries.
    pub declared_node_count: usize,
    pub trees: Vec<Tree>,
    pub nodes: Vec<Node>,
}
impl Animation {
    /// Unrepresented dependency edges remain errors when a consumer evaluates
    /// them. This list never invents nodes from stale auxiliary storage.
    pub fn unresolved_links(&self) -> Vec<UnresolvedLink> {
        self.nodes
            .iter()
            .flat_map(|node| {
                [(node.parent, false), (node.target, true)]
                    .into_iter()
                    .filter_map(|(target, look_at)| {
                        (target != 0 && self.node(target).is_err()).then_some(UnresolvedLink {
                            node: node.offset,
                            target,
                            look_at,
                        })
                    })
            })
            .collect()
    }
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }
    pub fn node(&self, offset: usize) -> Result<&Node, Error> {
        self.nodes
            .binary_search_by_key(&offset, |n| n.offset)
            .map(|i| &self.nodes[i])
            .map_err(|_| Error("unknown ANM node"))
    }
    pub fn default_node(&self, name: &str) -> Result<&Node, Error> {
        let tree = self
            .trees
            .iter()
            .find(|t| t.offset == self.default_tree)
            .ok_or(Error("missing default ANM tree"))?;
        let offset = tree
            .entries
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .ok_or(Error("missing named ANM node"))?
            .1;
        self.node(offset)
    }
    /// Observed ASCII case-insensitive NAM profile (descriptor flag bit 1).
    /// A missing tree is the serialized default tree, not a hard-coded name.
    pub fn named_node(&self, tree_name: Option<&str>, name: &str) -> Result<&Node, Error> {
        if name.is_empty()
            || !name.is_ascii()
            || tree_name.is_some_and(|s| s.is_empty() || !s.is_ascii())
        {
            return Err(Error("invalid ANM lookup name"));
        }
        let tree = if let Some(name) = tree_name {
            let dictionary = u32_at(&self.data, self.root + 32)? as usize;
            if u32_at(&self.data, dictionary + 8)? & 2 == 0 {
                return Err(Error("unsupported ANM tree name comparison profile"));
            }
            self.trees
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(name))
        } else {
            self.trees.iter().find(|t| t.offset == self.default_tree)
        }
        .ok_or(Error("missing selected ANM tree"))?;
        if u32_at(&self.data, tree.offset + 8)? & 2 == 0 {
            return Err(Error("unsupported ANM node name comparison profile"));
        }
        let offset = tree
            .entries
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .ok_or(Error("missing selected ANM node"))?
            .1;
        self.node(offset)
    }
}
fn pointer(data: &[u8], at: usize, size: usize, nullable: bool) -> Result<usize, Error> {
    let value = u32_at(data, at)? as usize;
    if nullable && value == 0 {
        return Ok(0);
    }
    if value == 0 || !value.is_multiple_of(4) {
        return Err(Error("unaligned/null ANM pointer"));
    }
    slice(data, value, size)?;
    Ok(value)
}
pub fn floats<const N: usize>(data: &[u8], at: usize) -> Result<[f32; N], Error> {
    let mut values = [0.; N];
    for (i, v) in values.iter_mut().enumerate() {
        *v = f32::from_bits(u32_at(data, at + i * 4)?);
        if !v.is_finite() {
            return Err(Error("non-finite ANM scalar"));
        }
    }
    Ok(values)
}
fn names(data: &[u8], descriptor: usize) -> Result<Vec<(String, usize)>, Error> {
    let pool = pointer(data, descriptor + 4, 40, false)?;
    let count = u32_at(data, pool + 28)? as usize;
    if !(1..=4096).contains(&count) || u32_at(data, pool + 32)? != 1 {
        return Err(Error("unsupported NAM table"));
    }
    let meta = pointer(data, pool + 36, 28, false)?;
    let array = pointer(data, meta + 16, count * 4, false)?;
    let mut result = Vec::with_capacity(count);
    let mut seen = BTreeSet::new();
    for i in 0..count {
        let entry = pointer(data, array + i * 4, 12, false)?;
        let size = u16::from_le_bytes(slice(data, entry, 2)?.try_into().unwrap()) as usize;
        let custom = u16::from_le_bytes(slice(data, entry + 2, 2)?.try_into().unwrap());
        if size < 12 || !size.is_multiple_of(4) || custom != 4 {
            return Err(Error("invalid NAM entry"));
        }
        let raw = slice(data, entry + 8, size - 8)?;
        let end = raw
            .iter()
            .position(|v| *v == 0)
            .ok_or(Error("unterminated NAM label"))?;
        if !(1..=127).contains(&end) || !raw[..end].is_ascii() {
            return Err(Error("invalid NAM label"));
        }
        let name = String::from_utf8(raw[..end].to_vec()).unwrap();
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(Error("duplicate NAM label"));
        }
        result.push((name, pointer(data, entry + 4, 4, false)?));
    }
    Ok(result)
}
pub fn parse(data: &[u8]) -> Result<Animation, Error> {
    parse_profile(data, false)
}
/// Decode represented nodes while retaining unresolved auxiliary links. Every
/// evaluated dependency must still resolve through Animation::node; bounds,
/// represented-node validation and cycle guards remain unchanged.
pub fn parse_for_bindings(data: &[u8]) -> Result<Animation, Error> {
    parse_profile(data, true)
}
fn parse_profile(data: &[u8], retain_unresolved: bool) -> Result<Animation, Error> {
    if !(32..=2 * 1024 * 1024).contains(&data.len()) || u32_at(data, 0)? as usize != data.len() {
        return Err(Error("invalid ANM image size"));
    }
    let group = pointer(data, 4, 32, false)?;
    let root = pointer(data, group + 28, 48, false)?;
    if u32_at(data, root + 4)? != 11 {
        return Err(Error("unsupported ANM root tag"));
    }
    let timeline = std::array::from_fn(|i| u32_at(data, root + 16 + i * 4).unwrap() as i32);
    if timeline[0] > timeline[1] || timeline[2] <= 0 || timeline[3] <= 0 {
        return Err(Error("invalid ANM timeline"));
    }
    let dictionary = pointer(data, root + 32, 12, false)?;
    let default_tree = pointer(data, root + 36, 12, false)?;
    let mut trees = Vec::new();
    let mut offsets = BTreeSet::new();
    for (name, offset) in names(data, dictionary)? {
        let entries = names(data, offset)?;
        offsets.extend(entries.iter().map(|(_, o)| *o));
        trees.push(Tree {
            name,
            offset,
            entries,
        });
    }
    let declared_node_count = u32_at(data, root + 40)? as usize;
    if !trees.iter().any(|t| t.offset == default_tree)
        || declared_node_count > 65536
        || offsets.len() > declared_node_count
    {
        return Err(Error("ANM tree/node count mismatch"));
    }
    let mut nodes = Vec::with_capacity(offsets.len());
    for &offset in &offsets {
        slice(data, offset, 164)?;
        let raw = u32_at(data, offset + 4)?;
        let kind = raw as u16;
        let transform = (raw >> 16) as u16;
        if transform > 3 || pointer(data, offset + 12, 48, false)? != root {
            return Err(Error("unsupported ANM transform/backlink"));
        }
        let parent = pointer(data, offset + 16, 164, true)?;
        let target = pointer(data, offset + 20, 164, true)?;
        if !retain_unresolved
            && (parent != 0 && !offsets.contains(&parent)
                || target != 0 && !offsets.contains(&target))
        {
            return Err(Error("unknown ANM parent/target"));
        }
        let tracks = [
            pointer(data, offset + 0x98, 12, true)?,
            pointer(data, offset + 0x9c, 12, true)?,
            pointer(data, offset + 0xa0, 12, true)?,
        ];
        let camera = if kind == 1 {
            Some((
                floats::<1>(data, offset + 0xec)?[0],
                pointer(data, offset + 0xf8, 12, true)?,
            ))
        } else {
            None
        };
        nodes.push(Node {
            offset,
            kind,
            transform,
            parent,
            target,
            translation: floats(data, offset + 0x6c)?,
            rotation: floats(data, offset + 0x78)?,
            scale: floats(data, offset + 0x88)?,
            flags: u32_at(data, offset + 0x94)?,
            tracks,
            callback: u32_at(data, offset + 24)?,
            camera,
        });
    }
    let parents: BTreeMap<_, _> = nodes.iter().map(|n| (n.offset, n.parent)).collect();
    for node in &nodes {
        let mut seen = BTreeSet::new();
        let mut at = node.offset;
        while at != 0 {
            if !seen.insert(at) || seen.len() > 128 {
                return Err(Error("cyclic/deep ANM parent chain"));
            }
            let Some(parent) = parents.get(&at) else {
                break;
            };
            at = *parent;
        }
    }
    Ok(Animation {
        data: data.to_vec(),
        root,
        timeline,
        default_tree,
        declared_node_count,
        trees,
        nodes,
    })
}
pub fn inline_flk(data: &[u8]) -> Result<Animation, Error> {
    if data.len() < 40 || u32_at(data, 0)? != 3 || u32_at(data, 4)? as usize != data.len() {
        return Err(Error("unsupported FLK framing"));
    }
    let at = u32_at(data, 36)? as usize;
    if at < 40 || !at.is_multiple_of(4) {
        return Err(Error("invalid FLK ANM offset"));
    }
    let size = u32_at(data, at)? as usize;
    if at.checked_add(size) != Some(data.len()) {
        return Err(Error("inline ANM does not end at FLK boundary"));
    }
    parse(slice(data, at, size)?)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn alloc(b: &mut Vec<u8>, size: usize) -> usize {
        let at = b.len();
        b.resize(at + ((size + 3) & !3), 0);
        at
    }
    fn word(b: &mut [u8], at: usize, v: usize) {
        b[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
    }
    fn mapping(b: &mut Vec<u8>, entries: &[(&str, usize)]) -> usize {
        let desc = alloc(b, 12);
        let pool = alloc(b, 40);
        let meta = alloc(b, 28);
        let array = alloc(b, entries.len() * 4);
        word(b, desc + 4, pool);
        word(b, pool + 28, entries.len());
        word(b, pool + 32, 1);
        word(b, pool + 36, meta);
        word(b, meta + 16, array);
        for (i, (label, target)) in entries.iter().enumerate() {
            let size = (8 + label.len() + 1 + 3) & !3;
            let e = alloc(b, size);
            b[e..e + 2].copy_from_slice(&(size as u16).to_le_bytes());
            b[e + 2..e + 4].copy_from_slice(&4u16.to_le_bytes());
            word(b, e + 4, *target);
            b[e + 8..e + 8 + label.len()].copy_from_slice(label.as_bytes());
            word(b, array + i * 4, e);
        }
        desc
    }
    fn fixture() -> (Vec<u8>, usize, usize) {
        let mut b = vec![0; 8];
        let node = alloc(&mut b, 164);
        let root = alloc(&mut b, 48);
        word(&mut b, node + 12, root);
        for at in [0x84, 0x88, 0x8c, 0x90] {
            b[node + at..node + at + 4].copy_from_slice(&1f32.to_le_bytes());
        }
        let tree = mapping(&mut b, &[("Point", node)]);
        let dict = mapping(&mut b, &[("Default Tree", tree)]);
        word(&mut b, root + 4, 11);
        for (i, v) in [0, 4800, 160, 30].into_iter().enumerate() {
            word(&mut b, root + 16 + i * 4, v);
        }
        word(&mut b, root + 32, dict);
        word(&mut b, root + 36, tree);
        word(&mut b, root + 40, 1);
        let group = alloc(&mut b, 32);
        word(&mut b, group + 28, root);
        let size = b.len();
        word(&mut b, 0, size);
        word(&mut b, 4, group);
        (b, node, root)
    }
    #[test]
    fn explicit_tree_lookup_honors_default_and_rejects_unproved_comparison_mode() {
        let (mut b, node, root) = fixture();
        let tree = u32_at(&b, root + 36).unwrap() as usize;
        let dictionary = u32_at(&b, root + 32).unwrap() as usize;
        word(&mut b, tree + 8, 2);
        word(&mut b, dictionary + 8, 2);
        let animation = parse(&b).unwrap();
        assert_eq!(animation.named_node(None, "POINT").unwrap().offset, node);
        assert_eq!(
            animation
                .named_node(Some("default tree"), "pOiNt")
                .unwrap()
                .offset,
            node
        );
        assert!(animation.named_node(Some("missing"), "Point").is_err());
        assert!(animation.named_node(None, "missing").is_err());
        word(&mut b, tree + 8, 0);
        assert!(parse(&b).unwrap().named_node(None, "Point").is_err());
    }
    #[test]
    fn surplus_declared_nodes_do_not_invent_named_nodes_or_offsets() {
        let (mut data, _, root) = fixture();
        word(&mut data, root + 40, 2);
        let animation = parse(&data).unwrap();
        assert_eq!(animation.declared_node_count, 2);
        assert_eq!(animation.nodes.len(), 1);
        word(&mut data, root + 40, 0);
        assert!(parse(&data).is_err());
        word(&mut data, root + 40, usize::MAX);
        assert!(parse(&data).is_err());
    }
    #[test]
    fn binding_profile_retains_unresolved_edges_without_inventing_nodes() {
        let (mut data, node, root) = fixture();
        let unrepresented = alloc(&mut data, 164);
        let size = data.len();
        word(&mut data, 0, size);
        word(&mut data, root + 40, 2);
        word(&mut data, node + 16, unrepresented);
        assert!(parse(&data).is_err());
        let a = parse_for_bindings(&data).unwrap();
        assert_eq!(a.nodes.len(), 1);
        assert_eq!(
            a.unresolved_links(),
            vec![UnresolvedLink {
                node,
                target: unrepresented,
                look_at: false
            }]
        );
        assert!(a.node(unrepresented).is_err());
        word(&mut data, node + 16, node);
        assert!(parse_for_bindings(&data).is_err());
        word(&mut data, node + 16, size + 4);
        assert!(parse_for_bindings(&data).is_err());
    }
    #[test]
    fn graph_bounds_parent_cycles_and_finite_pose() {
        let (b, node, root) = fixture();
        assert_eq!(
            parse(&b).unwrap().default_node("point").unwrap().offset,
            node
        );
        for end in 8..b.len() {
            let mut short = b[..end].to_vec();
            word(&mut short, 0, end);
            assert!(parse(&short).is_err());
        }
        for (at, v) in [(node + 16, node), (root + 36, 0xfffffff0)] {
            let mut bad = b.clone();
            word(&mut bad, at, v);
            assert!(parse(&bad).is_err());
        }
        let mut bad = b;
        bad[node + 0x6c..node + 0x70].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(parse(&bad).is_err());
    }
    #[test]
    fn inline_image_must_match_complete_container() {
        let (b, _, _) = fixture();
        let mut f = vec![0; 40];
        f.extend(b);
        let size = f.len();
        word(&mut f, 0, 3);
        word(&mut f, 4, size);
        word(&mut f, 36, 40);
        assert!(inline_flk(&f).is_ok());
        f.push(0);
        let size = f.len();
        word(&mut f, 4, size);
        assert!(inline_flk(&f).is_err());
    }
}
