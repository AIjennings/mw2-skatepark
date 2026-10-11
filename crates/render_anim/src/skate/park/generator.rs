//! Procedural skatepark generation and continuous horizon streaming.
use super::*;
use bevy::prelude::*;

/// Predefined flow archetypes for procedural park synthesis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowStyle {
    StreetPlaza,
    MegaRamp,
    FlowBowl,
    SnakeRun,
    Endless,
}

pub struct ParkGenerator {
    next_id: u64,
}

impl Default for ParkGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ParkGenerator {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    pub fn with_start_id(start_id: u64) -> Self {
        Self {
            next_id: start_id.max(1),
        }
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Creates a directional offset along a given yaw angle in degrees.
    fn forward_offset(yaw_degrees: f32, distance: f32) -> Vec3 {
        let (sin, cos) = yaw_degrees.to_radians().sin_cos();
        Vec3::new(-sin * distance, cos * distance, 0.0)
    }

    /// Spawns a roll-in bank for speed generation.
    pub fn spawn_roll_in(
        &mut self,
        layout: &mut ParkLayout,
        position: Vec3,
        yaw_degrees: f32,
        height: f32,
    ) {
        let length = (height * 3.0).clamp(64.0, 512.0);
        let width = 256.0;
        // Bank facing backward so skating down travels forward
        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::Bank,
            position: position.to_array(),
            yaw_degrees: yaw_degrees + 180.0,
            size: [width, length, height],
        });
    }

    /// Spawns a kicker launch gap with a matched landing transition.
    pub fn spawn_kicker_gap(
        &mut self,
        layout: &mut ParkLayout,
        start_position: Vec3,
        yaw_degrees: f32,
        kicker_height: f32,
        gap_distance: f32,
    ) -> Vec3 {
        let kicker_len = 96.0;
        let kicker_width = 160.0;
        let kicker_pos = start_position + Self::forward_offset(yaw_degrees, kicker_len * 0.5);

        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::Kicker,
            position: kicker_pos.to_array(),
            yaw_degrees,
            size: [kicker_width, kicker_len, kicker_height],
        });

        // Landing bank positioned past the gap, sloping down
        let landing_len = 128.0;
        let landing_pos = kicker_pos
            + Self::forward_offset(
                yaw_degrees,
                kicker_len * 0.5 + gap_distance + landing_len * 0.5,
            );

        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::Bank,
            position: landing_pos.to_array(),
            yaw_degrees: yaw_degrees + 180.0,
            size: [kicker_width + 48.0, landing_len, kicker_height],
        });

        landing_pos + Self::forward_offset(yaw_degrees, landing_len * 0.5)
    }

    /// Spawns a street plaza section with an elevated platform, center down-rail, and side ledges.
    pub fn spawn_plaza_section(
        &mut self,
        layout: &mut ParkLayout,
        start_position: Vec3,
        yaw_degrees: f32,
    ) -> Vec3 {
        let plat_w = 256.0;
        let plat_l = 192.0;
        let plat_h = 36.0;
        let plat_pos = start_position + Self::forward_offset(yaw_degrees, plat_l * 0.5);

        // Main elevated platform
        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::Platform,
            position: plat_pos.to_array(),
            yaw_degrees,
            size: [plat_w, plat_l, plat_h],
        });

        // Center down rail extending off the platform end
        let rail_len = 144.0;
        let rail_pos = plat_pos + Self::forward_offset(yaw_degrees, plat_l * 0.5 + rail_len * 0.5);
        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::Rail,
            position: rail_pos.to_array(),
            yaw_degrees,
            size: [12.0, rail_len, plat_h],
        });

        // Side bank off the platform
        let bank_w = 64.0;
        let bank_l = 96.0;
        let (sin, cos) = yaw_degrees.to_radians().sin_cos();
        let right_offset = Vec3::new(cos * 96.0, sin * 96.0, 0.0);
        let bank_pos = plat_pos
            + Self::forward_offset(yaw_degrees, plat_l * 0.5 + bank_l * 0.5)
            + right_offset;

        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::Bank,
            position: bank_pos.to_array(),
            yaw_degrees: yaw_degrees + 180.0,
            size: [bank_w, bank_l, plat_h],
        });

        rail_pos + Self::forward_offset(yaw_degrees, rail_len * 0.5)
    }

    /// Spawns a quarterpipe turnaround wall that catches the skater and redirects them.
    pub fn spawn_quarterpipe_turnaround(
        &mut self,
        layout: &mut ParkLayout,
        position: Vec3,
        yaw_degrees: f32,
        height: f32,
    ) {
        let width = 384.0;
        let length = 144.0;
        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::QuarterPipe,
            position: position.to_array(),
            yaw_degrees,
            size: [width, length, height],
        });
    }

    /// Spawns a double-sided spine transfer (two back-to-back quarterpipes).
    pub fn spawn_spine(
        &mut self,
        layout: &mut ParkLayout,
        center_pos: Vec3,
        yaw_degrees: f32,
        height: f32,
    ) {
        let width = 256.0;
        let length = 96.0;
        let fwd = Self::forward_offset(yaw_degrees, length * 0.5);

        // Side A
        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::QuarterPipe,
            position: (center_pos - fwd).to_array(),
            yaw_degrees,
            size: [width, length, height],
        });

        // Side B (opposing)
        layout.obstacles.push(Obstacle {
            id: self.alloc_id(),
            kind: ObstacleKind::QuarterPipe,
            position: (center_pos + fwd).to_array(),
            yaw_degrees: yaw_degrees + 180.0,
            size: [width, length, height],
        });
    }

    /// Synthesizes a complete themed park layout.
    pub fn generate_park(&mut self, map: &str, style: FlowStyle, origin: Vec3) -> ParkLayout {
        let mut layout = ParkLayout {
            version: PARK_VERSION,
            map: map.into(),
            obstacles: Vec::new(),
        };

        match style {
            FlowStyle::StreetPlaza => {
                let mut cursor = origin;
                self.spawn_roll_in(&mut layout, cursor, 0.0, 48.0);
                cursor += Self::forward_offset(0.0, 200.0);
                cursor = self.spawn_plaza_section(&mut layout, cursor, 0.0);
                cursor += Self::forward_offset(0.0, 160.0);
                self.spawn_kicker_gap(&mut layout, cursor, 0.0, 32.0, 96.0);
                cursor += Self::forward_offset(0.0, 360.0);
                self.spawn_quarterpipe_turnaround(&mut layout, cursor, 0.0, 56.0);
            }
            FlowStyle::MegaRamp => {
                let mut cursor = origin;
                // Giant 140-unit high roll-in drop
                self.spawn_roll_in(&mut layout, cursor, 0.0, 140.0);
                cursor += Self::forward_offset(0.0, 420.0);
                // Massive 64-unit kicker over 240-unit gap
                cursor = self.spawn_kicker_gap(&mut layout, cursor, 0.0, 64.0, 240.0);
                cursor += Self::forward_offset(0.0, 300.0);
                // Towering 96-unit vert quarterpipe
                self.spawn_quarterpipe_turnaround(&mut layout, cursor, 0.0, 96.0);
            }
            FlowStyle::FlowBowl => {
                let mut cursor = origin;
                self.spawn_spine(&mut layout, cursor, 0.0, 48.0);
                cursor += Self::forward_offset(0.0, 250.0);
                self.spawn_quarterpipe_turnaround(&mut layout, cursor, 0.0, 52.0);
                cursor -= Self::forward_offset(0.0, 500.0);
                self.spawn_quarterpipe_turnaround(&mut layout, cursor, 180.0, 52.0);
            }
            FlowStyle::SnakeRun | FlowStyle::Endless => {
                let mut cursor = origin;
                let mut yaw = 0.0;
                self.spawn_roll_in(&mut layout, cursor, yaw, 52.0);
                cursor += Self::forward_offset(yaw, 220.0);
                for i in 0..6 {
                    let angle = if i % 2 == 0 { 15.0 } else { -15.0 };
                    yaw += angle;
                    cursor = self.spawn_plaza_section(&mut layout, cursor, yaw);
                    cursor += Self::forward_offset(yaw, 120.0);
                }
            }
        }

        layout
    }
}

/// Continuous horizon streamer that generates skateable flow ahead of the player
/// and prunes trailing obstacles, keeping the obstacle count well within `MAX_OBSTACLES`.
pub struct ContinuousStreamer {
    pub layout: ParkLayout,
    pub generator: ParkGenerator,
    pub cursor: Vec3,
    pub heading_degrees: f32,
    pub style: FlowStyle,
    lookahead_distance: f32,
    prune_distance: f32,
}

impl ContinuousStreamer {
    pub fn new(map: &str, origin: Vec3, initial_heading: f32, style: FlowStyle) -> Self {
        let mut generator = ParkGenerator::new();
        let mut layout = ParkLayout {
            version: PARK_VERSION,
            map: map.into(),
            obstacles: Vec::new(),
        };

        let mut cursor = origin;
        // Roll-in ramp for speed
        generator.spawn_roll_in(&mut layout, cursor, initial_heading, 48.0);
        cursor += ParkGenerator::forward_offset(initial_heading, 200.0);

        // Street plaza section (ledge, rail, banks)
        cursor = generator.spawn_plaza_section(&mut layout, cursor, initial_heading);
        cursor += ParkGenerator::forward_offset(initial_heading, 160.0);

        // Kicker launch gap
        cursor = generator.spawn_kicker_gap(&mut layout, cursor, initial_heading, 36.0, 110.0);
        cursor += ParkGenerator::forward_offset(initial_heading, 200.0);

        // Quarterpipe turnaround / vert ramp
        generator.spawn_quarterpipe_turnaround(&mut layout, cursor, initial_heading, 54.0);
        cursor += ParkGenerator::forward_offset(initial_heading, 240.0);

        Self {
            layout,
            generator,
            cursor,
            heading_degrees: initial_heading,
            style,
            lookahead_distance: 1200.0,
            prune_distance: 2400.0,
        }
    }

    /// Evaluates skater position and velocity, extending the park runway ahead
    /// and pruning trailing obstacles. Returns `true` if the layout changed.
    pub fn update(&mut self, skater_pos: Vec3, skater_vel: Vec3) -> bool {
        let mut modified = false;

        // Steering alignment: smoothly adjust heading toward skater travel direction if moving
        let speed = skater_vel.truncate().length();
        if speed > 80.0 {
            let desired_yaw = skater_vel.x.atan2(skater_vel.y).to_degrees();
            let delta_yaw = (desired_yaw - self.heading_degrees + 540.0) % 360.0 - 180.0;
            self.heading_degrees += delta_yaw.clamp(-25.0, 25.0) * 0.08;
        }

        // Distance from skater to the generation cursor
        let to_cursor = self.cursor - skater_pos;
        let forward_dist = to_cursor.dot(ParkGenerator::forward_offset(self.heading_degrees, 1.0));

        // When skater approaches the forward threshold, generate the next segment
        if forward_dist < self.lookahead_distance
            && self.layout.obstacles.len() < MAX_OBSTACLES - 12
        {
            let next_cursor = match self.style {
                FlowStyle::MegaRamp => self.generator.spawn_kicker_gap(
                    &mut self.layout,
                    self.cursor,
                    self.heading_degrees,
                    56.0,
                    180.0,
                ),
                FlowStyle::FlowBowl => {
                    self.generator.spawn_spine(
                        &mut self.layout,
                        self.cursor,
                        self.heading_degrees,
                        48.0,
                    );
                    self.cursor + ParkGenerator::forward_offset(self.heading_degrees, 240.0)
                }
                _ => {
                    // Alternate between plaza street sections and kicker gaps
                    if self.layout.obstacles.len() % 2 == 0 {
                        self.generator.spawn_plaza_section(
                            &mut self.layout,
                            self.cursor,
                            self.heading_degrees,
                        )
                    } else {
                        self.generator.spawn_kicker_gap(
                            &mut self.layout,
                            self.cursor,
                            self.heading_degrees,
                            36.0,
                            110.0,
                        )
                    }
                }
            };

            self.cursor = next_cursor + ParkGenerator::forward_offset(self.heading_degrees, 100.0);
            modified = true;
        }

        // Prune trailing obstacles far behind the skater
        let initial_count = self.layout.obstacles.len();
        let fwd_norm = ParkGenerator::forward_offset(self.heading_degrees, 1.0);
        let prune_dist = self.prune_distance;

        self.layout.obstacles.retain(|obs| {
            let to_obs = Vec3::from_array(obs.position) - skater_pos;
            let dot = to_obs.dot(fwd_norm);
            // Keep if in front or within prune distance behind
            dot > -prune_dist
        });

        if self.layout.obstacles.len() != initial_count {
            modified = true;
        }

        modified
    }
}
