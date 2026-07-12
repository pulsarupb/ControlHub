export type PartType = "chassis" | "wheel" | "sensor" | "arm" | "battery" | "camera" | "antenna"

export interface Vec3 {
  x: number
  y: number
  z: number
}

export interface UrdfColor {
  r: number
  g: number
  b: number
  a: number
}

export interface UrdfMaterial {
  name: string
  color?: UrdfColor
  texture?: string
  /** Backend JSON format uses flat fields */
  r?: number
  g?: number
  b?: number
  a?: number
}

export type UrdfGeometry =
  | { type: "box"; size: Vec3 }
  | { type: "cylinder"; radius: number; length: number }
  | { type: "sphere"; radius: number }
  | { type: "mesh"; filename: string; scale?: Vec3 }

export interface UrdfOrigin {
  xyz: Vec3
  rpy: Vec3
}

export interface UrdfVisual {
  origin: UrdfOrigin
  geometry: UrdfGeometry
  material_name?: string
}

export interface UrdfCollision {
  origin: UrdfOrigin
  geometry: UrdfGeometry
}

export interface UrdfInertial {
  origin: UrdfOrigin
  mass: number
  inertia: {
    ixx: number
    ixy: number
    ixz: number
    iyy: number
    iyz: number
    izz: number
  }
}

export interface UrdfLink {
  name: string
  visuals: UrdfVisual[]
  collisions: UrdfCollision[]
  inertial?: UrdfInertial
}

export type JointType = "revolute" | "continuous" | "prismatic" | "fixed" | "floating" | "planar"

export interface UrdfJoint {
  name: string
  type: JointType
  parent: string
  child: string
  origin: UrdfOrigin
  axis?: Vec3
  limit?: {
    lower?: number
    upper?: number
    effort: number
    velocity: number
  }
  dynamics?: {
    damping?: number
    friction?: number
  }
}

export interface UrdfRobot {
  name: string
  materials: UrdfMaterial[]
  links: UrdfLink[]
  joints: UrdfJoint[]
}

export const PART_TYPE_COLORS: Record<PartType, string> = {
  chassis: "#3a3a3a",
  wheel: "#666666",
  sensor: "#2288cc",
  arm: "#cc8822",
  battery: "#33aa33",
  camera: "#8833cc",
  antenna: "#888888",
}

export const PART_TYPES: PartType[] = [
  "chassis",
  "wheel",
  "sensor",
  "arm",
  "battery",
  "camera",
  "antenna",
]
