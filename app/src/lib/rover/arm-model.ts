// URDF kinematic tree adapted from the existing arm viewer. No local IK solver.
import * as THREE from "three"
import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js"

export type Joint = { node: THREE.Group; axis: THREE.Vector3; mimic: string | null; multiplier: number; offset: number; upper: number }
export type ArmModel = { root: THREE.Group; joints: Map<string, Joint> }
const vector = (text: string | null | undefined) => (text ?? "0 0 0").trim().split(/\s+/).map(Number)

export async function loadArm(): Promise<ArmModel> {
  const response = await fetch("/kinova-models/gen3_lite.urdf")
  if (!response.ok) throw new Error("Cannot load Kinova URDF")
  const doc = new DOMParser().parseFromString(await response.text(), "text/xml")
  const robot = doc.querySelector("robot")
  if (!robot) throw new Error("Invalid arm URDF")
  const children = (tag: string) => Array.from(robot.children).filter(e => e.tagName === tag)
  const links = new Map<string, THREE.Group>()
  const loader = new STLLoader()
  const cache = new Map<string, THREE.BufferGeometry>()
  for (const link of children("link")) {
    const group = new THREE.Group()
    links.set(link.getAttribute("name")!, group)
    for (const visual of link.querySelectorAll("visual")) {
      const file = visual.querySelector("mesh")?.getAttribute("filename")?.split("/").pop()
      if (!file) continue
      let geometry = cache.get(file)
      if (!geometry) { geometry = await loader.loadAsync(`/kinova-models/meshes/${file}`); cache.set(file, geometry) }
      const rgb = vector(visual.querySelector("color")?.getAttribute("rgba") ?? "0.8 0.82 0.82")
      const mesh = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color: new THREE.Color(rgb[0], rgb[1], rgb[2]), roughness: .6 }))
      const origin = visual.querySelector("origin")
      const xyz = vector(origin?.getAttribute("xyz")), rpy = vector(origin?.getAttribute("rpy"))
      mesh.position.fromArray(xyz)
      mesh.rotation.set(rpy[0], rpy[1], rpy[2], "ZYX")
      group.add(mesh)
    }
  }
  const joints = new Map<string, Joint>()
  for (const j of children("joint")) {
    const parent = links.get(j.querySelector("parent")!.getAttribute("link")!)
    const child = links.get(j.querySelector("child")!.getAttribute("link")!)
    if (!parent || !child) continue
    const origin = j.querySelector("origin"), xyz = vector(origin?.getAttribute("xyz")), rpy = vector(origin?.getAttribute("rpy"))
    const pivot = new THREE.Group(), node = new THREE.Group()
    pivot.position.fromArray(xyz); pivot.rotation.set(rpy[0], rpy[1], rpy[2], "ZYX")
    const name = j.getAttribute("name")!
    node.name = name; node.add(child); pivot.add(node); parent.add(pivot)
    const mimic = j.querySelector("mimic")
    joints.set(name, { node, axis: new THREE.Vector3().fromArray(vector(j.querySelector("axis")?.getAttribute("xyz"))).normalize(),
      mimic: mimic?.getAttribute("joint") ?? null, multiplier: Number(mimic?.getAttribute("multiplier") ?? 1), offset: Number(mimic?.getAttribute("offset") ?? 0),
      upper: Number(j.querySelector("limit")?.getAttribute("upper") ?? 0) })
  }
  const root = new THREE.Group()
  root.add(links.get("base_link")!)
  return { root, joints }
}

export function setArm(model: ArmModel, angles: number[], gripper: number | null) {
  angles.forEach((q, i) => {
    const joint = model.joints.get(`joint_${i + 1}`)
    if (joint) joint.node.quaternion.setFromAxisAngle(joint.axis, THREE.MathUtils.degToRad(q))
  })
  if (gripper !== null) {
    const source = model.joints.get("right_finger_bottom_joint")
    const open = Math.min(.55, source?.upper ?? .55)
    const value = open + Math.max(0, Math.min(1, gripper)) * ((source?.upper ?? open) - open)
    source?.node.quaternion.setFromAxisAngle(source.axis, value)
    for (const joint of model.joints.values()) if (joint.mimic === "right_finger_bottom_joint") {
      joint.node.quaternion.setFromAxisAngle(joint.axis, value * joint.multiplier + joint.offset)
    }
  }
}

export function clonePreview(model: ArmModel): ArmModel {
  const root = model.root.clone(true)
  root.traverse(o => { if (o instanceof THREE.Mesh) o.material = new THREE.MeshBasicMaterial({ color: "#e8bd62", transparent: true, opacity: .25, depthWrite: false }) })
  return { root, joints: new Map([...model.joints].map(([name, j]) => [name, { ...j, node: root.getObjectByName(name) as THREE.Group }])) }
}
