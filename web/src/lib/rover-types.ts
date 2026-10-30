export type ConnectionState = "connecting" | "online" | "offline";

export type JoystickPosition = {
  x: number;
  y: number;
};

export type RoverStatus = {
  simulate: boolean;
  time: number;
  devices: import("$lib/hardware-types").Devices;
};

export type RoverControl = {
  readonly status: RoverStatus | null;
  readonly throttle: number;
  readonly steering: number;
  readonly joystickActive: boolean;
  mouseJoystickActive: boolean;
  readonly joystick: JoystickPosition;
  readonly connectionState: ConnectionState;
  readonly stopped: boolean;
  refreshStatus: () => Promise<void>;
  sendControlTick: () => Promise<void>;
  stopRover: () => Promise<void>;
  resetRover: () => Promise<void>;
  startJoystick: () => boolean;
  setThrottle: (value: number) => void;
  setSteering: (value: number) => void;
  releaseJoystick: () => void;
};
