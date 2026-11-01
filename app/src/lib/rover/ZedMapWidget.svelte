<script lang="ts">
  import { onMount } from "svelte"
  import * as THREE from "three"
  import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js"
  import { STLLoader } from "three/examples/jsm/loaders/STLLoader.js"
  import roverModelUrl from "../../../../R16_Rover.stl?url"
  import { request } from "$lib/config-api"
  import { hardware } from "$lib/hardware.svelte"
  import { rover } from "$lib/rover-control.svelte"
  import type { ZedMap, ZedMeshSnapshot } from "$lib/hardware-types"
  import { appearanceKey, parseAppearance } from "./zed-appearance"
  import DeviceStatus from "./DeviceStatus.svelte"
  let host: HTMLDivElement
  let error = $state("")
  let modelError = $state("")
  let modelLoaded = $state(false)
  let pointSize = $state(.025)
  let brightness = $state(1)
  let opacity = $state(1)
  let colorMode = $state("rgb")
  let viewMode = $state("points")
  let settingsReady = $state(false)
  let storageError = $state("")
  let meshError = $state("")
  let generatingMesh = $state(false)
  let meshAvailable = $state(false)
  let meshTextured = $state(false)
  let meshTruncated = $state(false)
  let generateMesh = () => {}
  let updateAppearance = () => {}
  let cloudMaterial: THREE.PointsMaterial | undefined
  function updatePointSize(event: Event) {
    pointSize = (event.currentTarget as HTMLInputElement).valueAsNumber
    if (cloudMaterial) cloudMaterial.size = pointSize
  }
  const device = $derived(rover.status?.devices.zed)
  $effect(() => {
    const settings = { pointSize, brightness, opacity, colorMode, viewMode }
    if (!settingsReady) return
    try { localStorage.setItem(appearanceKey, JSON.stringify(settings)); storageError = "" }
    catch { storageError = "Appearance settings could not be saved in this browser." }
  })

  onMount(() => {
    try {
      const settings = parseAppearance(localStorage.getItem(appearanceKey))
      pointSize = settings.pointSize; brightness = settings.brightness; opacity = settings.opacity
      colorMode = settings.colorMode; viewMode = settings.viewMode
    } catch { storageError = "Local storage is unavailable; appearance settings will not persist." }
    settingsReady = true
    const renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2))
    host.appendChild(renderer.domElement)
    const scene = new THREE.Scene()
    scene.background = new THREE.Color("#171b22")
    scene.add(new THREE.HemisphereLight("#e5f3ff", "#454039", 2))
    const light = new THREE.DirectionalLight("#ffffff", 3)
    light.position.set(3, 5, 2)
    scene.add(light)
    const camera = new THREE.PerspectiveCamera(55, 1, .01, 1000)
    camera.position.set(3, 3, 3)
    const controls = new OrbitControls(camera, renderer.domElement)
    controls.enableDamping = true
    const grid = new THREE.GridHelper(20, 40, "#475363", "#2a323e")
    scene.add(grid)
    // Proper rotation: native (x,y,z) -> Three.js (x,z,-y), determinant +1.
    const native = new THREE.Group()
    native.rotation.x = -Math.PI / 2
    scene.add(native)
    const sensor = new THREE.Group()
    sensor.add(new THREE.AxesHelper(.4))
    const body = new THREE.Group()
    native.add(sensor, body)
    sensor.visible = body.visible = false
    // Soft circular splats instead of square pixels. Alpha-to-coverage smooths
    // edges while retaining depth writes, so rear points cannot bleed through.
    const sprite = document.createElement("canvas")
    sprite.width = sprite.height = 64
    const context = sprite.getContext("2d")!
    const gradient = context.createRadialGradient(32, 32, 0, 32, 32, 32)
    gradient.addColorStop(0, "rgba(255,255,255,1)")
    gradient.addColorStop(.7, "rgba(255,255,255,1)")
    gradient.addColorStop(1, "rgba(255,255,255,0)")
    context.fillStyle = gradient
    context.fillRect(0, 0, 64, 64)
    const pointTexture = new THREE.CanvasTexture(sprite)
    cloudMaterial = new THREE.PointsMaterial({
      size: pointSize, vertexColors: true, map: pointTexture,
      alphaTest: .05, alphaToCoverage: true, depthWrite: true,
    })
    const cloud = new THREE.Points(new THREE.BufferGeometry(), cloudMaterial)
    const surfaceMaterial = new THREE.MeshBasicMaterial({ side: THREE.DoubleSide, vertexColors: true })
    const surface = new THREE.Mesh(new THREE.BufferGeometry(), surfaceMaterial)
    native.add(surface)
    let rgbColors = new THREE.BufferAttribute(new Float32Array(), 3)
    let heightColors = new THREE.BufferAttribute(new Float32Array(), 3)
    updateAppearance = () => {
      if (!cloudMaterial) return
      cloud.visible = viewMode === "points"
      surface.visible = viewMode === "mesh" && meshAvailable
      cloudMaterial.color.setScalar(brightness)
      cloudMaterial.opacity = opacity
      const transparent = opacity < 1
      if (cloudMaterial.transparent !== transparent) {
        cloudMaterial.transparent = transparent
        cloudMaterial.needsUpdate = true
      }
      cloudMaterial.vertexColors = colorMode !== "solid"
      cloudMaterial.needsUpdate = true
      cloud.geometry.setAttribute("color", colorMode === "height" ? heightColors : rgbColors)
      if (colorMode === "solid") cloudMaterial.color.set("#72c5d5").multiplyScalar(brightness)
    }
    updateAppearance()
    const path = new THREE.Line(new THREE.BufferGeometry(), new THREE.LineBasicMaterial({ color: "#d7bd70" }))
    native.add(cloud, path)
    const resize = new ResizeObserver(() => {
      renderer.setSize(host.clientWidth, host.clientHeight)
      camera.aspect = host.clientWidth / Math.max(1, host.clientHeight)
      camera.updateProjectionMatrix()
    })
    resize.observe(host)
    let alive = true, busy = false, frame = 0
    let meshBusy = false, meshRevision: number | null = null
    async function refreshMesh() {
      if (meshBusy || !device?.connected) return
      meshBusy = true
      try {
        const query = meshRevision === null ? "" : `?since=${meshRevision}`
        const snapshot = await request<ZedMeshSnapshot>(`/api/zed/mesh${query}`, undefined, "GET")
        if (!alive) return
        generatingMesh = snapshot.generating
        meshError = snapshot.error ?? ""
        if (snapshot.generated_at === null) {
          meshRevision = null; meshAvailable = false; surface.visible = false
          surface.geometry.dispose(); surface.geometry = new THREE.BufferGeometry()
          surfaceMaterial.map?.dispose(); surfaceMaterial.map = null
          return
        }
        if (!snapshot.mesh || snapshot.generated_at === meshRevision) return
        const data = snapshot.mesh
        const geometry = new THREE.BufferGeometry()
        geometry.setAttribute("position", new THREE.Float32BufferAttribute(data.vertices.flat(), 3))
        geometry.setIndex(data.triangles.flat())
        const colors = new Float32Array(data.vertices.length * 3)
        const color = new THREE.Color()
        data.vertices.forEach((_, i) => {
          const rgb = data.colors[i] ?? [128, 128, 128]
          color.setRGB(rgb[0] / 255, rgb[1] / 255, rgb[2] / 255, THREE.SRGBColorSpace).toArray(colors, i * 3)
        })
        geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3))
        let texture: THREE.Texture | null = null
        if (data.texture && data.uv.length === data.vertices.length) {
          geometry.setAttribute("uv", new THREE.Float32BufferAttribute(data.uv.flat(), 2))
          try {
            texture = await new THREE.TextureLoader().loadAsync(data.texture)
            texture.colorSpace = THREE.SRGBColorSpace
            texture.anisotropy = Math.min(8, renderer.capabilities.getMaxAnisotropy())
          } catch { if (alive) meshError = "Texture could not be loaded; showing vertex colors." }
        }
        if (!alive) { geometry.dispose(); texture?.dispose(); return }
        surface.geometry.dispose(); surfaceMaterial.map?.dispose()
        surface.geometry = geometry; surfaceMaterial.map = texture
        surfaceMaterial.vertexColors = !texture; surfaceMaterial.needsUpdate = true
        meshRevision = snapshot.generated_at; meshAvailable = true
        meshTextured = Boolean(texture); meshTruncated = data.truncated
        updateAppearance()
      } catch (e) { if (alive) meshError = String(e) }
      finally { meshBusy = false }
    }
    generateMesh = () => {
      if (generatingMesh) return
      generatingMesh = true; meshError = ""
      void request("/api/zed/mesh").then(() => { if (alive) void refreshMesh() })
        .catch(e => { if (alive) { meshError = String(e); generatingMesh = false } })
    }
    void refreshMesh()
    const meshTimer = setInterval(refreshMesh, 3000)
    modelLoaded = false
    modelError = ""
    void new STLLoader().loadAsync(roverModelUrl).then(geometry => {
      if (!alive) { geometry.dispose(); return }
      // CAD export uses millimeters and Y-up; map telemetry uses meters and Z-up.
      // Keep the CAD origin rather than centering the mesh on its bounding box.
      geometry.scale(.001, .001, .001)
      geometry.rotateX(Math.PI / 2)
      body.add(new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color: "#909090", roughness: .65, metalness: .15 })))
      modelLoaded = true
    }).catch(e => { if (alive) modelError = `Cannot load rover model: ${String(e)}` })
    async function refresh() {
      if (busy || !device?.connected) return
      busy = true
      try {
        const data = await request<ZedMap>("/api/zed/map", undefined, "GET")
        if (!alive) return
        sensor.visible = Boolean(data.pose)
        body.visible = Boolean(data.pose && hardware.config?.zed.mount_configured)
        if (data.pose) {
          sensor.position.fromArray(data.pose.translation)
          sensor.quaternion.fromArray(data.pose.orientation).normalize()
          sensor.updateMatrix()
          const config = hardware.config?.zed
          const rotation = config?.mount_rotation_deg.map(THREE.MathUtils.degToRad) ?? [0, 0, 0]
          const mount = new THREE.Matrix4().compose(new THREE.Vector3().fromArray(config?.mount_translation ?? [0, 0, 0]),
            new THREE.Quaternion().setFromEuler(new THREE.Euler(rotation[0], rotation[1], rotation[2], "ZYX")), new THREE.Vector3(1, 1, 1))
          sensor.matrix.clone().multiply(mount.invert()).decompose(body.position, body.quaternion, body.scale)
        }
        cloud.geometry.dispose()
        cloud.geometry = new THREE.BufferGeometry()
        cloud.geometry.setAttribute("position", new THREE.Float32BufferAttribute(data.points.flat(), 3))
        // Camera RGB is sRGB; Three.js vertex colors must be linear to avoid
        // washed-out scans after the renderer's output color conversion.
        const colors = new Float32Array(data.points.length * 3)
        const color = new THREE.Color()
        data.points.forEach((_, i) => {
          const rgb = data.colors[i] ?? [114, 197, 213]
          color.setRGB(rgb[0] / 255, rgb[1] / 255, rgb[2] / 255, THREE.SRGBColorSpace).toArray(colors, i * 3)
        })
        rgbColors = new THREE.BufferAttribute(colors, 3)
        const heights = new Float32Array(data.points.length * 3)
        let minZ = Infinity, maxZ = -Infinity
        for (const p of data.points) { minZ = Math.min(minZ, p[2]); maxZ = Math.max(maxZ, p[2]) }
        data.points.forEach((p, i) => {
          const t = (p[2] - minZ) / Math.max(.001, maxZ - minZ)
          color.setHSL((1 - t) * .7, .85, .55).toArray(heights, i * 3)
        })
        heightColors = new THREE.BufferAttribute(heights, 3)
        updateAppearance()
        path.geometry.dispose()
        path.geometry = new THREE.BufferGeometry().setFromPoints(data.path.map(p => new THREE.Vector3().fromArray(p.translation)))
        error = ""
      } catch (e) { if (alive) error = String(e) }
      finally { busy = false }
    }
    void refresh()
    const timer = setInterval(refresh, 1000)
    function animate() { if (!alive) return; frame = requestAnimationFrame(animate); controls.update(); renderer.render(scene, camera) }
    animate()
    return () => {
      alive = false; clearInterval(timer); clearInterval(meshTimer); cancelAnimationFrame(frame); resize.disconnect(); controls.dispose()
      scene.traverse(o => { if (o instanceof THREE.Mesh || o instanceof THREE.Points || o instanceof THREE.Line) {
        o.geometry.dispose(); const mats = Array.isArray(o.material) ? o.material : [o.material]; mats.forEach(m => m.dispose())
      } })
      pointTexture.dispose(); cloudMaterial = undefined; updateAppearance = () => {}
      surfaceMaterial.map?.dispose(); generateMesh = () => {}
      renderer.dispose(); renderer.domElement.remove()
    }
  })
</script>

<section><header><strong>ZED map</strong><button onclick={() => request("/api/zed/reset").catch(e => error = String(e))}>Reset tracking/map</button></header>
  <DeviceStatus {device} />
  <p>{device?.data.tracking_state ?? "OFF"} · {device?.data.fps ?? 0} FPS · Map {device?.data.mapping_state ?? "OFF"}</p>
  <p>Camera XYZ: {device?.data.pose?.translation.map(v => v.toFixed(3)).join(", ") ?? "No tracked pose"} m · Z-up</p>
  {#if device?.data.imu}<p>IMU acceleration: {device.data.imu.acceleration.map(v => v.toFixed(2)).join(", ")} m/s²</p>{/if}
  <div class="mesh-controls">
    <label>View <select value={viewMode} onchange={e => { viewMode = e.currentTarget.value; updateAppearance() }}><option value="points">Live point cloud</option><option value="mesh">Mesh snapshot</option></select></label>
    <button disabled={!device?.connected || generatingMesh} onclick={() => generateMesh()}>{generatingMesh ? "Generating mesh…" : "Generate mesh snapshot"}</button>
  </div>
  {#if viewMode === "mesh"}
    <small>{meshAvailable ? (meshTextured ? "Camera-textured mesh snapshot." : "Vertex-colored mesh. Enable ZED mapping save_texture and restart the camera for photographic textures.") : "No mesh snapshot yet. Enable ZED mapping with map_type MESH, then generate a snapshot."} Disarm the drive first; generation may pause tracking.</small>
    {#if meshTruncated}<small>Preview limited to 20,000 original faces; part of the scan is omitted.</small>{/if}
  {/if}
  {#if meshError}<p role="alert">{meshError}</p>{/if}
  <details class="cloud-settings">
    <summary>Point cloud appearance</summary>
    <label class="point-size">Point size <input type="range" min=".005" max=".15" step=".005" value={pointSize} oninput={updatePointSize} /> <span>{(pointSize * 100).toFixed(1)} cm</span></label>
    <label class="point-size">Brightness <input type="range" min=".25" max="3" step=".05" value={brightness} oninput={e => { brightness = e.currentTarget.valueAsNumber; updateAppearance() }} /> <span>{brightness.toFixed(2)}×</span></label>
    <label class="point-size">Opacity <input type="range" min=".1" max="1" step=".05" value={opacity} oninput={e => { opacity = e.currentTarget.valueAsNumber; updateAppearance() }} /> <span>{Math.round(opacity * 100)}%</span></label>
    <label class="point-size">Color <select value={colorMode} onchange={e => { colorMode = e.currentTarget.value; updateAppearance() }}><option value="rgb">Camera RGB</option><option value="height">Height</option><option value="solid">Solid cyan</option></select></label>
    {#if colorMode === "height"}<small>Height: violet/blue = low · red = high, relative to the current scan.</small>{/if}
    <button onclick={() => { pointSize = .025; brightness = opacity = 1; colorMode = "rgb"; if (cloudMaterial) cloudMaterial.size = pointSize; updateAppearance() }}>Reset appearance</button>
  </details>
  {#if storageError}<small role="status">{storageError}</small>{/if}
  <div class="viewport" bind:this={host}></div>
  <small>Axes = camera. Rover model follows the base pose only when the camera mount is configured. No wheel-derived pose.</small>
  {#if modelError}<p role="alert">{modelError}</p>{:else if !modelLoaded}<small>Loading rover model…</small>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</section>
<style>
  .point-size { display: flex; align-items: center; gap: .5rem; font-size: .75rem; }
  .point-size input { flex: 1; max-width: 160px; min-width: 60px; accent-color: #72c5d5; }
  .point-size span { font-variant-numeric: tabular-nums; }
  .mesh-controls { display: flex; flex-wrap: wrap; align-items: center; gap: .5rem; font-size: .75rem; }
  button:disabled { opacity: .5; cursor: not-allowed; }
  .cloud-settings { border: 1px solid #35404e; border-radius: .3rem; padding: .3rem .5rem; }
  .cloud-settings summary { cursor: pointer; font-size: .75rem; }
  .cloud-settings label { margin: .35rem 0; }
  select { background: #171b22; color: inherit; border: 1px solid #555; border-radius: .3rem; padding: .2rem; }
section { padding: .6rem; height: 100%; display: flex; flex-direction: column; gap: .3rem; }header { display: flex; justify-content: space-between; gap: .5rem; }p,small { margin: 0; font-size: .75rem; }.viewport { flex: 1; min-height: 100px; overflow: hidden; }button { background: transparent; color: inherit; border: 1px solid #555; border-radius: .3rem; cursor: pointer; }
</style>
