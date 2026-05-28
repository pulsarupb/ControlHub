use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, Default)]
pub struct Pose2d {
    pub x_m: f32,
    pub y_m: f32,
    pub heading_rad: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ChassisConfig {
    pub wheel_radius_m: f32,
    pub track_width_m: f32,
    pub motor_rotations_per_wheel_rotation: f32,
    pub left_encoder_sign: f32,
    pub right_encoder_sign: f32,
}

impl ChassisConfig {
    pub fn motor_rotations_to_meters(&self, motor_rotations: f32) -> f32 {
        let wheel_rotations = motor_rotations / self.motor_rotations_per_wheel_rotation;
        wheel_rotations * 2.0 * PI * self.wheel_radius_m
    }
}

pub struct Chassis {
    config: ChassisConfig,
    pose: Pose2d,
    last_left_motor_rotations: Option<f32>,
    last_right_motor_rotations: Option<f32>,
}

impl Chassis {
    pub fn new(config: ChassisConfig) -> Self {
        Self {
            config,
            pose: Pose2d::default(),
            last_left_motor_rotations: None,
            last_right_motor_rotations: None,
        }
    }

    pub fn pose(&self) -> Pose2d {
        self.pose
    }

    pub fn reset_pose(&mut self, pose: Pose2d) {
        self.pose = pose;
        self.last_left_motor_rotations = None;
        self.last_right_motor_rotations = None;
    }

    pub fn update_from_four_motors(
        &mut self,
        left_front_motor_rotations: f32,
        left_back_motor_rotations: f32,
        right_front_motor_rotations: f32,
        right_back_motor_rotations: f32,
    ) -> Pose2d {
        let left_motor_rotations = (left_front_motor_rotations + left_back_motor_rotations)
            * 0.5
            * self.config.left_encoder_sign;
        let right_motor_rotations = (right_front_motor_rotations + right_back_motor_rotations)
            * 0.5
            * self.config.right_encoder_sign;

        self.update_from_sides(left_motor_rotations, right_motor_rotations)
    }

    fn integrate(&mut self, left_delta_m: f32, right_delta_m: f32) -> Pose2d {
        let center_delta_m = (left_delta_m + right_delta_m) * 0.5;
        let heading_delta_rad = (right_delta_m - left_delta_m) / self.config.track_width_m;
        let mid_heading_rad = self.pose.heading_rad + heading_delta_rad * 0.5;

        self.pose.x_m += center_delta_m * mid_heading_rad.cos();
        self.pose.y_m += center_delta_m * mid_heading_rad.sin();
        self.pose.heading_rad = normalize_angle(self.pose.heading_rad + heading_delta_rad);

        self.pose
    }
}

fn normalize_angle(angle_rad: f32) -> f32 {
    (angle_rad + PI).rem_euclid(2.0 * PI) - PI
}
