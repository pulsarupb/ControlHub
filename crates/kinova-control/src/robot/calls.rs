//! Typed, high-level RPC helpers for the Kortex Base service.

use super::RobotClient;
use super::router::RobotError;

// Session service function ids.
const SESSION_UID_KEEPALIVE: u32 = 0x10003;
const SESSION_UID_CLOSE: u32 = 0x10002;

// Base service function ids (mirror the generated Python client stubs).
const BASE_UID_GET_ARM_STATE: u32 = 0x200af;
const BASE_UID_GET_MEASURED_JOINT_ANGLES: u32 = 0x2007e;
const BASE_UID_GET_MEASURED_CARTESIAN_POSE: u32 = 0x20073;
const BASE_UID_GET_SERVOING_MODE: u32 = 0x20099;
const BASE_UID_GET_MEASURED_GRIPPER_MOVEMENT: u32 = 0x20089;
const BASE_UID_READ_ALL_ACTIONS: u32 = 0x2002c;
const BASE_UID_EXECUTE_ACTION_FROM_REFERENCE: u32 = 0x2002f;
const BASE_UID_EXECUTE_ACTION: u32 = 0x20030;
const BASE_UID_SEND_GRIPPER_COMMAND: u32 = 0x20088;
const BASE_UID_SET_SERVOING_MODE: u32 = 0x20098;
const BASE_UID_STOP: u32 = 0x20070;
const BASE_UID_APPLY_EMERGENCY_STOP: u32 = 0x20091;
const BASE_UID_CLEAR_FAULTS: u32 = 0x20092;
const BASE_UID_VALIDATE_WAYPOINT_LIST: u32 = 0x200eb;
const BASE_UID_EXECUTE_WAYPOINT_TRAJECTORY: u32 = 0x200e2;
const BASE_UID_COMPUTE_FORWARD_KINEMATICS: u32 = 0x200e9;
const BASE_UID_COMPUTE_INVERSE_KINEMATICS: u32 = 0x200ea;
const BASE_UID_CREATE_PROTECTION_ZONE: u32 = 0x20014;
const BASE_UID_UPDATE_PROTECTION_ZONE: u32 = 0x20015;
const BASE_UID_READ_ALL_PROTECTION_ZONES: u32 = 0x20018;
pub const REAR_PROTECTION_ZONE_APPLICATION_DATA: &str = "pulsar-controlhub-rear-plane-v1";

use crate::api::base::action::ActionParameters;
use crate::api::base::cartesian_trajectory_constraint;
use crate::api::base::waypoint::TypeOfWaypoint;
use crate::api::base::{
    Action, ActionList, ActionType, ArmStateInformation, CartesianSpeed,
    CartesianTrajectoryConstraint, CartesianWaypoint, ConstrainedJointAngles, ConstrainedPose,
    Finger, Gripper, GripperCommand, GripperMode, GripperRequest, IkData, JointAngle, JointAngles,
    JointTrajectoryConstraint, JointTrajectoryConstraintType, Point, Pose, ProtectionZone,
    ProtectionZoneHandle, ProtectionZoneList, RequestedActionType, RotationMatrix,
    RotationMatrixRow, ServoingMode, ServoingModeInformation, ShapeType, Waypoint, WaypointList,
    WaypointValidationReport, ZoneShape,
};
use crate::api::common::CartesianReferenceFrame;
use crate::api::common::Empty;

impl RobotClient {
    /// Base-frame velocity jogging. The vendor's duration field is documented
    /// as unimplemented: callers MUST enforce freshness and send Stop.
    pub async fn jog_cartesian(&mut self, velocity: [f32; 6]) -> Result<(), RobotError> {
        if !velocity.iter().all(|v| v.is_finite()) {
            return Err(RobotError::Other("non-finite twist".into()));
        }
        use crate::api::base::{Twist, TwistCommand};
        let [
            linear_x,
            linear_y,
            linear_z,
            angular_x,
            angular_y,
            angular_z,
        ] = velocity;
        self.call_response::<Empty>(
            0x20079,
            Some(TwistCommand {
                reference_frame: CartesianReferenceFrame::Base as i32,
                twist: Some(Twist {
                    linear_x,
                    linear_y,
                    linear_z,
                    angular_x,
                    angular_y,
                    angular_z,
                }),
                duration: 0,
            }),
        )
        .await?;
        Ok(())
    }

    pub async fn jog_joints(&mut self, speeds: &[f32]) -> Result<(), RobotError> {
        use crate::api::base::{JointSpeed, JointSpeeds};
        if speeds.len() != self.actuator_count() as usize || !speeds.iter().all(|v| v.is_finite()) {
            return Err(RobotError::Other("invalid joint speed vector".into()));
        }
        self.call_response::<Empty>(
            0x20084,
            Some(JointSpeeds {
                joint_speeds: speeds
                    .iter()
                    .enumerate()
                    .map(|(i, value)| JointSpeed {
                        joint_identifier: i as u32,
                        value: *value,
                        duration: 0,
                    })
                    .collect(),
                duration: 0,
            }),
        )
        .await?;
        Ok(())
    }

    /// Keeps the current session alive.
    pub async fn keep_alive(&mut self) -> Result<(), RobotError> {
        self.call_empty(SESSION_UID_KEEPALIVE).await
    }

    /// Closes the current session gracefully.
    pub async fn close_session(&mut self) -> Result<(), RobotError> {
        self.call_empty(SESSION_UID_CLOSE).await
    }

    /// Current arm state (`Kinova.Api.Common.ArmState` value).
    pub async fn get_arm_state(&mut self) -> Result<i32, RobotError> {
        let resp: ArmStateInformation = self
            .call_response(BASE_UID_GET_ARM_STATE, None::<Empty>)
            .await?;
        Ok(resp.active_state)
    }

    /// Measured joint angles in degrees.
    pub async fn get_joint_angles(&mut self) -> Result<Vec<f32>, RobotError> {
        let resp: JointAngles = self
            .call_response(BASE_UID_GET_MEASURED_JOINT_ANGLES, None::<Empty>)
            .await?;
        Ok(resp.joint_angles.into_iter().map(|j| j.value).collect())
    }

    /// Measured Cartesian tool pose (meters / degrees).
    pub async fn get_cartesian_pose(&mut self) -> Result<Pose, RobotError> {
        self.call_response(BASE_UID_GET_MEASURED_CARTESIAN_POSE, None::<Empty>)
            .await
    }

    /// Current servoing mode.
    pub async fn get_servoing_mode(&mut self) -> Result<i32, RobotError> {
        let resp: ServoingModeInformation = self
            .call_response(BASE_UID_GET_SERVOING_MODE, None::<Empty>)
            .await?;
        Ok(resp.servoing_mode)
    }

    /// Measured gripper position (0..=1), if the gripper reports a finger.
    pub async fn get_gripper_position(&mut self) -> Result<Option<f32>, RobotError> {
        let req = GripperRequest {
            mode: GripperMode::GripperPosition as i32,
        };
        let resp: Gripper = self
            .call_response(BASE_UID_GET_MEASURED_GRIPPER_MOVEMENT, Some(req))
            .await?;
        Ok(resp.finger.first().map(|f| f.value))
    }

    /// Moves the arm to its built-in "Home" action.
    pub async fn move_home(&mut self) -> Result<(), RobotError> {
        let handle = self.home_action().await?;
        self.execute_action_reference(handle).await
    }

    /// Read the built-in action without motion, allowing a supervisor to check
    /// cancellation again before issuing ExecuteActionFromReference.
    pub async fn home_action(&mut self) -> Result<crate::api::base::ActionHandle, RobotError> {
        self.read_home_action()
            .await?
            .handle
            .ok_or_else(|| RobotError::ActionNotFound("Home".to_string()))
    }

    /// Returns the built-in Home pose without commanding motion.
    pub async fn home_pose(&mut self) -> Result<Pose, RobotError> {
        let action = self.read_home_action().await?;
        let joints = match action.action_parameters {
            Some(ActionParameters::ReachJointAngles(constrained)) => constrained
                .joint_angles
                .ok_or_else(|| RobotError::Other("Home action has no joint angles".into()))?,
            _ => {
                return Err(RobotError::Other(
                    "Home action is not a joint-space action".into(),
                ));
            }
        };
        self.call_response(BASE_UID_COMPUTE_FORWARD_KINEMATICS, Some(joints))
            .await
    }

    async fn read_home_action(&mut self) -> Result<Action, RobotError> {
        let req = RequestedActionType {
            action_type: ActionType::ReachJointAngles as i32,
        };
        let list: ActionList = self
            .call_response(BASE_UID_READ_ALL_ACTIONS, Some(req))
            .await?;

        list.action_list
            .into_iter()
            .find(|a| a.name == "Home")
            .ok_or_else(|| RobotError::ActionNotFound("Home".to_string()))
    }

    pub async fn read_protection_zones(&mut self) -> Result<Vec<ProtectionZone>, RobotError> {
        let zones: ProtectionZoneList = self
            .call_response(BASE_UID_READ_ALL_PROTECTION_ZONES, None::<Empty>)
            .await?;
        Ok(zones.protection_zones)
    }

    /// Installs or updates only ControlHub's tagged rear boundary. Kinova
    /// requires the arm to be powered off for these protection-zone writes.
    pub async fn configure_rear_protection_zone(
        &mut self,
        forward: [f32; 2],
        wall_offset_m: f32,
        length_m: f32,
        width_m: f32,
        height_m: f32,
    ) -> Result<(), RobotError> {
        let existing = self
            .read_protection_zones()
            .await?
            .into_iter()
            .find(|zone| zone.application_data == REAR_PROTECTION_ZONE_APPLICATION_DATA);
        let zone = make_rear_protection_zone(
            forward,
            wall_offset_m,
            length_m,
            width_m,
            height_m,
            existing.as_ref().and_then(|zone| zone.handle),
        )?;

        if existing.is_some() {
            self.call_response::<Empty>(BASE_UID_UPDATE_PROTECTION_ZONE, Some(zone))
                .await?;
        } else {
            self.call_response::<ProtectionZoneHandle>(BASE_UID_CREATE_PROTECTION_ZONE, Some(zone))
                .await?;
        }
        Ok(())
    }

    pub async fn execute_action_reference(
        &mut self,
        handle: crate::api::base::ActionHandle,
    ) -> Result<(), RobotError> {
        self.call_response::<Empty>(BASE_UID_EXECUTE_ACTION_FROM_REFERENCE, Some(handle))
            .await?;
        Ok(())
    }

    /// Executes a joint-space action to the given angles (degrees).
    ///
    /// `duration_s` optionally constrains the move to a duration; when omitted
    /// the robot moves at its default joint speed.
    pub async fn move_joints(
        &mut self,
        targets: &[f32],
        duration_s: Option<f32>,
    ) -> Result<(), RobotError> {
        let joint_angles = targets
            .iter()
            .enumerate()
            .map(|(id, value)| JointAngle {
                joint_identifier: id as u32,
                value: *value,
            })
            .collect();

        let constraint = duration_s.map(|duration| JointTrajectoryConstraint {
            r#type: JointTrajectoryConstraintType::JointConstraintDuration as i32,
            value: duration,
        });

        let constrained = ConstrainedJointAngles {
            joint_angles: Some(JointAngles { joint_angles }),
            constraint,
        };

        let action = Action {
            handle: None,
            name: "Joint move".to_string(),
            application_data: "kinova-control".to_string(),
            action_parameters: Some(ActionParameters::ReachJointAngles(constrained)),
        };

        self.call_response::<Empty>(BASE_UID_EXECUTE_ACTION, Some(action))
            .await?;
        Ok(())
    }

    /// Executes a Cartesian action to the given pose (meters / degrees) at the given speed.
    pub async fn move_cartesian(
        &mut self,
        pose: &Pose,
        translation_speed: f32,
        orientation_speed: f32,
    ) -> Result<(), RobotError> {
        let constrained = ConstrainedPose {
            target_pose: Some(*pose),
            constraint: Some(CartesianTrajectoryConstraint {
                r#type: Some(cartesian_trajectory_constraint::Type::Speed(
                    CartesianSpeed {
                        translation: translation_speed,
                        orientation: orientation_speed,
                    },
                )),
            }),
        };

        let action = Action {
            handle: None,
            name: "Cartesian move".to_string(),
            application_data: "kinova-control".to_string(),
            action_parameters: Some(ActionParameters::ReachPose(constrained)),
        };

        self.call_response::<Empty>(BASE_UID_EXECUTE_ACTION, Some(action))
            .await?;
        Ok(())
    }

    /// Asks the robot for a full-pose IK solution without commanding motion.
    pub async fn compute_inverse_kinematics(
        &mut self,
        pose: &Pose,
        guess: &[f32],
    ) -> Result<Vec<f32>, RobotError> {
        let seed = JointAngles {
            joint_angles: guess
                .iter()
                .enumerate()
                .map(|(id, value)| JointAngle {
                    joint_identifier: id as u32,
                    value: *value,
                })
                .collect(),
        };
        let result: JointAngles = self
            .call_response(
                BASE_UID_COMPUTE_INVERSE_KINEMATICS,
                Some(IkData {
                    cartesian_pose: Some(*pose),
                    guess: Some(seed),
                }),
            )
            .await?;
        ordered_ik_angles(result, guess.len())
    }

    /// Executes a smooth Cartesian trajectory through the given waypoints in a
    /// single call. The robot interpolates between all waypoints; set `blend`
    /// to round the corners (optimal blending).
    pub async fn move_cartesian_trajectory(
        &mut self,
        poses: &[Pose],
        max_linear_velocity: f32,
        max_angular_velocity: f32,
        blend: bool,
    ) -> Result<(), RobotError> {
        if poses.is_empty() {
            return Err(RobotError::Other("waypoint list is empty".into()));
        }

        let waypoints = poses
            .iter()
            .enumerate()
            .map(|(i, pose)| Waypoint {
                name: format!("waypoint_{i}"),
                type_of_waypoint: Some(TypeOfWaypoint::CartesianWaypoint(CartesianWaypoint {
                    pose: Some(*pose),
                    reference_frame: CartesianReferenceFrame::Base as i32,
                    maximum_linear_velocity: max_linear_velocity,
                    maximum_angular_velocity: max_angular_velocity,
                    blending_radius: 0.0,
                })),
            })
            .collect();

        let list = WaypointList {
            waypoints,
            duration: 0.0,
            use_optimal_blending: blend,
        };

        let report: WaypointValidationReport = self
            .call_response(BASE_UID_VALIDATE_WAYPOINT_LIST, Some(list.clone()))
            .await?;
        let errors = report
            .trajectory_error_report
            .as_ref()
            .map(|r| r.trajectory_error_elements.as_slice())
            .unwrap_or_default();
        if !errors.is_empty() {
            let detail = errors
                .iter()
                .map(|e| {
                    format!(
                        "waypoint {}: {} (value {}, allowed {}-{})",
                        e.waypoint_index, e.message, e.error_value, e.min_value, e.max_value
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            return Err(RobotError::Other(format!(
                "invalid waypoint trajectory: {detail}"
            )));
        }

        self.call_response::<Empty>(BASE_UID_EXECUTE_WAYPOINT_TRAJECTORY, Some(list))
            .await?;
        Ok(())
    }

    /// Commands the gripper to a position in `0..=1` (0 = open, 1 = closed).
    pub async fn set_gripper_position(&mut self, position: f32) -> Result<(), RobotError> {
        let command = GripperCommand {
            mode: GripperMode::GripperPosition as i32,
            gripper: Some(Gripper {
                finger: vec![Finger {
                    finger_identifier: 1,
                    value: position.clamp(0.0, 1.0),
                }],
            }),
            duration: 0,
        };
        self.call_response::<Empty>(BASE_UID_SEND_GRIPPER_COMMAND, Some(command))
            .await?;
        Ok(())
    }

    /// Sets the servoing mode.
    pub async fn set_servoing_mode(&mut self, mode: ServoingMode) -> Result<(), RobotError> {
        let info = ServoingModeInformation {
            servoing_mode: mode as i32,
        };
        self.call_response::<Empty>(BASE_UID_SET_SERVOING_MODE, Some(info))
            .await?;
        Ok(())
    }

    /// Stops ongoing motion without engaging the emergency stop.
    pub async fn stop(&mut self) -> Result<(), RobotError> {
        self.call_empty(BASE_UID_STOP).await
    }

    /// Engages the emergency stop; `clear_faults` is required to recover.
    pub async fn apply_emergency_stop(&mut self) -> Result<(), RobotError> {
        self.call_empty(BASE_UID_APPLY_EMERGENCY_STOP).await
    }

    /// Clears robot faults (also releases the emergency stop).
    pub async fn clear_faults(&mut self) -> Result<(), RobotError> {
        self.call_empty(BASE_UID_CLEAR_FAULTS).await
    }
}

fn make_rear_protection_zone(
    forward: [f32; 2],
    wall_offset_m: f32,
    length_m: f32,
    width_m: f32,
    height_m: f32,
    handle: Option<ProtectionZoneHandle>,
) -> Result<ProtectionZone, RobotError> {
    let norm = forward[0].hypot(forward[1]);
    if !norm.is_finite()
        || (norm - 1.0).abs() > 0.001
        || ![wall_offset_m, length_m, width_m, height_m]
            .iter()
            .all(|value| value.is_finite())
        || wall_offset_m >= 0.0
        || length_m <= 0.0
        || width_m <= 0.0
        || height_m <= 0.0
    {
        return Err(RobotError::Other(
            "invalid rear protection-zone geometry".into(),
        ));
    }

    let center_distance = wall_offset_m - length_m / 2.0;
    Ok(ProtectionZone {
        handle,
        name: "ControlHub rear boundary".into(),
        application_data: REAR_PROTECTION_ZONE_APPLICATION_DATA.into(),
        is_enabled: true,
        shape: Some(ZoneShape {
            shape_type: ShapeType::RectangularPrism as i32,
            origin: Some(Point {
                x: forward[0] * center_distance,
                y: forward[1] * center_distance,
                z: 0.0,
            }),
            orientation: Some(RotationMatrix {
                row1: Some(RotationMatrixRow {
                    column1: forward[0],
                    column2: -forward[1],
                    column3: 0.0,
                }),
                row2: Some(RotationMatrixRow {
                    column1: forward[1],
                    column2: forward[0],
                    column3: 0.0,
                }),
                row3: Some(RotationMatrixRow {
                    column1: 0.0,
                    column2: 0.0,
                    column3: 1.0,
                }),
            }),
            dimensions: vec![length_m, width_m, height_m],
            envelope_thickness: 0.0,
        }),
        limitations: Vec::new(),
        envelope_limitations: Vec::new(),
    })
}

/// Kortex can omit joint identifiers in IK results (all default to zero).
/// In that case the response order is actuator order, as in Kinova's SDK example.
fn ordered_ik_angles(result: JointAngles, expected: usize) -> Result<Vec<f32>, RobotError> {
    let mut joints = result.joint_angles;
    if expected == 0
        || joints.len() != expected
        || joints.iter().any(|joint| !joint.value.is_finite())
    {
        return Err(RobotError::Other(format!(
            "invalid IK joint response: expected {expected} finite angles, got {}",
            joints.len()
        )));
    }

    if joints.iter().all(|joint| joint.joint_identifier == 0) {
        return Ok(joints.into_iter().map(|joint| joint.value).collect());
    }

    joints.sort_by_key(|joint| joint.joint_identifier);
    let start = joints[0].joint_identifier;
    if !matches!(start, 0 | 1)
        || joints
            .iter()
            .enumerate()
            .any(|(index, joint)| joint.joint_identifier != start + index as u32)
    {
        return Err(RobotError::Other(format!(
            "invalid IK joint identifiers: {:?}",
            joints
                .iter()
                .map(|joint| joint.joint_identifier)
                .collect::<Vec<_>>()
        )));
    }
    Ok(joints.into_iter().map(|joint| joint.value).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ik_response_with_unset_ids_keeps_actuator_order() {
        let response = JointAngles {
            joint_angles: vec![
                JointAngle {
                    joint_identifier: 0,
                    value: 12.0,
                },
                JointAngle {
                    joint_identifier: 0,
                    value: -35.0,
                },
                JointAngle {
                    joint_identifier: 0,
                    value: 72.0,
                },
            ],
        };
        assert_eq!(
            ordered_ik_angles(response, 3).unwrap(),
            vec![12.0, -35.0, 72.0]
        );
    }

    #[test]
    fn ik_response_with_unique_ids_is_sorted() {
        let response = JointAngles {
            joint_angles: vec![
                JointAngle {
                    joint_identifier: 3,
                    value: 30.0,
                },
                JointAngle {
                    joint_identifier: 1,
                    value: 10.0,
                },
                JointAngle {
                    joint_identifier: 2,
                    value: 20.0,
                },
            ],
        };
        assert_eq!(
            ordered_ik_angles(response, 3).unwrap(),
            vec![10.0, 20.0, 30.0]
        );
    }

    #[test]
    fn ik_response_rejects_missing_or_ambiguous_joints() {
        let missing = JointAngles {
            joint_angles: vec![],
        };
        assert!(ordered_ik_angles(missing, 6).is_err());

        let ambiguous = JointAngles {
            joint_angles: vec![
                JointAngle {
                    joint_identifier: 0,
                    value: 10.0,
                },
                JointAngle {
                    joint_identifier: 1,
                    value: 20.0,
                },
                JointAngle {
                    joint_identifier: 1,
                    value: 30.0,
                },
            ],
        };
        assert!(ordered_ik_angles(ambiguous, 3).is_err());
    }

    #[test]
    fn rear_protection_zone_is_a_home_aligned_box_behind_the_wall() {
        let zone = make_rear_protection_zone([0.6, 0.8], -0.05, 2.0, 3.0, 3.0, None).unwrap();
        assert!(zone.is_enabled);
        assert_eq!(zone.application_data, REAR_PROTECTION_ZONE_APPLICATION_DATA);
        let shape = zone.shape.unwrap();
        assert_eq!(shape.shape_type, ShapeType::RectangularPrism as i32);
        assert_eq!(shape.dimensions, vec![2.0, 3.0, 3.0]);
        let origin = shape.origin.unwrap();
        assert!((origin.x + 0.63).abs() < 1e-6);
        assert!((origin.y + 0.84).abs() < 1e-6);
        let rotation = shape.orientation.unwrap();
        let row1 = rotation.row1.unwrap();
        let row2 = rotation.row2.unwrap();
        assert!((row1.column1 - 0.6).abs() < 1e-6);
        assert!((row1.column2 + 0.8).abs() < 1e-6);
        assert!((row2.column1 - 0.8).abs() < 1e-6);
        assert!((row2.column2 - 0.6).abs() < 1e-6);
    }

    #[test]
    fn rear_protection_zone_rejects_invalid_axis_or_dimensions() {
        assert!(make_rear_protection_zone([1.0, 1.0], -0.05, 2.0, 3.0, 3.0, None).is_err());
        assert!(make_rear_protection_zone([1.0, 0.0], 0.05, 2.0, 3.0, 3.0, None).is_err());
        assert!(make_rear_protection_zone([1.0, 0.0], -0.05, 0.0, 3.0, 3.0, None).is_err());
    }
}
