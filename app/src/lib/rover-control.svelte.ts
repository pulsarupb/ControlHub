import { ZERO_JOYSTICK } from "$lib/constants";
import { clamp } from "$lib/math";
import { MotionChannel, MotionChannelError } from "$lib/motion-channel.svelte";
import type { ConnectionState, DriveAxis, JoystickPosition, RoverControl, RoverStatus } from "$lib/rover-types";
import { roverConnection } from "$lib/rover-connection.svelte";
import { hardware, releaseArm } from "$lib/hardware.svelte";

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
  connectionState: ConnectionState;
  lastError: string;
};

const NO_DRAG: Record<DriveAxis, boolean> = { throttle: false, steering: false };

function createRoverControl(): RoverControl & {
  startChannel: (baseUrl?: string) => void;
  stopChannel: () => void;
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
    connectionState: "connecting",
    lastError: "",
  });

  const stopped = $derived(state.connectionState !== "online" || !state.status?.devices.drive.data.armed);

  const channel = new MotionChannel({
    onOpen: () => {
      const wasOffline = state.connectionState === "offline";
      if (wasOffline) state.lastError = "";
      state.connectionState = "online";
      roverConnection.setState("online", "Rover WebSocket online");
    },
    onClose: (reason) => {
      // Never auto-enable on disconnect/reconnect; clear joystick demand and mark offline.
      markOffline(new Error(reason));
    },
    onStatus: (nextStatus) => {
      state.status = nextStatus;
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
    channel.start(baseUrl);
  }

  function stopChannel(): void {
    channel.stop();
  }

  function releaseArmOverChannel(): void {
    channel.releaseArmJog();
  }

  async function takeChannelControl(): Promise<void> {
    await channel.takeControl();
    state.lastError = "";
  }

  function markOffline(error?: unknown): void {
    zeroControls();
    state.connectionState = "offline";
    state.lastError = error instanceof TypeError ? "Rover unreachable" : error instanceof Error ? error.message : "Rover WebSocket offline";
    roverConnection.setState("offline", state.lastError);
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
  }

  async function resetRover(): Promise<void> {
    zeroControls();

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
    startChannel,
    stopChannel,
    releaseArmOverChannel,
    takeChannelControl,
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
