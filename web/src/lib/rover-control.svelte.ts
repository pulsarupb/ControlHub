import { DEFAULT_POSE, ZERO_JOYSTICK } from "$lib/constants";
import { apiFetch } from "$lib/api-client";
import { clamp } from "$lib/math";
import type { ConnectionState, JoystickPosition, PathPoint, Pose, RoverControl, RoverStatus } from "$lib/moteus-types";
import { roverConnection } from "$lib/rover-connection.svelte";

type RoverState = {
  status: RoverStatus | null;
  throttle: number;
  steering: number;
  joystickActive: boolean;
  mouseJoystickActive: boolean;
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
    mouseJoystickActive: false,
    joystick: { ...ZERO_JOYSTICK },
    pendingRequest: false,
    connectionState: "connecting",
  });

  const pose = $derived<Pose>(state.status?.pose ?? DEFAULT_POSE);
  const path = $derived<PathPoint[]>(state.status?.path ?? []);
  const stopped = $derived(Boolean(state.status?.emergency_stop || state.status?.watchdog_stopped));

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
    if (state.pendingRequest || state.status?.emergency_stop) return;

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
    if (state.status?.emergency_stop) return false;

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
    get mouseJoystickActive() {
      return state.mouseJoystickActive;
    },
    set mouseJoystickActive(v: boolean) {
      state.mouseJoystickActive = v;
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
    refreshStatus,
    sendControlTick,
    stopRover,
    resetRover,
    startJoystick,
    setThrottle,
    setSteering,
    releaseJoystick,
  };
}

export const rover = createRoverControl();
