//! Held-object replacement used by the Mecc secondary and ANM auxiliary events.
//! This executes the acquired routing, not sound playback. Construction,
//! destruction and effect cancellation must execute in the caller's
//! owned world. A failed provider is an error, never an acknowledged request.

#[derive(Clone, Copy, Debug)]
pub struct Child<K> {
    pub id: K,
    pub class: i32,
    pub flags: u32,
    pub parent: Option<K>,
    pub reference: i32,
}

pub trait World<K: Copy + Eq> {
    /// Current native list6 order, including dead objects and other owners.
    fn children(&self) -> Result<Vec<Child<K>>, String>;
    /// Mark8000; actual resource/list cleanup belongs to the later world phase.
    fn destroy(&mut self, child: K) -> Result<(), String>;
    fn construct(&mut self, class: i32, mask: u32) -> Result<K, String>;
    /// Copy the parent's context, position and yaw; OR child flags20400000;
    /// bind parent, child reference0 and the supplied parent reference.
    fn bind(&mut self, child: K, parent: K, reference: i32) -> Result<(), String>;
    fn is_mecc(&self, parent: K) -> Result<bool, String>;
    /// If present, cancel the parent's actual payload19c context then clear it.
    fn cancel_footprint(&mut self, parent: K) -> Result<(), String>;
    fn find_equipment(
        &mut self,
        parent: K,
        reference: i32,
        class: i32,
    ) -> Result<Option<K>, String>;
    fn set_equipment_active(&mut self, child: K, active: bool) -> Result<(), String>;
    fn refresh_equipment(&mut self, parent: K) -> Result<(), String>;
}

/// Full replacement helper control flow, including the special class898 tail.
/// Providers may already have changed the world when a later operation fails;
/// a caller must quarantine that frame rather than blindly retry it.
pub fn replace<K: Copy + Eq>(
    world: &mut impl World<K>,
    parent: K,
    reference: i32,
    class: i32,
) -> Result<(), String> {
    let previous = world.children()?.into_iter().find(|child| {
        child.flags & 0x8000 == 0 && child.parent == Some(parent) && child.reference == reference
    });
    if previous.is_some_and(|child| child.class == class) {
        return Ok(());
    }
    let old_class = previous.map_or(-1, |child| child.class);
    if let Some(child) = previous {
        world.destroy(child.id)?;
    }
    if class <= 0 {
        return Ok(());
    }
    let child = world.construct(class, 0x20)?;
    world.bind(child, parent, reference)?;
    if world.is_mecc(parent)? {
        world.cancel_footprint(parent)?;
    }
    if class == 898 {
        if let Some(equipment) = world.find_equipment(parent, 10, 898)? {
            world.set_equipment_active(equipment, true)?;
        }
    } else if old_class == 898 {
        if let Some(equipment) = world.find_equipment(parent, 10, 898)? {
            world.set_equipment_active(equipment, false)?;
        }
        world.refresh_equipment(parent)?;
    }
    Ok(())
}

use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    specification, state_resources,
    state_storage::{Profile, Seed},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub parent: Id,
    pub reference: i32,
    pub context: u32,
}
struct Template {
    class: i32,
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    fields: Fields,
}
impl Template {
    fn load(resources: &SceneResources, class: i32) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(class)
            .ok_or("held-object class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let specification = Arc::new(specification::Prepared::load_registered(resources, class)?);
        let states = Arc::new(state_resources::Prepared::load(
            resources,
            class,
            Profile::Debug,
        )?);
        let fields = Fields::load(resources, class, &specification)?;
        if seed.storage.state() != 0
            || seed.storage.data().is_some()
            || !seed.diagnostics.is_empty()
            || !specification.needs.is_empty()
            || !states.needs().is_empty()
        {
            return Err("held-object constructor outside acquired source state0 profile".into());
        }
        Ok(Self {
            class,
            seed,
            bindings: resources.class_state_bindings_for(class, Profile::Debug)?,
            flags: DirectFlags::project(class, program),
            specification,
            states,
            fields,
        })
    }
    fn input(&self) -> Construction<'_> {
        Construction {
            class: self.class,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.specification.clone(),
            state_resources: self.states.clone(),
            fields: self.fields.instantiate(),
        }
    }
}
/// Mission-owned prepared hand templates retain their exact resource owner.
/// A temporary factory can borrow this cache without borrowing its own struct.
pub struct Cache {
    resources: std::sync::Arc<SceneResources>,
    templates: BTreeMap<i32, Template>,
}
impl Cache {
    pub fn new(resources: std::sync::Arc<SceneResources>) -> Self {
        Self {
            resources,
            templates: BTreeMap::new(),
        }
    }
}
enum Templates<'a> {
    Owned(BTreeMap<i32, Template>),
    Shared(&'a mut BTreeMap<i32, Template>),
}
impl std::ops::Deref for Templates<'_> {
    type Target = BTreeMap<i32, Template>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(v) => v,
            Self::Shared(v) => v,
        }
    }
}
impl std::ops::DerefMut for Templates<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(v) => v,
            Self::Shared(v) => v,
        }
    }
}
pub struct Factory<'a> {
    resources: &'a SceneResources,
    templates: Templates<'a>,
}
impl<'a> Factory<'a> {
    pub fn new(resources: &'a SceneResources) -> Self {
        Self {
            resources,
            templates: Templates::Owned(BTreeMap::new()),
        }
    }
    pub fn from_cache(cache: &'a mut Cache) -> Self {
        Self {
            resources: &cache.resources,
            templates: Templates::Shared(&mut cache.templates),
        }
    }
    fn prepare(&mut self, class: i32) -> Result<(), String> {
        if class == 898 {
            return Err("owned special equipment replacement services pending".into());
        }
        if class > 0 && !self.templates.contains_key(&class) {
            self.templates
                .insert(class, Template::load(self.resources, class)?);
        }
        Ok(())
    }
    pub(crate) fn preflight_weapon(
        &mut self,
        session: &Session,
        parent: Id,
        catalog: &giants_formats::weapons::Catalog,
    ) -> Result<(), String> {
        session.check_wield_owner(parent)?;
        let selected = session.wield_selection(parent)?;
        self.prepare(crate::held_weapon::wield_class(catalog, selected, false)?)?;
        self.prepare(crate::held_weapon::wield_class(catalog, selected, true)?)
    }
    /// Actual two-hand service followed by the selected-weapon cache/flag tail.
    /// The secondary still decides when this helper is called.
    pub fn commit_weapon(
        &mut self,
        session: &mut Session,
        parent: Id,
        catalog: &giants_formats::weapons::Catalog,
    ) -> Result<(), String> {
        session.check_wield_owner(parent)?;
        let selected = session.wield_selection(parent)?;
        let left = crate::held_weapon::wield_class(catalog, selected, false)?;
        let right = crate::held_weapon::wield_class(catalog, selected, true)?;
        self.prepare(left)?;
        self.prepare(right)?;
        let result = (|| {
            self.replace(session, parent, 0, left)?;
            self.replace(session, parent, 1, right)?;
            session.finish_wield_commit(parent, selected)
        })();
        if let Err(error) = &result {
            session.fail_wield_owner(parent, error.clone());
        }
        result
    }
    /// Real owned cold-player children. Unsupported resources are rejected
    /// before marking an existing child; started failures quarantine this owner.
    pub fn replace(
        &mut self,
        session: &mut Session,
        parent: Id,
        reference: i32,
        class: i32,
    ) -> Result<(), String> {
        session.check_wield_owner(parent)?;
        self.prepare(class)?;
        let mut world = OwnedWorld {
            session,
            template: self.templates.get(&class),
        };
        let result = replace(&mut world, parent, reference, class);
        if let Err(error) = &result {
            world.session.fail_wield_owner(parent, error.clone());
        }
        result
    }
}
struct OwnedWorld<'a> {
    session: &'a mut Session,
    template: Option<&'a Template>,
}
impl World<Id> for OwnedWorld<'_> {
    fn children(&self) -> Result<Vec<Child<Id>>, String> {
        self.session.wield_list()
    }
    fn destroy(&mut self, child: Id) -> Result<(), String> {
        self.session.mark_wield_child(child)
    }
    fn construct(&mut self, class: i32, mask: u32) -> Result<Id, String> {
        let template = self
            .template
            .ok_or("held-object constructor not prepared")?;
        if class != template.class || mask != 0x20 {
            return Err("held-object constructor identity/mask changed".into());
        }
        self.session.construct_wield_child(template.input())
    }
    fn bind(&mut self, child: Id, parent: Id, reference: i32) -> Result<(), String> {
        self.session.bind_wield_child(child, parent, reference)
    }
    fn is_mecc(&self, parent: Id) -> Result<bool, String> {
        Ok(self
            .session
            .get(parent)
            .ok_or("wield parent removed")?
            .seed
            .storage
            .state()
            == 2)
    }
    fn cancel_footprint(&mut self, parent: Id) -> Result<(), String> {
        self.session.check_wield_footprint(parent)
    }
    fn find_equipment(&mut self, _: Id, _: i32, _: i32) -> Result<Option<Id>, String> {
        Err("owned special equipment query pending".into())
    }
    fn set_equipment_active(&mut self, _: Id, _: bool) -> Result<(), String> {
        Err("owned special equipment update pending".into())
    }
    fn refresh_equipment(&mut self, _: Id) -> Result<(), String> {
        Err("owned special equipment refresh pending".into())
    }
}
