<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import { hardware, armAction, releaseArm, solveTarget } from "$lib/hardware.svelte"
  import { request } from "$lib/config-api"
  import DeviceStatus from "./DeviceStatus.svelte"
  const device = $derived(rover.status?.devices.kinova)
  const labels = ["X (m)", "Y (m)", "Z (m)", "Roll (°)", "Pitch (°)", "Yaw (°)"]
  let moving = $state(false)
  let moveMessage = $state("")
  function jog(e: PointerEvent, axis: number, direction: number, joints = false) {
    e.currentTarget instanceof HTMLElement && e.currentTarget.setPointerCapture(e.pointerId)
    const v = Array(joints ? device?.data.joints.length ?? 6 : 6).fill(0); v[axis] = direction
    hardware.preview = null; hardware.jog = joints ? { joints: v } : { cartesian: v }
  }
  async function enable() { releaseArm(); await armAction({ action: "enable" }); await rover.refreshStatus() }
  function useMeasured() { if (device?.data.pose) { hardware.target = [...device.data.pose]; hardware.preview = null } }
  async function execute() {
    if (!hardware.preview || !confirm("Execute the previewed arm move? IK does not check collisions.")) return
    const id = hardware.preview.id; hardware.preview = null
    await armAction({ action: "execute", preview_id: id })
  }
  async function moveCartesian() {
    if (!device?.data.armed || !device.connected || moving) return
    if (!confirm("Move using Kortex Cartesian ReachPose? This is limited to 20 mm and 5° from the measured pose and does not check collisions.")) return
    moving = true; moveMessage = "Moving with Kortex ReachPose…"; hardware.preview = null; hardware.error = ""
    try {
      await request("/api/arm/action", { action: "move_cartesian", pose: [...hardware.target] })
      moveMessage = "Cartesian move completed"
    } catch (e) {
      hardware.error = String(e); moveMessage = "Cartesian move failed or stopped"
    } finally {
      moving = false
      await rover.refreshStatus()
    }
  }
</script>
<section>
  <h3>Arm controls</h3><DeviceStatus {device} />
  <div class="row"><button onclick={enable}>Enable arm</button><button onclick={() => { releaseArm(); void request("/api/arm/stop").catch(e => hardware.error = String(e)) }}>Stop arm</button>
    <button onclick={() => armAction({ action: "clear_faults" })}>Clear faults</button>
    <button disabled={!device?.data.armed} onclick={() => { if (confirm("Move to built-in Home? Check the full path is clear.")) void armAction({ action: "home" }) }}>Home</button></div>
  <p>{device?.data.armed ? "Arm enabled" : "Arm stopped"} · State {device?.data.arm_state ?? "—"} · Gripper {device?.data.gripper?.toFixed(2) ?? "—"}</p>
  <h4>Hold to jog — base coordinates</h4>
  <div class="jogs">{#each labels as label, i}<div><span>{label.split(" ")[0]}</span>
    {#each [-1, 1] as direction}<button disabled={!device?.data.armed} onpointerdown={e => jog(e, i, direction)} onpointerup={releaseArm} onpointercancel={releaseArm} onlostpointercapture={releaseArm}>{direction < 0 ? "−" : "+"}</button>{/each}</div>{/each}</div>
  <h4>Joint jogs (degrees/s)</h4>
  <div class="jogs">{#each device?.data.joints ?? [] as q, i}<div><span>J{i + 1}: {q.toFixed(1)}°</span>
    {#each [-1, 1] as direction}<button disabled={!device?.data.armed} onpointerdown={e => jog(e, i, direction, true)} onpointerup={releaseArm} onpointercancel={releaseArm} onlostpointercapture={releaseArm}>{direction < 0 ? "−" : "+"}</button>{/each}</div>{/each}</div>
  <div class="row"><button disabled={!device?.data.armed} onclick={() => armAction({ action: "gripper", position: 0 })}>Open gripper</button><button disabled={!device?.data.armed} onclick={() => armAction({ action: "gripper", position: 1 })}>Close gripper</button></div>
  <h4>IK target</h4><button onclick={useMeasured}>Use measured pose</button>
  <div class="target">{#each labels as label, i}<label>{label}<input type="number" step={i < 3 ? .01 : 1} value={hardware.target[i]} oninput={e => { hardware.target[i] = e.currentTarget.valueAsNumber; hardware.preview = null }} /></label>{/each}</div>
  <div class="row"><button onclick={solveTarget}>Solve / preview</button><button disabled={!hardware.preview || !device?.data.armed} onclick={execute}>Execute preview</button><button disabled={!device?.connected || !device?.data.armed || moving} onclick={moveCartesian}>{moving ? "Moving…" : "Move via Kortex"}</button></div>
  {#if moveMessage}<p>{moveMessage}</p>{/if}
  {#if hardware.preview}<p>IK joints: {hardware.preview.joints.map(v => v.toFixed(1)).join(", ")}° · expires after 15 s</p>{/if}
  {#if hardware.error}<p role="alert">{hardware.error}</p>{/if}
</section>
<style>section { height: 100%; overflow: auto; padding: .8rem; }h3 { margin: 0 0 .4rem; }h4 { margin: .8rem 0 .4rem; }p { font-size: .8rem; }.row,.jogs { display: flex; gap: .4rem; flex-wrap: wrap; margin: .5rem 0; }.jogs>div { display: flex; align-items: center; gap: .3rem; }button { padding: .4rem; background: transparent; border: 1px solid #777; border-radius: .3rem; color: inherit; cursor: pointer; touch-action: none; }button:disabled { opacity: .4; cursor: not-allowed; }.target { display: grid; grid-template-columns: repeat(3, 1fr); gap: .5rem; margin: .5rem 0; }label { font-size: .8rem; }input { width: 100%; box-sizing: border-box; padding: .4rem; background: #222; color: inherit; border: 1px solid #555; }[role="alert"] { color: #f99; }</style>
