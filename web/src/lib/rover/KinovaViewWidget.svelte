<script lang="ts">
  import { onMount } from "svelte"
  import * as THREE from "three"
  import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js"
  import { TransformControls } from "three/examples/jsm/controls/TransformControls.js"
  import { hardware } from "$lib/hardware.svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { loadArm, clonePreview, setArm } from "./arm-model"
  import DeviceStatus from "./DeviceStatus.svelte"
  let host: HTMLDivElement
  let rotate = $state(false)
  let error = $state("")
  let transform: TransformControls | undefined
  $effect(() => { transform?.setMode(rotate ? "rotate" : "translate") })

  onMount(() => {
    let alive = true, frame = 0
    const renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2)); host.appendChild(renderer.domElement)
    const scene = new THREE.Scene(); scene.background = new THREE.Color("#171b22")
    scene.add(new THREE.HemisphereLight(0xffffff, 0x334455, 3))
    const camera = new THREE.PerspectiveCamera(50, 1, .01, 100); camera.position.set(1.1, .9, 1.1)
    const orbit = new OrbitControls(camera, renderer.domElement); orbit.target.set(0, .25, 0)
    scene.add(new THREE.GridHelper(2, 20))
    const native = new THREE.Group(); native.rotation.x = -Math.PI / 2; scene.add(native)
    const target = new THREE.Group(); target.add(new THREE.AxesHelper(.15)); native.add(target)
    transform = new TransformControls(camera, renderer.domElement); transform.attach(target); scene.add(transform.getHelper())
    transform.addEventListener("dragging-changed", e => orbit.enabled = !e.value)
    let lastTarget = ""
    transform.addEventListener("objectChange", () => {
      // Preserve the existing Kinova viewer's tool-angle convention.
      const r = new THREE.Euler().setFromQuaternion(target.quaternion, "XYZ")
      hardware.target = [...target.position.toArray(), ...[r.x, r.y, r.z].map(THREE.MathUtils.radToDeg)]
      hardware.preview = null; lastTarget = JSON.stringify(hardware.target)
    })
    const observer = new ResizeObserver(() => {
      renderer.setSize(host.clientWidth, host.clientHeight); camera.aspect = host.clientWidth / Math.max(1, host.clientHeight); camera.updateProjectionMatrix()
    }); observer.observe(host)
    void loadArm().then(model => {
      const preview = clonePreview(model)
      native.add(model.root, preview.root)
      if (!alive) { dispose(); return }
      function animate() {
        if (!alive) return
        frame = requestAnimationFrame(animate)
        const data = rover.status?.devices.kinova.data
        if (data) setArm(model, data.joints, data.gripper)
        preview.root.visible = Boolean(hardware.preview)
        if (hardware.preview) setArm(preview, hardware.preview.joints, data?.gripper ?? null)
        const signature = JSON.stringify(hardware.target)
        if (signature !== lastTarget) {
          target.position.fromArray(hardware.target)
          target.rotation.set(...hardware.target.slice(3).map(THREE.MathUtils.degToRad) as [number, number, number], "XYZ")
          hardware.preview = null; lastTarget = signature
        }
        orbit.update(); renderer.render(scene, camera)
      }
      animate()
    }).catch(e => { if (alive) error = String(e) })
    function dispose() {
      const geometries = new Set<THREE.BufferGeometry>(), materials = new Set<THREE.Material>()
      native.traverse(o => { if (o instanceof THREE.Mesh || o instanceof THREE.Line) { geometries.add(o.geometry); (Array.isArray(o.material) ? o.material : [o.material]).forEach(m => materials.add(m)) } })
      geometries.forEach(g => g.dispose()); materials.forEach(m => m.dispose())
    }
    return () => { alive = false; cancelAnimationFrame(frame); observer.disconnect(); orbit.dispose(); transform?.dispose(); transform = undefined; dispose(); renderer.dispose(); renderer.domElement.remove() }
  })
</script>
<section><header><strong>Kinova 3D / IK target</strong><label><input type="checkbox" bind:checked={rotate} /> Rotate target</label></header>
  <DeviceStatus device={rover.status?.devices.kinova} />
  <div class="viewport" bind:this={host}></div>
  <small>Drag edits target only. Gold ghost = robot IK preview. Use Solve then Execute in Arm Controls. No collision checking.</small>
  {#if error}<p role="alert">{error}</p>{/if}
</section>
<style>section { height: 100%; padding: .6rem; display: flex; flex-direction: column; gap: .4rem; }header { display: flex; justify-content: space-between; gap: .5rem; }.viewport { flex: 1; min-height: 100px; overflow: hidden; }label, small { font-size: .75rem; }</style>
