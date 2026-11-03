//! Current object references with explicit matrix/pose inputs, at53-bit precision.
//! This does not infer animated reference matrices or a projectile trajectory.
pub struct Object<'a> {
    pub position: [f32; 3],
    pub angles: [f32; 3],
    pub scale: f32,
    pub flags: u32,
    pub cached: [f32; 16],
    pub references: &'a [[f32; 16]],
}
pub fn point(object: &Object<'_>, index: i32) -> Result<[f32; 3], &'static str> {
    let local = if (index as u8 as i8) < 0 {
        [0.; 3]
    } else if let Ok(index) = usize::try_from(index) {
        if let Some(matrix) = object.references.get(index) {
            [matrix[3], matrix[7], matrix[11]]
        } else {
            return Ok(object.position);
        }
    } else {
        return Ok(object.position);
    };
    // This helper consumes fields D0/CC/C8 in a different order from the
    // render-cache writer represented by object_transform::euler.
    let matrix = if object.flags & 0x20 != 0 {
        object.cached
    } else {
        crate::object_transform::euler(
            object.position,
            [object.angles[2], object.angles[1], object.angles[0]],
            object.scale,
        )?
    };
    if !matrix.into_iter().chain(local).all(f32::is_finite) {
        return Err("non-finite object reference transform outside measured profile");
    }
    Ok(std::array::from_fn(|r| {
        let m = &matrix[r * 4..r * 4 + 4];
        let x = f64::from(m[0]) * f64::from(local[0]);
        let y = f64::from(m[1]) * f64::from(local[1]);
        let z = f64::from(m[2]) * f64::from(local[2]);
        // The two native branches sum in different orders at x87 precision53.
        let sum = if object.flags & 0x20 != 0 {
            (z + x) + y
        } else {
            (z + y) + x
        };
        (sum + f64::from(m[3])) as f32
    }))
}
