import {
  NEUTRAL_AXES,
  type Axes,
  type AxesEvent,
  type ButtonEvent,
  type DeviceInfo,
  type EffectiveConfig,
  type EffectiveStreamConfig,
  type PollOptions,
  type Snapshot,
  type StreamName,
  type StreamsConfig,
  type SwitchInputEvent,
  type SwitchInputTransport,
} from "./types";

const BUTTON_NAMES = [
  "A", "B", "X", "Y", "L", "R", "ZL", "ZR", "MINUS", "PLUS",
  "LSTICK", "RSTICK", "UP", "DOWN", "LEFT", "RIGHT", "HOME",
];
const STREAMS: StreamName[] = ["buttons", "axes", "touch", "imu"];

const defaultStream = (mode: "push" | "off", rateMs: number): EffectiveStreamConfig => ({
  mode,
  rateMs,
  deadzone: 0.01,
  includeRaw: false,
  sensorDelay: "ui",
  bufferEvents: false,
  bufferSize: 64,
});

/** Generic browser Gamepad API transport, used by the non-Switch Android edition. */
export function webGamepadTransport(): SwitchInputTransport {
  let enabled = true;
  let config = makeConfig();
  let timer: ReturnType<typeof setInterval> | undefined;
  let lastButtons = new Map<string, boolean>();
  let lastAxes = new Map<number, Axes>();
  const listeners = new Map<StreamName, Set<(event: SwitchInputEvent) => void>>();
  const buffered: SwitchInputEvent[] = [];

  function makeConfig(): EffectiveConfig {
    return {
      enabled,
      available: typeof navigator !== "undefined" && typeof navigator.getGamepads === "function",
      rumbleAvailable: devices().some((gamepad) => Boolean(gamepad.vibrationActuator)),
      deviceName: devices()[0]?.id || null,
      streams: {
        buttons: defaultStream("push", 0),
        axes: defaultStream("push", 16),
        touch: defaultStream("off", 0),
        imu: defaultStream("off", 50),
      },
    };
  }

  function devices(): Gamepad[] {
    if (typeof navigator === "undefined" || typeof navigator.getGamepads !== "function") return [];
    return Array.from(navigator.getGamepads()).filter((gamepad): gamepad is Gamepad => gamepad !== null);
  }

  function currentConfig(): EffectiveConfig {
    const connected = devices();
    return {
      ...config,
      enabled,
      available: typeof navigator !== "undefined" && typeof navigator.getGamepads === "function",
      rumbleAvailable: connected.some((gamepad) => Boolean(gamepad.vibrationActuator)),
      deviceName: connected[0]?.id || null,
    };
  }

  function emit(stream: StreamName, event: SwitchInputEvent) {
    if (config.streams[stream].mode === "off") return;
    if (config.streams[stream].bufferEvents) {
      buffered.push(event);
      if (buffered.length > config.streams[stream].bufferSize) buffered.shift();
    }
    if (config.streams[stream].mode === "push") {
      for (const listener of listeners.get(stream) ?? []) listener(event);
    }
  }

  function sample() {
    if (!enabled) return;
    const connected = devices();
    const present = new Set<string>();
    for (const gamepad of connected) {
      for (let index = 0; index < gamepad.buttons.length; index++) {
        const name = BUTTON_NAMES[index];
        if (!name) continue;
        const key = `${gamepad.index}:${name}`;
        present.add(key);
        const pressed = gamepad.buttons[index].pressed || gamepad.buttons[index].value > 0.5;
        const previous = lastButtons.get(key);
        lastButtons.set(key, pressed);
        if (previous === pressed || (previous === undefined && !pressed)) continue;
        const event: ButtonEvent = {
          type: "button",
          timestamp: Date.now(),
          name,
          pressed,
          repeat: 0,
          scanCode: 0,
          keyCode: index,
          deviceId: gamepad.index,
          source: 0,
          pressedButtons: [...lastButtons]
            .filter(([key, isPressed]) => isPressed && Number(key.split(":")[0]) === gamepad.index)
            .map(([key]) => key.split(":")[1]),
        };
        emit("buttons", event);
      }
      const axes: Axes = {
        leftX: gamepad.axes[0] ?? 0,
        leftY: gamepad.axes[1] ?? 0,
        rightX: gamepad.axes[2] ?? 0,
        rightY: gamepad.axes[3] ?? 0,
        l2: gamepad.buttons[6]?.value ?? 0,
        r2: gamepad.buttons[7]?.value ?? 0,
        hatX: (gamepad.buttons[15]?.pressed ? 1 : 0) - (gamepad.buttons[14]?.pressed ? 1 : 0),
        hatY: (gamepad.buttons[13]?.pressed ? 1 : 0) - (gamepad.buttons[12]?.pressed ? 1 : 0),
      };
      const previous = lastAxes.get(gamepad.index) ?? NEUTRAL_AXES;
      if (Object.keys(axes).some((axis) =>
        Math.abs(axes[axis as keyof Axes] - previous[axis as keyof Axes]) > config.streams.axes.deadzone
      )) {
        lastAxes.set(gamepad.index, axes);
        const event: AxesEvent = {
          type: "axes",
          timestamp: Date.now(),
          axes,
          deviceId: gamepad.index,
          source: 0,
        };
        emit("axes", event);
      }
    }

    for (const [key, wasPressed] of lastButtons) {
      if (!wasPressed || present.has(key)) continue;
      lastButtons.set(key, false);
      const [deviceId, name] = key.split(":");
      emit("buttons", {
        type: "button",
        timestamp: Date.now(),
        name,
        pressed: false,
        repeat: 0,
        scanCode: 0,
        keyCode: 0,
        deviceId: Number(deviceId),
        source: 0,
        pressedButtons: [],
      });
    }

    const connectedIds = new Set(connected.map((gamepad) => gamepad.index));
    for (const [deviceId, previous] of lastAxes) {
      if (connectedIds.has(deviceId)) continue;
      if (Object.values(previous).some((value) => value !== 0)) {
        emit("axes", {
          type: "axes",
          timestamp: Date.now(),
          axes: { ...NEUTRAL_AXES },
          deviceId,
          source: 0,
        });
      }
      lastAxes.delete(deviceId);
    }
  }

  function ensureSampling() {
    if (!timer && listeners.size) timer = setInterval(sample, 16);
  }

  function stopSamplingIfIdle() {
    if (listeners.size || !timer) return;
    clearInterval(timer);
    timer = undefined;
  }

  function applyConfig(input: StreamsConfig = {}): EffectiveConfig {
    for (const stream of STREAMS) {
      const requested = input[stream];
      if (!requested) continue;
      const target = config.streams[stream];
      // Browser Gamepad API has no touch or IMU streams.
      if (stream === "touch" || stream === "imu") {
        target.mode = "off";
        continue;
      }
      Object.assign(target, requested);
    }
    ensureSampling();
    return currentConfig();
  }

  return {
    async subscribe(stream, callback) {
      let streamListeners = listeners.get(stream);
      if (!streamListeners) {
        streamListeners = new Set();
        listeners.set(stream, streamListeners);
      }
      streamListeners.add(callback);
      ensureSampling();
      return () => {
        streamListeners?.delete(callback);
        if (!streamListeners?.size) listeners.delete(stream);
        stopSamplingIfIdle();
      };
    },
    async configure(streams) {
      return applyConfig(streams);
    },
    async poll(options?: PollOptions): Promise<Snapshot> {
      sample();
      const selected = options?.streams ?? STREAMS;
      const snapshot: Snapshot = { timestamp: Date.now() };
      if (selected.includes("buttons") && config.streams.buttons.mode !== "off") {
        const pressed = [...lastButtons].filter(([, value]) => value).map(([key]) => key.split(":")[1]);
        snapshot.buttons = { pressed, state: Object.fromEntries(pressed.map((name) => [name, true])) };
      }
      if (selected.includes("axes") && config.streams.axes.mode !== "off") {
        snapshot.axes = lastAxes.values().next().value ?? { ...NEUTRAL_AXES };
      }
      if (options?.drain) {
        snapshot.events = buffered.splice(0).filter((event) =>
          selected.includes(event.type === "button" ? "buttons" : event.type === "axes" ? "axes" : event.type === "touch" ? "touch" : "imu")
        );
      }
      return snapshot;
    },
    async getState() {
      return currentConfig();
    },
    async setEnabled(value) {
      enabled = value;
      if (!enabled) {
        lastButtons.clear();
        lastAxes.clear();
      }
      return currentConfig();
    },
    async vibrate(durationMs, amplitude) {
      const actuator = devices()[0]?.vibrationActuator;
      if (!actuator) return;
      await actuator.playEffect("dual-rumble", {
        duration: durationMs,
        strongMagnitude: (amplitude ?? 255) / 255,
        weakMagnitude: (amplitude ?? 255) / 255,
      });
    },
    async listDevices(): Promise<DeviceInfo[]> {
      return devices().map((gamepad) => ({
        id: gamepad.index,
        name: gamepad.id,
        vendorId: 0,
        productId: 0,
        sources: 0,
        isGamepad: true,
        hasVibrator: Boolean(gamepad.vibrationActuator),
      }));
    },
  };
}
