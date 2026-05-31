<div class="camera">
  <div class="noise"></div>
  <p>Camera Feed Placeholder</p>
</div>

<style>
  .camera {
    height: 100%;
    min-height: 160px;
    position: relative;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 0.55rem;
    background: var(--bgDark);
  }
  .noise {
    position: absolute;
    inset: 0;
    background-image: linear-gradient(rgba(255,255,255,.06) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.06) 1px, transparent 1px);
    background-size: 28px 28px;
  }
  p {
    position: absolute;
    inset: auto 1rem 1rem;
    margin: 0;
    color: var(--textMuted);
  }
</style>
