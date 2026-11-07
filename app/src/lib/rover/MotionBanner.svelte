<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import { hardware, armAction } from "$lib/hardware.svelte"

  const drive = $derived(rover.status?.devices.drive)
  const kinova = $derived(rover.status?.devices.kinova)
  const driveStop = $derived(
    drive?.enabled && !drive.data.armed ? (drive.stop_reason?.reason ?? "") : "",
  )
  const armStop = $derived(
    kinova?.enabled && !kinova.data.armed ? (kinova.stop_reason?.reason ?? "") : "",
  )
  const armError = $derived(
    hardware.error && kinova?.enabled && !kinova.data.armed ? hardware.error : "",
  )
  const uiError = $derived(rover.lastError && !driveStop && !armStop ? rover.lastError : "")

  async function enableArm() {
    await armAction({ action: "enable" })
    await rover.refreshStatus()
  }
</script>

{#if driveStop || armStop || armError || uiError}
  <section class="motion-banner" role="status" aria-live="polite">
    {#if driveStop}
      <div class="row">
        <span><strong>Drive stopped:</strong> {driveStop}</span>
        <button onclick={() => rover.resetRover()}>Enable Drive</button>
      </div>
    {/if}
    {#if armStop}
      <div class="row">
        <span><strong>Arm stopped:</strong> {armStop}</span>
        <button onclick={enableArm}>Enable arm</button>
      </div>
    {/if}
    {#if armError}
      <div class="row"><span>{armError}</span></div>
    {/if}
    {#if uiError}
      <div class="row"><span>{uiError}</span></div>
    {/if}
  </section>
{/if}

<style>
  .motion-banner {
    flex: none;
    display: grid;
    gap: 0.35rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid color-mix(in srgb, var(--warning) 55%, transparent);
    background: color-mix(in srgb, var(--warning) 12%, var(--bgMedium));
    color: #f0d9a8;
    font-size: 0.8rem;
    z-index: 20;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }
  .row span {
    overflow-wrap: anywhere;
  }
  button {
    flex: none;
    min-height: 2rem;
    padding: 0.3rem 0.65rem;
    border: 1px solid color-mix(in srgb, var(--warning) 70%, transparent);
    border-radius: 0.35rem;
    background: var(--surfaceRaised);
    color: var(--text);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  button:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
</style>
