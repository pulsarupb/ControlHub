import { DEFAULT_POSE, ZERO_JOYSTICK } from "$lib/constants";
import { apiFetch } from "$lib/api-client";
import { clamp } from "$lib/math";
import type { ConnectionState, FollowerStatus, JoystickPosition, PathPoint, Pose, RoverControl, RoverStatus } from "$lib/moteus-types";
import { roverConnection } from "$lib/rover-connection.svelte";

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
    const response = await apiFetch(roverConnection.apiUrl(path), {
      headers: { "content-type": "application/json" },
      ...options,
    });

    if (!response.ok) {
      throw new Error(`${path} failed with ${response.status}`);
    }

    state.status = (await response.json()) as RoverStatus;
    state.connectionState = "online";
    roverConnection.setState("online", "Rover API online");
    return state.status;
  }

  function markOffline(): void {
    state.connectionState = "offline";
    roverConnection.setState("offline", "Rover API request failed");
  }

  async function refreshStatus(): Promise<void> {
    try {
      await api("/api/status");
    } catch (error) {
      markOffline();
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
      markOffline();
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
      markOffline();
    }
  }

  async function resetRover(): Promise<void> {
    zeroControls();

    try {
      await api("/api/reset", { method: "POST" });
    } catch (error) {
      markOffline();
    }
  }

  function startJoystick(): boolean {
    if (state.status?.emergency_stop || followerActive) return false;

    state.joystickActive = true;
    return true;
  }

  function setThrottle(value: number): void {
    state.throttle = clamp(value, -1, 1);
  }

  function setSteering(value: number): void {
    state.steering = clamp(value, -1, 1);
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
      markOffline();
    }
  }

  async function cancelFollowerTarget(): Promise<void> {
    zeroControls();

    try {
      await api("/api/follow-target/cancel", { method: "POST" });
    } catch (error) {
      markOffline();
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
    setThrottle,
    setSteering,
    releaseJoystick,
  };
}

export const rover = createRoverControl();
