<script lang="ts">
  import { rover } from "$lib/rover-control.svelte"
  import { hardware } from "$lib/hardware.svelte"
  import DeviceStatus from "./DeviceStatus.svelte"
  const drive = $derived(rover.status?.devices.drive)
  const names = ["Left front", "Right front", "Left back", "Right back"]
</script>
<section>
  <header class="widget-header"><strong>Wheels / controllers</strong></header>
  <DeviceStatus device={drive} />
  <p>{drive?.data.armed ? "Drive enabled" : "Drive stopped"} · {hardware.config?.drive.yaw_control?.enabled ? "ZED yaw feedback · no wheel encoders required" : "Open-loop power, no wheel pose"}</p>
  {#if drive?.stop_reason}<p class="stop-reason">Stopped: {drive.stop_reason.reason}</p>{/if}
  {#if hardware.config?.drive.yaw_control?.enabled && drive?.data.yaw_control}
    {@const yaw = drive.data.yaw_control}
    <p>{yaw.active ? "Yaw PI active" : "Yaw PI idle / awaiting feedback"}{yaw.heading_held ? " · Heading hold" : ""}<br />
      Target {yaw.target_rate_rad_s.toFixed(3)} / measured {yaw.measured_rate_rad_s.toFixed(3)} rad/s · Correction {yaw.correction.toFixed(3)}</p>
  {/if}
  <table><thead><tr><th>Wheel</th><th>Link</th><th>Power</th><th>Counts</th><th>RPM</th><th>Fault</th></tr></thead>
    <tbody>{#each hardware.config?.drive.motors ?? [] as m, i}
      {@const c = drive?.data.controllers[m.controller]}
      <tr><th>{names[i]}</th><td>{m.controller === 0 ? "A" : "B"}/{m.channel}</td>
        <td>{drive?.data.power[m.controller]?.[m.channel - 1] ?? "-"}</td>
        <td>{c?.encoder_counts[m.channel - 1] ?? "-"}</td><td>{c?.speed_rpm[m.channel - 1] ?? "-"}</td><td>{c?.faults ?? "-"}</td></tr>
    {/each}</tbody>
  </table>
</section>
<style>section { padding: 0; overflow: auto; height: 100%; }p { font-size: .8rem; }.stop-reason { color: var(--textWarn); }table { width: 100%; font-size: .8rem; border-collapse: collapse; }th, td { text-align: left; padding: .35rem .4rem; border-bottom: 1px solid var(--border); }thead th { color: var(--textMuted); font-size: .72rem; font-weight: 500; letter-spacing: .04em; text-transform: uppercase; }tbody th { font-weight: 500; }td { font-variant-numeric: tabular-nums; }</style>
