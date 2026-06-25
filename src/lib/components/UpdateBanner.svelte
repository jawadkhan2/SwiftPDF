<script lang="ts">
  // Global, non-blocking update toast. Mounted once in the layout so it follows
  // the user across every screen while the app is running.
  import { updater } from "$lib/updater.svelte";

  // Whether the banner should be on screen at all.
  const open = $derived(
    !updater.dismissed &&
      updater.phase !== "idle" &&
      updater.phase !== "checking",
  );
</script>

{#if open}
  <div class="toast" role="status" aria-live="polite">
    {#if updater.phase === "available"}
      <div class="row">
        <span class="dot up"></span>
        <div class="body">
          <strong>Update available</strong>
          <span class="muted">Version {updater.version} is ready to install.</span>
          {#if updater.notes}
            <details class="notes">
              <summary>What's new</summary>
              <p>{updater.notes}</p>
            </details>
          {/if}
        </div>
      </div>
      <div class="actions">
        <button class="btn btn-ghost" onclick={() => updater.dismiss()}>Later</button>
        <button class="btn btn-primary" onclick={() => updater.downloadAndInstall()}>
          Update now
        </button>
      </div>
    {:else if updater.phase === "downloading"}
      <div class="row">
        <span class="spinner"></span>
        <div class="body">
          <strong>Downloading update…</strong>
          <span class="muted">{updater.progress}%</span>
        </div>
      </div>
      <div class="bar"><div class="fill" style={`width:${updater.progress}%`}></div></div>
    {:else if updater.phase === "ready"}
      <div class="row">
        <span class="dot ok"></span>
        <div class="body">
          <strong>Update ready</strong>
          <span class="muted">Restart to finish installing version {updater.version}.</span>
        </div>
      </div>
      <div class="actions">
        <button class="btn btn-ghost" onclick={() => updater.dismiss()}>Later</button>
        <button class="btn btn-primary" onclick={() => updater.restart()}>Restart now</button>
      </div>
    {:else if updater.phase === "uptodate"}
      <div class="row">
        <span class="dot ok"></span>
        <div class="body"><strong>You're up to date</strong></div>
        <button class="x" aria-label="Dismiss" onclick={() => updater.dismiss()}>✕</button>
      </div>
    {:else if updater.phase === "error"}
      <div class="row">
        <span class="dot err"></span>
        <div class="body">
          <strong>Update failed</strong>
          <span class="muted">{updater.error}</span>
        </div>
        <button class="x" aria-label="Dismiss" onclick={() => updater.dismiss()}>✕</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    right: 1rem;
    bottom: 1rem;
    z-index: 1000;
    width: min(360px, calc(100vw - 2rem));
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.22);
    padding: 0.9rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: 0.65rem;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    flex: 1;
    min-width: 0;
  }
  .body .muted {
    font-size: 0.85rem;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .dot {
    width: 0.7rem;
    height: 0.7rem;
    border-radius: 50%;
    margin-top: 0.25rem;
    flex: 0 0 auto;
  }
  .dot.up {
    background: var(--primary);
  }
  .dot.ok {
    background: var(--success);
  }
  .dot.err {
    background: var(--danger);
  }
  .spinner {
    width: 0.95rem;
    height: 0.95rem;
    margin-top: 0.15rem;
    border: 2px solid var(--border);
    border-top-color: var(--primary);
    border-radius: 50%;
    flex: 0 0 auto;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .bar {
    height: 6px;
    background: var(--border);
    border-radius: 999px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--primary);
    transition: width 0.15s ease;
  }
  .notes {
    margin-top: 0.2rem;
    font-size: 0.85rem;
  }
  .notes summary {
    cursor: pointer;
    color: var(--primary);
  }
  .notes p {
    margin: 0.3rem 0 0;
    white-space: pre-wrap;
    max-height: 8rem;
    overflow-y: auto;
  }
  .x {
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--text-soft);
    font-size: 0.85rem;
    line-height: 1;
    padding: 0.2rem;
  }
</style>
