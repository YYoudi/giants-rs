//! Ordinary Mecc camera invocation order, distinct from world frame numbering.
use crate::{
    first_person::Camera,
    mecc_camera_update as camera,
    resident_states::{Id, Session},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    Inner,
    Outer,
}
#[derive(Clone, Debug)]
pub struct Publication<K> {
    source: K,
    frame: u64,
    pass: Pass,
    previous: Option<Camera>,
    output: camera::Output,
}
impl<K: Copy> Publication<K> {
    pub fn source(&self) -> K {
        self.source
    }
    pub fn frame(&self) -> u64 {
        self.frame
    }
    pub fn pass(&self) -> Pass {
        self.pass
    }
    pub fn previous(&self) -> Option<Camera> {
        self.previous
    }
    pub fn output(&self) -> &camera::Output {
        &self.output
    }
}
pub struct State<K> {
    source: K,
    completed: u64,
    inner: Option<u64>,
    invocation: u64,
    seed: Option<Camera>,
    camera: camera::State,
    last: Option<Publication<K>>,
    failure: Option<String>,
}
impl<K: Copy + Eq> State<K> {
    /// Initial camera globals are an explicit startup boundary. An unknown
    /// initial view permits an outer pass, but not subjective placement first.
    pub fn new(source: K, seed: Option<Camera>) -> Self {
        Self {
            source,
            completed: 0,
            inner: None,
            invocation: 0,
            seed,
            camera: camera::State::default(),
            last: None,
            failure: None,
        }
    }
    pub fn last(&self) -> Option<&Publication<K>> {
        self.last.as_ref()
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub(crate) fn placement_previous(&self) -> Result<Camera, String> {
        self.last
            .as_ref()
            .map(|p| p.output.camera)
            .or(self.seed)
            .ok_or_else(|| "subjective camera requires known startup globals".into())
    }
    pub fn outer_output(&self, frame: u64) -> Result<&camera::Output, String> {
        self.last
            .as_ref()
            .filter(|p| p.frame == frame && p.pass == Pass::Outer && self.failure.is_none())
            .map(|p| &p.output)
            .ok_or_else(|| "render camera requires this frame's completed outer pass".into())
    }
    fn check(&self, frame: u64, pass: Pass) -> Result<(), String> {
        if self.failure.is_some()
            || self.completed.checked_add(1) != Some(frame)
            || match pass {
                Pass::Inner => self.inner.is_some(),
                Pass::Outer => self.inner.is_some_and(|n| n != frame),
            }
            || pass == Pass::Inner && self.last.is_none() && self.seed.is_none()
        {
            return Err(
                "camera pass is stale, failed, out of order or lacks startup globals".into(),
            );
        }
        Ok(())
    }
    fn advance(
        &mut self,
        frame: u64,
        pass: Pass,
        mut input: camera::Input,
        world: &mut impl camera::World,
    ) -> Result<Publication<K>, String> {
        self.check(frame, pass)?;
        let previous = self.last.as_ref().map(|p| p.output.camera).or(self.seed);
        input.number = self
            .invocation
            .checked_add(1)
            .ok_or("camera invocation overflow")?;
        let result = self.camera.advance(input, world);
        match result {
            Ok(output) => {
                self.invocation = output.number;
                if pass == Pass::Inner {
                    self.inner = Some(frame);
                } else {
                    self.completed = frame;
                    self.inner = None;
                }
                let publication = Publication {
                    source: self.source,
                    frame,
                    pass,
                    previous,
                    output,
                };
                self.last = Some(publication.clone());
                Ok(publication)
            }
            Err(error) => {
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }
}
impl State<Id> {
    pub(crate) fn check_inner(
        &self,
        session: &Session,
        source: Id,
        frame: u64,
    ) -> Result<(), String> {
        if self.source != source {
            return Err("camera belongs to another source".into());
        }
        self.check(frame, Pass::Inner)?;
        session.mecc_camera_phase_input(source, frame, true, false, 0.)?;
        Ok(())
    }
    fn inner(
        &mut self,
        session: &Session,
        frame: u64,
        first_person: bool,
        seconds: f32,
        world: &mut impl camera::World,
    ) -> Result<Publication<Id>, String> {
        self.check_inner(session, self.source, frame)?;
        let input =
            session.mecc_camera_phase_input(self.source, frame, true, first_person, seconds)?;
        self.advance(frame, Pass::Inner, input, world)
    }
    pub fn outer(
        &mut self,
        session: &Session,
        frame: u64,
        first_person: bool,
        seconds: f32,
        world: &mut impl camera::World,
    ) -> Result<Publication<Id>, String> {
        self.check(frame, Pass::Outer)?;
        let input =
            session.mecc_camera_phase_input(self.source, frame, false, first_person, seconds)?;
        self.advance(frame, Pass::Outer, input, world)
    }
}

pub(crate) trait Producer {
    fn publish(
        &mut self,
        session: &Session,
        source: Id,
        frame: u64,
    ) -> Result<Publication<Id>, String>;
}
pub(crate) struct Inner<'a, W> {
    pub state: &'a mut State<Id>,
    pub first_person: bool,
    pub seconds: f32,
    pub world: &'a mut W,
}
impl<W: camera::World> Producer for Inner<'_, W> {
    fn publish(
        &mut self,
        session: &Session,
        source: Id,
        frame: u64,
    ) -> Result<Publication<Id>, String> {
        self.state.check_inner(session, source, frame)?;
        self.state
            .inner(session, frame, self.first_person, self.seconds, self.world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct World {
        calls: usize,
        fail: bool,
    }
    impl camera::World for World {
        fn height(&mut self, _: [f32; 2]) -> Result<f32, String> {
            self.calls += 1;
            Ok(0.)
        }
        fn clip(&mut self, _: &mut [f32; 3], _: [f32; 3]) -> Result<(), String> {
            self.calls += 1;
            if self.fail {
                Err("injected clip failure".into())
            } else {
                Ok(())
            }
        }
    }
    fn input() -> camera::Input {
        camera::Input {
            number: 0,
            position: [0., 0., 10.],
            yaw: 0.,
            pitch: 0.,
            look_height: 20.,
            scale: 0.6,
            skip_floor: false,
            seconds: 1. / 60.,
            profile: camera::Profile::normal(true),
        }
    }
    #[test]
    fn inner_and_outer_have_separate_invocations_and_one_world_frame() {
        let seed = Camera {
            position: [0.; 3],
            angles: [0.; 3],
        };
        let mut s = State::new(1, Some(seed));
        let mut w = World {
            calls: 0,
            fail: false,
        };
        let inner = s.advance(1, Pass::Inner, input(), &mut w).unwrap();
        assert_eq!(inner.output.number, 1);
        assert_eq!(inner.frame, 1);
        assert_eq!(inner.previous.unwrap().position, seed.position);
        assert!(s.outer_output(1).is_err());
        let calls = w.calls;
        assert!(s.advance(1, Pass::Inner, input(), &mut w).is_err());
        assert!(s.advance(2, Pass::Outer, input(), &mut w).is_err());
        assert_eq!(w.calls, calls);
        let outer = s.advance(1, Pass::Outer, input(), &mut w).unwrap();
        assert_eq!(outer.output.number, 2);
        assert_eq!(s.outer_output(1).unwrap().number, 2);
        assert_eq!(
            outer.previous.unwrap().position,
            inner.output.camera.position
        );
        assert!(s.advance(1, Pass::Outer, input(), &mut w).is_err());
        let next = s.advance(2, Pass::Outer, input(), &mut w).unwrap();
        assert_eq!(next.output.number, 3);
    }
    #[test]
    fn unknown_startup_and_failed_pass_do_not_fabricate_or_republish_a_view() {
        let mut s = State::new(1, None);
        let mut w = World {
            calls: 0,
            fail: false,
        };
        assert!(s.advance(1, Pass::Inner, input(), &mut w).is_err());
        assert_eq!(w.calls, 0);
        s.advance(1, Pass::Outer, input(), &mut w).unwrap();
        w.fail = true;
        assert!(s.advance(2, Pass::Inner, input(), &mut w).is_err());
        assert!(s.failure().is_some());
        let calls = w.calls;
        assert_eq!(s.last().unwrap().frame, 1);
        assert!(s.outer_output(2).is_err());
        assert!(s.advance(2, Pass::Inner, input(), &mut w).is_err());
        assert_eq!(w.calls, calls);
    }
}
