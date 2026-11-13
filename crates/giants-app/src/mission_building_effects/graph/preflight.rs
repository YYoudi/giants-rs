//! Complete resource/service graph admission before ordinary damage can call it.
//! This walks source class identities and every retirement edge, not sampled
//! runtime timing or a successful native actor/world update.
use super::*;
use std::collections::BTreeSet;
impl Runtime {
    pub(crate) fn preflight_offset(
        &mut self,
        binding: &giants_game::model_registry::StateBinding,
    ) -> Result<(), String> {
        let giants_game::model_registry::StateTarget::Effect {
            image,
            entry,
            offset,
        } = binding.target.as_ref().map_err(Clone::clone)?
        else {
            return Err("building effect admission requires its captured effect binding".into());
        };
        let active = self
            .source
            .classes
            .models
            .effects
            .last()
            .ok_or("building effect image absent")?;
        if &active.path != image
            || active.image.entries.get(*entry).map(|e| e.offset) != Some(*offset)
        {
            return Err("building effect admission needs its captured source image owner".into());
        }
        if self.checked_graphs.contains(offset) {
            return Ok(());
        }
        let name = &active.image.entries[*entry].name;
        if giants_game::effect_burst::Plan::project(&active.image, name).is_ok() {
            return Ok(());
        }
        let mut staged = Runtime::new(self.source.clone(), self.terrain.clone());
        staged.bind(0)?;
        staged.check_graph(*offset, &mut BTreeSet::new(), &mut BTreeMap::new())?;
        self.checked_graphs.insert(*offset);
        Ok(())
    }
    fn check_graph(
        &mut self,
        offset: u32,
        visiting: &mut BTreeSet<u32>,
        completed: &mut BTreeMap<u32, usize>,
    ) -> Result<usize, String> {
        if let Some(count) = completed.get(&offset) {
            return Ok(*count);
        }
        if visiting.len() >= 9 || !visiting.insert(offset) {
            return Err(
                "building effect retirement graph has a cycle or exceeds depth budget".into(),
            );
        }
        let mut random = Random(17);
        let (Creation::Prepared(id), _) =
            self.contexts
                .prepare_positional_offset(Some(offset), [0.; 3], &mut random)?
        else {
            return Err("building effect admission source context absent".into());
        };
        let (runtime, references) = self.contexts.consume_positional_scene(
            id,
            &self.source,
            self.terrain.clone(),
            &mut random,
        )?;
        let graph = Graph::create(self, runtime, references, visiting.len() - 1)?;
        let mut total = graph.nodes.len();
        for (body, node) in graph.runtime.bodies[0].iter().zip(&graph.nodes) {
            if let Some(binding) = &body.construction.object_services.retirement {
                if node.object_flags & 0x400 != 0 {
                    return Err("building effect admission needs inherited child context".into());
                }
                let image = self
                    .image
                    .as_ref()
                    .ok_or("building effect admission image lost")?;
                let (_, entry) = image
                    .adaptive_find(&binding.effect, true)
                    .map_err(|e| e.to_string())?
                    .ok_or("building effect admission descendant absent")?;
                if entry.offset != binding.program_offset {
                    return Err(
                        "building effect admission needs actual descendant selection inputs".into(),
                    );
                }
                total = total
                    .checked_add(self.check_graph(binding.program_offset, visiting, completed)?)
                    .ok_or("building effect retirement population overflow")?;
                if total > 4096 {
                    return Err("building effect retirement population budget".into());
                }
            }
        }
        visiting.remove(&offset);
        completed.insert(offset, total);
        Ok(total)
    }
}
