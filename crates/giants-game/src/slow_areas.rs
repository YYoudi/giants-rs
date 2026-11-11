//! Owned slow-time population. Birth copies its center; queries resolve the
//! owner's current context. Newest-first order and expiry precede actor callbacks.
use crate::{arithmetic::Precision, object_time::SlowArea};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Id(u64);
#[derive(Clone, Copy, Debug)]
pub struct Birth<I> {
    pub owner: I,
    pub center: [f32; 3],
    pub radius: f32,
    pub seconds: f32,
    pub factor: f32,
}
#[derive(Clone, Debug)]
pub struct Record<I> {
    pub id: Id,
    pub owner: I,
    pub center: [f32; 3],
    pub radius_squared: f32,
    pub remaining: f32,
    pub factor: f32,
}
pub struct Pool<I> {
    next: u64,
    records: Vec<Record<I>>,
}
impl<I> Default for Pool<I> {
    fn default() -> Self {
        Self {
            next: 0,
            records: vec![],
        }
    }
}
impl<I: Copy> Pool<I> {
    /// Finite birth-input profile. Negative radii are squared; factors/durations
    /// are copied without clamping. Allocation identity is local, never an address.
    pub fn insert(&mut self, birth: Birth<I>, precision: Precision) -> Result<Id, &'static str> {
        if !birth
            .center
            .into_iter()
            .chain([birth.radius, birth.seconds, birth.factor])
            .all(f32::is_finite)
        {
            return Err("slow-area birth outside finite input profile");
        }
        let serial = self
            .next
            .checked_add(1)
            .ok_or("slow-area identity exhausted")?;
        let id = Id(serial);
        self.records.insert(
            0,
            Record {
                id,
                owner: birth.owner,
                center: birth.center,
                radius_squared: precision.round(f64::from(birth.radius) * f64::from(birth.radius))
                    as f32,
                remaining: birth.seconds,
                factor: birth.factor,
            },
        );
        self.next = serial;
        Ok(id)
    }
    pub fn records(&self) -> &[Record<I>] {
        &self.records
    }
    /// Retire using the wide subtraction result, after storing the new timer.
    /// Removed identities are returned in actual newest-first traversal order.
    pub fn advance(&mut self, seconds: f32, precision: Precision) -> Result<Vec<Id>, &'static str> {
        if !seconds.is_finite() {
            return Err("slow-area clock outside finite input profile");
        }
        let mut retired = vec![];
        self.records.retain_mut(|r| {
            let wide = precision.round(f64::from(r.remaining) - f64::from(seconds));
            r.remaining = wide as f32;
            if wide <= 0. {
                retired.push(r.id);
                false
            } else {
                true
            }
        });
        Ok(retired)
    }
    pub fn clear(&mut self) -> Vec<Id> {
        std::mem::take(&mut self.records)
            .into_iter()
            .map(|r| r.id)
            .collect()
    }
    /// A dead/foreign owner is a required-service error, not an absent area.
    /// The stored center stays fixed even when that owner subsequently moves.
    pub fn snapshot(
        &self,
        mut context: impl FnMut(I) -> Result<u32, String>,
    ) -> Result<Vec<SlowArea>, String> {
        self.records
            .iter()
            .map(|r| {
                Ok(SlowArea {
                    source_owner: context(r.owner)?,
                    center: r.center,
                    radius_squared: r.radius_squared,
                    factor: r.factor,
                })
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn priority_owner_resolution_expiry_and_clear_share_one_population() {
        let mut p = Pool::default();
        let birth = Birth {
            owner: 7,
            center: [1., 2., 3.],
            radius: -4.,
            seconds: 0.5,
            factor: 0.25,
        };
        let old = p.insert(birth, Precision::Double53).unwrap();
        let new = p
            .insert(
                Birth {
                    owner: 8,
                    seconds: 0.25,
                    factor: 0.5,
                    ..birth
                },
                Precision::Double53,
            )
            .unwrap();
        assert_eq!(
            p.records().iter().map(|r| r.id).collect::<Vec<_>>(),
            [new, old]
        );
        let areas = p.snapshot(|id| Ok(id + 100)).unwrap();
        assert_eq!(areas[0].center, [1., 2., 3.]);
        assert_eq!(areas[0].radius_squared, 16.);
        assert_eq!(
            crate::object_time::seconds(
                1.,
                Some(crate::object_time::Sample {
                    owner: 108,
                    position: [1., 2., 0.]
                }),
                None,
                &areas
            ),
            0.25
        );
        assert!(p.snapshot(|_| Err("owner retired".into())).is_err());
        assert_eq!(p.advance(0.25, Precision::Double53).unwrap(), [new]);
        assert_eq!(p.clear(), [old]);
        assert!(p.records().is_empty());
        let next = p.insert(birth, Precision::Double53).unwrap();
        assert_ne!(next, old);
        assert_eq!(p.advance(0.5, Precision::Double53).unwrap(), [next]);
    }
}
