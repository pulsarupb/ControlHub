<script lang="ts">
  import ConfigFields from "./ConfigFields.svelte"
  type Value = string | number | boolean | Value[] | { [key: string]: Value }
  let { value }: { value: { [key: string]: Value } | Value[] } = $props()
  function set(key: string, item: Value) {
    if (Array.isArray(value)) value[Number(key)] = item
    else value[key] = item
  }
  // Raw config keys become readable labels: "acceleration_time_ms" -> "Acceleration time (ms)".
  const acronyms: Record<string, string> = { mxid: "MXID", fps: "FPS", hdop: "HDOP", ntrip: "NTRIP", rtk: "RTK", zed: "ZED", oak: "OAK", id: "ID", ms: "ms" }
  function humanize(key: string) {
    const words = key.split("_").filter(Boolean)
    const unit = words.at(-1) === "ms" ? " (ms)" : ""
    const body = (unit ? words.slice(0, -1) : words).map((w, i) => acronyms[w] ?? (i === 0 ? w.charAt(0).toUpperCase() + w.slice(1) : w)).join(" ")
    return body + unit
  }
  function isRampTime(key: string) {
    return key === "acceleration_time_ms" || key === "deceleration_time_ms"
  }
</script>

<div class="fields">
  {#each Object.entries(value) as [key, item]}
    {#if typeof item === "object" && item !== null}
      <fieldset>
        <legend>{humanize(key)}</legend>
        <ConfigFields value={item} />
      </fieldset>
    {:else}
      <label>
        <span>{humanize(key)}</span>
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
  .fields { display: grid; gap: 0.9rem; }
  label { display: flex; justify-content: space-between; align-items: center; gap: 1rem; min-height: 2rem; font-size: 0.9rem; }
  span, legend { text-transform: capitalize; }
  input {
    min-width: 0;
    max-width: 55%;
    padding: 0.4rem 0.5rem;
    color: var(--text);
    background: var(--bgDark);
    border: 1px solid var(--border);
    border-radius: 0.35rem;
    font: inherit;
    font-size: 0.85rem;
  }
  input:focus { border-color: var(--borderStrong); outline: none; }
  input[type="checkbox"] { width: 1.1rem; height: 1.1rem; accent-color: var(--primary); }
  fieldset { min-width: 0; margin: 0; padding: 0.6rem 0.75rem 0.25rem; border: 1px solid var(--border); border-radius: 0.5rem; display: grid; gap: 0.6rem; }
  legend { padding: 0 0.3rem; font-size: 0.75rem; letter-spacing: 0.04em; color: var(--textMuted); }
</style>
