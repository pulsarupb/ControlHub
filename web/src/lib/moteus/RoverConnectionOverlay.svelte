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
      <p class="eyebrow">Rover Link</p>
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
    color: rgba(255, 255, 255, 0.78);
  }
  label :global(input) {
    width: 100%;
  }
  .status {
    border: 1px solid rgba(249, 115, 22, 0.45);
    border-radius: 999px;
    padding: 0.35rem 0.7rem;
    background: rgba(249, 115, 22, 0.12);
    color: #fed7aa;
    white-space: nowrap;
  }
  .status.online {
    border-color: rgba(45, 212, 191, 0.45);
    background: rgba(45, 212, 191, 0.12);
    color: #ccfbf1;
  }
  .tabs {
    display: flex;
    gap: 0;
    border-radius: 0.5rem;
    overflow: hidden;
    border: 1px solid var(--bgLight);
  }
  .tab {
    flex: 1;
    padding: 0.5rem 1rem;
    background: transparent;
    color: rgba(255, 255, 255, 0.55);
    border: none;
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.15s, color 0.15s;
  }
  .tab:hover {
    background: rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.85);
  }
  .tab.active {
    background: var(--bgLight);
    color: var(--text);
    font-weight: 600;
  }
  .local-info {
    border: 1px solid var(--bgLight);
    border-radius: 0.85rem;
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    background: linear-gradient(135deg, rgba(251, 191, 36, 0.08), transparent 42%), var(--bgDark);
  }
  .local-info p {
    margin: 0;
    color: rgba(255, 255, 255, 0.78);
    font-size: 0.9rem;
  }
  .local-info strong {
    color: var(--text);
  }
  .rover-ip code {
    background: rgba(0, 0, 0, 0.3);
    padding: 0.15rem 0.5rem;
    border-radius: 0.35rem;
    color: var(--accent);
    font-size: 0.95rem;
  }
  .health-card {
    border: 1px solid var(--bgLight);
    border-radius: 0.85rem;
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    background:
      linear-gradient(135deg, rgba(56, 189, 248, 0.08), transparent 42%),
      var(--bgDark);
  }
  .health-card span,
  .health-card small {
    color: rgba(255, 255, 255, 0.62);
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
