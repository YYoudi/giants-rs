//! Terrain route for the explicit offscreen input fixture only. This is not
//! reconstructed NPC navigation, a mission predicate or an actor-position writer.
use giants_formats::gti::Terrain;
use giants_game::{arithmetic::Precision, terrain_queries::StaticTerrainQuery};
use std::{cmp::Reverse, collections::BinaryHeap};
pub(super) struct Route {
    points: Vec<[f32; 3]>,
    cursor: usize,
}
impl Route {
    pub fn prepare(terrain: &Terrain, start: [f32; 3], goal: [f32; 3]) -> Result<Self, String> {
        let query = StaticTerrainQuery::new(terrain, 1. / terrain.spacing, Precision::Double53)?;
        let minimum = std::array::from_fn::<_, 2, _>(|i| {
            (start[i].min(goal[i]) - 800.).max(terrain.origin[i] + terrain.spacing)
        });
        let maximum = std::array::from_fn::<_, 2, _>(|i| {
            (start[i].max(goal[i]) + 800.).min(
                terrain.origin[i]
                    + (if i == 0 {
                        terrain.width - 2
                    } else {
                        terrain.height - 2
                    }) as f32
                        * terrain.spacing,
            )
        });
        let mut step = 8.;
        let dimensions = loop {
            let d = std::array::from_fn::<_, 2, _>(|i| {
                ((maximum[i] - minimum[i]) / step).ceil().max(1.) as usize + 1
            });
            if d[0] * d[1] <= 262144 {
                break d;
            }
            step *= 2.;
        };
        let [width, height] = dimensions;
        let point = |n: usize| {
            [
                minimum[0] + (n % width) as f32 * step,
                minimum[1] + (n / width) as f32 * step,
            ]
        };
        let mut heights = vec![None; width * height];
        for (n, height) in heights.iter_mut().enumerate() {
            let sample = query.sample(point(n))?;
            if sample.in_bounds && sample.height >= 0. && sample.plane[2] >= 0.8 {
                *height = Some(sample.height);
            }
        }
        let closest = |p: [f32; 3]| -> Result<usize, String> {
            heights
                .iter()
                .enumerate()
                .filter(|(_, h)| h.is_some())
                .min_by(|(a, _), (b, _)| {
                    let distance = |n| {
                        let q = point(n);
                        (p[0] - q[0]).hypot(p[1] - q[1])
                    };
                    distance(*a).total_cmp(&distance(*b))
                })
                .map(|(i, _)| i)
                .ok_or("input route has no walkable terrain".into())
        };
        let first = closest(start)?;
        let last = closest(goal)?;
        let heuristic = |n: usize| {
            let dx = (n % width).abs_diff(last % width);
            let dy = (n / width).abs_diff(last / width);
            (100 * dx.max(dy) + 41 * dx.min(dy)) as u64
        };
        let mut distance = vec![u64::MAX; heights.len()];
        let mut parent = vec![usize::MAX; heights.len()];
        let mut queue = BinaryHeap::new();
        distance[first] = 0;
        queue.push(Reverse((heuristic(first), 0u64, first)));
        while let Some(Reverse((_, cost, n))) = queue.pop() {
            if cost != distance[n] {
                continue;
            }
            if n == last {
                break;
            }
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let x = n % width;
                    let y = n / width;
                    let Some(x) = x.checked_add_signed(dx).filter(|&x| x < width) else {
                        continue;
                    };
                    let Some(y) = y.checked_add_signed(dy).filter(|&y| y < height) else {
                        continue;
                    };
                    let next = y * width + x;
                    let Some(z) = heights[next] else { continue };
                    let rise = (z - heights[n].unwrap()).abs();
                    let length = if dx != 0 && dy != 0 {
                        step * std::f32::consts::SQRT_2
                    } else {
                        step
                    };
                    if rise > length * 0.7 {
                        continue;
                    }
                    let a = point(n);
                    let b = point(next);
                    let middle = query.sample([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5])?;
                    if !middle.in_bounds || middle.plane[2] < 0.8 {
                        continue;
                    }
                    let value =
                        cost + if dx != 0 && dy != 0 { 141 } else { 100 } + (rise * 10.) as u64;
                    if value < distance[next] {
                        distance[next] = value;
                        parent[next] = n;
                        queue.push(Reverse((value + heuristic(next), value, next)));
                    }
                }
            }
        }
        if distance[last] == u64::MAX {
            return Err("no conservative land route to declared destination".into());
        }
        let mut indices = vec![last];
        while *indices.last().unwrap() != first {
            indices.push(parent[*indices.last().unwrap()]);
        }
        indices.reverse();
        let mut points = indices
            .into_iter()
            .map(|n| {
                let p = point(n);
                [p[0], p[1], heights[n].unwrap() + 8.]
            })
            .collect::<Vec<_>>();
        points.push(goal);
        println!(
            "mission_campaign_route points={} step={step} supplied_input_only=true terrain_sampled=true positions_written=false",
            points.len()
        );
        Ok(Self { points, cursor: 0 })
    }
    pub fn next(&mut self, position: [f32; 3]) -> Option<[f32; 3]> {
        while self.cursor + 1 < self.points.len()
            && (self.points[self.cursor][0] - position[0])
                .hypot(self.points[self.cursor][1] - position[1])
                < 6.
        {
            self.cursor += 1;
        }
        self.points.get(self.cursor).copied()
    }
}
