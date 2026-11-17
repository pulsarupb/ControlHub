<script lang="ts">
  import Modal from "./Modal.svelte"
  import Button from "./Button.svelte"
  import { confirmState, acceptConfirm, closeConfirm } from "./confirm.svelte"
</script>

<Modal show={confirmState.open} title={confirmState.title} width="min(420px, calc(100vw - 32px))" onclose={closeConfirm}>
  {#snippet children()}
    <div class="confirm">
      <p>{confirmState.message}</p>
      <footer>
        <Button onclick={closeConfirm}>Cancel</Button>
        <Button onclick={acceptConfirm} style="color: var(--danger); border-color: var(--danger);">{confirmState.label}</Button>
      </footer>
    </div>
  {/snippet}
</Modal>

<style>
  .confirm {
    display: grid;
    gap: 1rem;
    padding: 1rem 1.2rem 1.2rem;
  }
  p {
    margin: 0;
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--textMuted);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
</style>
