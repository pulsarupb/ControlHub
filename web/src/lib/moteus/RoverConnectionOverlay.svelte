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

  <label>
    Rover URL
    <TextInput
      bind:value={roverConnection.draftUrl}
      placeholder="http://192.168.1.42:8080"
    />
  </label>

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
    <Button
      transparent={true}
      disabled={roverConnection.isChecking}
      onclick={() => roverConnection.useLocal()}
    >
      Use local
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
