//! Shared acquired camera-override writers. Startup, actual owner resolution,
//! nonmatrix view production and world ordering remain separate services.
use crate::procedural_aim::Matrix;

#[derive(Clone, Debug)]
pub struct State<K> {
    pub enabled: bool,
    pub descriptor: Option<i32>,
    pub owner: Option<K>,
    /// Two uninterpreted three-word inputs, in setter argument order.
    pub triplets: [[u32; 3]; 2],
    pub matrix_selected: bool,
    /// Clear retains the actual matrix; absence never means identity.
    pub retained_view: Option<Matrix>,
    /// Additional acquired reset word; its wider meaning remains unknown.
    pub reset_word: u32,
    /// Producer metadata, separate from the acquired override-word projection.
    pub vertical_fov: Option<f32>,
    pub failure: Option<String>,
}
impl<K> Default for State<K> {
    /// Empty component staging, not a reconstruction of native camera startup.
    fn default() -> Self {
        Self {
            enabled: false,
            descriptor: None,
            owner: None,
            triplets: [[0; 3]; 2],
            matrix_selected: false,
            retained_view: None,
            reset_word: 0,
            vertical_fov: None,
            failure: None,
        }
    }
}
impl<K> State<K> {
    fn check(&self) -> Result<(), &'static str> {
        if self.failure.is_some() {
            Err("camera override producer is quarantined")
        } else {
            Ok(())
        }
    }
    pub fn clear_override(&mut self) -> Result<(), &'static str> {
        self.check()?;
        self.descriptor = None;
        self.enabled = false;
        self.matrix_selected = false;
        self.reset_word = 0;
        Ok(())
    }
    pub fn bind_owner(&mut self, owner: Option<K>, reference: i32) -> Result<(), &'static str> {
        self.check()?;
        self.owner = owner;
        self.descriptor = (reference != -1).then_some(reference);
        self.matrix_selected = false;
        self.enabled = true;
        Ok(())
    }
    /// Preserve descriptor/owner/matrix selection and retained matrix.
    pub fn copy_triplets(&mut self, first: [u32; 3], second: [u32; 3]) -> Result<(), &'static str> {
        self.check()?;
        self.triplets = [first, second];
        self.enabled = true;
        Ok(())
    }
    pub fn select_matrix(&mut self, matrix: Matrix) -> Result<(), &'static str> {
        self.check()?;
        self.descriptor = None;
        self.matrix_selected = true;
        self.retained_view = Some(matrix);
        self.enabled = true;
        Ok(())
    }
    /// This narrow consumer does not acknowledge active owner/vector routes.
    pub fn active_view(&self) -> Result<Option<Matrix>, &'static str> {
        self.check()?;
        if !self.enabled {
            return Ok(None);
        }
        if !self.matrix_selected {
            return Err("nonmatrix forced camera requires its view producer");
        }
        self.retained_view
            .map(Some)
            .ok_or("selected camera override matrix absent")
    }
    /// Read the same current mode and force state at every caller query.
    pub fn target_uses_viewport(&self, mode: &crate::camera_mode::State) -> Result<bool, String> {
        self.check()?;
        mode.target_uses_viewport(u32::from(self.enabled))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonmatrix_override_cannot_be_acknowledged_as_an_ordinary_view() {
        let mut s = State::<u32>::default();
        let mode = crate::camera_mode::State::new(0, 1, 0.).unwrap();
        assert!(s.target_uses_viewport(&mode).unwrap());
        s.bind_owner(Some(7), -7).unwrap();
        assert_eq!(s.descriptor, Some(-7));
        assert!(!s.target_uses_viewport(&mode).unwrap());
        assert!(s.active_view().is_err());
        s.select_matrix(Matrix::identity()).unwrap();
        s.reset_word = 19;
        s.clear_override().unwrap();
        assert_eq!(s.owner, Some(7));
        assert!(s.retained_view.is_some());
        assert_eq!(s.reset_word, 0);
        assert!(s.active_view().unwrap().is_none());
        assert!(s.target_uses_viewport(&mode).unwrap());
        s.failure = Some("required producer failure".into());
        assert!(s.bind_owner(None, -1).is_err());
        assert!(s.target_uses_viewport(&mode).is_err());
        assert_eq!(s.owner, Some(7));
    }
}

#[cfg(test)]
mod snapshot_tests {
    #[test]
    fn selector_snapshots_read_the_current_common_override_without_reusing_a_boolean() {
        let mode = crate::camera_mode::State::new(0, u32::MAX, 0.).unwrap();
        let mut state = super::State::<u32>::default();
        let snapshot = |state: &super::State<u32>| {
            crate::delphi_target_scan::View::from_shared(
                &mode,
                state,
                crate::first_person::Camera {
                    position: [1., 2., 3.],
                    angles: [0.; 3],
                },
                [1., 2., 3.],
                [0., -1., 0.],
            )
            .unwrap()
        };
        let before = snapshot(&state);
        assert!(before.uses_viewport());
        state
            .select_matrix(crate::procedural_aim::Matrix::identity())
            .unwrap();
        assert!(!snapshot(&state).uses_viewport());
        assert!(before.uses_viewport());
        state.clear_override().unwrap();
        assert!(snapshot(&state).uses_viewport());
    }
}
