import { DEFAULT_POSE, ZERO_JOYSTICK } from "$lib/constants";
import { clamp } from "$lib/math";
import type { ConnectionState, FollowerStatus, JoystickPosition, PathPoint, Pose, RoverControl, RoverStatus } from "$lib/moteus-types";

const IDLE_FOLLOWER: FollowerStatus = {
  active: false,
  target: null,
  distance_m: 0,
  heading_error_rad: 0,
  arrived: false,
};

type RoverState = {
  status: RoverStatus | null;
  throttle: number;
  steering: number;
  joystickActive: boolean;
  joystick: JoystickPosition;
  pendingRequest: boolean;
  connectionState: ConnectionState;
};

function createRoverControl(): RoverControl {
  const state = $state<RoverState>({
    status: null,
    throttle: 0,
    steering: 0,
    joystickActive: false,
    joystick: { ...ZERO_JOYSTICK },
    pendingRequest: false,
    connectionState: "connecting",
  });

  const pose = $derived<Pose>(state.status?.pose ?? DEFAULT_POSE);
  const path = $derived<PathPoint[]>(state.status?.path ?? []);
  const stopped = $derived(Boolean(state.status?.emergency_stop || state.status?.watchdog_stopped));
  const follower = $derived<FollowerStatus>(state.status?.follower ?? IDLE_FOLLOWER);
  const followerActive = $derived(Boolean(follower.active));

  async function api(path: string, options: RequestInit = {}): Promise<RoverStatus> {
    const response = await fetch(path, {
      headers: { "content-type": "application/json" },
      ...options,
    });

    if (!response.ok) {
      throw new Error(`${path} failed with ${response.status}`);
    }

    state.status = (await response.json()) as RoverStatus;
    state.connectionState = "online";
    return state.status;
  }

  async function refreshStatus(): Promise<void> {
    try {
      await api("/api/status");
    } catch (error) {
      state.connectionState = "offline";
    }
  }

  async function sendControlTick(): Promise<void> {
    if (state.pendingRequest || state.status?.emergency_stop || followerActive) return;

    state.pendingRequest = true;
    try {
      await api("/api/control", {
        method: "POST",
        body: JSON.stringify({ throttle: state.throttle, steering: state.steering }),
      });
    } catch (error) {
      state.connectionState = "offline";
    } finally {
      state.pendingRequest = false;
    }
  }

  function zeroControls(): void {
    state.throttle = 0;
    state.steering = 0;
    state.joystick = { ...ZERO_JOYSTICK };
    state.joystickActive = false;
  }

  async function stopRover(): Promise<void> {
    zeroControls();

    try {
      await api("/api/stop", { method: "POST" });
    } catch (error) {
      state.connectionState = "offline";
    }
  }

  async function resetRover(): Promise<void> {
    zeroControls();

    try {
      await api("/api/reset", { method: "POST" });
    } catch (error) {
      state.connectionState = "offline";
    }
  }

  function startJoystick(): boolean {
    if (state.status?.emergency_stop || followerActive) return false;

    state.joystickActive = true;
    return true;
  }

  function setJoystick(rawX: number, rawY: number): void {
    if (!state.joystickActive) return;

    const length = Math.hypot(rawX, rawY);
    const scale = length > 1 ? 1 / length : 1;
    const joystick = { x: rawX * scale, y: rawY * scale };

    state.joystick = joystick;
    state.steering = clamp(joystick.x, -1, 1);
    state.throttle = clamp(-joystick.y, -1, 1);
  }

  function releaseJoystick(): void {
    zeroControls();
  }

  async function startFollowerTarget(x_m: number, y_m: number): Promise<void> {
    zeroControls();

    try {
      await api("/api/follow-target", {
        method: "POST",
        body: JSON.stringify({ x_m, y_m }),
      });
    } catch (error) {
      state.connectionState = "offline";
    }
  }

  async function cancelFollowerTarget(): Promise<void> {
    zeroControls();

    try {
      await api("/api/follow-target/cancel", { method: "POST" });
    } catch (error) {
      state.connectionState = "offline";
    }
  }

  return {
    get status() {
      return state.status;
    },
    get throttle() {
      return state.throttle;
    },
    get steering() {
      return state.steering;
    },
    get joystickActive() {
      return state.joystickActive;
    },
    get joystick() {
      return state.joystick;
    },
    get connectionState() {
      return state.connectionState;
    },
    get pose() {
      return pose;
    },
    get path() {
      return path;
    },
    get stopped() {
      return stopped;
    },
    get follower() {
      return follower;
    },
    get followerActive() {
      return followerActive;
    },
    refreshStatus,
    sendControlTick,
    stopRover,
    resetRover,
    startFollowerTarget,
    cancelFollowerTarget,
    startJoystick,
    setJoystick,
    releaseJoystick,
  };
}

export function roverControl(): RoverControl {
  return createRoverControl();
}

export const rover = roverControl();
