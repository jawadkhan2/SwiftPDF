<script lang="ts">
  // Custom window title bar (replaces the native Windows frame; the app runs
  // with `decorations: false`). The bar is the OS drag region; the right-hand
  // buttons drive the real window via the Tauri window API.
  import { onMount } from "svelte";
  import AppIcon from "./AppIcon.svelte";

  let { title = "SwiftPDF" }: { title?: string } = $props();

  type AppWindow = {
    isMaximized: () => Promise<boolean>;
    onResized: (handler: () => void | Promise<void>) => Promise<() => void>;
    minimize: () => Promise<void>;
    toggleMaximize: () => Promise<void>;
    close: () => Promise<void>;
  };

  let appWindow = $state<AppWindow | null>(null);
  let maximized = $state(false);

  // Keep the maximize/restore glyph in sync with the actual window state.
  onMount(() => {
    if (!isTauriRuntime()) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    void import("@tauri-apps/api/window").then(async ({ getCurrentWindow }) => {
      if (disposed) return;
      const win = getCurrentWindow();
      appWindow = win;
      maximized = await win.isMaximized();
      unlisten = await win.onResized(async () => {
        maximized = await win.isMaximized();
      });
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function isTauriRuntime(): boolean {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <AppIcon size={18} title="" />
    <span class="title">{title}</span>
  </div>

  <div class="spacer" data-tauri-drag-region></div>

  {#if appWindow}
    <div class="controls">
    <button
      class="ctl"
      title="Minimize"
      aria-label="Minimize"
      onclick={() => appWindow?.minimize()}
    >
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 6h8" /></svg>
    </button>

    <button
      class="ctl"
      title={maximized ? "Restore" : "Maximize"}
      aria-label={maximized ? "Restore" : "Maximize"}
      onclick={() => appWindow?.toggleMaximize()}
    >
      {#if maximized}
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <rect x="2.5" y="3.5" width="6" height="6" />
          <path d="M4.5 3.5V2.5h5v5h-1" fill="none" />
        </svg>
      {:else}
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <rect x="2.5" y="2.5" width="7" height="7" />
        </svg>
      {/if}
    </button>

    <button
      class="ctl close"
      title="Close"
      aria-label="Close"
      onclick={() => appWindow?.close()}
    >
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" /></svg>
    </button>
    </div>
  {/if}
</div>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    height: 40px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    user-select: none;
    flex: 0 0 auto;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 14px;
    line-height: 0;
  }
  .title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text);
    line-height: 1;
  }
  .spacer {
    flex: 1;
    align-self: stretch;
  }
  .controls {
    display: flex;
    height: 100%;
  }
  .ctl {
    width: 46px;
    height: 100%;
    border: 0;
    background: transparent;
    color: var(--text-soft);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .ctl:hover {
    background: rgba(27, 35, 51, 0.08);
  }
  .ctl.close:hover {
    background: var(--danger);
    color: #fff;
  }
  .ctl svg {
    width: 11px;
    height: 11px;
    stroke: currentColor;
    stroke-width: 1.3;
    fill: none;
  }
</style>
