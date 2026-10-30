<script lang="ts">
  import { onMount } from "svelte"
  import * as THREE from "three"
  import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js"
  import { request } from "$lib/config-api"
  import { hardware } from "$lib/hardware.svelte"
  import { rover } from "$lib/rover-control.svelte"
  import type { ZedMap } from "$lib/hardware-types"
  import DeviceStatus from "./DeviceStatus.svelte"
  let host: HTMLDivElement
  let error = $state("")
  const device = $derived(rover.status?.devices.zed)

  onMount(() => {
    const renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2))
    host.appendChild(renderer.domElement)
    const scene = new THREE.Scene()
    scene.background = new THREE.Color("#171b22")
    const camera = new THREE.PerspectiveCamera(55, 1, .01, 1000)
    camera.position.set(3, 3, 3)
    const controls = new OrbitControls(camera, renderer.domElement)
    scene.add(new THREE.GridHelper(20, 40))
    // Proper rotation: native (x,y,z) -> Three.js (x,z,-y), determinant +1.
    const native = new THREE.Group()
    native.rotation.x = -Math.PI / 2
    scene.add(native)
    const sensor = new THREE.Group()
    sensor.add(new THREE.AxesHelper(.4))
    const body = new THREE.Mesh(new THREE.BoxGeometry(.8, .5, .2), new THREE.MeshBasicMaterial({ color: "#72c5d5", wireframe: true }))
    native.add(sensor, body)
    sensor.visible = body.visible = false
    const cloud = new THREE.Points(new THREE.BufferGeometry(), new THREE.PointsMaterial({ size: .025, vertexColors: true }))
    const path = new THREE.Line(new THREE.BufferGeometry(), new THREE.LineBasicMaterial({ color: "#d7bd70" }))
    native.add(cloud, path)
    const resize = new ResizeObserver(() => {
      renderer.setSize(host.clientWidth, host.clientHeight)
      camera.aspect = host.clientWidth / Math.max(1, host.clientHeight)
      camera.updateProjectionMatrix()
    })
    resize.observe(host)
    let alive = true, busy = false, frame = 0
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
        cloud.geometry.setAttribute("color", new THREE.Float32BufferAttribute(data.colors.flat().map(v => v / 255), 3))
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
      alive = false; clearInterval(timer); cancelAnimationFrame(frame); resize.disconnect(); controls.dispose()
      scene.traverse(o => { if (o instanceof THREE.Mesh || o instanceof THREE.Points || o instanceof THREE.Line) {
        o.geometry.dispose(); const mats = Array.isArray(o.material) ? o.material : [o.material]; mats.forEach(m => m.dispose())
      } })
      renderer.dispose(); renderer.domElement.remove()
    }
  })
</script>

<section><header><strong>ZED map</strong><button onclick={() => request("/api/zed/reset").catch(e => error = String(e))}>Reset tracking/map</button></header>
  <DeviceStatus {device} />
  <p>{device?.data.tracking_state ?? "OFF"} · {device?.data.fps ?? 0} FPS · Map {device?.data.mapping_state ?? "OFF"}</p>
  <p>Camera XYZ: {device?.data.pose?.translation.map(v => v.toFixed(3)).join(", ") ?? "No tracked pose"} m · Z-up</p>
  {#if device?.data.imu}<p>IMU acceleration: {device.data.imu.acceleration.map(v => v.toFixed(2)).join(", ")} m/s²</p>{/if}
  <div class="viewport" bind:this={host}></div>
  <small>Axes = camera. Box = base marker only when mount is configured, not chassis geometry. No wheel-derived pose.</small>
  {#if error}<p role="alert">{error}</p>{/if}
</section>
<style>section { padding: .6rem; height: 100%; display: flex; flex-direction: column; gap: .3rem; }header { display: flex; justify-content: space-between; gap: .5rem; }p,small { margin: 0; font-size: .75rem; }.viewport { flex: 1; min-height: 100px; overflow: hidden; }button { background: transparent; color: inherit; border: 1px solid #555; border-radius: .3rem; cursor: pointer; }</style>
