<script lang="ts">
  import Modal from "$lib/ui/Modal.svelte"
  import Button from "$lib/ui/Button.svelte"
  import { getConfig, updateConfig } from "$lib/config-api"
  import type { FullConfig, ChassisConfigData, MotorConfig, MotorsConfig } from "$lib/config-api"
  import { global } from "$lib"
  import { onMount } from "svelte"

  let {
    show = false,
    onclose,
  }: {
    show?: boolean
    onclose?: () => void
  } = $props()

  type Tab = "chassis" | "motors"
  let activeTab: Tab = $state("chassis")
  let loading = $state(true)
  let saving = $state(false)
  let config: FullConfig | null = $state(null)

  let chassis: ChassisConfigData = $state({ wheel_radius_mm: 107.95, track_width_mm: 440.0, motor_rotations_per_wheel_rotation: 1.0, max_velocity: 2.0 })
  let motors: MotorsConfig = $state({
    left_front: { id: 1, direction: 1.0 },
    right_front: { id: 3, direction: -1.0 },
    left_back: { id: 4, direction: -1.0 },
    right_back: { id: 2, direction: 1.0 },
  })

  let originalChassis: ChassisConfigData = $state({ ...chassis })
  let originalMotors: MotorsConfig = $state(JSON.parse(JSON.stringify(motors)))

  function chassisEqual(a: ChassisConfigData, b: ChassisConfigData): boolean {
    return a.wheel_radius_mm === b.wheel_radius_mm
      && a.track_width_mm === b.track_width_mm
      && a.motor_rotations_per_wheel_rotation === b.motor_rotations_per_wheel_rotation
      && a.max_velocity === b.max_velocity
  }

  function motorsEqual(a: MotorsConfig, b: MotorsConfig): boolean {
    return a.left_front.id === b.left_front.id && a.left_front.direction === b.left_front.direction
      && a.right_front.id === b.right_front.id && a.right_front.direction === b.right_front.direction
      && a.left_back.id === b.left_back.id && a.left_back.direction === b.left_back.direction
      && a.right_back.id === b.right_back.id && a.right_back.direction === b.right_back.direction
  }

  let dirty = $derived(!chassisEqual(chassis, originalChassis) || !motorsEqual(motors, originalMotors))

  onMount(() => {
    loadConfig()
  })

  function snapshotOriginals() {
    originalChassis = { ...chassis }
    originalMotors = JSON.parse(JSON.stringify(motors))
  }

  async function loadConfig() {
    loading = true
    try {
      config = await getConfig()
      chassis = { ...config.chassis }
      motors = JSON.parse(JSON.stringify(config.motors))
      snapshotOriginals()
    } catch (e) {
      global.notificationsManager.add("Failed to load config: " + (e instanceof Error ? e.message : String(e)))
    } finally {
      loading = false
    }
  }

  async function save() {
    saving = true
    try {
      await updateConfig({ chassis, motors })
      global.notificationsManager.add("Config saved — motor loop restarting")
      onclose?.()
    } catch (e) {
      global.notificationsManager.add("Failed to save: " + (e instanceof Error ? e.message : String(e)))
    } finally {
      saving = false
    }
  }

  const motorKeys: (keyof MotorsConfig)[] = ["left_front", "right_front", "left_back", "right_back"]
  const motorLabels: Record<keyof MotorsConfig, string> = {
    left_front: "Left Front",
    right_front: "Right Front",
    left_back: "Left Back",
    right_back: "Right Back",
  }
</script>

<Modal {show} title="Settings" width="min(800px, calc(100vw - 32px))" {onclose}>
  {#snippet children()}
    <div class="layout">
      <aside class="sidebar">
        <button
          class="sidebar-item"
          class:active={activeTab === "chassis"}
          onclick={() => (activeTab = "chassis")}
        >
          Chassis
        </button>
        <button
          class="sidebar-item"
          class:active={activeTab === "motors"}
          onclick={() => (activeTab = "motors")}
        >
          Motors
        </button>
      </aside>
      <main class="main">
        {#if loading}
          <div class="loading">Loading config...</div>
        {:else if activeTab === "chassis"}
          <div class="form-grid">
            <label>
              <span>Wheel Radius (mm)</span>
              <input type="number" step="any" bind:value={chassis.wheel_radius_mm} />
            </label>
            <label>
              <span>Track Width (mm)</span>
              <input type="number" step="any" bind:value={chassis.track_width_mm} />
            </label>
            <label>
              <span>Gear Ratio</span>
              <input type="number" step="any" bind:value={chassis.motor_rotations_per_wheel_rotation} />
            </label>
            <label>
              <span>Max Velocity</span>
              <input type="number" step="any" bind:value={chassis.max_velocity} />
            </label>
          </div>
        {:else}
          <div class="motor-grid">
            {#each motorKeys as key}
              <div class="motor-card">
                <h3>{motorLabels[key]}</h3>
                <label>
                  <span>ID</span>
                  <input type="number" step="1" bind:value={motors[key].id} />
                </label>
                <label>
                  <span>Direction</span>
                  <select bind:value={motors[key].direction}>
                    <option value={1.0}>+1.0</option>
                    <option value={-1.0}>-1.0</option>
                  </select>
                </label>
              </div>
            {/each}
          </div>
        {/if}

        <div class="actions">
          <Button onclick={async () => { await loadConfig(); onclose?.(); }}>Cancel</Button>
          <Button onclick={save} disabled={saving || loading || !dirty}>
            {saving ? "Saving..." : "Save"}
          </Button>
        </div>
      </main>
    </div>
  {/snippet}
</Modal>

<style>
  .layout {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .sidebar {
    width: 160px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0.75rem;
    border-right: 1px solid var(--border);
    background: var(--bgDark);
  }
  .sidebar-item {
    all: unset;
    cursor: pointer;
    padding: 0.6rem 0.85rem;
    border-radius: 0.4rem;
    font-size: 0.92rem;
    color: var(--textMuted);
    transition: background 0.12s, color 0.12s;
  }
  .sidebar-item:hover {
    background: var(--bgLight);
    color: var(--text);
  }
  .sidebar-item.active {
    background: var(--bgLight);
    color: var(--text);
    font-weight: 600;
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 1.2rem;
    gap: 1.2rem;
    min-width: 0;
    height: 520px;
    overflow-y: auto;
  }
  .loading {
    color: var(--textMuted);
    display: grid;
    place-items: center;
    flex: 1;
  }
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    align-content: start;
  }
  .form-grid label,
  .motor-card label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.85rem;
    color: var(--textMuted);
  }
  .form-grid input,
  .motor-card input,
  .motor-card select {
    border: 1px solid var(--border);
    background: var(--bgDark);
    color: var(--text);
    border-radius: 0.45rem;
    padding: 0.5em 0.7em;
    font-family: inherit;
    font-size: 0.92rem;
  }
  .motor-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.85rem;
    align-content: start;
  }
  .motor-card {
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    background: var(--bgDark);
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }
  .motor-card h3 {
    margin: 0;
    font-size: 0.92rem;
    font-weight: 600;
  }
  .motor-card label {
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
  }
  .motor-card label span {
    min-width: 60px;
  }
  .motor-card input,
  .motor-card select {
    flex: 1;
    min-width: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    margin-top: auto;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }
</style>
