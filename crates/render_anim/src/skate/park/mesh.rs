//! Converts skatepark layouts and geometry into renderable 3D meshes.
use super::*;
use bevy::prelude::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkateParkVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

#[derive(Resource, Clone, Default)]
pub struct SkateParkMesh {
    pub vertices: Vec<SkateParkVertex>,
    pub indices: Vec<u32>,
    pub revision: u64,
}

impl ParkGeometry {
    /// Builds renderable 3D vertices and indices from obstacles and grind rails.
    pub fn build_mesh(&self, revision: u64) -> SkateParkMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // 1. Convert solid obstacle surface triangles
        for tri in &self.triangles {
            let [p0, p1, p2] = *tri;
            let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();

            // Distinct visual themes for riding surfaces vs sides vs bases
            let (color, uv_scale) = if normal.z > 0.25 {
                // Riding surface: bright, clean concrete/composite skate deck
                ([0.84, 0.85, 0.88, 1.0], 0.005)
            } else if normal.z < -0.25 {
                // Base bottom
                ([0.18, 0.18, 0.20, 1.0], 0.005)
            } else {
                // Ramp sides/walls: stylish contrasting charcoal with wood-grain warmth
                ([0.30, 0.32, 0.35, 1.0], 0.008)
            };

            let base_idx = vertices.len() as u32;
            for p in [p0, p1, p2] {
                vertices.push(SkateParkVertex {
                    position: p.to_array(),
                    normal: normal.to_array(),
                    color,
                    uv: [p.x * uv_scale, p.y * uv_scale],
                });
            }
            indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
        }

        // 2. Extrude 3D grind rails and support legs
        let rail_radius = 2.2;
        let rail_color = [0.95, 0.82, 0.18, 1.0]; // Bright powder-coated rail yellow
        let leg_color = [0.22, 0.22, 0.24, 1.0];  // Dark cast iron support stanchion
        let segments_around = 8;

        for rail in &self.rails {
            if rail.len() < 2 {
                continue;
            }

            for window in rail.windows(2) {
                let (start, end) = (window[0], window[1]);
                let dir = (end - start).normalize_or_zero();
                if dir.length_squared() < 0.001 {
                    continue;
                }

                let perp = if dir.z.abs() < 0.9 {
                    dir.cross(Vec3::Z).normalize_or(Vec3::Y)
                } else {
                    dir.cross(Vec3::Y).normalize_or(Vec3::X)
                };
                let up = perp.cross(dir).normalize();

                // Tube vertices along start and end
                let tube_base = vertices.len() as u32;
                for i in 0..segments_around {
                    let angle = (i as f32 / segments_around as f32) * std::f32::consts::TAU;
                    let (sin, cos) = angle.sin_cos();
                    let radial = perp * cos + up * sin;
                    let n = radial.normalize_or_zero();

                    // Start ring
                    vertices.push(SkateParkVertex {
                        position: (start + radial * rail_radius).to_array(),
                        normal: n.to_array(),
                        color: rail_color,
                        uv: [i as f32 / segments_around as f32, 0.0],
                    });
                    // End ring
                    vertices.push(SkateParkVertex {
                        position: (end + radial * rail_radius).to_array(),
                        normal: n.to_array(),
                        color: rail_color,
                        uv: [i as f32 / segments_around as f32, 1.0],
                    });
                }

                // Connect tube quads
                for i in 0..segments_around {
                    let next = (i + 1) % segments_around;
                    let v0 = tube_base + (i * 2) as u32;
                    let v1 = tube_base + (i * 2 + 1) as u32;
                    let v2 = tube_base + (next * 2 + 1) as u32;
                    let v3 = tube_base + (next * 2) as u32;
                    indices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
                }

                // Add vertical support stanchions every ~80 units down to ground
                let length = (end - start).length();
                let post_count = (length / 80.0).ceil() as usize;
                for step in 0..=post_count {
                    let t = step as f32 / post_count.max(1) as f32;
                    let top = start.lerp(end, t);
                    // Ground post reaches down to floor (assuming obstacle base height or ground)
                    let ground_z = (top.z - 36.0).max(0.0);
                    let bottom = Vec3::new(top.x, top.y, ground_z);
                    if (top.z - bottom.z) < 4.0 {
                        continue;
                    }

                    let leg_base = vertices.len() as u32;
                    let leg_r = 1.6;
                    for [dx, dy] in [[-leg_r, -leg_r], [leg_r, -leg_r], [leg_r, leg_r], [-leg_r, leg_r]] {
                        let offset = perp * dx + dir * dy;
                        vertices.push(SkateParkVertex {
                            position: (top + offset).to_array(),
                            normal: perp.to_array(),
                            color: leg_color,
                            uv: [0.0, 0.0],
                        });
                        vertices.push(SkateParkVertex {
                            position: (bottom + offset).to_array(),
                            normal: perp.to_array(),
                            color: leg_color,
                            uv: [0.0, 1.0],
                        });
                    }

                    for i in 0..4 {
                        let next = (i + 1) % 4;
                        let p0 = leg_base + (i * 2) as u32;
                        let p1 = leg_base + (i * 2 + 1) as u32;
                        let p2 = leg_base + (next * 2 + 1) as u32;
                        let p3 = leg_base + (next * 2) as u32;
                        indices.extend_from_slice(&[p0, p1, p2, p0, p2, p3]);
                    }
                }
            }
        }

        SkateParkMesh {
            vertices,
            indices,
            revision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mesh_produces_finite_geometry() {
        let park = ParkLayout {
            version: PARK_VERSION,
            map: "mp_rust".into(),
            obstacles: vec![
                Obstacle {
                    id: 1,
                    kind: ObstacleKind::Kicker,
                    position: [0.0, 100.0, 0.0],
                    yaw_degrees: 0.0,
                    size: [120.0, 120.0, 36.0],
                },
                Obstacle {
                    id: 2,
                    kind: ObstacleKind::Rail,
                    position: [50.0, 100.0, 0.0],
                    yaw_degrees: 0.0,
                    size: [12.0, 150.0, 24.0],
                },
            ],
        };

        let geom = park.geometry().unwrap();
        let mesh = geom.build_mesh(1);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);

        for v in &mesh.vertices {
            assert!(v.position.iter().all(|f| f.is_finite()));
            assert!(v.normal.iter().all(|f| f.is_finite()));
            assert!(v.color.iter().all(|f| f.is_finite()));
            assert!(v.uv.iter().all(|f| f.is_finite()));
        }

        for &idx in &mesh.indices {
            assert!((idx as usize) < mesh.vertices.len());
        }
    }
}
