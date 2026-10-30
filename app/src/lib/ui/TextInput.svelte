<script lang="ts">
  let {
    value = $bindable(""),
    placeholder = "",
    oninput,
  }: {
    value: string
    placeholder?: string
    oninput?: (event: Event) => void
  } = $props()
</script>

<input bind:value {placeholder} oninput={oninput} />

<style>
  input {
    border: 1px solid var(--bgLight);
    background: var(--bgDark);
    color: var(--text);
    border-radius: 0.5rem;
    padding: 0.55em 0.75em;
    min-width: 0;
  }
</style>
