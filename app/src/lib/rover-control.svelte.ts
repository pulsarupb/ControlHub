import { ZERO_JOYSTICK } from "$lib/constants";
import { apiFetch } from "$lib/api-client";
import { clamp } from "$lib/math";
import type { ConnectionState, JoystickPosition, RoverControl, RoverStatus } from "$lib/rover-types";
import { roverConnection } from "$lib/rover-connection.svelte";
import { hardware, releaseArm } from "$lib/hardware.svelte";

type RoverState = {
  status: RoverStatus | null;
  throttle: number;
  steering: number;
  joystickActive: boolean;
  mouseJoystickActive: boolean;
  joystick: JoystickPosition;
  pendingRequest: boolean;
  connectionState: ConnectionState;
  lastError: string;
};

class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

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
    lastError: "",
  });

  const stopped = $derived(state.connectionState !== "online" || !state.status?.devices.drive.data.armed);

  async function api(path: string, options: RequestInit = {}): Promise<RoverStatus> {
    const motionRequest = path === "/api/status" || path.startsWith("/api/control");
    const response = await apiFetch(
      roverConnection.apiUrl(path),
      {
        headers: { "content-type": "application/json" },
        ...options,
      },
      motionRequest ? 1500 : 3000,
    );

    if (!response.ok) {
      const body = (await response.json().catch(() => null)) as { error?: string } | null;
      throw new ApiError(response.status, body?.error ?? `${path} failed with ${response.status}`);
    }

    if (path === "/api/status") {
      const wasOffline = state.connectionState === "offline";
      state.status = (await response.json()) as RoverStatus;
      // A recovered link should not keep showing the old network error. Motion
      // refusals (4xx) stay visible until the operator re-enables or stops.
      if (wasOffline) state.lastError = "";
    }
    state.connectionState = "online";
    roverConnection.setState("online", "Rover API online");
    return state.status!;
  }

  function markOffline(error?: unknown): void {
    zeroControls();
    state.connectionState = "offline";
    state.lastError = error instanceof Error ? error.message : "Rover API request failed";
    roverConnection.setState("offline", state.lastError);
  }

  async function refreshStatus(): Promise<void> {
    try {
      await api("/api/status");
    } catch (error) {
      markOffline(error);
    }
  }

  async function sendControlTick(): Promise<void> {
    if (state.pendingRequest || stopped) return;

    state.pendingRequest = true;
    try {
      await api("/api/control", {
        method: "POST",
        // Reverse the UI drive axes for this rover without changing backend or motor config.
        body: JSON.stringify({ throttle: -state.throttle, steering: -state.steering }),
      });
    } catch (error) {
      // A 4xx here is a motion refusal (for example "drive stopped; explicit
      // enable required"), not a dead rover. Surface it instead of going offline.
      if (error instanceof ApiError && error.status < 500) {
        state.lastError = error.message;
        void refreshStatus();
      } else {
        markOffline(error);
      }
    } finally {
      state.pendingRequest = false;
    }
  }

  function zeroControls(): void {
    state.mouseJoystickActive = false;
    state.throttle = 0;
    state.steering = 0;
    state.joystick = { ...ZERO_JOYSTICK };
    state.joystickActive = false;
  }

  async function stopRover(reason = "operator stop"): Promise<void> {
    zeroControls();
    releaseArm();
    hardware.preview = null;

    try {
      await api(`/api/stop?reason=${encodeURIComponent(reason)}`, { method: "POST" });
      state.lastError = "";
      await refreshStatus();
    } catch (error) {
      markOffline(error);
    }
  }

  async function resetRover(): Promise<void> {
    zeroControls();

    try {
      await api("/api/drive/enable", { method: "POST" });
      state.lastError = "";
      await refreshStatus();
    } catch (error) {
      if (error instanceof ApiError && error.status < 500) {
        state.lastError = error.message;
        await refreshStatus();
      } else {
        markOffline(error);
      }
    }
  }

  function startJoystick(): boolean {
    if (stopped) return false;

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
    get stopped() {
      return stopped;
    },
    get lastError() {
      return state.lastError;
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
