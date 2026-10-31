<script lang="ts">
  import { onMount, onDestroy } from "svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { hardware, armAction, releaseArm, solveTarget } from "$lib/hardware.svelte"
  import { request } from "$lib/config-api"
  import DeviceStatus from "./DeviceStatus.svelte"

  const device = $derived(rover.status?.devices.kinova)
  const axes = ["X", "Y", "Z", "Roll", "Pitch", "Yaw"]
  let tab = $state<"manual" | "ik">("manual")
  let jogMode = $state<"cartesian" | "joints">("cartesian")
  let moving = $state(false)
  let solving = $state(false)
  let enabling = $state(false)
  let moveMessage = $state("")
  let now = $state(Date.now() / 1000)
  let expiredTarget = $state("")
  let invalidInputs = $state<number[]>([])
  let confirmationElement = $state<HTMLDivElement>()
  let pendingConfirmation = $state<"execute" | "move" | "home" | "install-zone" | null>(null)
  let confirmedTarget = $state("")
  let confirmedPreview = $state<number | null>(null)
  const targetSignature = $derived(JSON.stringify(hardware.target))
  const validTarget = $derived(invalidInputs.length === 0 && hardware.target.length === 6 && hardware.target.every(Number.isFinite))
  const ready = $derived(Boolean(device?.connected && device.data.armed))
  const canMove = $derived(ready && !moving && !solving && !pendingConfirmation)
  const previewReady = $derived(Boolean(hardware.preview && hardware.preview.expires_at > now && JSON.stringify(hardware.preview.target) === targetSignature))
  const remaining = $derived(Math.max(0, Math.ceil((hardware.preview?.expires_at ?? now) - now)))
  const blockedReason = $derived(!device?.connected ? "Connect the arm to use motion controls." : !device.data.armed ? "Enable the arm to use motion controls." : moving ? "A Cartesian move is in progress." : "")
  const confirmationValid = $derived(pendingConfirmation === "install-zone"
    ? Boolean(device?.connected && !device.data.armed)
    : ready && !moving && (pendingConfirmation === "home" || (validTarget && confirmedTarget === targetSignature && (pendingConfirmation !== "execute" || (previewReady && hardware.preview?.id === confirmedPreview)))))

  onMount(() => {
    const timer = setInterval(() => now = Date.now() / 1000, 250)
    return () => clearInterval(timer)
  })
  onDestroy(releaseArm)
  $effect(() => {
    if (pendingConfirmation && confirmationElement) {
      confirmationElement.scrollIntoView({ block: "nearest" })
      confirmationElement.focus({ preventScroll: true })
    }
  })
  $effect(() => {
    if (hardware.preview && hardware.preview.expires_at <= now) {
      expiredTarget = JSON.stringify(hardware.preview.target)
      hardware.preview = null
    }
  })

  function changeTab(value: "manual" | "ik") { releaseArm(); tab = value }
  function changeJogMode(value: "cartesian" | "joints") { releaseArm(); jogMode = value }
  function jog(e: PointerEvent, axis: number, direction: number) {
    if (!canMove || e.button !== 0) return
    e.currentTarget instanceof HTMLElement && e.currentTarget.setPointerCapture(e.pointerId)
    const v = Array(jogMode === "joints" ? device?.data.joints.length ?? 6 : 6).fill(0)
    v[axis] = direction
    hardware.preview = null
    hardware.jog = jogMode === "joints" ? { joints: v } : { cartesian: v }
  }
  async function enable() {
    releaseArm(); enabling = true
    try { await armAction({ action: "enable" }); await rover.refreshStatus() }
    finally { enabling = false }
  }
  async function stop() {
    releaseArm(); pendingConfirmation = null; hardware.preview = null
    try { await request("/api/arm/stop"); await rover.refreshStatus() }
    catch (e) { hardware.error = String(e) }
  }
  function useMeasured() {
    if (device?.data.pose) { hardware.target = [...device.data.pose]; hardware.preview = null; expiredTarget = ""; invalidInputs = [] }
  }
  function editTarget(i: number, value: number) {
    invalidInputs = invalidInputs.filter(axis => axis !== i)
    if (Number.isFinite(value)) hardware.target[i] = value
    else invalidInputs = [...invalidInputs, i]
    hardware.preview = null; expiredTarget = ""
  }
  async function solve() {
    if (!validTarget || solving) return
    releaseArm(); solving = true; expiredTarget = ""
    try { await solveTarget() }
    finally { solving = false }
  }
  function confirm(action: typeof pendingConfirmation) {
    releaseArm(); confirmedTarget = targetSignature; confirmedPreview = hardware.preview?.id ?? null
    pendingConfirmation = action
  }
  async function installRearZone() {
    if (!device?.connected || device.data.armed || !device.data.rear_boundary) return
    hardware.error = ""
    try { await request("/api/arm/action", { action: "install_rear_zone" }); await rover.refreshStatus() }
    catch (e) { hardware.error = String(e); await rover.refreshStatus() }
  }
  async function confirmPendingAction() {
    if (!confirmationValid) return
    const action = pendingConfirmation
    pendingConfirmation = null
    if (action === "execute" && hardware.preview) {
      const id = hardware.preview.id; hardware.preview = null
      await armAction({ action: "execute", preview_id: id })
    } else if (action === "move") await moveCartesian()
    else if (action === "home") await armAction({ action: "home" })
    else if (action === "install-zone") await installRearZone()
  }
  async function moveCartesian() {
    if (!ready || !validTarget || moving) return
    moving = true; moveMessage = "Moving with Kortex ReachPose…"; hardware.preview = null; hardware.error = ""
    try {
      await request("/api/arm/action", { action: "move_cartesian", pose: [...hardware.target] })
      moveMessage = "Cartesian move completed"
    } catch (e) { hardware.error = String(e); moveMessage = "Cartesian move failed or stopped" }
    finally { moving = false; await rover.refreshStatus() }
  }
</script>

<section aria-label="Kinova arm controls">
  <header>
    <div class="heading"><h3>Kinova</h3><span class="state" class:enabled={ready}>{device?.data.armed ? "Enabled" : "Stopped"}</span></div>
    <DeviceStatus {device} />
    <div class="header-actions">
      <button disabled={!device?.connected || !device?.data.rear_boundary?.controller_zone_installed || device?.data.armed || enabling} onclick={enable}>{enabling ? "Enabling…" : "Enable arm"}</button>
      <button class="stop" onclick={stop}>Stop arm</button>
    </div>
  </header>

  <div class="content">
    {#if device?.data.rear_boundary && !device.data.rear_boundary.controller_zone_installed}
      <div class="notice warning"><strong>Safety setup required</strong><p>Install the rear controller zone before enabling motion. Arm power must be off; keep the controller connection powered.</p><button disabled={!device.connected || device.data.armed} onclick={() => confirm("install-zone")}>Install controller zone</button></div>
    {/if}
    {#if device?.data.rear_boundary_error}<p class="notice error" role="alert">Rear boundary: {device.data.rear_boundary_error}</p>{/if}
    {#if device?.data.telemetry_error}<p class="notice error" role="alert">Arm telemetry: {device.data.telemetry_error}</p>{/if}
    {#if hardware.error}<p class="notice error" role="alert">{hardware.error}</p>{/if}

    <div class="segmented" role="group" aria-label="Control view">
      <button class:active={tab === "manual"} aria-pressed={tab === "manual"} onclick={() => changeTab("manual")}>Manual</button>
      <button class:active={tab === "ik"} aria-pressed={tab === "ik"} onclick={() => changeTab("ik")}>IK target</button>
    </div>
    {#if blockedReason}<p class="hint">{blockedReason}</p>{/if}

    {#if tab === "manual"}
      <div class="section-heading"><h4>Hold to jog</h4><span class="hint">Release to stop</span></div>
      <div class="segmented secondary" role="group" aria-label="Jog coordinates">
        <button class:active={jogMode === "cartesian"} aria-pressed={jogMode === "cartesian"} onclick={() => changeJogMode("cartesian")}>Cartesian · base frame</button>
        <button class:active={jogMode === "joints"} aria-pressed={jogMode === "joints"} onclick={() => changeJogMode("joints")}>Joints</button>
      </div>
      <div class="jog-table">
        <div class="table-heading"><span>Axis</span><span>Measured</span><span>Jog</span></div>
        {#each jogMode === "cartesian" ? axes : (device?.data.joints ?? []).map((_, i) => `J${i + 1}`) as label, i}
          <div class="jog-row" class:group-start={jogMode === "cartesian" && i === 3}>
            <strong>{label}</strong>
            <span class="measurement">{jogMode === "cartesian" ? device?.data.pose?.[i]?.toFixed(i < 3 ? 3 : 1) ?? "—" : device?.data.joints[i]?.toFixed(1) ?? "—"}<small>{jogMode === "cartesian" && i < 3 ? "m" : "°"}</small></span>
            <div class="jog-buttons">{#each [-1, 1] as direction}<button class="jog" disabled={!canMove} aria-label={`Jog ${label} ${direction < 0 ? "negative" : "positive"}`} onpointerdown={e => jog(e, i, direction)} onpointerup={releaseArm} onpointercancel={releaseArm} onlostpointercapture={releaseArm}>{direction < 0 ? "−" : "+"}</button>{/each}</div>
          </div>
        {/each}
      </div>
      <div class="card gripper"><div><h4>Gripper</h4><span class="hint">{device?.data.gripper == null ? "No measurement" : `${Math.round(device.data.gripper * 100)}% closed`}</span></div><div class="actions"><button disabled={!canMove} onclick={() => armAction({ action: "gripper", position: 0 })}>Open</button><button disabled={!canMove} onclick={() => armAction({ action: "gripper", position: 1 })}>Close</button></div></div>
    {:else}
      <div class="section-heading"><h4>1. Set target</h4><button class="compact" disabled={!device?.data.pose || moving || solving} onclick={useMeasured}>Use current pose</button></div>
      <p class="hint">Edit in base coordinates or drag the target in the 3D view.</p>
      {#each [0, 3] as start}
        <fieldset disabled={moving || solving}><legend>{start === 0 ? "Position · metres" : "Orientation · degrees"}</legend><div class="target-grid">{#each axes.slice(start, start + 3) as label, offset}<label>{label}<input type="number" step={start === 0 ? .01 : 1} aria-invalid={invalidInputs.includes(start + offset)} value={invalidInputs.includes(start + offset) ? "" : hardware.target[start + offset]} oninput={e => editTarget(start + offset, e.currentTarget.valueAsNumber)} /></label>{/each}</div></fieldset>
      {/each}
      {#if !validTarget}<p class="error" role="alert">Enter a valid number for all six target values.</p>{/if}
      <div class="card preview-card">
        <h4>2. Preview & execute</h4>
        <p class="preview-status" role="status">{solving ? "Solving IK…" : previewReady ? `Preview ready · expires in ${remaining}s` : expiredTarget === targetSignature ? "Preview expired. Solve again before executing." : "Solve to preview the target before moving."}</p>
        <div class="actions"><button disabled={!device?.connected || !validTarget || solving || moving || Boolean(pendingConfirmation)} onclick={solve}>{solving ? "Solving…" : "Solve / preview"}</button><button class="primary" disabled={!canMove || !previewReady} onclick={() => confirm("execute")}>Execute preview</button></div>
        <p class="hint">IK does not check collisions. Inspect the gold preview in the 3D view and check the path is clear.</p>
        {#if previewReady && hardware.preview}<details><summary>Preview joint angles</summary><p class="joint-values">{hardware.preview.joints.map((v, i) => `J${i + 1}: ${v.toFixed(1)}°`).join(" · ")}</p></details>{/if}
      </div>
      <details class="card"><summary>Advanced · direct Cartesian move</summary><p class="hint">Uses Kortex ReachPose instead of the IK preview. Limited to 20 mm and 5° from the measured pose; no collision checking.</p><button disabled={!canMove || !validTarget} onclick={() => confirm("move")}>{moving ? "Moving…" : "Move via Kortex"}</button></details>
    {/if}

    {#if pendingConfirmation}
      <div class="notice warning confirmation" role="group" aria-label="Confirm arm motion" tabindex="-1" bind:this={confirmationElement}>
        <strong>{pendingConfirmation === "install-zone" ? "Confirm safety setup" : "Confirm arm motion"}</strong>
        <p>{pendingConfirmation === "execute" ? "Execute the IK preview? IK does not check collisions. Check the full path is clear." : pendingConfirmation === "move" ? "Move using Kortex Cartesian ReachPose? Limited to 20 mm and 5° from measured pose; no collision checking." : pendingConfirmation === "home" ? "Move to built-in Home? Check the full path is clear." : "Install the persistent Kortex rear safety zone? Arm power must be off. Leave the USB/controller connection powered; this action does not move the arm."}</p>
        {#if pendingConfirmation !== "install-zone"}<p class="hint">The arm heartbeat remains active while you decide.</p>{/if}
        {#if !confirmationValid}<p role="status">Target, preview, or arm state changed. Cancel and review before continuing.</p>{/if}
        <div class="actions"><button onclick={() => pendingConfirmation = null}>Cancel</button><button class="primary" disabled={!confirmationValid} onclick={confirmPendingAction}>{pendingConfirmation === "install-zone" ? "Install zone" : "Confirm motion"}</button></div>
      </div>
    {/if}
    {#if moveMessage}<p class="notice" role="status">{moveMessage}</p>{/if}

    <div class="utilities"><button disabled={!canMove} onclick={() => confirm("home")}>Home arm</button><span class="hint">{device?.data.rear_boundary?.controller_zone_installed ? "Rear controller zone installed" : "Rear zone not verified"}</span></div>
    <details class="diagnostics"><summary>Safety & diagnostics</summary><p>Rear no-go plane: 8 cm behind the base, aligned to the base X axis.</p><p>Arm state: {device?.data.arm_state ?? "—"} · Controller zone: {device?.data.rear_boundary?.controller_zone_installed ? "Installed" : "Not verified"}</p><button disabled={!device?.connected || moving || Boolean(pendingConfirmation)} onclick={() => armAction({ action: "clear_faults" })}>Clear faults</button></details>
  </div>
</section>

<style>
  section { height: 100%; min-height: 0; display: flex; flex-direction: column; color: var(--text); font-size: .85rem; }
  header { flex: none; padding: .85rem; border-bottom: 1px solid var(--border); background: var(--bgMedium); }
  .heading, .section-heading, .gripper, .utilities { display: flex; align-items: center; justify-content: space-between; gap: .6rem; flex-wrap: wrap; }
  .heading { margin-bottom: .25rem; }
  h3 { margin: 0; font-size: 1rem; } h4 { margin: 0; font-size: .85rem; }
  .state { font-size: .7rem; border: 1px solid var(--borderStrong); border-radius: 1rem; padding: .15rem .5rem; color: var(--textMuted); }
  .state.enabled { color: var(--success); border-color: var(--success); }
  .header-actions { display: grid; grid-template-columns: 1fr 1fr; gap: .5rem; margin-top: .65rem; }
  .content { flex: 1; min-height: 0; overflow: auto; padding: .85rem; }
  button { min-height: 2.5rem; padding: .45rem .7rem; border: 1px solid var(--borderStrong); border-radius: .4rem; background: var(--surfaceRaised); color: inherit; font: inherit; font-weight: 600; cursor: pointer; }
  button:hover:not(:disabled) { border-color: var(--primary); background: var(--bgLight); }
  button:disabled { opacity: .4; cursor: not-allowed; }
  button.stop { background: #9f302d; border-color: #b84d45; color: white; text-transform: uppercase; letter-spacing: .04em; }
  button.stop:hover { background: #b43a35; border-color: #d06a62; }
  button.primary { background: var(--primary); color: var(--bgDark); border-color: var(--primary); }
  button.primary:hover:not(:disabled) { background: #edbd62; }
  .segmented { display: flex; gap: .25rem; padding: .25rem; border: 1px solid var(--border); border-radius: .5rem; background: var(--bgDark); margin-bottom: .8rem; }
  .segmented button { flex: 1; border-color: transparent; background: transparent; padding-inline: .4rem; }
  .segmented button.active { background: var(--surfaceRaised); border-color: var(--borderStrong); color: var(--primary); }
  .secondary { margin-top: .6rem; } .secondary button { font-size: .75rem; }
  .hint { color: var(--textMuted); font-size: .75rem; line-height: 1.5; }
  p { line-height: 1.5; margin: .5rem 0; overflow-wrap: anywhere; }
  .table-heading, .jog-row { display: grid; grid-template-columns: minmax(2.5rem, 1fr) minmax(4rem, 1fr) auto; align-items: center; gap: .5rem; }
  .table-heading { color: var(--textMuted); font-size: .7rem; padding: 0 .25rem .35rem; }
  .table-heading span:last-child { width: 6rem; text-align: center; }
  .jog-row { padding: .35rem .25rem; border-top: 1px solid var(--border); }
  .jog-row.group-start { margin-top: .5rem; }
  .measurement { font-variant-numeric: tabular-nums; white-space: nowrap; }
  small { color: var(--textMuted); margin-left: .3rem; }
  .jog-buttons { display: flex; gap: .35rem; }
  button.jog { width: 2.8rem; min-height: 2.8rem; font-size: 1.2rem; touch-action: none; user-select: none; }
  button.jog:active:not(:disabled) { background: var(--primary); color: var(--bgDark); }
  .card { padding: .75rem; border: 1px solid var(--border); border-radius: .5rem; margin-top: .8rem; background: var(--bgMedium); }
  .actions { display: flex; gap: .5rem; flex-wrap: wrap; }
  .actions button { flex: 1; }
  button.compact { min-height: 2rem; font-size: .75rem; padding: .3rem .5rem; }
  fieldset { padding: 0; margin: .9rem 0; border: 0; min-width: 0; }
  legend { color: var(--textMuted); font-size: .75rem; padding: 0; margin-bottom: .4rem; }
  .target-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: .5rem; }
  label { display: grid; gap: .3rem; font-size: .75rem; }
  input { width: 100%; min-width: 0; min-height: 2.5rem; padding: .4rem; border: 1px solid var(--borderStrong); border-radius: .35rem; background: var(--bgDark); color: inherit; font: inherit; font-variant-numeric: tabular-nums; }
  .preview-status { color: var(--primary); }
  .joint-values { font-variant-numeric: tabular-nums; font-size: .75rem; }
  .notice { padding: .75rem; margin: 0 0 .8rem; border: 1px solid var(--borderStrong); border-radius: .5rem; background: var(--surfaceRaised); }
  .warning { border-color: var(--warning); } .error { color: #ff9a92; }
  .confirmation { margin-top: .8rem; }
  .utilities { border-top: 1px solid var(--border); padding-top: .8rem; margin-top: .8rem; }
  details { font-size: .75rem; } summary { cursor: pointer; color: var(--textMuted); padding: .25rem 0; }
  .diagnostics { margin-top: .6rem; }
</style>
