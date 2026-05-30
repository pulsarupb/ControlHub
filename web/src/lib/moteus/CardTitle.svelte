<script lang="ts">
  let { title, detail }: { title: string; detail?: string } = $props();
</script>

<div class="card-title">
  <h2>{title}</h2>
  {#if detail}
    <span>{detail}</span>
  {/if}
</div>

<style>
  .card-title {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.6rem;
    font-family: "Courier New", monospace;
    text-transform: uppercase;
  }

  h2 {
    margin: 0;
    font-size: 1.15rem;
    letter-spacing: -0.04em;
  }

  span {
    color: #8fe8ff;
    font-size: 0.82rem;
    text-align: right;
  }

  @media (max-width: 720px) {
    .card-title {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
