//! Contact material names from the same prepared collision objects and parts.
//! No supporting mesh is a known null material; an unknown mesh/part is an error.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    materials: BTreeMap<(u32, u32), String>,
}
impl Catalog {
    pub fn from_prepared(
        objects: &[crate::player_world_collision::Object],
        materials: BTreeMap<(u32, u32), String>,
    ) -> Result<Self, String> {
        let mut ids = BTreeSet::new();
        for object in objects {
            let id = object.fields.id;
            if id == 0 || !ids.insert(id) {
                return Err("contact material collision identity absent or duplicated".into());
            }
            for triangle in &object.triangles {
                if !materials.contains_key(&(id, triangle.part)) {
                    return Err("contact material collision part has no source binding".into());
                }
            }
        }
        if materials
            .iter()
            .any(|(&(id, _), name)| !ids.contains(&id) || !name.is_ascii())
        {
            return Err(
                "contact material binding is foreign or outside ASCII source profile".into(),
            );
        }
        Ok(Self { materials })
    }
    pub fn resolve(&self, support: u32, part: u32) -> Result<Option<&str>, String> {
        if support == 0 {
            return Ok(None);
        }
        self.materials
            .get(&(support, part))
            .map(|s| Some(s.as_str()))
            .ok_or_else(|| "current contact material object/part is not bound".into())
    }
}
