//! Initial base-layout requests, with explicit owner/state inputs.
//! Allocation, reference-point evaluation, buildings and Ready remain separate.

pub struct Input {
    pub owner_flags: u32,
    pub layout_class: u32,
    pub owner_position: [f32; 3],
    pub placement_offset: [f32; 3],
    pub owner_yaw: f32,
    pub placement_yaw: f32,
}
#[derive(Debug)]
pub struct TemplateRequest {
    pub class: u32,
    pub position: [f32; 3],
    pub yaw: f32,
    pub animation_arguments: [u32; 5],
    /// Supplied initialization advance, not a cinematic or wall-clock timestep.
    pub initial_advance: f32,
}
pub struct Bootstrap {
    pub owner_flags: u32,
    /// None corresponds to the original obsolete-placement diagnostic.
    pub template: Option<TemplateRequest>,
}
pub fn bootstrap(input: Input) -> Result<Bootstrap, &'static str> {
    let owner_flags = input.owner_flags | 1;
    if input.layout_class == 0 {
        return Ok(Bootstrap {
            owner_flags,
            template: None,
        });
    }
    if input
        .owner_position
        .iter()
        .chain(&input.placement_offset)
        .chain([&input.owner_yaw, &input.placement_yaw])
        .any(|v| !v.is_finite())
    {
        return Err("base layout request outside finite source profile");
    }
    let subtract = |a: f32, b: f32| (f64::from(a) - f64::from(b)) as f32;
    let position =
        std::array::from_fn(|i| subtract(input.owner_position[i], input.placement_offset[i]));
    let yaw = subtract(input.owner_yaw, input.placement_yaw);
    if position.iter().any(|v| !v.is_finite()) || !yaw.is_finite() {
        return Err("base layout difference overflows source profile");
    }
    Ok(Bootstrap {
        owner_flags,
        template: Some(TemplateRequest {
            class: input.layout_class,
            position,
            yaw,
            animation_arguments: [1, 0x20, 0, 0, 0],
            initial_advance: 0.05,
        }),
    })
}
