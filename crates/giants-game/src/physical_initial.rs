//! Shared inline physical defaults. Word five is deliberately preserved.
pub fn reset(words: &mut [u32; 15]) {
    for (index, word) in words.iter_mut().enumerate() {
        if index != 5 {
            *word = if matches!(index, 1 | 2 | 4) {
                1f32.to_bits()
            } else {
                0
            };
        }
    }
}
