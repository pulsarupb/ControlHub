export type ConnectionState = "connecting" | "online" | "offline";

export type Pose = {
  x_m: number;
  y_m: number;
  heading_rad: number;
};

export type PathPoint = Pick<Pose, "x_m" | "y_m">;

export type FollowerTarget = Pick<Pose, "x_m" | "y_m">;

export type FollowerStatus = {
  active: boolean;
  target?: FollowerTarget | null;
  distance_m: number;
  heading_error_rad: number;
  arrived: boolean;
};

export type MapSize = {
  width: number;
  height: number;
};

export type JoystickPosition = {
  x: number;
  y: number;
};

export type MotorTelemetry = {
  id: number;
  position: number;
  velocity: number;
  fault: number | string;
};

export type RoverStatus = {
  pose?: Pose;
  path?: PathPoint[];
  motors?: MotorTelemetry[];
  emergency_stop?: boolean;
  watchdog_stopped?: boolean;
  last_error?: string | null;
  follower?: FollowerStatus;
};

export type RoverControl = {
  readonly status: RoverStatus | null;
  readonly throttle: number;
  readonly steering: number;
  readonly joystickActive: boolean;
  readonly joystick: JoystickPosition;
  readonly connectionState: ConnectionState;
  readonly pose: Pose;
  readonly path: PathPoint[];
  readonly stopped: boolean;
  readonly follower: FollowerStatus;
  readonly followerActive: boolean;
  refreshStatus: () => Promise<void>;
  sendControlTick: () => Promise<void>;
  stopRover: () => Promise<void>;
  resetRover: () => Promise<void>;
  startFollowerTarget: (x_m: number, y_m: number) => Promise<void>;
  cancelFollowerTarget: () => Promise<void>;
  startJoystick: () => boolean;
  setThrottle: (value: number) => void;
  setSteering: (value: number) => void;
  releaseJoystick: () => void;
};
