use crate::state::{AppState, ArmData, epoch};
use kinova_control::{
    RobotClient,
    api::base::{Pose, ProtectionZone, ShapeType},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const MAX_CARTESIAN_STEP_M: f32 = 0.02;
const MAX_CARTESIAN_ROTATION_DEG: f32 = 5.0;
const CARTESIAN_MOTION_TIMEOUT: Duration = Duration::from_secs(15);
const CARTESIAN_POSITION_TOLERANCE_M: f32 = 0.001;
const CARTESIAN_ROTATION_TOLERANCE_DEG: f32 = 0.5;
const REAR_WALL_OFFSET_M: f32 = -0.10;
const REAR_ZONE_LENGTH_M: f32 = 2.0;
const REAR_ZONE_WIDTH_M: f32 = 3.0;
const REAR_ZONE_HEIGHT_M: f32 = 3.0;
const REAR_JOG_STOP_MARGIN_M: f32 = 0.02;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Jog {
    /// Normalized base-frame XYZ, roll/pitch/yaw; scaled by configured caps.
    pub cartesian: Option<[f32; 6]>,
    pub joints: Option<Vec<f32>>,
}

impl Jog {
    pub fn validate(&self) -> Result<(), String> {
        if self.cartesian.is_some() == self.joints.is_some()
            || !self
                .cartesian
                .iter()
                .flatten()
                .chain(self.joints.iter().flatten())
                .all(|v| v.is_finite() && v.abs() <= 1.0)
            || self
                .joints
                .as_ref()
                .is_some_and(|v| v.is_empty() || v.len() > 7)
        {
            return Err("supply one finite normalized Cartesian or joint vector".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Preview {
    pub id: u64,
    pub target: [f32; 6],
    pub joints: Vec<f32>,
    pub seed: Vec<f32>,
    pub expires_at: f64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct RearBoundary {
    /// Base +X forward axis; the rear wall is parallel to the base YZ plane.
    pub forward: [f32; 2],
    /// Plane offset along `forward`; the forbidden side is below this value.
    pub wall_offset_m: f32,
    pub zone_length_m: f32,
    pub zone_width_m: f32,
    pub zone_height_m: f32,
    pub controller_zone_installed: bool,
}

impl RearBoundary {
    fn from_home_pose(pose: Pose) -> Result<Self, String> {
        let length = pose.x.hypot(pose.y);
        if !pose.x.is_finite() || !pose.y.is_finite() || length < 0.1 {
            return Err("built-in Home pose cannot define a stable forward axis".into());
        }
        Ok(Self {
            forward: [1.0, 0.0],
            wall_offset_m: REAR_WALL_OFFSET_M,
            zone_length_m: REAR_ZONE_LENGTH_M,
            zone_width_m: REAR_ZONE_WIDTH_M,
            zone_height_m: REAR_ZONE_HEIGHT_M,
            controller_zone_installed: false,
        })
    }

    fn signed_distance(&self, pose: &[f32; 6]) -> f32 {
        pose[0] * self.forward[0] + pose[1] * self.forward[1] - self.wall_offset_m
    }

    fn validate_target(&self, target: &[f32; 6]) -> Result<(), String> {
        if self.signed_distance(target) < 0.0 {
            return Err("target is behind the 10 cm rear protection plane".into());
        }
        Ok(())
    }

    fn controller_zone_matches(&self, zone: &ProtectionZone) -> bool {
        if zone.application_data
            != kinova_control::robot::calls::REAR_PROTECTION_ZONE_APPLICATION_DATA
            || !zone.is_enabled
        {
            return false;
        }
        let Some(shape) = zone.shape.as_ref() else {
            return false;
        };
        let Some(origin) = shape.origin.as_ref() else {
            return false;
        };
        let Some(orientation) = shape.orientation.as_ref() else {
            return false;
        };
        let (Some(row1), Some(row2), Some(row3)) = (
            orientation.row1.as_ref(),
            orientation.row2.as_ref(),
            orientation.row3.as_ref(),
        ) else {
            return false;
        };
        let center_distance = self.wall_offset_m - self.zone_length_m / 2.0;
        let close = |a: f32, b: f32| (a - b).abs() <= 0.001;
        shape.shape_type == ShapeType::RectangularPrism as i32
            && shape.dimensions.len() == 3
            && close(shape.dimensions[0], self.zone_length_m)
            && close(shape.dimensions[1], self.zone_width_m)
            && close(shape.dimensions[2], self.zone_height_m)
            && close(origin.x, self.forward[0] * center_distance)
            && close(origin.y, self.forward[1] * center_distance)
            && close(origin.z, 0.0)
            && close(row1.column1, self.forward[0])
            && close(row1.column2, -self.forward[1])
            && close(row1.column3, 0.0)
            && close(row2.column1, self.forward[1])
            && close(row2.column2, self.forward[0])
            && close(row2.column3, 0.0)
            && close(row3.column1, 0.0)
            && close(row3.column2, 0.0)
            && close(row3.column3, 1.0)
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Enable,
    Home,
    Rest,
    ClearFaults,
    Gripper { position: f32 },
    Solve { pose: [f32; 6] },
    Execute { preview_id: u64 },
    MoveCartesian { pose: [f32; 6] },
    InstallRearZone,
}

pub struct Request {
    pub action: Action,
    pub generation: u64,
    pub reply: tokio::sync::oneshot::Sender<Result<Value, String>>,
}

pub async fn run(state: AppState, mut requests: tokio::sync::mpsc::Receiver<Request>) {
    loop {
        let config = state.config.lock().unwrap().kinova.clone();
        state.stop_arm();
        {
            let mut s = state.status.lock().unwrap();
            s.kinova = Default::default();
            s.kinova.enabled = config.enabled;
            s.kinova.simulated = state.simulate && config.enabled;
        }
        if !config.enabled {
            while let Ok(req) = requests.try_recv() {
                let _ = req.reply.send(Err("Kinova disabled".into()));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        let cfg = kinova_control::Config {
            robot_ip: config.ip.clone(),
            robot_port: config.port,
            username: config.username.clone(),
            password: config.password.clone(),
            connect_timeout_ms: 1000,
            ..Default::default()
        };
        let mut robot = RobotClient::new(cfg);
        robot.set_rpc_timeout(Duration::from_millis(250));
        let result = session(&state, &config, &mut robot, &mut requests).await;
        state.stop_arm();
        if robot.is_connected() {
            let _ = robot.stop().await;
            let _ = robot.close_session().await;
        }
        if let Err(error) = result {
            state.status.lock().unwrap().kinova.fail(error);
        }
        while let Ok(req) = requests.try_recv() {
            let _ = req
                .reply
                .send(Err("arm session stopped; retry explicitly".into()));
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn session(
    state: &AppState,
    config: &crate::config::KinovaConfig,
    robot: &mut RobotClient,
    requests: &mut tokio::sync::mpsc::Receiver<Request>,
) -> Result<(), String> {
    if !state.simulate {
        robot.ensure_connected().await.map_err(|e| e.to_string())?;
    }
    let mut data = ArmData::default();
    if !state.simulate {
        if let Err(error) = robot.stop().await {
            data.telemetry_error = Some(format!("initial stop was not acknowledged: {error}"));
        }
        match robot.home_pose().await {
            Ok(home_pose) => match RearBoundary::from_home_pose(home_pose) {
                Ok(mut boundary) => {
                    match robot.read_protection_zones().await {
                        Ok(zones) => {
                            boundary.controller_zone_installed = zones
                                .iter()
                                .any(|zone| boundary.controller_zone_matches(zone));
                        }
                        Err(error) => data.rear_boundary_error = Some(error.to_string()),
                    }
                    data.rear_boundary = Some(boundary);
                }
                Err(error) => data.rear_boundary_error = Some(error),
            },
            Err(error) => data.rear_boundary_error = Some(error.to_string()),
        }
    }
    if state.simulate {
        data.joints = vec![0.0; 6];
        data.pose = Some([0.0; 6]);
    }
    let mut stopped_generation = state.arm_stop.load(Ordering::SeqCst);
    let mut preview_counter = (epoch() * 1000.0) as u64;
    let mut jogging = false;
    let mut telemetry_field = 0;
    state.status.lock().unwrap().kinova.update(data.clone());
    loop {
        if state.config.lock().unwrap().kinova != *config {
            return Ok(());
        }
        if state.arm_enabled.load(Ordering::SeqCst)
            && state.arm_seen.lock().unwrap().elapsed() > Duration::from_millis(500)
        {
            state.stop_arm();
        }
        let generation = state.arm_stop.load(Ordering::SeqCst);
        if generation != stopped_generation {
            state.arm_enabled.store(false, Ordering::SeqCst);
            *state.arm_jog.lock().unwrap() = None;
            if !state.simulate {
                let _ = robot.stop().await;
            }
            stopped_generation = generation;
            data.preview = None;
            jogging = false;
        }
        data.armed = state.arm_enabled.load(Ordering::SeqCst);
        let jog = state.arm_jog.lock().unwrap().clone();
        if let Some((jog, _)) =
            jog.filter(|(_, when)| when.elapsed() <= Duration::from_millis(300) && data.armed)
        {
            if !state.simulate {
                if let Some(v) = jog.cartesian {
                    let mut velocity = v;
                    for (i, v) in velocity.iter_mut().enumerate() {
                        *v *= if i < 3 {
                            config.linear_speed
                        } else {
                            config.angular_speed
                        };
                    }
                    if let (Some(boundary), Some(pose)) = (data.rear_boundary, data.pose.as_ref()) {
                        constrain_rearward_jog(&mut velocity, pose, boundary);
                    }
                    robot
                        .jog_cartesian(velocity)
                        .await
                        .map_err(|e| e.to_string())?;
                } else if let Some(joints) = jog.joints {
                    robot
                        .jog_joints(
                            &joints
                                .iter()
                                .map(|v| v * config.joint_speed)
                                .collect::<Vec<_>>(),
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
            jogging = true;
            data.preview = None;
        } else if jogging {
            if !state.simulate {
                let _ = robot.stop().await;
            }
            jogging = false;
            *state.arm_jog.lock().unwrap() = None;
        }
        // One bounded request per tick. Stop is out-of-band and never queued
        // behind UI actions. Requests carry a generation and cannot replay it.
        if state.arm_stop.load(Ordering::SeqCst) != stopped_generation {
            continue;
        }
        if let Ok(req) = requests.try_recv() {
            let result = if req.generation != state.arm_stop.load(Ordering::SeqCst)
                || req.reply.is_closed()
            {
                Err("request invalidated by Stop or client cancellation".into())
            } else {
                action(
                    state,
                    robot,
                    &req.action,
                    req.generation,
                    &mut data,
                    &mut preview_counter,
                )
                .await
            };
            if result.is_err() {
                state.stop_arm();
            }
            let _ = req.reply.send(result);
        }
        if state.arm_stop.load(Ordering::SeqCst) != stopped_generation {
            continue;
        }
        if !state.simulate {
            // One measured field per tick, not four RPCs ahead of a pending Stop.
            let telemetry_result = match telemetry_field {
                0 => robot
                    .get_joint_angles()
                    .await
                    .map(|value| data.joints = value),
                1 => robot.get_cartesian_pose().await.map(|p| {
                    data.pose = Some(pose_array(p));
                }),
                2 => robot
                    .get_gripper_position()
                    .await
                    .map(|value| data.gripper = value),
                _ => robot
                    .get_arm_state()
                    .await
                    .map(|value| data.arm_state = Some(value)),
            };
            match telemetry_result {
                Ok(()) => data.telemetry_error = None,
                Err(error) if !robot.is_connected() => return Err(error.to_string()),
                Err(error) => {
                    data.telemetry_error = Some(error.to_string());
                    if state.arm_enabled.load(Ordering::SeqCst) {
                        state.stop_arm();
                        let _ = robot.stop().await;
                    }
                }
            }
            if data.arm_state == Some(4) {
                state.stop_arm();
            }
            telemetry_field = (telemetry_field + 1) % 4;
        }
        data.armed = state.arm_enabled.load(Ordering::SeqCst);
        state.status.lock().unwrap().kinova.update(data.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn action(
    state: &AppState,
    robot: &mut RobotClient,
    action: &Action,
    generation: u64,
    data: &mut ArmData,
    counter: &mut u64,
) -> Result<Value, String> {
    let cfg = state.config.lock().unwrap().kinova.clone();
    match action {
        Action::Enable => {
            if !state.simulate {
                require_rear_zone(data)?;
            }
            if !state.simulate {
                robot.stop().await.map_err(|e| e.to_string())?;
                robot
                    .set_servoing_mode(kinova_control::api::base::ServoingMode::SingleLevelServoing)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            *state.arm_seen.lock().unwrap() = Instant::now();
            let _jog = state.arm_jog.lock().unwrap();
            check_generation(state, generation)?;
            state.arm_enabled.store(true, Ordering::SeqCst);
            data.preview = None;
        }
        Action::ClearFaults => {
            state.stop_arm();
            if !state.simulate {
                robot.clear_faults().await.map_err(|e| e.to_string())?;
            }
            data.preview = None;
        }
        Action::InstallRearZone => {
            if state.simulate {
                return Err("controller protection zones require a real Kinova".into());
            }
            if state.arm_enabled.load(Ordering::SeqCst) {
                return Err("stop the arm before configuring its rear protection zone".into());
            }
            check_generation(state, generation)?;
            let boundary = data
                .rear_boundary
                .as_mut()
                .ok_or_else(|| "Home-derived rear boundary is unavailable".to_string())?;
            robot.set_rpc_timeout(Duration::from_secs(3));
            let result = async {
                robot
                    .configure_rear_protection_zone(
                        boundary.forward,
                        boundary.wall_offset_m,
                        boundary.zone_length_m,
                        boundary.zone_width_m,
                        boundary.zone_height_m,
                    )
                    .await?;
                robot.read_protection_zones().await
            }
            .await;
            robot.set_rpc_timeout(Duration::from_millis(250));
            let zones = result.map_err(|error| error.to_string())?;
            boundary.controller_zone_installed = zones
                .iter()
                .any(|zone| boundary.controller_zone_matches(zone));
            if !boundary.controller_zone_installed {
                return Err("Kortex did not read back the configured rear protection zone".into());
            }
            data.rear_boundary_error = None;
            state.status.lock().unwrap().kinova.update(data.clone());
            return Ok(json!({"ok": true, "controller_zone_installed": true}));
        }
        Action::Solve { pose } => {
            data.preview = None;
            if !pose.iter().all(|v| v.is_finite()) {
                return Err("target pose must be finite".into());
            }
            if state.simulate {
                return Err(
                    "IK requires a real Kortex connection; no simulated solution is fabricated"
                        .into(),
                );
            }
            data.rear_boundary
                .as_ref()
                .ok_or_else(|| "Home-derived rear boundary is unavailable".to_string())?
                .validate_target(pose)?;
            let seed = robot.get_joint_angles().await.map_err(|e| e.to_string())?;
            let [x, y, z, theta_x, theta_y, theta_z] = *pose;
            let joints = robot
                .compute_inverse_kinematics(
                    &Pose {
                        x,
                        y,
                        z,
                        theta_x,
                        theta_y,
                        theta_z,
                    },
                    &seed,
                )
                .await
                .map_err(|e| e.to_string())?;
            check_generation(state, generation)?;
            *counter += 1;
            let preview = Preview {
                id: *counter,
                target: *pose,
                joints,
                seed,
                expires_at: epoch() + 15.0,
            };
            data.preview = Some(preview.clone());
            return Ok(json!(preview));
        }
        _ => {
            check_generation(state, generation)?;
            if !state.arm_enabled.load(Ordering::SeqCst) {
                return Err("arm stopped; explicit enable required".into());
            }
            if !matches!(action, Action::Gripper { .. })
                && state
                    .arm_jog
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|(_, when)| when.elapsed() < Duration::from_millis(300))
            {
                return Err("release jogging before starting an arm action".into());
            }
            match action {
                Action::Gripper { position } => {
                    if !position.is_finite() || !(0.0..=1.0).contains(position) {
                        return Err("gripper position must be 0..1".into());
                    }
                    if !state.simulate {
                        robot
                            .set_gripper_position(*position)
                            .await
                            .map_err(|e| e.to_string())?;
                    } else {
                        data.gripper = Some(*position);
                    }
                }
                Action::Home | Action::Rest => {
                    data.preview = None;
                    if !state.simulate {
                        let handle = match action {
                            Action::Home => robot.home_action().await,
                            Action::Rest => robot.rest_action().await,
                            _ => unreachable!(),
                        }
                        .map_err(|e| e.to_string())?;
                        check_generation(state, generation)?;
                        robot
                            .execute_action_reference(handle)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
                Action::Execute { preview_id } => {
                    let preview = data.preview.take().ok_or("solve IK first")?;
                    validate_preview(&preview, *preview_id, epoch())?;
                    let measured = robot.get_joint_angles().await.map_err(|e| e.to_string())?;
                    if !seed_matches(&preview.seed, &measured) {
                        return Err("arm moved since preview; solve again".into());
                    }
                    let duration = measured
                        .iter()
                        .zip(&preview.joints)
                        .map(|(a, b)| angle_difference(*a, *b) / cfg.joint_speed)
                        .fold(1.0f32, f32::max);
                    check_generation(state, generation)?;
                    robot
                        .move_joints(&preview.joints, Some(duration))
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Action::MoveCartesian { pose } => {
                    data.preview = None;
                    if state.simulate {
                        return Err("Cartesian movement requires a real Kortex connection".into());
                    }
                    let target = Pose {
                        x: pose[0],
                        y: pose[1],
                        z: pose[2],
                        theta_x: pose[3],
                        theta_y: pose[4],
                        theta_z: pose[5],
                    };
                    let measured = robot
                        .get_cartesian_pose()
                        .await
                        .map_err(|e| e.to_string())?;
                    validate_cartesian_step(pose, &pose_array(measured))?;
                    let boundary = data
                        .rear_boundary
                        .as_ref()
                        .ok_or_else(|| "Home-derived rear boundary is unavailable".to_string())?;
                    boundary.validate_target(pose)?;
                    check_generation(state, generation)?;
                    if !state.arm_enabled.load(Ordering::SeqCst) {
                        return Err("arm stopped before Cartesian move".into());
                    }
                    robot
                        .move_cartesian(&target, cfg.linear_speed, cfg.angular_speed)
                        .await
                        .map_err(|e| e.to_string())?;

                    let deadline = Instant::now() + CARTESIAN_MOTION_TIMEOUT;
                    loop {
                        if state.arm_stop.load(Ordering::SeqCst) != generation
                            || !state.arm_enabled.load(Ordering::SeqCst)
                        {
                            let _ = robot.stop().await;
                            return Err("Cartesian move stopped".into());
                        }
                        if state.arm_seen.lock().unwrap().elapsed() > Duration::from_millis(500) {
                            state.stop_arm();
                            let _ = robot.stop().await;
                            return Err("arm heartbeat expired during Cartesian move".into());
                        }

                        let measured = robot
                            .get_cartesian_pose()
                            .await
                            .map_err(|e| e.to_string())?;
                        data.arm_state =
                            Some(robot.get_arm_state().await.map_err(|e| e.to_string())?);
                        if data.arm_state == Some(4) {
                            state.stop_arm();
                            let _ = robot.stop().await;
                            return Err("Kinova entered fault state during Cartesian move".into());
                        }
                        data.pose = Some(pose_array(measured));
                        data.armed = state.arm_enabled.load(Ordering::SeqCst);
                        state.status.lock().unwrap().kinova.update(data.clone());
                        if cartesian_pose_reached(&pose_array(measured), pose) {
                            return Ok(json!({
                                "ok": true,
                                "method": "kortex_reach_pose",
                                "pose": pose_array(measured),
                            }));
                        }
                        if Instant::now() >= deadline {
                            state.stop_arm();
                            let _ = robot.stop().await;
                            return Err("Cartesian move timed out; robot stopped".into());
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(json!({"ok": true}))
}

fn require_rear_zone(data: &ArmData) -> Result<(), String> {
    let boundary = data
        .rear_boundary
        .as_ref()
        .ok_or_else(|| "Home-derived rear boundary is unavailable".to_string())?;
    if !boundary.controller_zone_installed {
        return Err("install the Kortex rear protection zone before enabling arm motion".into());
    }
    Ok(())
}

fn constrain_rearward_jog(velocity: &mut [f32; 6], pose: &[f32; 6], boundary: RearBoundary) {
    let distance = boundary.signed_distance(pose);
    let forward_speed = velocity[0] * boundary.forward[0] + velocity[1] * boundary.forward[1];
    if forward_speed < 0.0 && distance <= REAR_JOG_STOP_MARGIN_M {
        velocity[0] -= forward_speed * boundary.forward[0];
        velocity[1] -= forward_speed * boundary.forward[1];
    }
}

fn check_generation(state: &AppState, generation: u64) -> Result<(), String> {
    if state.arm_stop.load(Ordering::SeqCst) != generation {
        Err("request invalidated by Stop".into())
    } else {
        Ok(())
    }
}

fn validate_preview(preview: &Preview, id: u64, now: f64) -> Result<(), String> {
    if preview.id != id || preview.expires_at <= now {
        Err("IK preview expired or replaced; solve again".into())
    } else {
        Ok(())
    }
}

fn angle_difference(a: f32, b: f32) -> f32 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}

fn pose_array(pose: Pose) -> [f32; 6] {
    [
        pose.x,
        pose.y,
        pose.z,
        pose.theta_x,
        pose.theta_y,
        pose.theta_z,
    ]
}

fn validate_cartesian_step(target: &[f32; 6], measured: &[f32; 6]) -> Result<(), String> {
    if !target.iter().chain(measured).all(|value| value.is_finite()) {
        return Err("Cartesian target and measured pose must be finite".into());
    }
    let distance = (0..3)
        .map(|index| (target[index] - measured[index]).powi(2))
        .sum::<f32>()
        .sqrt();
    if distance > MAX_CARTESIAN_STEP_M {
        return Err(format!(
            "Cartesian move is limited to {:.0} mm from the measured pose",
            MAX_CARTESIAN_STEP_M * 1000.0
        ));
    }
    if (3..6)
        .any(|index| angle_difference(target[index], measured[index]) > MAX_CARTESIAN_ROTATION_DEG)
    {
        return Err(format!(
            "Cartesian orientation change is limited to {:.0} degrees per axis",
            MAX_CARTESIAN_ROTATION_DEG
        ));
    }
    Ok(())
}

fn cartesian_pose_reached(measured: &[f32; 6], target: &[f32; 6]) -> bool {
    let distance = (0..3)
        .map(|index| (target[index] - measured[index]).powi(2))
        .sum::<f32>()
        .sqrt();
    distance <= CARTESIAN_POSITION_TOLERANCE_M
        && (3..6).all(|index| {
            angle_difference(target[index], measured[index]) <= CARTESIAN_ROTATION_TOLERANCE_DEG
        })
}

fn seed_matches(a: &[f32], b: &[f32]) -> bool {
    !a.is_empty()
        && a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.is_finite() && b.is_finite() && angle_difference(*a, *b) <= 3.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_seed_checks_wrapping_drift_and_invalid_data() {
        assert!(seed_matches(&[359.0], &[1.0]));
        assert!(!seed_matches(&[0.0], &[5.0]));
        assert!(!seed_matches(&[], &[]));
        assert!(!seed_matches(&[f32::NAN], &[0.0]));
    }
    #[test]
    fn jog_requires_one_bounded_vector() {
        assert!(
            Jog {
                cartesian: Some([0.0; 6]),
                joints: None
            }
            .validate()
            .is_ok()
        );
        assert!(
            Jog {
                cartesian: Some([2.0; 6]),
                joints: None
            }
            .validate()
            .is_err()
        );
        assert!(
            Jog {
                cartesian: None,
                joints: None
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn previews_expire_and_stop_invalidates_requests() {
        let preview = Preview {
            id: 42,
            target: [0.0; 6],
            joints: vec![1.0],
            seed: vec![0.0],
            expires_at: 100.0,
        };
        assert!(validate_preview(&preview, 42, 99.0).is_ok());
        assert!(validate_preview(&preview, 42, 100.0).is_err());
        assert!(validate_preview(&preview, 41, 99.0).is_err());
        let (state, _) = AppState::new(Default::default(), String::new(), true);
        assert!(check_generation(&state, 0).is_ok());
        state.stop_arm();
        assert!(check_generation(&state, 0).is_err());
    }

    #[test]
    fn cartesian_move_is_bounded_from_measured_pose() {
        let measured = [0.0, 0.0, 0.4, 179.0, 0.0, 0.0];
        let mut target = measured;
        target[0] = 0.02;
        target[3] = -179.0;
        assert!(validate_cartesian_step(&target, &measured).is_ok());
        target[2] = 0.421;
        assert!(validate_cartesian_step(&target, &measured).is_err());
        target = measured;
        target[4] = 5.1;
        assert!(validate_cartesian_step(&target, &measured).is_err());
        target[0] = f32::NAN;
        assert!(validate_cartesian_step(&target, &measured).is_err());
    }

    #[test]
    fn cartesian_completion_uses_position_and_wrapped_orientation_tolerances() {
        let target = [0.0, 0.0, 0.4, 179.9, 0.0, 0.0];
        assert!(cartesian_pose_reached(
            &[0.0005, 0.0, 0.4, -179.9, 0.0, 0.0],
            &target
        ));
        assert!(!cartesian_pose_reached(
            &[0.002, 0.0, 0.4, 179.9, 0.0, 0.0],
            &target
        ));
    }

    #[test]
    fn rear_plane_is_base_axis_aligned_and_rejects_targets_behind_it() {
        let boundary = RearBoundary::from_home_pose(Pose {
            x: 0.2,
            y: 0.2,
            z: 0.5,
            theta_x: 0.0,
            theta_y: 0.0,
            theta_z: 0.0,
        })
        .unwrap();
        assert_eq!(boundary.forward, [1.0, 0.0]);
        assert_eq!(boundary.wall_offset_m, -0.10);

        let mut target = [0.0; 6];
        target[0] = -0.09;
        target[1] = 0.5;
        assert!(boundary.validate_target(&target).is_ok());
        target[0] = -0.10;
        assert!(boundary.validate_target(&target).is_ok());
        target[0] = -0.11;
        target[1] = -0.5;
        assert!(boundary.validate_target(&target).is_err());
    }

    #[test]
    fn only_the_matching_enabled_controller_zone_satisfies_the_boundary() {
        use kinova_control::api::base::{Point, RotationMatrix, RotationMatrixRow, ZoneShape};
        let boundary = RearBoundary::from_home_pose(Pose {
            x: 0.2,
            y: 0.2,
            z: 0.5,
            theta_x: 0.0,
            theta_y: 0.0,
            theta_z: 0.0,
        })
        .unwrap();
        let [forward_x, forward_y] = boundary.forward;
        let center = boundary.wall_offset_m - boundary.zone_length_m / 2.0;
        let zone = ProtectionZone {
            handle: None,
            name: "ControlHub rear boundary".into(),
            application_data: kinova_control::robot::calls::REAR_PROTECTION_ZONE_APPLICATION_DATA
                .into(),
            is_enabled: true,
            shape: Some(ZoneShape {
                shape_type: ShapeType::RectangularPrism as i32,
                origin: Some(Point {
                    x: forward_x * center,
                    y: forward_y * center,
                    z: 0.0,
                }),
                orientation: Some(RotationMatrix {
                    row1: Some(RotationMatrixRow {
                        column1: forward_x,
                        column2: -forward_y,
                        column3: 0.0,
                    }),
                    row2: Some(RotationMatrixRow {
                        column1: forward_y,
                        column2: forward_x,
                        column3: 0.0,
                    }),
                    row3: Some(RotationMatrixRow {
                        column1: 0.0,
                        column2: 0.0,
                        column3: 1.0,
                    }),
                }),
                dimensions: vec![
                    boundary.zone_length_m,
                    boundary.zone_width_m,
                    boundary.zone_height_m,
                ],
                envelope_thickness: 0.0,
            }),
            limitations: Vec::new(),
            envelope_limitations: Vec::new(),
        };
        assert!(boundary.controller_zone_matches(&zone));
        let mut old_zone = zone.clone();
        old_zone.shape.as_mut().unwrap().origin.as_mut().unwrap().x += 0.02;
        assert!(!boundary.controller_zone_matches(&old_zone));
        let mut disabled = zone.clone();
        disabled.is_enabled = false;
        assert!(!boundary.controller_zone_matches(&disabled));
        let mut wrong_shape = zone;
        wrong_shape.shape.as_mut().unwrap().dimensions[0] += 0.1;
        assert!(!boundary.controller_zone_matches(&wrong_shape));
    }

    #[test]
    fn rearward_cartesian_jog_is_removed_near_the_plane() {
        let boundary = RearBoundary::from_home_pose(Pose {
            x: 0.6,
            y: 0.8,
            z: 0.5,
            theta_x: 0.0,
            theta_y: 0.0,
            theta_z: 0.0,
        })
        .unwrap();
        let projection = boundary.wall_offset_m + 0.01;
        let pose = [
            projection * boundary.forward[0],
            projection * boundary.forward[1],
            0.5,
            0.0,
            0.0,
            0.0,
        ];
        let mut velocity = [
            -boundary.forward[0],
            -boundary.forward[1],
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        constrain_rearward_jog(&mut velocity, &pose, boundary);
        assert!(
            (velocity[0] * boundary.forward[0] + velocity[1] * boundary.forward[1]).abs() < 1e-6
        );
    }

    #[test]
    fn controller_zone_readback_is_required_before_real_enable() {
        let mut data = ArmData::default();
        assert!(require_rear_zone(&data).is_err());
        data.rear_boundary = Some(RearBoundary {
            forward: [1.0, 0.0],
            wall_offset_m: REAR_WALL_OFFSET_M,
            zone_length_m: REAR_ZONE_LENGTH_M,
            zone_width_m: REAR_ZONE_WIDTH_M,
            zone_height_m: REAR_ZONE_HEIGHT_M,
            controller_zone_installed: false,
        });
        assert!(require_rear_zone(&data).is_err());
        data.rear_boundary
            .as_mut()
            .unwrap()
            .controller_zone_installed = true;
        assert!(require_rear_zone(&data).is_ok());
    }
}
