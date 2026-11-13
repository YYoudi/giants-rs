use super::*;

struct Supplied {
    seconds: f32,
    film: Option<String>,
    trace: Vec<&'static str>,
    failure: crate::mission_failure::State,
    fail_at: Option<&'static str>,
}
impl Supplied {
    fn new(seconds: f32, film: Option<&str>) -> Self {
        Self {
            seconds,
            film: film.map(str::to_owned),
            trace: Vec::new(),
            failure: crate::mission_failure::State {
                failed: 0,
                elapsed: 0.,
            },
            fail_at: None,
        }
    }
    fn boundary(&mut self, name: &'static str) -> Result<(), String> {
        self.trace.push(name);
        if self.fail_at == Some(name) {
            Err(format!("supplied {name} refusal"))
        } else {
            Ok(())
        }
    }
}
impl Services for Supplied {
    fn seconds(&mut self) -> Result<f32, String> {
        self.boundary("clock")?;
        Ok(self.seconds)
    }
    fn failure_film(&mut self, state: &State) -> Result<Option<String>, String> {
        assert!(state.remaining <= 0.);
        self.boundary("metadata")?;
        Ok(self.film.clone())
    }
    fn request_failure(&mut self, state: &mut State) -> Result<(), String> {
        assert!(state.remaining <= 0.);
        self.failure.request(crate::mission_failure::Context {
            counter: 0,
            required: 1,
            completed: 0,
            seconds: self.seconds,
        })?;
        self.boundary("failure")
    }
    fn start_film(&mut self, state: &mut State, name: &str) -> Result<(), String> {
        assert!(state.remaining <= 0.);
        assert_eq!(state.film_started, 0);
        assert_eq!(Some(name), self.film.as_deref());
        self.boundary("film")
    }
    fn failure_menu(&mut self, state: &mut State) -> Result<(), String> {
        assert!(state.remaining <= 0.);
        assert_ne!(state.film_started, 0);
        self.boundary("menu")
    }
}
fn gates() -> Gates {
    Gates { failed: 0, edit: 0 }
}

#[test]
fn display_call_gate_never_expires_an_inactive_timer() {
    for remaining in [-1., -0., 0., f32::from_bits(0x7fc00001)] {
        let mut state = State {
            remaining,
            film_started: 0,
        };
        let mut services = Supplied::new(0.5, None);
        services.fail_at = Some("clock");
        assert_eq!(
            state
                .advance_display_countdown(gates(), &mut services)
                .unwrap(),
            None
        );
        assert_eq!(state.remaining.to_bits(), remaining.to_bits());
        assert!(services.trace.is_empty());
    }
    let mut state = State {
        remaining: 0.125,
        film_started: 0,
    };
    let mut services = Supplied::new(0.5, None);
    assert_eq!(
        state
            .advance_display_countdown(gates(), &mut services)
            .unwrap(),
        Some(Outcome::FailureRequested)
    );
    assert_eq!(services.failure.failed, 1);
    assert_eq!(state.remaining.to_bits(), 0.001f32.to_bits());
}

#[test]
fn suppressed_timer_never_reads_clock_or_unavailable_metadata() {
    for gates in [Gates { failed: 3, edit: 0 }, Gates { failed: 0, edit: 7 }] {
        let mut state = State {
            remaining: f32::from_bits(0x7fc00001),
            film_started: 9,
        };
        let mut services = Supplied::new(f32::NAN, None);
        services.fail_at = Some("clock");
        assert_eq!(
            state.advance(gates, &mut services).unwrap(),
            Outcome::Suppressed
        );
        assert_eq!(state.remaining.to_bits(), 0x7fc00001);
        assert_eq!(state.film_started, 9);
        assert!(services.trace.is_empty());
    }
}

#[test]
fn counting_and_negative_clock_do_not_require_expired_services() {
    let mut state = State {
        remaining: 0.25,
        film_started: 9,
    };
    let mut services = Supplied::new(-0.5, None);
    services.fail_at = Some("metadata");
    assert_eq!(
        state.advance(gates(), &mut services).unwrap(),
        Outcome::Counting
    );
    assert_eq!(state.remaining, 0.75);
    assert_eq!(services.trace, ["clock"]);
    services.seconds = f32::INFINITY;
    assert!(state.advance(gates(), &mut services).is_err());
    assert_eq!(state.remaining, 0.75);
}

#[test]
fn expired_metadata_failure_preserves_the_reached_timer_store() {
    let mut state = State {
        remaining: 0.125,
        film_started: 0,
    };
    let mut services = Supplied::new(0.5, None);
    services.fail_at = Some("metadata");
    assert!(state.advance(gates(), &mut services).is_err());
    assert_eq!(state.remaining, -0.375);
    assert_eq!(state.film_started, 0);
    assert_eq!(services.failure.failed, 0);
    assert_eq!(services.trace, ["clock", "metadata"]);
}

#[test]
fn missing_consumers_do_not_acknowledge_retry_reset_or_film_start() {
    for (film, started, failure) in [
        (None, 0, "failure"),
        (Some("failure_scene"), 0, "film"),
        (None, 7, "menu"),
    ] {
        let mut state = State {
            remaining: 0.,
            film_started: started,
        };
        let mut services = Supplied::new(0.5, film);
        services.fail_at = Some(failure);
        assert!(state.advance(gates(), &mut services).is_err());
        assert_eq!(state.remaining, -0.5);
        assert_eq!(state.film_started, started);
        // Actual failure-producer writes survive a later boundary refusal.
        assert_eq!(services.failure.failed, u32::from(failure == "failure"));
        assert_eq!(services.trace, ["clock", "metadata", failure]);
    }
}

#[test]
#[ignore = "requires private exact-PC timer controls; supplied film/menu effects"]
fn original_mission_timer_controls() {
    let path = std::env::var("GIANTS_MISSION_TIMER_CONTROLS").expect("private controls path");
    let rows = std::fs::read_to_string(path).unwrap();
    let mut count = 0;
    for row in rows.lines() {
        let w: Vec<u32> = row
            .split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 14);
        let mut state = State {
            remaining: f32::from_bits(w[2]),
            film_started: w[3],
        };
        let mut services =
            Supplied::new(f32::from_bits(w[4]), (w[5] != 0).then_some("failure_scene"));
        services.failure.failed = w[0];
        state
            .advance(
                Gates {
                    failed: w[0],
                    edit: w[1],
                },
                &mut services,
            )
            .unwrap();
        let actual = [
            state.remaining.to_bits(),
            state.film_started,
            services.failure.failed,
            services.failure.elapsed.to_bits(),
            services.trace.iter().filter(|v| **v == "clock").count() as u32,
            services.trace.iter().filter(|v| **v == "film").count() as u32,
            services.trace.iter().filter(|v| **v == "failure").count() as u32,
            services.trace.iter().filter(|v| **v == "menu").count() as u32,
        ];
        assert_eq!(actual, w[6..], "row {count}");
        count += 1;
    }
    assert_eq!(count, 192);
    let path = std::env::var("GIANTS_MISSION_TIMER_DISPLAY_CONTROLS")
        .expect("private display gate controls");
    let rows = std::fs::read_to_string(path).unwrap();
    let mut count = 0;
    for row in rows.lines() {
        let w: Vec<u32> = row
            .split_whitespace()
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 3);
        let mut state = State {
            remaining: f32::from_bits(w[1]),
            film_started: 0,
        };
        let mut services = Supplied::new(0., None);
        services.fail_at = Some("clock"); // Stop at the admitted helper's clock boundary.
        let result = state.advance_display_countdown(
            Gates {
                failed: w[0],
                edit: 0,
            },
            &mut services,
        );
        assert_eq!(u32::from(result.is_err()), w[2], "display row {count}");
        assert_eq!(state.remaining.to_bits(), w[1]);
        assert_eq!(services.trace.len() as u32, w[2]);
        count += 1;
    }
    assert_eq!(count, 16);
}
