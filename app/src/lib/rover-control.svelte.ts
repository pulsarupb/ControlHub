import { ZERO_JOYSTICK } from "$lib/constants";
import { apiFetch } from "$lib/api-client";
import { clamp } from "$lib/math";
import { MotionChannel, MotionChannelError } from "$lib/motion-channel.svelte";
import type { ConnectionState, DriveAxis, JoystickPosition, RoverControl, RoverStatus } from "$lib/rover-types";
import { roverConnection } from "$lib/rover-connection.svelte";
import { hardware, releaseArm } from "$lib/hardware.svelte";

export type DriveTransport = "ws" | "http";

type RoverState = {
  status: RoverStatus | null;
  throttle: number;
  steering: number;
  joystickActive: boolean;
  mouseJoystickActive: boolean;
  joystick: JoystickPosition;
  // Which axes a pointer drag currently owns. Lets independent widgets drive one
  // axis each without one widget's release zeroing the other axis.
  dragAxes: Record<DriveAxis, boolean>;
  pendingRequest: boolean;
  connectionState: ConnectionState;
  lastError: string;
  driveTransport: DriveTransport;
};

const NO_DRAG: Record<DriveAxis, boolean> = { throttle: false, steering: false };
const STATUS_OFFLINE_THRESHOLD = 3;

class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

function createRoverControl(): RoverControl & {
  startChannel: (baseUrl?: string) => void;
  stopChannel: () => void;
  setDriveTransport: (mode: DriveTransport) => void;
  releaseArmOverChannel: () => void;
  takeChannelControl: () => Promise<void>;
} {
  const state = $state<RoverState>({
    status: null,
    throttle: 0,
    steering: 0,
    joystickActive: false,
    mouseJoystickActive: false,
    joystick: { ...ZERO_JOYSTICK },
    dragAxes: { ...NO_DRAG },
    pendingRequest: false,
    connectionState: "connecting",
    lastError: "",
    driveTransport: "ws",
  });

  let statusFailures = 0;

  const stopped = $derived(state.connectionState !== "online" || !state.status?.devices.drive.data.armed);

  const channel = new MotionChannel({
    onOpen: () => {
      statusFailures = 0;
      const wasOffline = state.connectionState === "offline";
      if (wasOffline) state.lastError = "";
      state.connectionState = "online";
      roverConnection.setState("online", "Rover WebSocket online");
    },
    onClose: (reason) => {
      // Never auto-enable on disconnect/reconnect; clear joystick demand and mark offline.
      markOffline(new Error(reason));
    },
    onFallback: () => {
      state.driveTransport = "http";
      void refreshStatus();
    },
    onStatus: (nextStatus) => {
      state.status = nextStatus;
      statusFailures = 0;
      const wasOffline = state.connectionState === "offline";
      if (wasOffline) state.lastError = "";
      state.connectionState = "online";
      roverConnection.setState("online", "Rover WebSocket online");
    },
    onStopped: (devices, reason) => {
      if (devices.includes("drive")) {
        zeroControls();
      }
      if (devices.includes("kinova")) {
        releaseArm();
        hardware.preview = null;
      }
      if (state.status) {
        const stopInfo = { reason, at: Date.now() / 1000 };
        if (devices.includes("drive")) {
          state.status.devices.drive.data.armed = false;
          state.status.devices.drive.stop_reason = stopInfo;
        }
        if (devices.includes("kinova")) {
          state.status.devices.kinova.data.armed = false;
          state.status.devices.kinova.stop_reason = stopInfo;
        }
      }
    },
    onError: (message, code) => {
      state.lastError = message;
      if (code === "busy" || code === "conflict" || code === "bad_request") {
        zeroControls();
      }
    },
    readDriveDemand: () => ({
      armed: !stopped,
      throttle: state.throttle,
      steering: state.steering,
    }),
    readArmState: () => ({
      active:
        typeof document !== "undefined" &&
        !document.hidden &&
        document.hasFocus() &&
        Boolean(state.status?.devices.kinova.connected),
      jog: hardware.jog,
    }),
    clearArmJog: () => {
      hardware.jog = null;
    },
  });

  function startChannel(baseUrl = roverConnection.baseUrl): void {
    if (state.driveTransport === "ws") {
      channel.start(baseUrl);
    } else {
      void refreshStatus();
    }
  }

  function stopChannel(): void {
    channel.stop();
  }

  function setDriveTransport(mode: DriveTransport): void {
    state.driveTransport = mode;
    if (mode === "ws") {
      channel.start(roverConnection.baseUrl);
    } else {
      channel.stop();
      void refreshStatus();
    }
  }

  function releaseArmOverChannel(): void {
    channel.releaseArmJog();
  }

  async function takeChannelControl(): Promise<void> {
    await channel.takeControl();
    state.lastError = "";
  }

  async function api(path: string, options: RequestInit = {}): Promise<RoverStatus> {
    const motionRequest = path === "/api/status" || path.startsWith("/api/control");
    const response = await apiFetch(
      roverConnection.apiUrl(path),
      {
        headers: { "content-type": "application/json" },
        // Cap TCP connect time on motion requests so packet loss fails fast
        // instead of blocking behind OS SYN retries.
        connectTimeout: motionRequest ? 1000 : 2500,
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
    } else {
      // Cancel/consume non-status bodies (especially /api/control) so Tauri's
      // plugin-http releases the Rust ReqwestResponse and its socket every tick.
      await response.body?.cancel().catch(() => {});
    }
    statusFailures = 0;
    state.connectionState = "online";
    roverConnection.setState("online", "Rover API online");
    return state.status!;
  }

  function markOffline(error?: unknown): void {
    zeroControls();
    state.connectionState = "offline";
    // A failed fetch is a network-level failure: say what it means instead of the browser's wording.
    state.lastError = error instanceof TypeError ? "Rover unreachable" : error instanceof Error ? error.message : "Rover API request failed";
    roverConnection.setState("offline", state.lastError);
  }

  async function refreshStatus(): Promise<void> {
    if (state.driveTransport === "ws" && channel.isOpen()) {
      return;
    }
    try {
      await api("/api/status");
    } catch (error) {
      statusFailures += 1;
      // Require 3 consecutive failed status polls before marking offline so a
      // single transient poll glitch does not zero controls and starve the 500 ms drive lease.
      if (statusFailures >= STATUS_OFFLINE_THRESHOLD) {
        markOffline(error);
      }
    }
  }

  async function sendControlTick(): Promise<void> {
    if (state.driveTransport === "ws") return;
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
    state.dragAxes = { ...NO_DRAG };
  }

  async function stopRover(reason = "operator stop"): Promise<void> {
    zeroControls();
    releaseArm();
    hardware.preview = null;

    if (state.driveTransport === "ws" && channel.isOpen()) {
      try {
        await channel.stopAll(reason);
        state.lastError = "";
      } catch (error) {
        if (error instanceof MotionChannelError) {
          state.lastError = error.message;
        } else {
          markOffline(error);
        }
      }
      return;
    }

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

    if (state.driveTransport === "ws" && channel.isOpen()) {
      try {
        await channel.enableDrive();
        state.lastError = "";
      } catch (error) {
        if (error instanceof MotionChannelError) {
          state.lastError = error.message;
        } else {
          markOffline(error);
        }
      }
      return;
    }

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

  // Start a pointer drag that owns the given axes. Returns false when driving is refused.
  function beginDrag(axes: DriveAxis[]): boolean {
    if (!startJoystick()) return false;

    for (const axis of axes) state.dragAxes[axis] = true;
    state.mouseJoystickActive = true;
    return true;
  }

  // End a drag: zero only the axes it owned. The joystick goes idle once no drag owns an axis.
  function endDrag(axes: DriveAxis[]): void {
    for (const axis of axes) {
      state.dragAxes[axis] = false;
      if (axis === "throttle") state.throttle = 0;
      else state.steering = 0;
    }

    if (!state.dragAxes.throttle && !state.dragAxes.steering) {
      state.mouseJoystickActive = false;
      state.joystickActive = false;
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
    get driveTransport() {
      return state.driveTransport;
    },
    startChannel,
    stopChannel,
    setDriveTransport,
    releaseArmOverChannel,
    takeChannelControl,
    refreshStatus,
    sendControlTick,
    stopRover,
    resetRover,
    startJoystick,
    setThrottle,
    setSteering,
    releaseJoystick,
    beginDrag,
    endDrag,
  };
}

export const rover = createRoverControl();
