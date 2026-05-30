use crate::localizer::Pose2d;
use serde::Serialize;
use std::f32::consts::PI;

pub(crate) const MAX_TARGET_RADIUS_M: f32 = 1.0;

const ARRIVAL_TOLERANCE_M: f32 = 0.05;
const MAX_FOLLOWER_THROTTLE: f32 = 0.35;
const DISTANCE_GAIN: f32 = 0.9;
const HEADING_GAIN: f32 = 1.8;
const TURN_IN_PLACE_HEADING_ERROR_RAD: f32 = 35.0_f32.to_radians();

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct FollowerTarget {
    pub(crate) x_m: f32,
    pub(crate) y_m: f32,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub(crate) struct FollowerStatus {
    pub(crate) active: bool,
    pub(crate) target: Option<FollowerTarget>,
    pub(crate) distance_m: f32,
    pub(crate) heading_error_rad: f32,
    pub(crate) arrived: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FollowerCommand {
    pub(crate) throttle: f32,
    pub(crate) steering: f32,
    pub(crate) status: FollowerStatus,
}

pub(crate) fn clamp_relative_target(x_m: f32, y_m: f32) -> (f32, f32) {
    let radius = x_m.hypot(y_m);
    if radius <= MAX_TARGET_RADIUS_M || radius <= f32::EPSILON {
        return (x_m, y_m);
    }

    let scale = MAX_TARGET_RADIUS_M / radius;
    (x_m * scale, y_m * scale)
}

pub(crate) fn relative_to_world_target(pose: Pose2d, relative_x_m: f32, relative_y_m: f32) -> FollowerTarget {
    let heading_cos = pose.heading_rad.cos();
    let heading_sin = pose.heading_rad.sin();

    FollowerTarget {
        x_m: pose.x_m + relative_x_m * heading_cos - relative_y_m * heading_sin,
        y_m: pose.y_m + relative_x_m * heading_sin + relative_y_m * heading_cos,
    }
}

pub(crate) fn compute_follower_command(pose: Pose2d, target: FollowerTarget) -> FollowerCommand {
    let dx = target.x_m - pose.x_m;
    let dy = target.y_m - pose.y_m;
    let distance_m = dx.hypot(dy);
    let target_heading_rad = dy.atan2(dx);
    let heading_error_rad = normalize_angle(target_heading_rad - pose.heading_rad);
    let arrived = distance_m <= ARRIVAL_TOLERANCE_M;

    let mut status = FollowerStatus {
        active: !arrived,
        target: Some(target),
        distance_m,
        heading_error_rad,
        arrived,
    };

    if arrived {
        return FollowerCommand {
            throttle: 0.0,
            steering: 0.0,
            status,
        };
    }

    let steering = (heading_error_rad * HEADING_GAIN).clamp(-1.0, 1.0);
    let heading_scale = if heading_error_rad.abs() > TURN_IN_PLACE_HEADING_ERROR_RAD {
        0.0
    } else {
        1.0 - heading_error_rad.abs() / TURN_IN_PLACE_HEADING_ERROR_RAD
    };
    let throttle = (distance_m * DISTANCE_GAIN).clamp(0.0, MAX_FOLLOWER_THROTTLE) * heading_scale;

    status.active = true;
    FollowerCommand {
        throttle,
        steering,
        status,
    }
}

fn normalize_angle(angle_rad: f32) -> f32 {
    (angle_rad + PI).rem_euclid(2.0 * PI) - PI
}
