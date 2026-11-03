//! CPU reconstruction of isolated unlit clip inputs; not GPU draw evidence.
use super::*;

#[derive(Default, Debug)]
struct Summary {
    planes: [usize; 6],
    candidates: usize,
    degenerate: usize,
    positive_w_vertices: usize,
}

fn summarize(clip: &[Vec4]) -> Summary {
    let mut out = Summary {
        positive_w_vertices: clip.iter().filter(|p| p.w > 0.).count(),
        ..Default::default()
    };
    for triangle in clip.as_chunks::<3>().0 {
        let planes: [bool; 6] = std::array::from_fn(|plane| {
            triangle.iter().all(|p| match plane {
                0 => p.x < -p.w,
                1 => p.x > p.w,
                2 => p.y < -p.w,
                3 => p.y > p.w,
                4 => p.z < 0.,
                _ => p.z > p.w,
            })
        });
        for (count, outside) in out.planes.iter_mut().zip(planes) {
            *count += usize::from(outside);
        }
        out.candidates += usize::from(!planes.iter().any(|outside| *outside));
        if triangle.iter().all(|p| p.w > 0.) {
            let p: Vec<_> = triangle
                .iter()
                .map(|p| p.truncate().truncate() / p.w)
                .collect();
            let a = p[1] - p[0];
            let b = p[2] - p[0];
            out.degenerate += usize::from(a.x * b.y - a.y * b.x == 0.);
        }
    }
    out
}

impl Scene {
    pub fn capture_effect_projection(
        &self,
        cinematic: &crate::cinematic_camera::Cinematic,
        camera: &crate::validation_camera::ValidationCamera,
        meshes: &bevy::asset::Assets<Mesh>,
        materials: &bevy::asset::Assets<Material>,
        offset: usize,
    ) {
        for draw in &self.draws {
            let instance = &self.instances[draw.instance];
            let Source::Effect { request, class, .. } = &instance.source else {
                continue;
            };
            if request.offset != offset {
                continue;
            }
            let DrawMaterial::Unlit(handle) = &draw.material else {
                continue;
            };
            let Some(material) = materials.get(handle) else {
                continue;
            };
            let Some(mesh) = meshes.get(&draw.mesh) else {
                continue;
            };
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };
            let Some(motion) = self.motion.iter().find(|m| m.instance == draw.instance) else {
                continue;
            };
            let clip = positions
                .iter()
                .map(|p| {
                    let p =
                        camera.clip_from_bevy() * (motion.matrix * Vec3::from_array(*p).extend(1.));
                    Vec4::new(
                        p.x + material.pixel_fog.x * p.w,
                        p.y + material.pixel_fog.y * p.w,
                        p.w - p.z,
                        p.w,
                    )
                })
                .collect::<Vec<_>>();
            let summary = summarize(&clip);
            let bounds: [[f32; 4]; 2] = std::array::from_fn(|end| {
                std::array::from_fn(|axis| {
                    clip.iter().map(|p| p[axis]).fold(
                        if end == 0 {
                            f32::INFINITY
                        } else {
                            f32::NEG_INFINITY
                        },
                        if end == 0 { f32::min } else { f32::max },
                    )
                })
            });
            println!(
                "cinematic_fx_projection sequence={} tick={} offset={} class={} instance={} part={} vertices={} positive_w={} plane_rejects={:?} candidates={} degenerate={} clip_bounds={:?} material_alpha_bits={} blend={} front={} locally_visible={} world_matrix_bits={:?} cpu_reconstruction=true gpu_draw_verified=false",
                cinematic.sequence(),
                cinematic.tick(),
                offset,
                class,
                draw.instance,
                draw.part,
                clip.len(),
                summary.positive_w_vertices,
                summary.planes,
                summary.candidates,
                summary.degenerate,
                bounds,
                material.color_alpha.w.to_bits(),
                material.blend,
                material.front,
                instance.visible(cinematic),
                motion.matrix.to_cols_array().map(f32::to_bits)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipping_rejects_one_plane_without_claiming_candidate_pixels() {
        let inside = [
            Vec4::new(-0.5, -0.5, 0.5, 1.),
            Vec4::new(0.5, -0.5, 0.5, 1.),
            Vec4::new(0., 0.5, 0.5, 1.),
        ];
        assert_eq!(summarize(&inside).candidates, 1);
        for plane in 0..6 {
            let outside = inside.map(|mut p| {
                match plane {
                    0 => p.x = -2.,
                    1 => p.x = 2.,
                    2 => p.y = -2.,
                    3 => p.y = 2.,
                    4 => p.z = -1.,
                    _ => p.z = 2.,
                };
                p
            });
            let result = summarize(&outside);
            assert_eq!(result.candidates, 0);
            assert_eq!(result.planes[plane], 1);
        }
    }
    #[test]
    fn collapsed_and_behind_eye_inputs_are_not_silently_good_geometry() {
        assert_eq!(summarize(&[Vec4::new(0., 0., 0.5, 1.); 3]).degenerate, 1);
        assert_eq!(
            summarize(&[Vec4::new(0., 0., 0.5, -1.); 3]).positive_w_vertices,
            0
        );
    }
}
