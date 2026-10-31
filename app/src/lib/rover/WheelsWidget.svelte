<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import { hardware } from "$lib/hardware.svelte"
  import DeviceStatus from "./DeviceStatus.svelte"
  const drive = $derived(rover.status?.devices.drive)
  const names = ["Left front", "Right front", "Left back", "Right back"]
</script>
<section>
  <h3>Wheels / controllers</h3>
  <DeviceStatus device={drive} />
  <p>{drive?.data.armed ? "Drive enabled" : "Drive stopped"} · Open-loop power, no wheel pose</p>
  <table><thead><tr><th>Wheel</th><th>Link</th><th>Power</th><th>Counts</th><th>RPM</th><th>Fault</th></tr></thead>
    <tbody>{#each hardware.config?.drive.motors ?? [] as m, i}
      {@const c = drive?.data.controllers[m.controller]}
      <tr><th>{names[i]}</th><td>{m.controller === 0 ? "A" : "B"}/{m.channel}</td>
        <td>{drive?.data.power[m.controller]?.[m.channel - 1] ?? "-"}</td>
        <td>{c?.encoder_counts[m.channel - 1] ?? "-"}</td><td>{c?.speed_rpm[m.channel - 1] ?? "-"}</td><td>{c?.faults ?? "-"}</td></tr>
    {/each}</tbody>
  </table>
</section>
<style>section { padding: 1rem; overflow: auto; height: 100%; }h3 { margin: 0 0 .5rem; }p { font-size: .8rem; }table { width: 100%; font-size: .8rem; border-collapse: collapse; }th, td { text-align: left; padding: .4rem; border-bottom: 1px solid #5555; }</style>
