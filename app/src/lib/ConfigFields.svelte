<script lang="ts">
  import ConfigFields from "./ConfigFields.svelte"
  type Value = string | number | boolean | Value[] | { [key: string]: Value }
  let { value }: { value: { [key: string]: Value } | Value[] } = $props()
  function set(key: string, item: Value) {
    if (Array.isArray(value)) value[Number(key)] = item
    else value[key] = item
  }
  function isRampTime(key: string) {
    return key === "acceleration_time_ms" || key === "deceleration_time_ms"
  }
</script>

<div class="fields">
  {#each Object.entries(value) as [key, item]}
    {#if typeof item === "object" && item !== null}
      <fieldset>
        <legend>{key.replaceAll("_", " ")}</legend>
        <ConfigFields value={item} />
      </fieldset>
    {:else}
      <label>
        <span>{key.replaceAll("_", " ")}</span>
        {#if typeof item === "boolean"}
          <input type="checkbox" checked={item} onchange={(e) => set(key, e.currentTarget.checked)} />
        {:else if typeof item === "number"}
          <input type="number" step={isRampTime(key) ? 1 : "any"} min={isRampTime(key) ? 0 : undefined}
            max={isRampTime(key) ? 5000 : undefined} value={item}
            oninput={(e) => set(key, e.currentTarget.valueAsNumber)} />
        {:else}
          <input type={key === "password" ? "password" : "text"} value={item}
            placeholder={key === "password" ? "Blank keeps stored password" : ""}
            disabled={key === "coordinate_units" || key === "coordinate_system"}
            autocomplete={key === "password" ? "new-password" : "off"}
            oninput={(e) => set(key, e.currentTarget.value)} />
        {/if}
      </label>
    {/if}
  {/each}
</div>

<style>
  .fields { display: grid; gap: .6rem; }
  label { display: flex; justify-content: space-between; align-items: center; gap: 1rem; }
  span, legend { text-transform: capitalize; }
  input { min-width: 0; max-width: 55%; padding: .4rem; color: inherit; background: var(--bgMain, #20242b); border: 1px solid #555; border-radius: .3rem; }
  input[type="checkbox"] { width: 1.2rem; height: 1.2rem; }
  fieldset { min-width: 0; border: 1px solid #5555; border-radius: .4rem; }
</style>
