<script lang="ts">
  import { activeWorkspace, goToWorkspace } from "$lib/grid/workspace.svelte"
</script>

<nav
  class="workspace-nav"
  aria-label="Workspaces. Swipe to move between workspaces or tap a square to select one."
>
  {#each [0, 1] as row}
    <div class="workspace-row">
      {#each [0, 1, 2] as column}
        <button
          class:active={activeWorkspace.column === column && activeWorkspace.row === row}
          aria-label="Go to workspace {row * 3 + column + 1}"
          aria-current={activeWorkspace.column === column && activeWorkspace.row === row ? "page" : undefined}
          onclick={() => goToWorkspace(column, row)}
        ></button>
      {/each}
    </div>
  {/each}
</nav>

<style>
  .workspace-nav {
    display: grid;
    gap: 3px;
    padding: 3px;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
  }

  .workspace-row {
    display: flex;
    gap: 3px;
  }

  button {
    width: 16px;
    height: 12px;
    padding: 0;
    border: 1px solid var(--borderStrong);
    border-radius: 3px;
    background: var(--bgLight);
  }

  button.active {
    background: var(--primary);
    border-color: var(--primary);
  }
</style>
