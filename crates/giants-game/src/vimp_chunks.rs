//! Source-owned state80 chunk births and offline callbacks. Rendering and
//! whole-world scheduling remain separate consumers.
//! Prototype PS2 search keys: `create_one_vimp_chunk`, `vimpchunk_move`.
//! Birth/update roles are navigation candidates; exact PC attributions remain
//! unknown. See docs/SYMBOL_NAVIGATION.md; no signature/layout is imported.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    audio_parameters::Random,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    specification, state_resources,
    state_storage::{Profile, Seed},
};
use giants_formats::global_data::GlobalData;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct Chunk {
    pub context: u32,
    pub amount: f32,
    pub timer: f32,
    pub rates: [f32; 6],
    pub fade: f32,
    pub(crate) number: u64,
    pub(crate) failed: bool,
    /// Host ownership witness set only by the acquired collection callback.
    /// This is not a native field and never changes the dissolution value.
    pub(crate) collected: bool,
}
pub struct Birth {
    pub position: [f32; 3],
    pub flags: u32,
    pub amount: i32,
    /// Zero uses and advances the owned class rotation.
    pub class: i32,
    pub scale: f32,
}
struct Template {
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    fields: Fields,
    context: u32,
}
impl Template {
    fn load(resources: &SceneResources, class: i32) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(class)
            .ok_or("chunk class absent")?
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
        let (fields, context) = Fields::load_with_context(resources, class, &specification)?;
        if seed.storage.state() != 80
            || seed.storage.data().is_some()
            || !seed.diagnostics.is_empty()
            || !specification.needs.is_empty()
            || !states.needs().is_empty()
        {
            return Err("chunk template outside acquired source state80 profile".into());
        }
        Ok(Self {
            seed,
            bindings: resources.class_state_bindings_for(class, Profile::Debug)?,
            flags: DirectFlags::project(class, program),
            specification,
            states,
            fields,
            context,
        })
    }
}
pub struct Factory<'a> {
    resources: &'a SceneResources,
    templates: BTreeMap<i32, Template>,
    next_class: i32,
}
impl<'a> Factory<'a> {
    /// Restore the owning scene's acquired round-robin class cursor while
    /// borrowing its resources for this batch; no class choice is resampled.
    pub fn with_next_class(resources: &'a SceneResources, next_class: i32) -> Result<Self, String> {
        if !(1117..=1119).contains(&next_class) {
            return Err("chunk class cursor outside source rotation".into());
        }
        Ok(Self {
            resources,
            templates: BTreeMap::new(),
            next_class,
        })
    }
    pub fn new(resources: &'a SceneResources) -> Self {
        Self {
            resources,
            templates: BTreeMap::new(),
            next_class: 1117,
        }
    }
    pub fn next_class(&self) -> i32 {
        self.next_class
    }
    pub fn birth(
        &mut self,
        session: &mut Session,
        birth: Birth,
        globals: &GlobalData,
        random: &mut Random,
    ) -> Result<Id, String> {
        if !birth
            .position
            .into_iter()
            .chain([birth.scale])
            .all(f32::is_finite)
            || birth.scale <= 0.
            || !(birth.class == 0 || (1117..=1119).contains(&birth.class))
        {
            return Err("chunk birth outside finite acquired class/scale profile".into());
        }
        let class = if birth.class == 0 {
            self.next_class
        } else {
            birth.class
        };
        let offsets = [0x24, 0x2e4, 0x2e8, 0x2ec, 0x2f0, 0x2f4];
        let values = offsets.map(|at| globals.scalar(at).map_err(|e| e.to_string()));
        let values = values.into_iter().collect::<Result<Vec<_>, _>>()?;
        if let std::collections::btree_map::Entry::Vacant(entry) = self.templates.entry(class) {
            entry.insert(Template::load(self.resources, class)?);
        }
        let t = &self.templates[&class];
        let mut staged_random = *random;
        let interval = |low: f32, high: f32, r: &mut Random| {
            f64::from(r.next_15()) * (f64::from(high) - f64::from(low)) * (1. / 32768.)
                + f64::from(low)
        };
        let mut fields = t.fields.instantiate();
        fields.scale = birth.scale;
        fields.position = birth.position;
        fields.current = birth.position;
        fields.previous = birth.position;
        fields.angles = std::array::from_fn(|_| {
            (f64::from(staged_random.next_15()) * 360. * (1. / 32768.)) as f32
        });
        fields.configuration[15] |= 1;
        fields.control_flags = birth.flags;
        let mut rates = [0.; 6];
        if birth.flags & 4 != 0 {
            rates[0] = (interval(values[2], values[3], &mut staged_random) * 30.) as f32;
        }
        if birth.flags & 8 != 0 {
            rates[1] = (interval(-values[1], values[1], &mut staged_random) * 15.) as f32;
            rates[2] = (interval(-values[1], values[1], &mut staged_random) * 15.) as f32;
        }
        rates[3] = interval(-values[5], values[5], &mut staged_random) as f32;
        rates[4] = interval(-values[4], values[4], &mut staged_random) as f32;
        rates[5] = interval(-values[4], values[4], &mut staged_random) as f32;
        if !rates.into_iter().all(f32::is_finite) {
            return Err("chunk rate overflow".into());
        }
        let chunk = Chunk {
            context: t.context,
            amount: birth.amount as f32,
            timer: values[0],
            rates,
            fade: 0.,
            number: 0,
            failed: false,
            collected: false,
        };
        let id = session.construct_vimp_chunk(
            Construction {
                class,
                seed: &t.seed,
                bindings: &t.bindings,
                flags: t.flags,
                specification: t.specification.clone(),
                state_resources: t.states.clone(),
                fields,
            },
            chunk,
        )?;
        self.next_class = if birth.class != 0 {
            class
        } else if class == 1119 {
            1117
        } else {
            class + 1
        };
        *random = staged_random;
        Ok(id)
    }
}

/// Clock and offline authority supplied by the actual owning world.
pub struct Environment<'a> {
    pub number: u64,
    pub globals: &'a GlobalData,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub offline: bool,
}
impl Chunk {
    pub fn collected(&self) -> bool {
        self.collected
    }
    pub fn number(&self) -> u64 {
        self.number
    }
    pub fn failed(&self) -> bool {
        self.failed
    }
    pub(crate) fn advance(
        &mut self,
        fields: &mut Fields,
        environment: Environment<'_>,
        mut ground: impl FnMut([f32; 3]) -> Result<f32, String>,
    ) -> Result<(), String> {
        if self.failed
            || self.number.checked_add(1) != Some(environment.number)
            || !environment.offline
            || !environment.global_seconds.is_finite()
            || environment.global_seconds < 0.
            || fields.object_flags & 0x8000 != 0
        {
            return Err("chunk update requires contiguous unretired offline state".into());
        }
        let values = [0x24, 0x2fc, 0x300, 0x304]
            .map(|at| environment.globals.scalar(at).map_err(|e| e.to_string()));
        let values = values.into_iter().collect::<Result<Vec<_>, _>>()?;
        if values.iter().any(|x| *x < 0.)
            || !fields
                .position
                .into_iter()
                .chain(fields.angles)
                .chain(self.rates)
                .chain([self.timer, self.fade])
                .all(f32::is_finite)
            || environment.areas.iter().any(|a| {
                !a.center
                    .into_iter()
                    .chain([a.radius_squared, a.factor])
                    .all(f32::is_finite)
                    || a.radius_squared < 0.
                    || a.factor < 0.
            })
        {
            return Err("chunk update outside acquired finite time/field profile".into());
        }
        let seconds = crate::object_time::seconds(
            environment.global_seconds,
            Some(crate::object_time::Sample {
                owner: self.context,
                position: fields.position,
            }),
            None,
            environment.areas,
        );
        let result = (|| {
            if fields.control_flags & 1 != 0 {
                if self.timer > 0. {
                    self.timer = (f64::from(self.timer) - seconds) as f32;
                } else {
                    let fade = f64::from(self.fade) + seconds;
                    self.fade = fade as f32;
                    if fade >= 1. {
                        self.fade = 1.;
                        fields.object_flags |= 0x8000;
                    }
                }
                if ![self.timer, self.fade].into_iter().all(f32::is_finite) {
                    return Err("chunk active timer overflow".into());
                }
                return Ok(());
            }
            self.rates[0] = (f64::from(self.rates[0]) - f64::from(values[1]) * seconds) as f32;
            for (index, rate) in self.rates.iter_mut().enumerate().skip(1) {
                let amount = f64::from(values[if index < 3 { 2 } else { 3 }]) * seconds;
                let old = f64::from(*rate);
                let next = if old > 0. {
                    (old - amount).max(0.)
                } else if old < 0. {
                    (old + amount).min(0.)
                } else {
                    old
                };
                *rate = next as f32;
            }
            for (position, rate) in
                fields
                    .position
                    .iter_mut()
                    .zip([self.rates[1], self.rates[2], self.rates[0]])
            {
                *position = (f64::from(*position) + seconds * f64::from(rate)) as f32;
            }
            for (angle, rate) in fields.angles.iter_mut().zip(&self.rates[3..]) {
                *angle = (f64::from(*angle) + seconds * f64::from(*rate)) as f32;
            }
            if !fields
                .position
                .into_iter()
                .chain(fields.angles)
                .chain(self.rates)
                .all(f32::is_finite)
            {
                return Err("chunk motion overflow before ground".into());
            }
            let height = ground(fields.position)?;
            if !height.is_finite() {
                return Err("chunk ground height is nonfinite".into());
            }
            if self.rates[0] <= 0. && f64::from(fields.position[2]) - 2. <= f64::from(height) {
                fields.position[2] = (f64::from(height) + 2.) as f32;
                self.timer = values[0];
                fields.angles[1..].fill(0.);
                self.rates.fill(0.);
                fields.control_flags |= 1;
            }
            if !fields
                .position
                .into_iter()
                .chain(fields.angles)
                .chain(self.rates)
                .chain([self.timer, self.fade])
                .all(f32::is_finite)
            {
                return Err("chunk update overflow".into());
            }
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        } else {
            self.number = environment.number;
        }
        result
    }
}
