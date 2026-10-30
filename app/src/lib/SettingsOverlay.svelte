<script lang="ts">
  import Modal from "$lib/ui/Modal.svelte"
  import Button from "$lib/ui/Button.svelte"
  import ConfigFields from "$lib/ConfigFields.svelte"
  import { getConfig, updateConfig, type FullConfig } from "$lib/config-api"
  import { hardware, releaseArm } from "$lib/hardware.svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { releaseGamepad } from "$lib/gamepad-control.svelte"

  let { show = false, onclose }: { show?: boolean; onclose?: () => void } = $props()
  let config: FullConfig | null = $state(null)
  let active: keyof FullConfig = $state("drive")
  let error = $state("")
  let saving = $state(false)
  const tabs: [keyof FullConfig, string][] = [["drive", "Motors"], ["kinova", "Kinova"], ["rtk", "RTK / NTRIP"], ["zed", "ZED"], ["gamepad", "Gamepad"]]
  $effect(() => {
    if (!show) return
    let active = true
    config = null; error = ""
    void getConfig().then(value => { if (active) config = value }).catch(e => { if (active) error = String(e) })
    return () => { active = false }
  })
  async function save() {
    if (!config) return
    saving = true
    error = ""
    try {
      releaseArm()
      rover.releaseJoystick()
      releaseGamepad()
      await updateConfig(config)
      hardware.config = config
      onclose?.()
    } catch (e) { error = String(e) }
    finally { saving = false }
  }
</script>

<Modal {show} title="Hardware Settings" width="min(900px, calc(100vw - 32px))" {onclose}>
  {#snippet children()}
    <div class="settings-content">
      <nav>{#each tabs as [key, label]}<button class:active={active === key} onclick={() => active = key}>{label}</button>{/each}</nav>
      <p>Devices are independent. Connection changes stop and reconnect only the affected device. No motion resumes automatically.</p>
      {#if error}<p role="alert">{error}</p>{/if}
      {#if config}
        {#if active === "drive"}
          <p>Ports: 0 = A, 1 = B. Motor order: left front, right front, left back, right back. Channels: 1/2; directions: ±1. Power cap: 1–1000, not wheel speed. Use persistent /dev/serial/by-id paths.</p>
        {:else if active === "kinova"}
          <p>Linear speed: m/s. Joint/angular speed: deg/s. Targets use robot-base XYZ meters and roll/pitch/yaw degrees. Blank password retains the stored value.</p>
        {:else if active === "rtk"}
          <p>NTRIP corrections go only to the receiver, not ZED. RTK2go and the Agora mount point are editable. HDOP is not accuracy in meters.</p>
        {:else if active === "zed"}
          <p>Native right-handed Z-up meters. Mount translation/rotation describe camera in rover base (meters / RPY degrees). Check Mount configured only after measuring it; otherwise no rover-base marker is shown. No RTK fusion.</p>
        {:else}
          <p>Standard Gamepad API indices. Y/△ toggles target, LB/L1 holds to jog, B/○ stops both by default. Axes and button bindings are configurable below.</p>
        {/if}
        <ConfigFields value={config[active]} />
        <footer><Button onclick={() => onclose?.()}>Cancel</Button><Button onclick={save} disabled={saving}>{saving ? "Saving…" : "Save"}</Button></footer>
      {:else}<p>Loading configuration…</p>{/if}
    </div>
  {/snippet}
</Modal>

<style>
  .settings-content { display: flex; flex-direction: column; gap: .75rem; width: 100%; min-width: 0; padding: 1rem; }
  nav { display: flex; flex-wrap: wrap; gap: .4rem; margin-bottom: 1rem; }
  button { padding: .5rem .8rem; border: 1px solid #555; border-radius: .4rem; background: transparent; color: inherit; cursor: pointer; }
  button.active { border-color: var(--success, #9b9); }
  p { margin: 0; font-size: .85rem; line-height: 1.5; opacity: .8; }
  [role="alert"] { color: #f99; }
  footer { display: flex; justify-content: flex-end; gap: .5rem; margin-top: .25rem; }

  @media (max-width: 520px) {
    .settings-content { padding: .75rem; }
    nav button { flex: 1 1 auto; }
  }
</style>
