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
    border-radius: 0.75rem;
    background: linear-gradient(135deg, #111827, #292524);
  }
  .noise {
    position: absolute;
    inset: 0;
    background: radial-gradient(circle at 20% 30%, rgba(255,255,255,.16), transparent 12%), radial-gradient(circle at 70% 55%, rgba(249,115,22,.18), transparent 16%);
  }
  p {
    position: absolute;
    inset: auto 1rem 1rem;
    margin: 0;
    opacity: 0.8;
  }
</style>
