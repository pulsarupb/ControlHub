import type { DashboardComponent, Template } from "$lib/types"
import ControlsNavlet from "$lib/moteus/ControlsNavlet.svelte"
import EmergencyWidget from "$lib/moteus/EmergencyWidget.svelte"
import GamepadWidget from "./GamepadWidget.svelte"
import JoystickWidget from "$lib/moteus/JoystickWidget.svelte"
import MinimapWidget from "$lib/moteus/MinimapWidget.svelte"
import MotorTelemetryWidget from "$lib/moteus/MotorTelemetryWidget.svelte"
import StatusNavlet from "$lib/moteus/StatusNavlet.svelte"
import TargetFollowerWidget from "$lib/moteus/TargetFollowerWidget.svelte"

export const components: DashboardComponent[] = [
  { id: "emergency", name: "Emergency Controls", type: "widget", component: EmergencyWidget, minW: 3, minH: 3 },
  { id: "joystick", name: "Joystick", type: "widget", component: JoystickWidget, minW: 4, minH: 4 },
  { id: "target-follower", name: "Target Follower", type: "widget", component: TargetFollowerWidget, minW: 4, minH: 4 },
  { id: "minimap", name: "Minimap", type: "widget", component: MinimapWidget, minW: 5, minH: 4 },
  { id: "motors", name: "Motor Telemetry", type: "widget", component: MotorTelemetryWidget, minW: 4, minH: 3 },
  { id: "gamepad", name: "Gamepad", type: "widget", component: GamepadWidget, minW: 5, minH: 4 },
  { id: "status", name: "Status", type: "navlet", component: StatusNavlet },
  { id: "controls", name: "Controls", type: "navlet", component: ControlsNavlet },
]

export const templates: Template[] = [
  {
    name: "Moteus Overview",
    navlets: [{ navletID: "status" }, { navletID: "controls" }],
    widgets: [
      { x: 0, y: 0, w: 4, h: 6, widgets: [{ widgetID: "emergency" }] },
      { x: 4, y: 0, w: 5, h: 6, widgets: [{ widgetID: "joystick" }] },
      { x: 9, y: 0, w: 7, h: 8, widgets: [{ widgetID: "minimap" }] },
      { x: 0, y: 6, w: 9, h: 6, widgets: [{ widgetID: "motors" }] },
      { x: 9, y: 8, w: 4, h: 4, widgets: [{ widgetID: "target-follower" }] },
      { x: 13, y: 8, w: 3, h: 4, widgets: [{ widgetID: "gamepad" }] },
    ],
  },
  {
    name: "Driving Focus",
    navlets: [{ navletID: "status" }, { navletID: "controls" }],
    widgets: [
      { x: 0, y: 0, w: 5, h: 7, widgets: [{ widgetID: "joystick" }] },
      { x: 5, y: 0, w: 8, h: 7, widgets: [{ widgetID: "minimap" }] },
      { x: 13, y: 0, w: 3, h: 7, widgets: [{ widgetID: "emergency" }] },
      { x: 0, y: 7, w: 5, h: 5, widgets: [{ widgetID: "target-follower" }] },
      { x: 5, y: 7, w: 11, h: 5, widgets: [{ widgetID: "motors" }] },
    ],
  },
]

export function getComponent(id: string, type?: "widget" | "navlet") {
  return components.find((component) => component.id === id && (!type || component.type === type))
}

export function getComponents(type: "widget" | "navlet") {
  return components.filter((component) => component.type === type)
}
