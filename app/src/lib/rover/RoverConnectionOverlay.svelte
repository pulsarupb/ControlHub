<script lang="ts">
  import Button from "$lib/ui/Button.svelte"
  import TextInput from "$lib/ui/TextInput.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"
  import { rover } from "$lib/rover-control.svelte"
  import { global } from "$lib"
  import {
    PowerRequestError,
    requestPower,
    roverHealthOk,
    type PowerAction,
  } from "$lib/system-api"

  type PowerState = "idle" | "confirm" | "sending" | "waiting" | "done" | "error"

  let powerState = $state<PowerState>("idle")
  let powerAction = $state<PowerAction>("shutdown")
  let powerError = $state("")

  const simulated = $derived(
    rover.status?.simulate === true || roverConnection.robotMode === "simulated",
  )
  const powerAvailable = $derived(
    roverConnection.state === "online" && !simulated,
  )
  const powerBusy = $derived(
    powerState === "sending" || powerState === "waiting",
  )

  function lastChecked(): string {
    if (!roverConnection.lastCheckedAt) return "not checked"
    return new Date(roverConnection.lastCheckedAt).toLocaleTimeString()
  }

  function begin(action: PowerAction) {
    if (!powerAvailable || powerBusy) return
    powerAction = action
    powerError = ""
    powerState = "confirm"
  }

  function cancel() {
    if (powerBusy) return
    powerState = "idle"
    powerError = ""
  }

  async function waitFor(
    predicate: () => Promise<boolean>,
    timeoutMs: number,
  ): Promise<boolean> {
    const deadline = Date.now() + timeoutMs
    while (Date.now() < deadline) {
      if (await predicate()) return true
      await new Promise((resolve) => setTimeout(resolve, 1000))
    }
    return false
  }

  async function confirmPower() {
    if (!powerAvailable || powerBusy) return
    const action = powerAction
    powerState = "sending"
    powerError = ""
    try {
      await requestPower(action)
    } catch (error) {
      if (error instanceof PowerRequestError) {
        powerState = "error"
        powerError = error.message
        return
      }
      // The host may start shutting down before the response arrives. Treat a
      // transport failure as pending and verify with the health endpoint.
    }

    powerState = "waiting"
    const offline = await waitFor(async () => !(await roverHealthOk()), 20_000)
    if (!offline) {
      powerState = "error"
      powerError = `Rover is still online. The ${action} request may have failed; check the driver logs.`
      return
    }

    if (action === "shutdown") {
      powerState = "done"
      global.notificationsManager.add("Rover is shutting down.")
      return
    }

    const online = await waitFor(roverHealthOk, 180_000)
    if (!online) {
      powerState = "error"
      powerError = "Rover went offline but did not come back online."
      return
    }
    powerState = "done"
    global.notificationsManager.add("Rover rebooted and is online.")
  }
</script>

<div class="rover-connect">
  <div class="heading">
    <div>
      <h1>Connect to rover</h1>
    </div>
    <span class="status" class:online={roverConnection.state === "online"}>
      {#if roverConnection.state === "online" && roverConnection.robotMode === "simulated"}
        SIM
      {:else}
        {roverConnection.state}
      {/if}
    </span>
  </div>

  <div class="tabs">
    <button
      class="tab"
      class:active={roverConnection.connectionMode === "local"}
      onclick={() => roverConnection.useLocal()}
    >Local</button>
    <button
      class="tab"
      class:active={roverConnection.connectionMode === "dev"}
      onclick={() => roverConnection.useDev()}
    >Dev</button>
    <button
      class="tab"
      class:active={roverConnection.connectionMode === "remote"}
      onclick={() => roverConnection.useRemote()}
    >Remote</button>
  </div>

  {#if roverConnection.connectionMode === "local"}
    <div class="local-info">
      <p>Connect your device to the <strong>{roverConnection.roverSsid}</strong> WiFi network</p>
      <p class="rover-ip">Rover IP: <code>http://10.42.0.1:8080</code></p>
    </div>
  {:else if roverConnection.connectionMode === "dev"}
    <div class="local-info">
      <p>Connect to a <strong>simulated rover</strong> running on localhost</p>
      <p class="rover-ip">Dev URL: <code>http://127.0.0.1:8080</code></p>
    </div>
  {:else}
    <label>
      Rover URL
      <TextInput
        bind:value={roverConnection.draftUrl}
        placeholder="http://192.168.1.42:8080"
      />
    </label>
  {/if}

  <div class="health-card">
    <span>
      Healthcheck
      {#if roverConnection.robotMode === "simulated"}
        <span class="mode-badge">SIMULATED</span>
      {:else if roverConnection.robotMode === "real"}
        <span class="mode-badge real">REAL</span>
      {/if}
    </span>
    <strong>{roverConnection.message}</strong>
    <small>
      GET /api/health · {lastChecked()}
      {#if roverConnection.latencyMs !== null}
        · {roverConnection.latencyMs}ms
      {/if}
    </small>
  </div>

  <div class="actions">
    <Button
      style="flex: 1;"
      disabled={roverConnection.isChecking}
      onclick={() => roverConnection.connect()}
    >
      {roverConnection.isChecking ? "Checking..." : "Connect"}
    </Button>
  </div>

  <div class="power">
    <div class="power-heading">
      <strong>Rover power</strong>
      {#if simulated}
        <span class="power-badge">SIMULATED</span>
      {/if}
    </div>
    <p class="power-hint">
      {#if simulated}
        Host power control is disabled in simulation.
      {:else if roverConnection.state !== "online"}
        Connect to the rover to control its power.
      {:else}
        Stops all motion, then asks the Jetson to shut down or reboot.
      {/if}
    </p>

    {#if powerState === "idle"}
      <div class="power-actions">
        <button
          class="power-button"
          disabled={!powerAvailable}
          onclick={() => begin("shutdown")}
        >
          Shut down
        </button>
        <button
          class="power-button"
          disabled={!powerAvailable}
          onclick={() => begin("reboot")}
        >
          Reboot
        </button>
      </div>
    {:else if powerState === "confirm"}
      <p class="power-warning">
        {powerAction === "shutdown"
          ? "Shut down the Jetson? The rover will go offline and must be powered on manually."
          : "Reboot the Jetson? The rover will go offline for a while."}
      </p>
      <div class="power-actions">
        <Button style="flex: 1;" onclick={cancel}>Cancel</Button>
        <button class="power-button danger" onclick={confirmPower}>
          {powerAction === "shutdown" ? "Yes, shut down" : "Yes, reboot"}
        </button>
      </div>
    {:else if powerState === "sending"}
      <p class="power-status">Sending {powerAction} request…</p>
    {:else if powerState === "waiting"}
      <p class="power-status">
        {powerAction === "shutdown"
          ? "Waiting for the rover to go offline…"
          : "Waiting for the rover to reboot…"}
      </p>
    {:else if powerState === "done"}
      <p class="power-status success">
        {powerAction === "shutdown"
          ? "Rover is shutting down."
          : "Rover rebooted and is online."}
      </p>
    {:else if powerState === "error"}
      <p class="power-status error">{powerError}</p>
      <div class="power-actions">
        <Button style="flex: 1;" onclick={cancel}>Dismiss</Button>
      </div>
    {/if}
  </div>
</div>

<style>
  .rover-connect {
    width: min(420px, calc(100vw - 32px));
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .heading,
  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }
  .eyebrow,
  h1 {
    margin: 0;
  }
  .eyebrow {
    color: var(--accent);
    font-size: 0.75rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  h1 {
    font-size: 1.35rem;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    color: var(--textMuted);
  }
  label :global(input) {
    width: 100%;
  }
  .status {
    border: 1px solid color-mix(in srgb, var(--warning) 60%, transparent);
    border-radius: 0.4rem;
    padding: 0.35rem 0.7rem;
    background: color-mix(in srgb, var(--warning) 10%, transparent);
    color: #e5cf9e;
    white-space: nowrap;
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.05em;
  }
  .status.online {
    border-color: color-mix(in srgb, var(--success) 55%, transparent);
    background: color-mix(in srgb, var(--success) 10%, transparent);
    color: #cfe0c8;
  }
  .tabs {
    display: flex;
    gap: 0;
    border-radius: 0.4rem;
    overflow: hidden;
    border: 1px solid var(--border);
  }
  .tab {
    flex: 1;
    padding: 0.5rem 1rem;
    background: transparent;
    color: var(--textSubtle);
    border: none;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.15s, color 0.15s;
  }
  .tab:hover {
    background: var(--surfaceRaised);
    color: var(--text);
  }
  .tab.active {
    background: var(--bgLight);
    color: var(--text);
    font-weight: 600;
  }
  .local-info {
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    background: var(--bgDark);
  }
  .local-info p {
    margin: 0;
    color: var(--textMuted);
    font-size: 0.9rem;
  }
  .local-info strong {
    color: var(--text);
  }
  .rover-ip code {
    background: var(--surfaceRaised);
    padding: 0.15rem 0.5rem;
    border-radius: 0.35rem;
    color: var(--accent);
    font-size: 0.95rem;
  }
  .health-card {
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    background: var(--bgDark);
  }
  .health-card span,
  .health-card small {
    color: var(--textSubtle);
  }
  .health-card strong {
    color: var(--text);
    font-weight: 500;
  }
  .mode-badge {
    display: inline-block;
    margin-left: 0.5rem;
    padding: 0.1rem 0.4rem;
    border-radius: 0.3rem;
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    background: color-mix(in srgb, var(--warning) 25%, transparent);
    color: #e5cf9e;
    border: 1px solid color-mix(in srgb, var(--warning) 50%, transparent);
    vertical-align: middle;
  }
  .mode-badge.real {
    background: color-mix(in srgb, var(--success) 20%, transparent);
    color: #bfd7b7;
    border-color: color-mix(in srgb, var(--success) 40%, transparent);
  }
  .power {
    border-top: 1px solid var(--border);
    padding-top: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .power-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .power-badge {
    padding: 0.1rem 0.4rem;
    border-radius: 0.3rem;
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    background: color-mix(in srgb, var(--warning) 25%, transparent);
    color: #e5cf9e;
    border: 1px solid color-mix(in srgb, var(--warning) 50%, transparent);
  }
  .power-hint,
  .power-status,
  .power-warning {
    margin: 0;
    font-size: 0.8rem;
    line-height: 1.4;
  }
  .power-hint,
  .power-status {
    color: var(--textMuted);
  }
  .power-warning {
    color: #e5cf9e;
  }
  .power-status.success {
    color: #cfe0c8;
  }
  .power-status.error {
    color: #e7b3ae;
  }
  .power-actions {
    display: flex;
    gap: 0.5rem;
  }
  .power-actions > :global(button) {
    flex: 1;
  }
  .power-button {
    border: 1px solid var(--border);
    border-radius: 0.4rem;
    padding: 0.5em 0.85em;
    background: var(--surfaceRaised);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-weight: 500;
    text-align: center;
  }
  .power-button:hover:not(:disabled) {
    border-color: var(--borderStrong);
    background: var(--bgLight);
  }
  .power-button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }
  .power-button.danger {
    border-color: color-mix(in srgb, var(--danger) 60%, transparent);
    background: color-mix(in srgb, var(--danger) 14%, transparent);
    color: #e7b3ae;
  }
  .power-button.danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 22%, transparent);
  }
  @media (max-width: 520px) {
    .heading,
    .actions,
    .power-actions {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
