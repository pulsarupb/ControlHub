<script lang="ts">
  import { onMount } from "svelte"
  import * as THREE from "three"
  import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js"
  import { MapControls } from "three/examples/jsm/controls/MapControls.js"
  import { rover } from "$lib/rover-control.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"
  import type { UrdfRobot, UrdfOrigin, UrdfGeometry, UrdfMaterial, Vec3, JointType } from "$lib/rover-model/types"

  const STORAGE_KEY = "minimapState"

  function loadState(): { mode: "orbit" | "planar"; followRover: boolean } {
    try {
      const saved = typeof localStorage !== "undefined" ? localStorage.getItem(STORAGE_KEY) : null
      if (saved) return JSON.parse(saved)
    } catch { /* ignore */ }
    return { mode: "orbit", followRover: true }
  }

  function saveState() {
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(STORAGE_KEY, JSON.stringify({ mode, followRover }))
      }
    } catch { /* ignore */ }
  }

  const saved = loadState()

  let container: HTMLDivElement | undefined = $state()
  let statusText = $state("Loading...")
  let loaded = $state(false)
  let mode: "orbit" | "planar" = $state(saved.mode)
  let followRover = $state(saved.followRover)

  let scene!: THREE.Scene
  let perspCamera!: THREE.PerspectiveCamera
  let orthoCamera!: THREE.OrthographicCamera
  let activeCamera!: THREE.PerspectiveCamera | THREE.OrthographicCamera
  let renderer!: THREE.WebGLRenderer
  let orbitControls!: OrbitControls
  let mapControls!: MapControls
  let activeControls!: OrbitControls | MapControls
  let roverGroup!: THREE.Group
  let pathLine!: THREE.Line
  let robot!: UrdfRobot
  let animFrameId = 0

  const ORBIT_POS = new THREE.Vector3(1.8, 1.2, 2.2)
  const ORTHO_SIZE = 3

  function urdfOriginToMatrix(origin: UrdfOrigin): THREE.Matrix4 {
    const pos = new THREE.Vector3(-origin.xyz.y, origin.xyz.z, -origin.xyz.x)
    const qRoll = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 0, -1), origin.rpy.x)
    const qPitch = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(-1, 0, 0), origin.rpy.y)
    const qYaw = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), origin.rpy.z)
    const quat = qYaw.clone().multiply(qPitch).multiply(qRoll)
    return new THREE.Matrix4().compose(pos, quat, new THREE.Vector3(1, 1, 1))
  }

  function createGeometry(geo: UrdfGeometry): THREE.BufferGeometry | null {
    switch (geo.type) {
      case "box":
        return new THREE.BoxGeometry(geo.size.y, geo.size.z, geo.size.x)
      case "cylinder":
        return new THREE.CylinderGeometry(geo.radius, geo.radius, geo.length, 24)
      case "sphere":
        return new THREE.SphereGeometry(geo.radius, 24, 24)
      case "mesh":
        return null
    }
  }

  function getMaterialColor(materialName: string | undefined, r: UrdfRobot): THREE.Color {
    if (!materialName) return new THREE.Color(0x888888)
    const mat = r.materials.find((m) => m.name === materialName)
    if (mat?.color) return new THREE.Color(mat.color.r, mat.color.g, mat.color.b)
    return new THREE.Color(0x888888)
  }

  function renderLink(r: UrdfRobot, linkName: string, parentMatrix: THREE.Matrix4, parentGroup: THREE.Group) {
    const link = r.links.find((l) => l.name === linkName)
    if (!link) return

    const linkGroup = new THREE.Group()
    linkGroup.applyMatrix4(parentMatrix)
    parentGroup.add(linkGroup)

    for (const visual of link.visuals) {
      const geo = createGeometry(visual.geometry)
      if (!geo) continue
      const color = getMaterialColor(visual.material_name, r)
      const mat = new THREE.MeshStandardMaterial({ color, roughness: 0.7, metalness: 0.1 })
      const mesh = new THREE.Mesh(geo, mat)
      mesh.applyMatrix4(urdfOriginToMatrix(visual.origin))
      linkGroup.add(mesh)
    }

    for (const joint of r.joints.filter((j) => j.parent === linkName)) {
      const childMatrix = parentMatrix.clone().multiply(urdfOriginToMatrix(joint.origin))
      renderLink(r, joint.child, childMatrix, parentGroup)
    }
  }

  function setCameraMode(newMode: "orbit" | "planar") {
    mode = newMode
    saveState()
    if (!renderer) return

    activeControls?.dispose()

    const rx = roverGroup?.position.x ?? 0
    const rz = roverGroup?.position.z ?? 0
    const aspect = (container?.clientWidth ?? 1) / (container?.clientHeight ?? 1)

    if (newMode === "planar") {
      orthoCamera = new THREE.OrthographicCamera(
        -ORTHO_SIZE * aspect, ORTHO_SIZE * aspect,
        ORTHO_SIZE, -ORTHO_SIZE,
        0.1, 100,
      )
      orthoCamera.position.set(rx, 5, rz)
      orthoCamera.lookAt(rx, 0, rz)
      activeCamera = orthoCamera

      const ctrl = new MapControls(orthoCamera, renderer.domElement)
      ctrl.target.set(rx, 0, rz)
      ctrl.enableDamping = true
      ctrl.update()
      mapControls = ctrl
      activeControls = ctrl
      followRover = true
      saveState()
    } else {
      activeCamera = perspCamera
      const ctrl = new OrbitControls(perspCamera, renderer.domElement)
      ctrl.target.set(rx, 0, rz)
      ctrl.enableDamping = true
      ctrl.minDistance = 0.3
      ctrl.maxDistance = 15
      perspCamera.position.set(rx + ORBIT_POS.x, ORBIT_POS.y, rz + ORBIT_POS.z)
      ctrl.update()
      orbitControls = ctrl
      activeControls = ctrl
    }
  }

  function buildScene() {
    if (!robot || !scene || !roverGroup) return
    scene.remove(roverGroup)
    roverGroup.traverse((child) => {
      if (child instanceof THREE.Mesh) {
        child.geometry.dispose()
        ;(child.material as THREE.Material).dispose()
      }
    })
    roverGroup.clear()

    const base = robot.links.find((l) => l.name === "base_link")
    if (!base) {
      statusText = "No base_link found"
      loaded = true
      return
    }

    renderLink(robot, base.name, new THREE.Matrix4(), roverGroup)
    scene.add(roverGroup)
    statusText = `${robot.links.length} links, ${robot.joints.length} joints`
    renderer?.render(scene, activeCamera)
    loaded = true
  }

  function createInfiniteGrid(): THREE.Mesh {
    const vert = `
      varying vec3 worldPosition;
      void main() {
        worldPosition = (modelMatrix * vec4(position, 1.0)).xyz;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `
    const frag = `
      uniform float uGridSize;
      uniform vec3 uColor;
      uniform float uFadeStart;
      uniform float uFadeEnd;
      uniform vec2 uCenter;

      varying vec3 worldPosition;

      void main() {
        vec2 coord = worldPosition.xz;
        vec2 grid = abs(fract(coord / uGridSize - 0.5) - 0.5) * uGridSize;
        vec2 width = fwidth(coord);
        vec2 line = 1.0 - smoothstep(0.0, 1.0, grid / width);

        float gridIntensity = max(line.x, line.y);
        vec2 offset = worldPosition.xz - uCenter;
        float dist = length(offset);
        float fade = 1.0 - smoothstep(uFadeStart, uFadeEnd, dist);
        float alpha = gridIntensity * fade * 0.5;

        gl_FragColor = vec4(uColor, alpha);
      }
    `

    const geo = new THREE.PlaneGeometry(60, 60)
    geo.rotateX(-Math.PI / 2)

    const mat = new THREE.ShaderMaterial({
      uniforms: {
        uGridSize: { value: 1.0 },
        uColor: { value: new THREE.Color(0xffffff) },
        uFadeStart: { value: 5.0 },
        uFadeEnd: { value: 15.0 },
        uCenter: { value: new THREE.Vector2(0, 0) },
      },
      vertexShader: vert,
      fragmentShader: frag,
      transparent: true,
      depthWrite: false,
      side: THREE.DoubleSide,
    })

    return new THREE.Mesh(geo, mat)
  }

  function toVec3(v: [number, number, number] | Vec3): Vec3 {
    return Array.isArray(v) ? { x: v[0], y: v[1], z: v[2] } : v
  }

  function convertModel(raw: Record<string, unknown>): UrdfRobot {
    return {
      name: (raw.name as string) ?? "",
      materials: ((raw.materials ?? []) as UrdfMaterial[])?.map((m) => ({
        name: (m.name as string) ?? "",
        r: (m.r as number) ?? 0.5,
        g: (m.g as number) ?? 0.5,
        b: (m.b as number) ?? 0.5,
        a: (m.a as number) ?? 1.0,
      })) ?? [],
      links: (raw.links as Record<string, unknown>[])?.map((l) => ({
        name: (l.name as string) ?? "",
        visuals: ((l.visuals as Record<string, unknown>[]) ?? []).map((v) => ({
          origin: {
            xyz: toVec3((v.origin as Record<string, unknown>)?.xyz as [number, number, number] | Vec3 ?? [0, 0, 0]),
            rpy: toVec3((v.origin as Record<string, unknown>)?.rpy as [number, number, number] | Vec3 ?? [0, 0, 0]),
          },
          geometry: convertGeometry(v.geometry as Record<string, unknown>),
          material_name: v.material_name as string | undefined,
        })),
        collisions: [],
      })) ?? [],
      joints: ((raw.joints ?? []) as Record<string, unknown>[])?.map((j) => ({
        name: (j.name as string) ?? "",
        type: (j.type as JointType) ?? "fixed",
        parent: (j.parent as string) ?? "",
        child: (j.child as string) ?? "",
        origin: {
          xyz: toVec3((j.origin as Record<string, unknown>)?.xyz as [number, number, number] | Vec3 ?? [0, 0, 0]),
          rpy: toVec3((j.origin as Record<string, unknown>)?.rpy as [number, number, number] | Vec3 ?? [0, 0, 0]),
        },
      })) ?? [],
    }
  }

  function convertGeometry(g: Record<string, unknown>): UrdfGeometry {
    const type = (g.type as string) ?? "box"
    switch (type) {
      case "box": {
        const size = (g.size as [number, number, number] | Vec3) ?? [0, 0, 0]
        return { type: "box", size: toVec3(size) }
      }
      case "cylinder":
        return { type: "cylinder", radius: (g.radius as number) ?? 0, length: (g.length as number) ?? 0 }
      case "sphere":
        return { type: "sphere", radius: (g.radius as number) ?? 0 }
      default:
        return { type: "box", size: { x: 0, y: 0, z: 0 } }
    }
  }

  async function loadModel() {
    try {
      const resp = await fetch(roverConnection.apiUrl("/api/urdf-model"))
      const raw = await resp.json()
      robot = convertModel(raw)
      buildScene()
    } catch (e) {
      statusText = "Failed to load URDF"
      loaded = true
      console.error(e)
    }
  }

  onMount(() => {
    if (!container) return

    scene = new THREE.Scene()
    scene.background = new THREE.Color(0x101214)

    perspCamera = new THREE.PerspectiveCamera(40, container.clientWidth / container.clientHeight, 0.1, 100)
    activeCamera = perspCamera

    renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setSize(container.clientWidth, container.clientHeight, false)
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    container.appendChild(renderer.domElement)


    const ambient = new THREE.AmbientLight(0x606080, 0.8)
    scene.add(ambient)
    const dirLight = new THREE.DirectionalLight(0xffeedd, 2.5)
    dirLight.position.set(5, 10, 5)
    scene.add(dirLight)
    const fillLight = new THREE.DirectionalLight(0x8888ff, 0.6)
    fillLight.position.set(-5, 0, -5)
    scene.add(fillLight)

    const grid = createInfiniteGrid()
    grid.position.y = -0.35
    scene.add(grid)
    const gridMat = grid.material as THREE.ShaderMaterial

    const pathMat = new THREE.LineBasicMaterial({ color: 0xd8a84e, linewidth: 2 })
    const pathGeo = new THREE.BufferGeometry()
    pathLine = new THREE.Line(pathGeo, pathMat)
    scene.add(pathLine)

    roverGroup = new THREE.Group()
    scene.add(roverGroup)

    setCameraMode(saved.mode)

    loadModel()

    function animate() {
      animFrameId = requestAnimationFrame(animate)

      if (loaded && roverGroup && rover.status) {
        const px = -(rover.pose?.y_m ?? 0)
        const pz = rover.pose?.x_m ?? 0
        const heading = -(rover.pose?.heading_rad ?? 0)

        roverGroup.position.x = px
        roverGroup.position.z = pz
        roverGroup.rotation.y = heading
        gridMat.uniforms.uCenter.value.set(px, pz)

        if (followRover) {
          activeControls?.target.set(px, 0, pz)
          if (mode === "planar" && orthoCamera) {
            orthoCamera.position.set(px, orthoCamera.position.y, pz)
          }
        }

        if (rover.path?.length) {
          const pts = rover.path.map((p: { x_m: number; y_m: number }) =>
            new THREE.Vector3(-p.y_m, -0.35, p.x_m),
          )
          pathLine.geometry.dispose()
          pathLine.geometry = new THREE.BufferGeometry().setFromPoints(pts)
        }
      }

      activeControls?.update()
      if (loaded && renderer && scene && activeCamera) {
        renderer.render(scene, activeCamera)
      }
    }
    animate()

    const resizeObserver = new ResizeObserver(([entry]) => {
      const w = entry.contentRect.width
      const h = entry.contentRect.height
      if (renderer) renderer.setSize(w, h, false)
      if (perspCamera) {
        perspCamera.aspect = w / h
        perspCamera.updateProjectionMatrix()
      }
      if (orthoCamera) {
        const aspect = w / h
        orthoCamera.left = -ORTHO_SIZE * aspect
        orthoCamera.right = ORTHO_SIZE * aspect
        orthoCamera.top = ORTHO_SIZE
        orthoCamera.bottom = -ORTHO_SIZE
        orthoCamera.updateProjectionMatrix()
      }
    })
    resizeObserver.observe(container)

    return () => {
      cancelAnimationFrame(animFrameId)
      resizeObserver.disconnect()
      activeControls?.dispose()
      if (renderer) {
        renderer.dispose()
        if (container?.contains(renderer.domElement)) {
          container.removeChild(renderer.domElement)
        }
      }
    }
  })
</script>

<div class="minimap-widget">
  <div class="topbar">
    <div class="mode-toggle">
      <button
        class="mode-btn"
        class:active={mode === "orbit"}
        onclick={() => setCameraMode("orbit")}
      >Orbit</button>
      <button
        class="mode-btn"
        class:active={mode === "planar"}
        onclick={() => setCameraMode("planar")}
      >Planar</button>
      <button
        class="mode-btn follow-btn"
        class:active={followRover}
        onclick={() => {
          followRover = !followRover
          saveState()
          if (followRover && roverGroup) {
            const rx = roverGroup.position.x
            const rz = roverGroup.position.z
            activeControls?.target.set(rx, 0, rz)
            if (mode === "planar" && orthoCamera) {
              orthoCamera.position.set(rx, orthoCamera.position.y, rz)
            }
            activeControls?.update()
          }
        }}
      >{followRover ? "Follow" : "Free"}</button>
    </div>
    <span class="status">{statusText}</span>
  </div>
  <div class="viewport" class:loaded>
    {#if !loaded}
      <div class="loading-overlay">Loading rover model…</div>
    {/if}
    <div bind:this={container} class="three-inner"></div>
  </div>
</div>

<style>
  .minimap-widget {
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 0.4rem;
    overflow: hidden;
  }
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .mode-toggle {
    display: flex;
    gap: 0.2rem;
  }
  .mode-btn {
    padding: 0.2rem 0.5rem;
    border-radius: 0.3rem;
    border: 1px solid var(--border, #343a40);
    background: transparent;
    color: var(--textMuted, #a6adb5);
    cursor: pointer;
    font-size: 0.7rem;
    font-family: inherit;
  }
  .mode-btn:hover {
    background: var(--surfaceRaised, #202428);
    color: var(--text, #f2f4f6);
  }
  .mode-btn.active {
    background: var(--accent, #7aa7c7);
    border-color: var(--accent, #7aa7c7);
    color: var(--text, #f2f4f6);
  }
  .status {
    color: var(--textMuted, #a6adb5);
    font-size: 0.7rem;
    text-transform: uppercase;
  }
  .viewport {
    border-radius: 0.55rem;
    overflow: hidden;
    position: relative;
  }
  .three-inner {
    width: 100%;
    height: 100%;
  }
  .three-inner :global(canvas) {
    display: block;
    width: 100%;
    height: 100%;
    touch-action: none;
  }
  .loading-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bgDark, #101214);
    color: var(--textMuted, #a6adb5);
    font-size: 0.8rem;
    z-index: 2;
    border-radius: 0.55rem;
  }
</style>
