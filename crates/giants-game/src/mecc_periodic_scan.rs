//! Quarter-second global-clock scan over the ordered common actor list.
//! Selected identities require the real collection callback; this is its plan.
pub struct Candidate<I> {
    pub id: I,
    pub class: i32,
    pub flags: u32,
    pub control: u32,
    pub position: [f32; 3],
}
pub struct Plan<I> {
    pub timer: f32,
    pub scanned: bool,
    pub selected: Vec<I>,
}
pub fn plan<I: Copy>(
    timer: f32,
    global_seconds: f32,
    position: [f32; 3],
    candidates: &[Candidate<I>],
) -> Result<Plan<I>, &'static str> {
    if ![timer, global_seconds, position[0], position[1]]
        .into_iter()
        .all(f32::is_finite)
        || global_seconds < 0.
    {
        return Err("Mecc periodic scan outside finite global-clock profile");
    }
    // The native compare uses the unrounded register value after the FST store.
    let next = f64::from(timer) + f64::from(global_seconds);
    let scanned = next >= 0.25;
    let mut selected = Vec::new();
    if scanned {
        for c in candidates {
            if c.flags & 0x8000 != 0 || !(0x45d..=0x45f).contains(&c.class) || c.control & 1 == 0 {
                continue;
            }
            if !c.position[..2].iter().copied().all(f32::is_finite) {
                return Err("Mecc periodic candidate has nonfinite XY position");
            }
            let dx = f64::from(c.position[0]) - f64::from(position[0]);
            let dy = f64::from(c.position[1]) - f64::from(position[1]);
            if dx * dx + dy * dy < 100. {
                selected.push(c.id);
            }
        }
    }
    Ok(Plan {
        timer: if scanned { 0. } else { next as f32 },
        scanned,
        selected,
    })
}
