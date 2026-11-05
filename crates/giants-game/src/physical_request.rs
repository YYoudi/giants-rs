//! Finite inline request reduction. Zero is the acquired unset sentinel.
pub fn reduce(current: f32, requested: f32) -> Result<f32, &'static str> {
    if !current.is_finite() || !requested.is_finite() {
        return Err("physical request requires finite operands");
    }
    Ok(if current > requested || current == 0. {
        requested
    } else {
        current
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lower_request_and_zero_sentinel_preserve_the_native_order() {
        assert_eq!(reduce(1., 2.), Ok(1.));
        assert_eq!(reduce(2., 1.), Ok(1.));
        assert_eq!(reduce(0., 2.), Ok(2.));
        assert_eq!(reduce(-0., 2.), Ok(2.));
        assert_eq!(reduce(-1., 0.5), Ok(-1.));
        assert!(reduce(f32::NAN, 1.).is_err());
    }
}
