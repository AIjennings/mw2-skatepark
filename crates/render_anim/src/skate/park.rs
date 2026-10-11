//! Versioned, asset-free skatepark layouts and their shared collision geometry.
pub mod generator;
pub mod mesh;
pub use generator::{ContinuousStreamer, FlowStyle, ParkGenerator};
pub use mesh::{SkateParkMesh, SkateParkVertex};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const PARK_VERSION: u32 = 1;
pub const MAX_OBSTACLES: usize = 256;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParkLayout {
    pub version: u32,
    pub map: String,
    pub obstacles: Vec<Obstacle>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obstacle {
    pub id: u64,
    pub kind: ObstacleKind,
    /// MW2 map units; base centre of the obstacle.
    pub position: [f32; 3],
    pub yaw_degrees: f32,
    /// Width, length, height in MW2 map units.
    pub size: [f32; 3],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObstacleKind {
    Platform,
    Kicker,
    Bank,
    QuarterPipe,
    Rail,
}

#[derive(Default)]
pub struct ParkGeometry {
    /// Counter-clockwise, MW2-space triangles for drawing or Skate collision.
    pub triangles: Vec<[Vec3; 3]>,
    /// Explicit grind paths in MW2 coordinates.
    pub rails: Vec<Vec<Vec3>>,
}

impl ParkGeometry {
    /// Converts MW2-space geometry into Skate 3 physics coordinate space.
    pub fn to_skate_triangles_and_rails(&self) -> (Vec<[[f32; 3]; 3]>, Vec<Vec<[f32; 3]>>) {
        let triangles = self
            .triangles
            .iter()
            .map(|t| t.map(|p| super::collision::to_skate(p).to_array()))
            .collect();
        let rails = self
            .rails
            .iter()
            .map(|rail| {
                rail.iter()
                    .map(|p| super::collision::to_skate(*p).to_array())
                    .collect()
            })
            .collect();
        (triangles, rails)
    }
}

impl ParkLayout {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != PARK_VERSION {
            return Err(format!("unsupported park version {}", self.version));
        }
        if self.map.is_empty()
            || self.map.len() > 64
            || !self
                .map
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err("invalid map name".into());
        }
        if self.obstacles.len() > MAX_OBSTACLES {
            return Err("park has too many obstacles".into());
        }
        let mut ids = HashSet::new();
        for obstacle in &self.obstacles {
            if obstacle.id == 0 || !ids.insert(obstacle.id) {
                return Err("obstacle IDs must be unique and nonzero".into());
            }
            if !obstacle
                .position
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 131_072.0)
                || !obstacle.yaw_degrees.is_finite()
            {
                return Err(format!("obstacle {} has an invalid pose", obstacle.id));
            }
            let [width, length, height] = obstacle.size;
            if ![width, length, height]
                .into_iter()
                .all(|v| v.is_finite() && (8.0..=2_048.0).contains(&v))
            {
                return Err(format!("obstacle {} has an invalid size", obstacle.id));
            }
        }
        Ok(())
    }

    pub fn geometry(&self) -> Result<ParkGeometry, String> {
        self.validate()?;
        let mut geometry = ParkGeometry::default();
        for obstacle in &self.obstacles {
            obstacle.append_geometry(&mut geometry);
        }
        Ok(geometry)
    }
}

impl Obstacle {
    fn point(&self, x: f32, y: f32, z: f32) -> Vec3 {
        let (sin, cos) = self.yaw_degrees.to_radians().sin_cos();
        Vec3::from_array(self.position) + Vec3::new(cos * x - sin * y, sin * x + cos * y, z)
    }

    fn tri(&self, out: &mut ParkGeometry, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
        let points = [
            self.point(a[0], a[1], a[2]),
            self.point(b[0], b[1], b[2]),
            self.point(c[0], c[1], c[2]),
        ];
        if (points[1] - points[0])
            .cross(points[2] - points[0])
            .length_squared()
            > 0.001
        {
            out.triangles.push(points);
        }
    }

    fn quad(&self, out: &mut ParkGeometry, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]) {
        self.tri(out, a, b, c);
        self.tri(out, a, c, d);
    }

    fn append_geometry(&self, out: &mut ParkGeometry) {
        let [w, l, h] = self.size;
        let (x0, x1, y0, y1) = (-w * 0.5, w * 0.5, -l * 0.5, l * 0.5);
        if self.kind == ObstacleKind::Rail {
            out.rails
                .push(vec![self.point(0.0, y0, h), self.point(0.0, y1, h)]);
            return;
        }
        let profile: Vec<[f32; 2]> = match self.kind {
            ObstacleKind::Platform => vec![[y0, h], [y1, h]],
            ObstacleKind::Kicker | ObstacleKind::Bank => vec![[y0, 0.0], [y1, h]],
            ObstacleKind::QuarterPipe => (0..=12)
                .map(|i| {
                    let t = i as f32 / 12.0;
                    [y0 + l * t, h * t * t]
                })
                .collect(),
            ObstacleKind::Rail => unreachable!(),
        };
        for pair in profile.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            self.quad(
                out,
                [x0, a[0], a[1]],
                [x1, a[0], a[1]],
                [x1, b[0], b[1]],
                [x0, b[0], b[1]],
            );
            if a[1] > 0.0 || b[1] > 0.0 {
                // Left and right sides, outward winding.
                self.tri(out, [x0, a[0], 0.0], [x0, b[0], b[1]], [x0, b[0], 0.0]);
                self.tri(out, [x0, a[0], 0.0], [x0, a[0], a[1]], [x0, b[0], b[1]]);
                self.tri(out, [x1, a[0], 0.0], [x1, b[0], 0.0], [x1, b[0], b[1]]);
                self.tri(out, [x1, a[0], 0.0], [x1, b[0], b[1]], [x1, a[0], a[1]]);
            }
        }
        let front = profile[0];
        if front[1] > 0.0 {
            self.quad(
                out,
                [x0, y0, 0.0],
                [x1, y0, 0.0],
                [x1, y0, front[1]],
                [x0, y0, front[1]],
            );
        }
        let back = *profile.last().unwrap();
        self.quad(
            out,
            [x1, y1, 0.0],
            [x0, y1, 0.0],
            [x0, y1, back[1]],
            [x1, y1, back[1]],
        );
        self.quad(
            out,
            [x0, y0, 0.0],
            [x0, y1, 0.0],
            [x1, y1, 0.0],
            [x1, y0, 0.0],
        );
        if self.kind == ObstacleKind::Platform
            || self.kind == ObstacleKind::Bank
            || self.kind == ObstacleKind::QuarterPipe
        {
            out.rails
                .push(vec![self.point(x0, y1, h), self.point(x1, y1, h)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(kind: ObstacleKind) -> ParkLayout {
        ParkLayout {
            version: PARK_VERSION,
            map: "mp_boneyard".into(),
            obstacles: vec![Obstacle {
                id: 1,
                kind,
                position: [128.0, -64.0, 16.0],
                yaw_degrees: 30.0,
                size: [128.0, 192.0, 48.0],
            }],
        }
    }

    #[test]
    fn rejects_nonfinite_and_duplicate_network_layout_data() {
        let mut park = layout(ObstacleKind::Kicker);
        park.obstacles[0].position[0] = f32::NAN;
        assert!(park.validate().is_err());
        park.obstacles[0].position[0] = 0.0;
        park.obstacles.push(park.obstacles[0].clone());
        assert!(park.validate().is_err());
    }

    #[test]
    fn geometry_has_upward_riding_surfaces_and_explicit_rail() {
        for kind in [
            ObstacleKind::Platform,
            ObstacleKind::Kicker,
            ObstacleKind::Bank,
            ObstacleKind::QuarterPipe,
        ] {
            let geometry = layout(kind).geometry().unwrap();
            assert!(
                geometry
                    .triangles
                    .iter()
                    .any(|tri| { (tri[1] - tri[0]).cross(tri[2] - tri[0]).z > 0.0 })
            );
            assert!(
                geometry
                    .triangles
                    .iter()
                    .flat_map(|tri| tri.iter())
                    .all(|point| point.is_finite())
            );
        }
        let rail = layout(ObstacleKind::Rail).geometry().unwrap();
        assert!(rail.triangles.is_empty());
        assert_eq!(rail.rails.len(), 1);
        assert_eq!(rail.rails[0].len(), 2);
    }

    #[test]
    fn versioned_layout_round_trips() {
        let json = serde_json::to_string(&layout(ObstacleKind::QuarterPipe)).unwrap();
        let parsed: ParkLayout = serde_json::from_str(&json).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed.obstacles[0].kind, ObstacleKind::QuarterPipe);
    }

    #[test]
    fn procedural_generator_builds_valid_flowing_park() {
        let mut generator = ParkGenerator::new();
        for style in [
            FlowStyle::StreetPlaza,
            FlowStyle::MegaRamp,
            FlowStyle::FlowBowl,
            FlowStyle::SnakeRun,
        ] {
            let layout = generator.generate_park("mp_rust", style, Vec3::ZERO);
            assert!(layout.validate().is_ok());
            let geom = layout.geometry().unwrap();
            assert!(!geom.triangles.is_empty());
            assert!(!geom.rails.is_empty());

            let (skate_triangles, skate_rails) = geom.to_skate_triangles_and_rails();
            assert_eq!(skate_triangles.len(), geom.triangles.len());
            assert_eq!(skate_rails.len(), geom.rails.len());
        }
    }

    #[test]
    fn continuous_streamer_extends_ahead_and_prunes_behind() {
        let mut streamer =
            ContinuousStreamer::new("mp_terminal", Vec3::ZERO, 0.0, FlowStyle::StreetPlaza);
        assert!(streamer.layout.validate().is_ok());
        let initial_count = streamer.layout.obstacles.len();

        // Skater moving forward along +Y at 500 units/s
        let advanced = streamer.update(Vec3::new(0.0, 600.0, 0.0), Vec3::new(0.0, 500.0, 0.0));
        assert!(advanced);
        assert!(streamer.layout.validate().is_ok());
        assert!(streamer.layout.obstacles.len() >= initial_count);
    }
}
