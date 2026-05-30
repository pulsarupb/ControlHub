import type { JoystickPosition, Pose } from "$lib/moteus-types";

export const UI_RATE_MS = 50;
export const STATUS_RATE_MS = 200;
export const ROVER_LENGTH_M = 1.5;
export const ROVER_WIDTH_M = 0.5;
export const DEFAULT_POSE: Pose = { x_m: 0, y_m: 0, heading_rad: 0 };
export const ZERO_JOYSTICK: JoystickPosition = { x: 0, y: 0 };
