<script lang="ts">
  let { title = "Drop PDFs", detail }: { title?: string; detail: string } =
    $props();
</script>

<div class="drop-overlay" aria-hidden="true">
  <div class="drop-message">
    <strong>{title}</strong>
    <span>{detail}</span>
  </div>
</div>

<style>
  .drop-overlay {
    position: fixed;
    inset: 0;
    z-index: 20;
    pointer-events: none;
    display: grid;
    place-items: start center;
    padding-top: 2.2rem;
    background: rgba(244, 246, 251, 0.35);
    border: 3px dashed var(--primary);
  }
  .drop-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    padding: 0.7rem 1.4rem;
    border-radius: 999px;
    border: 1px solid var(--primary);
    background: var(--surface);
    box-shadow: var(--shadow-lg);
  }
  .drop-message strong {
    font-size: 1.2rem;
  }
  .drop-message span {
    color: var(--text-soft);
    font-size: 0.95rem;
  }
</style>
