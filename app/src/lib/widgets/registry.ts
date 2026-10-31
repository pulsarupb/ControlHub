import type { DashboardComponent, Template } from "$lib/types"
import EmergencyWidget from "$lib/rover/EmergencyWidget.svelte"
import GamepadWidget from "$lib/widgets/GamepadWidget.svelte"
import JoystickWidget from "$lib/rover/JoystickWidget.svelte"
import ZedMapWidget from "$lib/rover/ZedMapWidget.svelte"
import WheelsWidget from "$lib/rover/WheelsWidget.svelte"
import RtkWidget from "$lib/rover/RtkWidget.svelte"
import RtkMapWidget from "$lib/rover/RtkMapWidget.svelte"
import ZedVideoWidget from "$lib/rover/ZedVideoWidget.svelte"
import OakCamerasWidget from "$lib/rover/OakCamerasWidget.svelte"
import KinovaViewWidget from "$lib/rover/KinovaViewWidget.svelte"
import KinovaControlsWidget from "$lib/rover/KinovaControlsWidget.svelte"
import StatusNavlet from "$lib/rover/StatusNavlet.svelte"
import WorkspacesNavlet from "$lib/navlets/WorkspacesNavlet.svelte"
export const components: DashboardComponent[] = [
  { id: "emergency", name: "Emergency Controls", type: "widget", component: EmergencyWidget, minW: 3, minH: 3 },
  { id: "joystick", name: "Joystick", type: "widget", component: JoystickWidget, minW: 4, minH: 4 },
  { id: "minimap", name: "ZED Map / Pose", type: "widget", component: ZedMapWidget, minW: 6, minH: 5 },
  { id: "motors", name: "Wheel / Controller Status", type: "widget", component: WheelsWidget, minW: 6, minH: 4 },
  { id: "rtk", name: "RTK Position", type: "widget", component: RtkWidget, minW: 5, minH: 4 },
  { id: "rtk-map", name: "RTK Map (Offline)", type: "widget", component: RtkMapWidget, minW: 7, minH: 6 },
  { id: "zed-video", name: "ZED Color / Depth", type: "widget", component: ZedVideoWidget, minW: 5, minH: 4 },
  { id: "oak-cameras", name: "OAK-D Cameras (3)", type: "widget", component: OakCamerasWidget, minW: 6, minH: 5 },
  { id: "kinova-view", name: "Kinova 3D / IK Target", type: "widget", component: KinovaViewWidget, minW: 6, minH: 5 },
  { id: "kinova-controls", name: "Kinova Manual / IK", type: "widget", component: KinovaControlsWidget, minW: 7, minH: 7 },
  { id: "gamepad", name: "Xbox Controller", type: "widget", component: GamepadWidget, minW: 5, minH: 4 },
  { id: "status", name: "Status", type: "navlet", component: StatusNavlet },
  { id: "workspaces", name: "Workspaces", type: "navlet", component: WorkspacesNavlet },
]

export const templates: Template[] = [
  {
    name: "Rover Overview",
    navlets: [{ navletID: "status" }, { navletID: "workspaces" }],
    widgets: [
      { x: 0, y: 0, w: 4, h: 6, widgets: [{ widgetID: "emergency" }] },
      { x: 4, y: 0, w: 8, h: 6, widgets: [{ widgetID: "motors" }] },
      { x: 12, y: 0, w: 5, h: 6, widgets: [{ widgetID: "joystick" }] },
      { x: 17, y: 0, w: 7, h: 6, widgets: [{ widgetID: "minimap" }] },
      { x: 24, y: 0, w: 12, h: 6, widgets: [{ widgetID: "gamepad" }] },
    ],
  },
  {
    name: "Arm and Sensors",
    navlets: [{ navletID: "status" }, { navletID: "workspaces" }],
    widgets: [
      { x: 0, y: 0, w: 8, h: 10, widgets: [{ widgetID: "kinova-controls" }] },
      { x: 8, y: 0, w: 9, h: 10, widgets: [{ widgetID: "kinova-view" }] },
      { x: 17, y: 0, w: 9, h: 6, widgets: [{ widgetID: "minimap" }] },
      { x: 26, y: 0, w: 7, h: 6, widgets: [{ widgetID: "rtk" }] },
      { x: 17, y: 6, w: 9, h: 5, widgets: [{ widgetID: "zed-video" }] },
      { x: 26, y: 6, w: 7, h: 5, widgets: [{ widgetID: "emergency" }] },
      { x: 0, y: 11, w: 33, h: 7, widgets: [{ widgetID: "oak-cameras" }] },
    ],
  },
  {
    name: "Driving Focus",
    navlets: [{ navletID: "status" }, { navletID: "workspaces" }],
    widgets: [
      { x: 0, y: 0, w: 8, h: 6, widgets: [{ widgetID: "motors" }] },
      { x: 8, y: 0, w: 4, h: 6, widgets: [{ widgetID: "emergency" }] },
      { x: 12, y: 0, w: 5, h: 6, widgets: [{ widgetID: "joystick" }] },
      { x: 17, y: 0, w: 7, h: 6, widgets: [{ widgetID: "minimap" }] },
    ],
  },
]

export function getComponent(id: string, type?: "widget" | "navlet") {
  return components.find((component) => component.id === id && (!type || component.type === type))
}

export function getComponents(type: "widget" | "navlet") {
  return components.filter((component) => component.type === type)
}
