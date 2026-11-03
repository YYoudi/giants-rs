//! Dead-Mecc clip selection fragment. Ground/contact flags and pose override
//! are supplied; corpse creation, ground alignment and respawn remain separate.
pub struct Input {
    pub flags: u32,
    pub current: u32,
    pub forced: i32,
    pub z: f32,
    pub vertical_speed: f32,
    pub ground_offset: f32,
    pub scale: f32,
    pub floor: f32,
}
pub fn select(input: Input) -> Result<Option<u32>, &'static str> {
    if ![
        input.z,
        input.vertical_speed,
        input.ground_offset,
        input.scale,
        input.floor,
    ]
    .into_iter()
    .all(f32::is_finite)
    {
        return Err("death selector outside finite profile");
    }
    if input.flags & 0x8000000 != 0 || input.z < 0. {
        return Ok(Some(143));
    }
    if input.forced > 0 {
        return Ok(Some(input.forced as u32));
    }
    let height = f64::from(input.z)
        - f64::from(input.ground_offset) * f64::from(input.scale)
        - f64::from(input.floor);
    let mut selected =
        (input.current != 136 && (input.vertical_speed > 10. || height > 10.)).then_some(136);
    if input.flags & 0x80000 != 0 {
        if input.current == 136 {
            selected = Some(23);
        } else if ![144, 23, 202].contains(&input.current) {
            selected = Some(if input.flags & 0x800 != 0 { 202 } else { 144 });
        }
    }
    Ok(selected)
}
