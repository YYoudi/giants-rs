//! Normal firing aim over an explicit camera/view and the acquired static
//! terrain query. Camera dynamics, view admission and ripple remain upstream.
use crate::{arithmetic::Precision, first_person::Camera, terrain_queries::StaticTerrainQuery};

/// Observed normal-pistol caller range, distinct from declaration flight range.
pub const NORMAL_PISTOL_RANGE: f32 = 500.;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub point: Option<[f32; 3]>,
    /// Native sampled/refined distance, not the final point's Euclidean length.
    pub distance: f32,
    pub queries: usize,
}

pub fn direction(angles: [f32; 3]) -> Result<[f32; 3], &'static str> {
    if !angles.into_iter().all(f32::is_finite) || angles[..2].iter().any(|v| v.abs() > 1_000_000.) {
        return Err("aim camera angles outside finite table profile");
    }
    let (sy, cy) = crate::object_transform::sincos(angles[0]);
    let (sp, cp) = crate::object_transform::sincos(angles[1]);
    Ok([(sy * cp) as f32, (-cy * cp) as f32, sp as f32])
}

pub fn terrain_ray(
    query: &StaticTerrainQuery<'_>,
    origin: [f32; 3],
    direction: [f32; 3],
    range: f32,
) -> Result<Ray, &'static str> {
    if !matches!(query.precision(), Precision::Double53)
        || !origin
            .into_iter()
            .chain(direction)
            .chain([range])
            .all(f32::is_finite)
    {
        return Err("aim ray requires finite inputs and static Double53 terrain");
    }
    let step = f64::from(query.terrain().spacing) * 0.5;
    let mut distance = step as f32;
    if distance <= 0. || !distance.is_finite() {
        return Err("aim ray step outside positive finite profile");
    }
    let point = |wide: f64, spilled: f32| -> Result<[f32; 3], &'static str> {
        let value = std::array::from_fn(|axis| {
            let d = if axis == 0 { wide } else { f64::from(spilled) };
            (d * f64::from(direction[axis]) + f64::from(origin[axis])) as f32
        });
        if value.into_iter().all(f32::is_finite) {
            Ok(value)
        } else {
            Err("aim ray point overflow")
        }
    };
    let mut queries = 0;
    if step < f64::from(range) {
        loop {
            if queries >= 65536 {
                return Err("aim ray exceeds bounded query budget");
            }
            let p = point(f64::from(distance), distance)?;
            let sample = query.sample([p[0], p[1]])?;
            queries += 1;
            if sample.height >= p[2] {
                break;
            }
            let next = f64::from(distance) + step;
            let stored = next as f32;
            if stored <= distance {
                return Err("aim ray distance no longer advances");
            }
            distance = stored;
            if next >= f64::from(range) {
                break;
            }
        }
    }
    if distance >= range {
        return Ok(Ray {
            point: None,
            distance,
            queries,
        });
    }
    let mut upper = distance;
    let mut lower = (f64::from(distance) - step) as f32;
    for _ in 0..8 {
        let wide = (f64::from(upper) + f64::from(lower)) * 0.5;
        let middle = wide as f32;
        let p = point(wide, middle)?;
        let sample = query.sample([p[0], p[1]])?;
        queries += 1;
        if sample.height < p[2] {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    let p = point(f64::from(upper), upper)?;
    let plane = query.sample([p[0], p[1]])?.plane;
    queries += 1;
    let delta: [f64; 3] = std::array::from_fn(|axis| f64::from(p[axis]) - f64::from(origin[axis]));
    let denominator = (delta[2] * f64::from(plane[2]) + delta[1] * f64::from(plane[1]))
        + delta[0] * f64::from(plane[0]);
    let fraction = if denominator.abs() < f64::from(0.0001_f32) {
        1.
    } else {
        -(((f64::from(plane[0]) * f64::from(origin[0])
            + f64::from(plane[2]) * f64::from(origin[2]))
            + f64::from(plane[1]) * f64::from(origin[1]))
            + f64::from(plane[3]))
            / denominator
    };
    let hit = std::array::from_fn(|axis| (delta[axis] * fraction + f64::from(origin[axis])) as f32);
    if !hit.into_iter().all(f32::is_finite) {
        return Err("aim terrain-plane intersection overflow");
    }
    Ok(Ray {
        point: Some(hit),
        distance: upper,
        queries,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub camera: Camera,
    pub first_person: bool,
    /// Supplied native source-field branch. Its broader meaning is not inferred.
    pub force_owner_angles: bool,
    pub viewport_forward: Option<[f32; 3]>,
    pub owner_pitch: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Branch {
    Terrain,
    Viewport,
    Owner,
}
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub point: [f32; 3],
    pub branch: Branch,
    pub ray: Ray,
}
pub fn resolve(
    query: &StaticTerrainQuery<'_>,
    view: View,
    owner_yaw: f32,
    muzzle: [f32; 3],
    range: f32,
) -> Result<Target, &'static str> {
    let ray = terrain_ray(
        query,
        view.camera.position,
        direction(view.camera.angles)?,
        range,
    )?;
    let (point, branch) = if !view.force_owner_angles
        && let Some(point) = ray.point
    {
        (point, Branch::Terrain)
    } else {
        let (forward, branch) = if !view.force_owner_angles && view.first_person {
            (
                view.viewport_forward
                    .ok_or("first-person aim viewport basis absent")?,
                Branch::Viewport,
            )
        } else {
            if !view.owner_pitch.is_finite() {
                return Err("aim owner pitch nonfinite");
            }
            (
                direction([owner_yaw, view.owner_pitch.min(85.), 0.])?,
                Branch::Owner,
            )
        };
        if !muzzle.into_iter().chain(forward).all(f32::is_finite) {
            return Err("aim fallback inputs nonfinite");
        }
        (
            std::array::from_fn(|axis| {
                (f64::from(forward[axis]) * f64::from(range) + f64::from(muzzle[axis])) as f32
            }),
            branch,
        )
    };
    if !point.into_iter().all(f32::is_finite) {
        return Err("aim target overflow");
    }
    Ok(Target { point, branch, ray })
}

/// Supplies the actual target producer to the owned shot transaction.
/// The caller must still supply the camera/view snapshot at the correct phase.
pub struct Services<'a, 't, S: ?Sized> {
    pub terrain: &'a StaticTerrainQuery<'t>,
    pub source: crate::resident_states::Id,
    pub view: View,
    pub other: &'a mut S,
    pub sampled: Option<Target>,
}
impl<S: crate::mecc_shot::Services + ?Sized> crate::mecc_shot::Services for Services<'_, '_, S> {
    fn aim(
        &mut self,
        session: &mut crate::resident_states::Session,
        source: crate::resident_states::Id,
        muzzle: [f32; 3],
        range: f32,
    ) -> Result<[f32; 3], String> {
        if source != self.source {
            return Err("aim source outside owned view binding".into());
        }
        session.check_wield_owner(source)?;
        let yaw = session
            .get(source)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .angles[0];
        let target = resolve(self.terrain, self.view, yaw, muzzle, range)?;
        self.sampled = Some(target);
        Ok(target.point)
    }
    fn muzzle(
        &mut self,
        session: &crate::resident_states::Session,
        binding: crate::mecc_fire_lock::Binding<crate::resident_states::Id>,
    ) -> Result<[f32; 3], String> {
        self.other.muzzle(session, binding)
    }
    fn effect(
        &mut self,
        session: &mut crate::resident_states::Session,
        recipient: crate::resident_states::Id,
        name: &str,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::attached_effect_context::Creation, String> {
        self.other.effect(session, recipient, name, random)
    }
    fn sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        source: crate::resident_states::Id,
        cue: u32,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.other.sound(session, source, cue)
    }
}
