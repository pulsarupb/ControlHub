<script lang="ts">
  import Button from "$lib/ui/Button.svelte"
  import TextInput from "$lib/ui/TextInput.svelte"
  import { roverConnection } from "$lib/rover-connection.svelte"

  function lastChecked(): string {
    if (!roverConnection.lastCheckedAt) return "not checked"

    return new Date(roverConnection.lastCheckedAt).toLocaleTimeString()
  }
</script>

<div class="rover-connect">
  <div class="heading">
    <div>
      <h1>Connect to rover</h1>
    </div>
    <span class="status" class:online={roverConnection.state === "online"}>
      {roverConnection.state}
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
      class:active={roverConnection.connectionMode === "remote"}
      onclick={() => roverConnection.useRemote()}
    >Remote</button>
  </div>

  {#if roverConnection.connectionMode === "local"}
    <div class="local-info">
      <p>Connect your device to the <strong>{roverConnection.roverSsid}</strong> WiFi network</p>
      <p class="rover-ip">Rover IP: <code>http://10.42.0.1:8080</code></p>
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
    <span>Healthcheck</span>
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
  @media (max-width: 520px) {
    .heading,
    .actions {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
