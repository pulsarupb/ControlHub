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

  // Trackpad double-tap + drag to pan
  let tapTimestamps: number[] = []
  let panModeArmed = false
  let panModeActive = false
  let pendingTap = false
  let pendingTapStart = 0
  let panModeTimeout: ReturnType<typeof setTimeout> | null = null

  // Touch state (all touch handled ourselves, never reaches OrbitControls)
  let touchPtrs = new Map<number, { x: number; y: number }>()
  let touchStart: { x: number; y: number } | null = null
  let touchMulti: { centroid: { x: number; y: number }; angle: number; distance: number } | null = null

  let cleanupInputHandlers: (() => void) | null = null

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

    touchPtrs.clear()
    touchStart = null
    touchMulti = null

    if (panModeTimeout) clearTimeout(panModeTimeout)
    panModeArmed = false
    panModeActive = false
    pendingTap = false

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
      ctrl.enableRotate = false
      ctrl.mouseButtons = {
        LEFT: THREE.MOUSE.PAN,
        MIDDLE: THREE.MOUSE.DOLLY,
        RIGHT: THREE.MOUSE.DOLLY,
      }
      ctrl.touches = {
        ONE: THREE.TOUCH.PAN,
        TWO: THREE.TOUCH.DOLLY_PAN,
      }
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
      ctrl.mouseButtons = {
        LEFT: THREE.MOUSE.ROTATE,
        MIDDLE: THREE.MOUSE.PAN,
      }
      ctrl.touches = { ONE: THREE.TOUCH.ROTATE }
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

    setupInputHandlers()
    cleanupInputHandlers = () => teardownInputHandlers()

    function setupInputHandlers() {
      const DOUBLE_TAP_WINDOW = 500
      const TAP_MAX_MS = 400

      function onPointerDown(e: PointerEvent) {
        // --- Double-tap detection (mouse / trackpad) ---
        if (e.pointerType === 'mouse' && e.button === 0) {
          if (panModeArmed) {
            panModeArmed = false
            panModeActive = true
            if (mode === "orbit" && orbitControls) {
              orbitControls.mouseButtons.LEFT = THREE.MOUSE.PAN
            }
            return
          }
          pendingTap = true
          pendingTapStart = performance.now()
        }

        // --- Touch (orbit mode: handle ALL ourselves) ---
        if (e.pointerType === 'touch') {
          if (mode !== "orbit" || !orbitControls) return
          e.stopImmediatePropagation()

          touchPtrs.set(e.pointerId, { x: e.clientX, y: e.clientY })

          if (touchPtrs.size === 1) {
            touchStart = { x: e.clientX, y: e.clientY }
          } else if (touchPtrs.size === 2) {
            touchStart = null
            const pts = Array.from(touchPtrs.values())
            touchMulti = {
              centroid: { x: (pts[0].x + pts[1].x) / 2, y: (pts[0].y + pts[1].y) / 2 },
              angle: Math.atan2(pts[1].y - pts[0].y, pts[1].x - pts[0].x),
              distance: Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y),
            }
          }
        }
      }

      function onPointerMove(e: PointerEvent) {
        // --- Touch (orbit mode only) ---
        if (e.pointerType !== 'touch') return
        if (mode !== "orbit" || !orbitControls) return

        e.stopImmediatePropagation()
        if (!touchStart && !touchMulti) return

        const ptr = touchPtrs.get(e.pointerId)
        if (!ptr) return
        ptr.x = e.clientX
        ptr.y = e.clientY

        // --- 1-finger orbit ---
        if (touchPtrs.size === 1 && touchStart) {
          const dx = e.clientX - touchStart.x
          const dy = e.clientY - touchStart.y
          touchStart.x = e.clientX
          touchStart.y = e.clientY

          if (Math.abs(dx) > 0.5 || Math.abs(dy) > 0.5) {
            const target = orbitControls.target
            const offset = new THREE.Vector3().copy(perspCamera.position).sub(target)
            const spherical = new THREE.Spherical().setFromVector3(offset)
            spherical.theta -= dx * 0.01
            spherical.phi = Math.max(0.1, Math.min(Math.PI - 0.1, spherical.phi - dy * 0.01))
            perspCamera.position.copy(target).add(new THREE.Vector3().setFromSpherical(spherical))
            perspCamera.lookAt(target)
          }
          return
        }

        // --- 2-finger pan + rotate + zoom ---
        if (touchPtrs.size === 2 && touchMulti) {
          const pts = Array.from(touchPtrs.values())
          const cx = (pts[0].x + pts[1].x) / 2
          const cy = (pts[0].y + pts[1].y) / 2
          const angle = Math.atan2(pts[1].y - pts[0].y, pts[1].x - pts[0].x)
          const dist = Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y)

          const cam = perspCamera
          const target = orbitControls.target
          const offset = new THREE.Vector3().copy(cam.position).sub(target)
          const spherical = new THREE.Spherical().setFromVector3(offset)

          const panX = (cx - touchMulti.centroid.x) * 0.005
          const panZ = (cy - touchMulti.centroid.y) * 0.005
          if (Math.abs(panX) > 0.0001 || Math.abs(panZ) > 0.0001) {
            const right = new THREE.Vector3()
            const up = new THREE.Vector3(0, 1, 0)
            right.crossVectors(offset, up).normalize()
            target.add(right.multiplyScalar(-panX))
            target.add(up.clone().multiplyScalar(panZ))
          }

          const dAngle = angle - touchMulti.angle
          if (Math.abs(dAngle) > 0.002) {
            spherical.theta += dAngle
          }

          const dDist = touchMulti.distance - dist
          if (Math.abs(dDist) > 0.5) {
            spherical.radius = Math.max(0.3, Math.min(15, spherical.radius * (1 + dDist * 0.003)))
          }

          cam.position.copy(target).add(new THREE.Vector3().setFromSpherical(spherical))
          cam.lookAt(target)

          touchMulti.centroid.x = cx
          touchMulti.centroid.y = cy
          touchMulti.angle = angle
          touchMulti.distance = dist
        }
      }

      function onPointerUp(e: PointerEvent) {
        // --- Trackpad ---
        if (e.pointerType === 'mouse' && e.button === 0) {
          if (panModeActive) {
            panModeActive = false
            if (mode === "orbit" && orbitControls) {
              orbitControls.mouseButtons.LEFT = THREE.MOUSE.ROTATE
            }
            return
          }
          if (!pendingTap) return
          pendingTap = false
          const duration = performance.now() - pendingTapStart
          if (duration > TAP_MAX_MS) return
          const now = performance.now()
          tapTimestamps = tapTimestamps.filter(t => now - t < DOUBLE_TAP_WINDOW)
          tapTimestamps.push(now)
          if (tapTimestamps.length >= 2) {
            panModeArmed = true
            if (panModeTimeout) clearTimeout(panModeTimeout)
            panModeTimeout = setTimeout(() => { panModeArmed = false }, 500)
          }
        }

        // --- Touch (orbit mode only) ---
        if (e.pointerType === 'touch') {
          if (mode !== "orbit" || !orbitControls) return
          e.stopImmediatePropagation()
          touchPtrs.delete(e.pointerId)
          if (touchPtrs.size === 1) {
            const remaining = Array.from(touchPtrs.entries())[0]
            touchStart = { x: remaining[1].x, y: remaining[1].y }
            touchMulti = null
          } else if (touchPtrs.size === 0) {
            touchStart = null
            touchMulti = null
          }
        }
      }

      function onPointerCancel(e: PointerEvent) {
        if (e.pointerType === 'touch' && mode === "orbit" && orbitControls) {
          e.stopImmediatePropagation()
          touchPtrs.clear()
          touchStart = null
          touchMulti = null
        }
      }

      container!.addEventListener('pointerdown', onPointerDown, { capture: true })
      container!.addEventListener('pointerup', onPointerUp, { capture: true })
      container!.addEventListener('pointermove', onPointerMove, { capture: true })
      container!.addEventListener('pointercancel', onPointerCancel, { capture: true })
      container!.addEventListener('pointerleave', onPointerCancel, { capture: true })
    }

    function teardownInputHandlers() {
      if (panModeTimeout) clearTimeout(panModeTimeout)
    }

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
          if (mode === "planar" && orthoCamera) {
            orthoCamera.position.set(px, orthoCamera.position.y, pz)
            activeControls?.target.set(px, 0, pz)
          } else if (mode === "orbit" && orbitControls) {
            const offset = new THREE.Vector3().copy(perspCamera.position).sub(orbitControls.target)
            orbitControls.target.set(px, 0, pz)
            perspCamera.position.copy(orbitControls.target).add(offset)
            orbitControls.enablePan = false
            orbitControls.enableDamping = false
          }
        } else if (mode === "orbit" && orbitControls) {
          orbitControls.enablePan = true
          orbitControls.enableDamping = true
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
      cleanupInputHandlers?.()
      if (panModeTimeout) clearTimeout(panModeTimeout)
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
            if (mode === "planar" && orthoCamera) {
              orthoCamera.position.set(rx, orthoCamera.position.y, rz)
              activeControls?.target.set(rx, 0, rz)
            } else if (mode === "orbit" && orbitControls) {
              const offset = new THREE.Vector3().copy(perspCamera.position).sub(orbitControls.target)
              orbitControls.target.set(rx, 0, rz)
              perspCamera.position.copy(orbitControls.target).add(offset)
              orbitControls.enablePan = false
              orbitControls.enableDamping = false
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
