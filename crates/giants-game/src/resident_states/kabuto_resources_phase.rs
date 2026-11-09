use super::*;
use crate::{
    kabuto_post_control as guard, kabuto_resource_phase as phase, kabuto_resource_timers as timers,
};

struct Adapter<'a, S> {
    session: &'a mut Session,
    services: &'a mut S,
    owner: Id,
    resources: &'a Resources,
    number: u64,
    valid: bool,
}
impl<S: phase::Services> Adapter<'_, S> {
    fn actor(&self) -> Result<guard::Actor, String> {
        let a = self
            .session
            .get(self.owner)
            .ok_or("Kabuto resource owner absent")?;
        let g = a
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto resource group absent")?;
        if a.class != 1
            || a.seed.storage.state() != 1
            || g.cleaned_up
            || g.geometry_transition_failure.is_some()
            || g.resource_phase_entered != Some(self.number)
            || !Arc::ptr_eq(&g.declared_class, &self.resources.specification)
            || self
                .session
                .specification(self.owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &self.resources.specification))
        {
            return Err("Kabuto resource owner changed during its phase".into());
        }
        let f = a
            .construction_fields
            .as_ref()
            .ok_or("Kabuto resource fields absent")?;
        let p = a
            .seed
            .storage
            .data()
            .filter(|p| p.len() == 844)
            .ok_or("Kabuto resource payload changed")?;
        let (damage, limit) = self.session.kabuto_damage_state(self.owner)?;
        Ok(guard::Actor {
            heading: f.angles[0],
            flags: f.object_flags,
            damage,
            limit,
            state: u32::from(a.seed.storage.state()),
            buttons: u32::from_le_bytes(g.motion.commands[4..].try_into().unwrap()),
            physics_flags: u32::from_le_bytes(p[0xe4..0xe8].try_into().unwrap()),
        })
    }
    fn publish(&mut self, header: Option<&guard::Actor>, p: &[u8]) -> Result<(), String> {
        self.actor()?;
        if p.len() != 844 {
            return Err("Kabuto resource publication payload differs".into());
        }
        let a = self.session.residents.get_mut(&self.owner.serial).unwrap();
        if let Some(h) = header {
            a.construction_fields.as_mut().unwrap().object_flags = h.flags;
            a.mecc_damage = Some(h.damage);
        }
        a.seed.storage.data_mut().unwrap().copy_from_slice(p);
        a.kabuto_group.as_mut().unwrap().physics =
            Some(crate::kabuto_physics::Physics::from_owned_payload(p));
        Ok(())
    }
    fn send(
        &mut self,
        request: phase::Request,
        mut header: Option<&mut guard::Actor>,
        p: &mut [u8],
    ) -> Result<(), String> {
        self.publish(header.as_deref(), p)?;
        let result = self.services.execute(self.session, self.owner, request);
        // Pull completed effects even when the provider refuses. A changed or
        // removed owner is not overwritten by the stale working copy.
        match self.actor() {
            Ok(current) => {
                if let Some(h) = header.as_mut() {
                    **h = current;
                }
                p.copy_from_slice(
                    self.session
                        .get(self.owner)
                        .unwrap()
                        .seed
                        .storage
                        .data()
                        .unwrap(),
                );
            }
            Err(error) => {
                self.valid = false;
                return result.and(Err(error));
            }
        }
        result
    }
    fn notifications(&mut self) -> Result<bool, String> {
        self.services.network(self.session, self.owner)?.eligible()
    }
}
impl<S: phase::Services> guard::World for Adapter<'_, S> {
    fn checkpoint(&mut self, a: &guard::Actor, p: &[u8]) -> Result<(), String> {
        self.publish(Some(a), p)
    }
    fn local_owner(&mut self) -> Result<bool, String> {
        self.services.local_owner(self.session, self.owner)
    }
    fn timer(&mut self) -> Result<f32, String> {
        self.services.timer(self.session)
    }
    fn global_seconds(&mut self) -> Result<f32, String> {
        self.services.global_seconds(self.session)
    }
    fn store_timer(&mut self, v: f32) -> Result<(), String> {
        self.services.store_timer(self.session, v)
    }
    fn mode(&mut self) -> Result<u32, String> {
        self.services.mode(self.session)
    }
    fn notifications(&mut self) -> Result<bool, String> {
        Adapter::notifications(self)
    }
    fn start_sound(&mut self, id: u32, a: &mut guard::Actor, p: &mut [u8]) -> Result<(), String> {
        self.send(phase::Request::StartSound(id), Some(a), p)
    }
    fn notify(
        &mut self,
        n: guard::Notification,
        a: &mut guard::Actor,
        p: &mut [u8],
    ) -> Result<(), String> {
        self.send(
            match n {
                guard::Notification::Sound(id) => phase::Request::NotifySound(id),
                guard::Notification::Growth(n) => phase::Request::NotifyGrowth(n),
            },
            Some(a),
            p,
        )
    }
    fn reinitialize(
        &mut self,
        class: u32,
        a: &mut guard::Actor,
        p: &mut [u8],
    ) -> Result<(), String> {
        self.send(phase::Request::Reinitialize(class), Some(a), p)
    }
}
impl<S: phase::Services> timers::World for Adapter<'_, S> {
    fn seconds(&mut self, p: &[u8]) -> Result<f64, String> {
        self.publish(None, p)?;
        let c = self.services.clock(self.session, self.owner)?;
        let f = self
            .session
            .get(self.owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        crate::object_time_precision::seconds(
            c.global_seconds,
            Some(crate::object_time_precision::Actor {
                owner_id: f.dispatch_context,
                position: f.position[..2].try_into().unwrap(),
            }),
            None,
            &c.areas,
            crate::arithmetic::Precision::Double53,
        )
        .map_err(str::to_owned)
    }
    fn notifications(&mut self, p: &[u8]) -> Result<bool, String> {
        self.publish(None, p)?;
        Adapter::notifications(self)
    }
    fn event(
        &mut self,
        e: timers::Event,
        p: &mut [u8],
        input: &mut timers::Input,
    ) -> Result<(), String> {
        let result = self.send(
            match e {
                timers::Event::StartSound(id) => phase::Request::StartSound(id),
                timers::Event::NotifySound(id) => phase::Request::NotifySound(id),
            },
            None,
            p,
        );
        if self.valid {
            // Native later weight/charge consumers reread these header values.
            // Early frame/model/activity scratch remains the earlier sample.
            let a = self.actor()?;
            input.damage = a.damage;
            input.limit = a.limit;
            input.buttons = a.buttons;
        }
        result
    }
}
impl Session {
    pub(crate) fn advance_kabuto_resources(
        &mut self,
        owner: Id,
        source: (&Resources, &giants_formats::global_data::GlobalData),
        number: u64,
        services: &mut impl phase::Services,
    ) -> Result<phase::View, String> {
        let (resources, globals) = source;
        let a = self
            .get(owner)
            .ok_or("Kabuto resource owner absent or foreign")?;
        let g = a
            .kabuto_group
            .as_ref()
            .ok_or("Kabuto resource group absent")?;
        if a.class != 1
            || a.seed.storage.state() != 1
            || g.cleaned_up
            || g.geometry_transition_failure.is_some()
            || g.motion_history != Some(number)
            || g.control_force_completed != Some(number)
            || g.resource_phase_entered.is_some()
            || g.resource_phase_completed.is_some()
            || !Arc::ptr_eq(&g.declared_class, &resources.specification)
            || self
                .specification(owner)
                .is_none_or(|s| !Arc::ptr_eq(s, &resources.specification))
        {
            return Err(
                "Kabuto resources require their factory-owned unconsumed control-force phase"
                    .into(),
            );
        }
        let p = a
            .seed
            .storage
            .data()
            .filter(|p| p.len() == 844)
            .ok_or("Kabuto resource payload absent")?;
        if g.physics.is_none_or(|v| {
            v.words() != crate::kabuto_physics::Physics::from_owned_payload(p).words()
        }) {
            return Err("Kabuto resource physical cache differs from payload".into());
        }
        self.kabuto_damage_state(owner)?;
        let mut p = p.to_vec();
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .resource_phase_entered = Some(number);
        let mut w = Adapter {
            session: self,
            services,
            owner,
            resources,
            number,
            valid: true,
        };
        let mut result: Result<phase::View, String> = (|| {
            let mut header = w.actor()?;
            let pass = guard::advance(&mut header, &mut p, &mut w)?;
            w.publish(Some(&header), &p)?;
            // Retain scratch even when a later required consumer refuses.
            w.session
                .residents
                .get_mut(&owner.serial)
                .unwrap()
                .kabuto_group
                .as_mut()
                .unwrap()
                .resource_phase_view = Some(phase::View {
                guard: pass,
                resources: None,
            });
            if pass.outcome != guard::Outcome::ResourceTimers {
                return Ok(phase::View {
                    guard: pass,
                    resources: None,
                });
            }
            let a = w.session.get(owner).unwrap();
            let f = a.construction_fields.as_ref().unwrap();
            if a.channels != Some(f.channels) {
                return Err("Kabuto resource channel mirror differs".into());
            }
            let slot = u8::try_from(f.channels[0][0])
                .map_err(|_| "Kabuto resource descriptor slot outside table")?;
            let descriptor = resources
                .specification
                .animations
                .get(&slot)
                .ok_or("Kabuto resource current descriptor absent")?;
            let g = a.kabuto_group.as_ref().unwrap();
            let header = w.actor()?;
            let input = timers::Input {
                channels: f.channels,
                descriptor: crate::animation_frames::Descriptor {
                    timeline: descriptor.animation.timeline,
                    flags: descriptor.template.flags,
                },
                model: g.motion.model,
                mode: g.motion.mode,
                buttons: header.buttons,
                damage: header.damage,
                limit: header.limit,
            };
            let output = timers::advance(&mut p, input, globals, &mut w)?;
            Ok(phase::View {
                guard: pass,
                resources: Some(output),
            })
        })();
        if w.valid {
            // Error returns still publish stores since the last service boundary.
            if let Err(error) = w.publish(None, &p) {
                w.valid = false;
                if result.is_ok() {
                    result = Err(error);
                }
            }
        }
        let result = result.and_then(|v| {
            if w.valid {
                Ok(v)
            } else {
                Err("Kabuto resource owner invalidated by service".into())
            }
        });
        if w.session.get(owner).is_some()
            && let Some(g) = w
                .session
                .residents
                .get_mut(&owner.serial)
                .and_then(|a| a.kabuto_group.as_mut())
                .filter(|g| {
                    g.resource_phase_entered == Some(number)
                        && Arc::ptr_eq(&g.declared_class, &resources.specification)
                })
        {
            match &result {
                Ok(v) => {
                    g.resource_phase_view = Some(*v);
                    if v.resources.is_some() {
                        g.resource_phase_completed = Some(number);
                    }
                }
                Err(e) => g.geometry_transition_failure = Some(e.clone()),
            }
        }
        // Exit/tail requests leave completion unset and cannot replay. A full
        // primary/history/world consumer is still required in every case.
        result
    }
}
