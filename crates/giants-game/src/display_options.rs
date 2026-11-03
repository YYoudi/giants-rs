//! Independent display candidate selection. The app supplies actual supported modes.
#[derive(Default)]
pub struct DisplayOptions {
    pub modes: Vec<[u32; 2]>,
    pub current: [u32; 2],
    pub pending: [u32; 2],
    pub initial: [u32; 2],
    pub top: usize,
}
impl DisplayOptions {
    pub fn new(modes: impl IntoIterator<Item = [u32; 2]>, current: [u32; 2]) -> Self {
        let mut modes: Vec<_> = modes
            .into_iter()
            .filter(|mode| mode.iter().all(|v| (1..=16384).contains(v)))
            .collect();
        if current.iter().all(|v| (1..=16384).contains(v)) {
            modes.push(current);
        }
        modes.sort_unstable();
        modes.dedup();
        let top = modes
            .iter()
            .position(|mode| *mode == current)
            .unwrap_or(0)
            .saturating_sub(6);
        Self {
            modes,
            current,
            pending: current,
            initial: current,
            top,
        }
    }
    pub fn choose(&mut self, mode: [u32; 2]) -> bool {
        if !self.modes.contains(&mode) {
            return false;
        }
        self.pending = mode;
        true
    }
    pub fn page(&mut self, direction: i8) {
        self.top = self
            .top
            .saturating_add_signed(isize::from(direction.signum()) * 7)
            .min(self.modes.len().saturating_sub(7));
    }
    pub fn cycle(&mut self, direction: i8) {
        if let Some(index) = self.modes.iter().position(|mode| *mode == self.pending) {
            let index = index
                .saturating_add_signed(isize::from(direction.signum()))
                .min(self.modes.len() - 1);
            self.pending = self.modes[index];
            if index < self.top {
                self.top = index;
            }
            if index >= self.top + 7 {
                self.top = index - 6;
            }
        }
    }
    pub fn visible(&self) -> &[[u32; 2]] {
        &self.modes[self.top..(self.top + 7).min(self.modes.len())]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn candidates_do_not_change_active_mode_and_inventory_is_bounded() {
        let mut options = DisplayOptions::new(
            [[800, 600], [0, 600], [800, 600], [20000, 600], [1280, 720]],
            [1920, 1080],
        );
        assert_eq!(options.modes, [[800, 600], [1280, 720], [1920, 1080]]);
        assert!(!options.choose([42, 42]));
        assert!(options.choose([800, 600]));
        assert_eq!(options.current, [1920, 1080]);
        options.cycle(1);
        assert_eq!(options.pending, [1280, 720]);
        for _ in 0..20 {
            options.page(1);
            options.cycle(1);
        }
        assert_eq!(options.pending, [1920, 1080]);
        assert_eq!(options.visible().len(), 3);
    }
}
