<script lang="ts">
  import { tick } from "svelte";
  import { goto } from "$app/navigation";
  import AppIcon from "$lib/components/AppIcon.svelte";
  import Thumbnail from "$lib/components/Thumbnail.svelte";
  import ViewerIcon, { type IconName } from "$lib/components/ViewerIcon.svelte";
  import {
    editedName,
    openPdf,
    pickPdf,
    renderPage,
    saveBuiltPdf,
    toDataUrl,
    type PagePlan,
    type RenderResult,
  } from "$lib/api";
  import { setDoc, doc } from "$lib/stores/document.svelte";
  import { viewer } from "$lib/stores/viewer.svelte";

  type ToolMode = "select" | "hand";

  interface ViewerTool {
    key: string;
    label: string;
    detail: string;
    icon: IconName;
    route: string;
  }

  const current = $derived(doc.current);
  const toolActions: ViewerTool[] = [
    {
      key: "organize",
      label: "Organize",
      detail: "Reorder pages",
      icon: "organize",
      route: "/organize",
    },
    {
      key: "sign",
      label: "Fill & Sign",
      detail: "Add text or signature",
      icon: "sign",
      route: "/sign",
    },
    {
      key: "split",
      label: "Split",
      detail: "Extract pages",
      icon: "split",
      route: "/split",
    },
    {
      key: "merge",
      label: "Merge",
      detail: "Combine files",
      icon: "merge",
      route: "/merge",
    },
  ];

  let render = $state<RenderResult | null>(null);
  let pageIndex = $state(0);
  let pageText = $state("1");
  let zoom = $state(1);
  let fitMode = $state(true);
  let viewRotation = $state(0);
  let loading = $state(false);
  let opening = $state(false);
  let saving = $state(false);
  let renderError = $state<string | null>(null);
  let notice = $state<{ kind: "ok" | "err"; msg: string } | null>(null);
  let toolMode = $state<ToolMode>("select");
  let pagesCollapsed = $state(false);
  let toolsCollapsed = $state(true);
  let loadedDocId = $state<string | null>(null);
  let renderRequest = 0;

  // Rendered-page cache for instant navigation. Keyed by page index for the
  // currently loaded document; cleared whenever the document changes. We also
  // prefetch the neighbouring pages so Prev/Next feel instant.
  const RENDER_SIZE = 1500;
  const pageCache = new Map<number, RenderResult>();
  const prefetching = new Set<number>();

  // Run low-priority work (neighbour prefetch) when the backend render thread is
  // idle, so it never delays the page the user is actually waiting on.
  const whenIdle = (fn: () => void) =>
    typeof requestIdleCallback === "function"
      ? requestIdleCallback(fn, { timeout: 800 })
      : setTimeout(fn, 200);

  let stageEl = $state<HTMLDivElement | null>(null);
  let stageWidth = $state(0);
  let stageHeight = $state(0);

  // Floating controls dock: user can drag it around the page area or collapse it.
  let dockEl = $state<HTMLDivElement | null>(null);
  let dockCollapsed = $state(false);
  // Anchored by the RIGHT edge: `x` = gap from panel right edge to dock right
  // edge, `y` = gap from panel top. Right-anchoring keeps the dock pinned to the
  // right as it collapses/expands, so it never grows off-screen.
  let dockPos = $state<{ x: number; y: number } | null>(null);
  let dockDrag: { px: number; py: number; ox: number; oy: number } | null = null;
  let pan = $state<
    | {
        x: number;
        y: number;
        left: number;
        top: number;
      }
    | null
  >(null);

  const totalPages = $derived(current?.page_count ?? 0);
  const zoomLabel = $derived(`${Math.round(zoom * 100)}%`);
  const rotated = $derived(viewRotation % 180 !== 0);
  const sheetWidth = $derived(
    render ? (rotated ? render.height_pt : render.width_pt) * zoom : 0,
  );
  const sheetHeight = $derived(
    render ? (rotated ? render.width_pt : render.height_pt) * zoom : 0,
  );
  const imageWidth = $derived(render ? render.width_pt * zoom : 0);
  const imageHeight = $derived(render ? render.height_pt * zoom : 0);

  const clamp = (value: number, min: number, max: number) =>
    Math.min(max, Math.max(min, value));

  function copyName(sourceName: string): string {
    const dot = sourceName.toLowerCase().endsWith(".pdf")
      ? sourceName.length - 4
      : sourceName.length;
    return `${sourceName.slice(0, dot)} (copy).pdf`;
  }

  $effect(() => {
    const active = current;
    if (!active) return;
    if (active.doc_id !== loadedDocId) {
      loadedDocId = active.doc_id;
      viewRotation = 0;
      fitMode = true;
      pageCache.clear();
      prefetching.clear();
      void loadPage(0, active.doc_id);
    }
  });

  $effect(() => {
    pageText = totalPages > 0 ? String(pageIndex + 1) : "1";
  });

  $effect(() => {
    if (!stageEl) return;
    const measure = () => {
      stageWidth = stageEl?.clientWidth ?? 0;
      stageHeight = stageEl?.clientHeight ?? 0;
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(stageEl);
    return () => ro.disconnect();
  });

  $effect(() => {
    if (!fitMode || !render || stageWidth <= 0 || stageHeight <= 0) return;
    const width = rotated ? render.height_pt : render.width_pt;
    const height = rotated ? render.width_pt : render.height_pt;
    const fitWidth = (stageWidth - 96) / width;
    const fitHeight = (stageHeight - 96) / height;
    zoom = clamp(Math.min(fitWidth, fitHeight), 0.25, 3);
  });

  function prefetchAround(center: number, docId: string, total: number) {
    // Only the immediate next/previous page, and only once the render thread is
    // idle — prefetching too many big pages up front is what made opening slow.
    for (const offset of [1, -1]) {
      const i = center + offset;
      if (i < 0 || i >= total) continue;
      if (pageCache.has(i) || prefetching.has(i)) continue;
      prefetching.add(i);
      whenIdle(() => {
        if (current?.doc_id !== docId || pageCache.has(i)) {
          prefetching.delete(i);
          return;
        }
        renderPage(docId, i, RENDER_SIZE)
          .then((r) => {
            if (current?.doc_id === docId) pageCache.set(i, r);
          })
          .catch(() => {})
          .finally(() => prefetching.delete(i));
      });
    }
  }

  async function loadPage(index: number, docId = current?.doc_id) {
    const active = current;
    if (!active || !docId) return;

    const nextIndex = clamp(index, 0, active.page_count - 1);
    const request = ++renderRequest;
    pageIndex = nextIndex;
    viewer.page = nextIndex;
    renderError = null;

    // Cache hit: swap in immediately, no spinner, just warm the neighbours.
    const cached = pageCache.get(nextIndex);
    if (cached) {
      render = cached;
      loading = false;
      prefetchAround(nextIndex, docId, active.page_count);
      return;
    }

    render = null;
    loading = true;

    try {
      const result = await renderPage(docId, nextIndex, RENDER_SIZE);
      if (current?.doc_id === docId) pageCache.set(nextIndex, result);
      if (request !== renderRequest || current?.doc_id !== docId) return;
      render = result;
      prefetchAround(nextIndex, docId, active.page_count);
    } catch (e) {
      if (request === renderRequest) renderError = String(e);
    } finally {
      if (request === renderRequest) loading = false;
    }
  }

  async function openNewPdf() {
    notice = null;
    const path = await pickPdf();
    if (!path) return;

    opening = true;
    try {
      const result = await openPdf(path);
      await setDoc(result);
    } catch (e) {
      notice = { kind: "err", msg: String(e) };
    } finally {
      opening = false;
    }
  }

  async function saveCopy() {
    if (!current) return;

    saving = true;
    notice = null;
    try {
      const plan: PagePlan[] = current.pages.map((page, index) => ({
        page: index,
        rotation: page.rotation,
      }));
      const saved = await saveBuiltPdf(
        current.doc_id,
        plan,
        copyName(current.source_name),
      );
      if (saved) {
        const name = saved.split(/[\\/]/).pop() ?? saved;
        notice = { kind: "ok", msg: `Saved ${name}` };
      }
    } catch (e) {
      notice = { kind: "err", msg: String(e) };
    } finally {
      saving = false;
    }
  }

  function setZoom(nextZoom: number) {
    fitMode = false;
    zoom = clamp(nextZoom, 0.25, 3);
  }

  function fitPage() {
    fitMode = true;
  }

  function rotateView() {
    viewRotation = (viewRotation + 90) % 360;
  }

  function commitPageText() {
    const target = Number.parseInt(pageText, 10);
    if (Number.isFinite(target) && target >= 1 && target <= totalPages) {
      void loadPage(target - 1);
    } else {
      pageText = String(pageIndex + 1);
    }
  }

  function goToTool(route: string) {
    void goto(route);
  }

  function startPan(e: PointerEvent) {
    if (toolMode !== "hand" || !stageEl) return;
    pan = {
      x: e.clientX,
      y: e.clientY,
      left: stageEl.scrollLeft,
      top: stageEl.scrollTop,
    };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function movePan(e: PointerEvent) {
    if (!pan || !stageEl) return;
    stageEl.scrollLeft = pan.left - (e.clientX - pan.x);
    stageEl.scrollTop = pan.top - (e.clientY - pan.y);
  }

  function stopPan(e: PointerEvent) {
    pan = null;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // The pointer may already be released if it left the window.
    }
  }

  // --- Floating dock drag ---------------------------------------------------
  function startDockDrag(e: PointerEvent) {
    const panel = dockEl?.offsetParent as HTMLElement | null;
    if (!dockEl || !panel) return;
    const rect = dockEl.getBoundingClientRect();
    const prect = panel.getBoundingClientRect();
    dockDrag = {
      px: e.clientX,
      py: e.clientY,
      ox: panel.clientWidth - (rect.right - prect.left),
      oy: rect.top - prect.top,
    };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function moveDockDrag(e: PointerEvent) {
    if (!dockDrag || !dockEl) return;
    const panel = dockEl.offsetParent as HTMLElement | null;
    if (!panel) return;
    // Pointer moving right shrinks the right-edge gap.
    const x = dockDrag.ox - (e.clientX - dockDrag.px);
    const y = dockDrag.oy + (e.clientY - dockDrag.py);
    const maxX = Math.max(0, panel.clientWidth - dockEl.offsetWidth);
    const maxY = Math.max(0, panel.clientHeight - dockEl.offsetHeight);
    dockPos = { x: clamp(x, 0, maxX), y: clamp(y, 0, maxY) };
  }

  function endDockDrag(e: PointerEvent) {
    dockDrag = null;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Pointer may already be released.
    }
  }

  // Keep the dock fully inside the panel — used after expanding, in case the
  // wider toolbar would now poke past the left edge.
  function clampDockPos() {
    const panel = dockEl?.offsetParent as HTMLElement | null;
    if (!dockEl || !panel || !dockPos) return;
    const maxX = Math.max(0, panel.clientWidth - dockEl.offsetWidth);
    const maxY = Math.max(0, panel.clientHeight - dockEl.offsetHeight);
    dockPos = { x: clamp(dockPos.x, 0, maxX), y: clamp(dockPos.y, 0, maxY) };
  }

  async function toggleDock() {
    dockCollapsed = !dockCollapsed;
    if (dockCollapsed || !dockPos) return;
    // Re-clamp once the expand transition has settled to its final width.
    await tick();
    setTimeout(clampDockPos, 260);
  }

</script>

<svelte:head>
  <title>SwiftPDF Viewer</title>
</svelte:head>

<div class="viewer-screen">
  <header class="topbar">
    <button class="btn btn-ghost home-btn" onclick={() => goto("/")}>← Home</button>

    <div class="file-chip" title={current?.source_name ?? "No PDF open"}>
      <ViewerIcon name="document" size={18} />
      <strong>{current?.source_name ?? "No PDF open"}</strong>
    </div>

    <div class="top-actions">
      <button class="action-button" onclick={openNewPdf} disabled={opening}>
        <ViewerIcon name="folder-open" />
        <span>{opening ? "Opening..." : "Open PDF"}</span>
      </button>
      <button
        class="action-button"
        onclick={saveCopy}
        disabled={!current || saving}
      >
        <ViewerIcon name="save-copy" />
        <span>{saving ? "Saving..." : "Save Copy"}</span>
      </button>
    </div>
  </header>

  {#if current}
    <div
      class:pages-collapsed={pagesCollapsed}
      class:tools-collapsed={toolsCollapsed}
      class="viewer-workspace"
    >
      <aside class:collapsed={pagesCollapsed} class="sidebar pages-sidebar">
        <div class="sidebar-head">
          <div class="sidebar-title">
            <ViewerIcon name="pages" />
            <span>Pages</span>
          </div>
          <button
            class="icon-button"
            title={pagesCollapsed ? "Show pages" : "Hide pages"}
            aria-label={pagesCollapsed ? "Show pages" : "Hide pages"}
            aria-expanded={!pagesCollapsed}
            onclick={() => (pagesCollapsed = !pagesCollapsed)}
          >
            <ViewerIcon name="panel-left" />
          </button>
        </div>

        {#if !pagesCollapsed}
          <div class="page-count">{totalPages} page{totalPages === 1 ? "" : "s"}</div>
          <div class="thumbnail-list">
            {#each current.pages as _page, i (i)}
              <button
                class:active={i === pageIndex}
                class="thumbnail-button"
                onclick={() => loadPage(i)}
                aria-label={`Open page ${i + 1}`}
              >
                <Thumbnail docId={current.doc_id} page={i} size={180} />
                <span>Page {i + 1}</span>
              </button>
            {/each}
          </div>
        {/if}
      </aside>

      <main class:hand-mode={toolMode === "hand"} class:panning={!!pan} class="document-panel">
        {#if notice}
          <div class="notice {notice.kind}" role="status">{notice.msg}</div>
        {/if}

        <div
          class="document-stage"
          bind:this={stageEl}
          role="region"
          aria-label="Document page"
          onpointerdown={startPan}
          onpointermove={movePan}
          onpointerup={stopPan}
          onpointerleave={stopPan}
        >
          {#if render}
            <div
              class="page-sheet"
              style={`width: ${sheetWidth}px; height: ${sheetHeight}px;`}
            >
              <img
                class="page-image"
                src={toDataUrl(render)}
                alt={`Page ${pageIndex + 1}`}
                draggable="false"
                style={`width: ${imageWidth}px; height: ${imageHeight}px; transform: translate(-50%, -50%) rotate(${viewRotation}deg);`}
              />
            </div>
          {:else if loading}
            <div class="stage-state">
              <ViewerIcon name="document" size={42} />
              <strong>Loading page...</strong>
            </div>
          {:else if renderError}
            <div class="stage-state error">
              <ViewerIcon name="document" size={42} />
              <strong>Could not render this page</strong>
              <span>{renderError}</span>
            </div>
          {/if}
        </div>

        <div
          class="floating-toolbar"
          class:collapsed={dockCollapsed}
          style={dockPos
            ? `right:${dockPos.x}px; top:${dockPos.y}px; left:auto; bottom:auto;`
            : ""}
          bind:this={dockEl}
          aria-label="PDF controls"
        >
          <button
            class="dock-grip"
            title="Drag to move controls"
            aria-label="Drag to move controls"
            onpointerdown={startDockDrag}
            onpointermove={moveDockDrag}
            onpointerup={endDockDrag}
            onpointercancel={endDockDrag}
          >
            <span aria-hidden="true">⠿</span>
          </button>

          <div class="dock-body-wrap" class:collapsed={dockCollapsed} aria-hidden={dockCollapsed}>
          <div class="dock-body">
          <div class="toolbar-group">
            <button
              class="icon-button"
              title="Previous page"
              aria-label="Previous page"
              disabled={pageIndex === 0 || loading}
              onclick={() => loadPage(pageIndex - 1)}
            >
              <ViewerIcon name="chevron-left" />
            </button>
            <label class="page-jump">
              <span class="sr-only">Page</span>
              <input
                bind:value={pageText}
                inputmode="numeric"
                aria-label="Page number"
                onkeydown={(e) => {
                  if (e.key === "Enter") commitPageText();
                }}
                onblur={commitPageText}
              />
              <span>of {totalPages}</span>
            </label>
            <button
              class="icon-button"
              title="Next page"
              aria-label="Next page"
              disabled={pageIndex >= totalPages - 1 || loading}
              onclick={() => loadPage(pageIndex + 1)}
            >
              <ViewerIcon name="chevron-right" />
            </button>
          </div>

          <div class="toolbar-group">
            <button
              class="icon-button"
              title="Zoom out"
              aria-label="Zoom out"
              onclick={() => setZoom(zoom - 0.15)}
            >
              <ViewerIcon name="zoom-out" />
            </button>
            <button class="zoom-label" onclick={fitPage} title="Fit page">
              {fitMode ? "Fit" : zoomLabel}
            </button>
            <button
              class="icon-button"
              title="Zoom in"
              aria-label="Zoom in"
              onclick={() => setZoom(zoom + 0.15)}
            >
              <ViewerIcon name="zoom-in" />
            </button>
          </div>

          <div class="toolbar-group segmented" aria-label="Pointer mode">
            <button
              class:active={toolMode === "select"}
              title="Select"
              aria-label="Select"
              onclick={() => (toolMode = "select")}
            >
              <ViewerIcon name="cursor" />
            </button>
            <button
              class:active={toolMode === "hand"}
              title="Hand tool"
              aria-label="Hand tool"
              onclick={() => (toolMode = "hand")}
            >
              <ViewerIcon name="hand" />
            </button>
            <button title="Rotate view" aria-label="Rotate view" onclick={rotateView}>
              <ViewerIcon name="rotate" />
            </button>
          </div>
          </div>
          </div>

          <button
            class="icon-button dock-toggle"
            title={dockCollapsed ? "Show controls" : "Hide controls"}
            aria-label={dockCollapsed ? "Show controls" : "Hide controls"}
            aria-expanded={!dockCollapsed}
            onclick={toggleDock}
          >
            <ViewerIcon name={dockCollapsed ? "chevron-left" : "chevron-right"} />
          </button>
        </div>

        <footer class="statusbar">
          <span class="status-name" title={current.source_name}>{current.source_name}</span>
          <span class="status-page">Page {pageIndex + 1} of {totalPages}</span>
        </footer>
      </main>

      <aside class:collapsed={toolsCollapsed} class="sidebar tools-sidebar">
        <div class="sidebar-head">
          <div class="sidebar-title">
            <ViewerIcon name="viewer" />
            <span>Tools</span>
          </div>
          <button
            class="icon-button"
            title={toolsCollapsed ? "Show tools" : "Hide tools"}
            aria-label={toolsCollapsed ? "Show tools" : "Hide tools"}
            aria-expanded={!toolsCollapsed}
            onclick={() => (toolsCollapsed = !toolsCollapsed)}
          >
            <ViewerIcon name="panel-right" />
          </button>
        </div>

        {#if !toolsCollapsed}
          <div class="tool-list">
            {#each toolActions as tool (tool.key)}
              <button
                class:active={tool.route === "/view"}
                class="tool-row"
                onclick={() => goToTool(tool.route)}
              >
                <span class="tool-icon">
                  <ViewerIcon name={tool.icon} />
                </span>
                <span>
                  <strong>{tool.label}</strong>
                  <small>{tool.detail}</small>
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </aside>
    </div>
  {:else}
    <main class="empty-viewer">
      {#if notice}
        <div class="notice {notice.kind}" role="status">{notice.msg}</div>
      {/if}
      <div class="empty-card">
        <AppIcon size={70} title="" />
        <h1>Open a PDF</h1>
        <p class="muted">Read, zoom, browse pages, and jump into the editing tools from one place.</p>
        <div class="empty-actions">
          <button class="action-button primary" onclick={openNewPdf} disabled={opening}>
            <ViewerIcon name="folder-open" />
            <span>{opening ? "Opening..." : "Open PDF"}</span>
          </button>
          <button class="action-button" onclick={() => goto("/")}>
            <ViewerIcon name="home" />
            <span>Home</span>
          </button>
        </div>
      </div>
    </main>
  {/if}
</div>

<style>
  .viewer-screen {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #eef1f6;
    color: var(--text);
  }

  .topbar {
    flex: 0 0 auto;
    min-height: 58px;
    display: grid;
    grid-template-columns: auto minmax(220px, 1fr) auto;
    align-items: center;
    gap: 0.75rem;
    padding: 0.55rem 0.85rem;
    background: #f8fafc;
    border-bottom: 1px solid #d4dae6;
  }

  .action-button,
  .icon-button,
  .zoom-label,
  .segmented button,
  .thumbnail-button,
  .tool-row {
    border: 0;
    font: inherit;
  }

  .home-btn {
    min-height: 38px;
  }

  .file-chip {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.45rem;
    justify-self: start;
    max-width: 100%;
    padding: 0.42rem 0.7rem;
    background: #edf2fa;
    border: 1px solid #d3dbea;
    border-radius: 10px;
    color: #5a6478;
  }

  .file-chip strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }

  .top-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    justify-content: flex-end;
  }

  .action-button {
    min-height: 38px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    padding: 0.45rem 0.72rem;
    background: #fff;
    border: 1px solid #cfd7e6;
    border-radius: 8px;
    color: var(--text);
    font-weight: 700;
    box-shadow: 0 1px 2px rgba(27, 35, 51, 0.04);
  }

  .action-button:hover:not(:disabled),
  .icon-button:hover:not(:disabled),
  .zoom-label:hover {
    background: #edf3ff;
    border-color: #aebee0;
  }

  .action-button.primary {
    background: var(--primary);
    border-color: var(--primary);
    color: #fff;
  }

  .action-button.primary:hover:not(:disabled) {
    background: var(--primary-dark);
  }

  .action-button:disabled,
  .icon-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .icon-button {
    width: 36px;
    height: 36px;
    display: inline-grid;
    place-items: center;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: #263248;
  }

  .floating-toolbar {
    position: absolute;
    right: 1.1rem;
    bottom: 2.6rem;
    z-index: 6;
    display: flex;
    align-items: center;
    flex-wrap: nowrap;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.4rem;
    background: rgba(255, 255, 255, 0.96);
    border: 1px solid #d2d9e6;
    border-radius: 12px;
    box-shadow: 0 10px 30px rgba(27, 35, 51, 0.18);
    backdrop-filter: blur(6px);
  }

  /* Animated collapse: the grid column slides from 1fr → 0fr while the inner
     body is clipped, so the dock shrinks toward its pinned right edge. */
  .dock-body-wrap {
    display: grid;
    grid-template-columns: 1fr;
    min-width: 0;
    opacity: 1;
    transition:
      grid-template-columns 0.24s ease,
      opacity 0.2s ease,
      margin 0.24s ease;
  }

  .dock-body-wrap.collapsed {
    grid-template-columns: 0fr;
    opacity: 0;
    margin-left: -0.5rem;
    pointer-events: none;
  }

  .dock-body {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    overflow: hidden;
  }

  .dock-grip {
    width: 26px;
    height: 38px;
    display: inline-grid;
    place-items: center;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text-soft);
    font-size: 1.05rem;
    line-height: 1;
    cursor: grab;
    touch-action: none;
  }

  .dock-grip:hover {
    background: #edf3ff;
  }

  .dock-grip:active {
    cursor: grabbing;
  }

  .toolbar-group {
    min-height: 38px;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.15rem;
    background: #f3f6fb;
    border: 1px solid #dce2ec;
    border-radius: 10px;
  }

  .page-jump {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0 0.4rem;
    color: var(--text-soft);
    font-weight: 600;
  }

  .page-jump input {
    width: 3rem;
    height: 28px;
    border: 1px solid #cbd4e3;
    border-radius: 6px;
    background: #fff;
    color: var(--text);
    font: inherit;
    font-weight: 700;
    text-align: center;
  }

  .page-jump input:focus {
    outline: 2px solid #b8ceff;
    border-color: var(--primary);
  }

  .zoom-label {
    min-width: 58px;
    height: 30px;
    padding: 0 0.45rem;
    border: 1px solid transparent;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font-weight: 800;
  }

  .segmented button {
    width: 34px;
    height: 32px;
    display: inline-grid;
    place-items: center;
    border-radius: 7px;
    background: transparent;
    color: var(--text-soft);
  }

  .segmented button.active {
    background: #fff;
    color: var(--primary);
    box-shadow: inset 0 0 0 1px #cbd7f2;
  }

  .viewer-workspace {
    flex: 1 1 auto;
    min-height: 0;
    display: grid;
    grid-template-columns: 230px minmax(0, 1fr) 270px;
    transition: grid-template-columns 0.18s ease;
  }

  .viewer-workspace.pages-collapsed {
    grid-template-columns: 54px minmax(0, 1fr) 270px;
  }

  .viewer-workspace.tools-collapsed {
    grid-template-columns: 230px minmax(0, 1fr) 54px;
  }

  .viewer-workspace.pages-collapsed.tools-collapsed {
    grid-template-columns: 54px minmax(0, 1fr) 54px;
  }

  .sidebar {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #f8fafc;
    border-color: #d8deea;
  }

  .pages-sidebar {
    border-right: 1px solid #d8deea;
  }

  .tools-sidebar {
    border-left: 1px solid #d8deea;
  }

  .sidebar-head {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
    min-height: 54px;
    padding: 0.5rem 0.55rem 0.5rem 0.8rem;
    border-bottom: 1px solid #e0e5ee;
  }

  .sidebar-title {
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: 0.48rem;
    font-size: 1rem;
    font-weight: 800;
  }

  .sidebar.collapsed .sidebar-head {
    justify-content: center;
    padding: 0.5rem 0.25rem;
  }

  .sidebar.collapsed .sidebar-title {
    display: none;
  }

  .sidebar.collapsed.pages-sidebar .icon-button :global(svg),
  .sidebar.collapsed.tools-sidebar .icon-button :global(svg) {
    transform: rotate(180deg);
  }

  .page-count {
    flex: 0 0 auto;
    padding: 0.65rem 0.85rem 0.25rem;
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 700;
  }

  .thumbnail-list {
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    padding: 0.65rem 0.85rem 1rem;
  }

  .thumbnail-button {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.45rem;
    background: #fff;
    border: 1px solid #d9deea;
    border-radius: 8px;
    color: var(--text-soft);
    font-weight: 700;
    text-align: center;
  }

  .thumbnail-button:hover,
  .thumbnail-button.active {
    border-color: var(--primary);
    background: #edf3ff;
    color: var(--primary);
  }

  .document-panel {
    position: relative;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #dfe4ed;
  }

  .notice {
    flex: 0 0 auto;
    margin: 0.75rem 1rem 0;
    padding: 0.65rem 0.8rem;
    border-radius: 8px;
    font-weight: 700;
  }

  .notice.ok {
    background: #e6f4ec;
    color: var(--success);
  }

  .notice.err {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .document-stage {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    display: grid;
    place-items: start center;
    padding: 2rem;
    cursor: default;
  }

  .hand-mode .document-stage {
    cursor: grab;
  }

  .panning .document-stage {
    cursor: grabbing;
  }

  .page-sheet {
    position: relative;
    flex: 0 0 auto;
    margin: 0 auto;
    background: #fff;
    box-shadow: 0 8px 28px rgba(27, 35, 51, 0.24);
    overflow: hidden;
  }

  .page-image {
    position: absolute;
    top: 50%;
    left: 50%;
    transform-origin: center;
    user-select: none;
    pointer-events: none;
  }

  .stage-state {
    align-self: center;
    justify-self: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-soft);
    text-align: center;
  }

  .stage-state.error {
    max-width: 32rem;
    color: var(--danger);
  }

  .statusbar {
    flex: 0 0 auto;
    min-height: 34px;
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 0.35rem 1.1rem;
    background: #f8fafc;
    border-top: 1px solid #d8deea;
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 700;
  }

  .status-name {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-page {
    flex: 0 0 auto;
    white-space: nowrap;
  }

  .tool-list {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.8rem;
    overflow-y: auto;
  }

  .tool-row {
    display: grid;
    grid-template-columns: 38px minmax(0, 1fr);
    align-items: center;
    gap: 0.65rem;
    padding: 0.62rem;
    border: 1px solid #d9deea;
    border-radius: 8px;
    background: #fff;
    color: var(--text);
    text-align: left;
  }

  .tool-row:hover,
  .tool-row.active {
    border-color: #aec0e7;
    background: #edf3ff;
  }

  .tool-icon {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: #edf2fa;
    color: var(--primary);
  }

  .tool-row strong,
  .tool-row small {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tool-row small {
    margin-top: 0.12rem;
    color: var(--text-soft);
    font-size: 0.78rem;
  }

  .empty-viewer {
    flex: 1 1 auto;
    min-height: 0;
    display: grid;
    place-items: center;
    padding: 1.5rem;
  }

  .empty-card {
    width: min(34rem, 100%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.8rem;
    text-align: center;
  }

  .empty-card h1 {
    font-size: 2rem;
  }

  .empty-card p {
    max-width: 28rem;
    margin: 0;
    line-height: 1.45;
  }

  .empty-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    justify-content: center;
    margin-top: 0.4rem;
  }

  @media (max-width: 980px) {
    .topbar {
      grid-template-columns: 1fr;
    }

    .top-actions,
    .file-chip {
      justify-self: stretch;
    }

    .top-actions {
      justify-content: flex-start;
      flex-wrap: wrap;
    }

    .viewer-workspace,
    .viewer-workspace.pages-collapsed,
    .viewer-workspace.tools-collapsed,
    .viewer-workspace.pages-collapsed.tools-collapsed {
      grid-template-columns: minmax(0, 1fr);
    }

    .sidebar {
      display: none;
    }
  }

  @media (max-width: 720px) {
    .document-stage {
      padding: 1rem;
    }

    .statusbar {
      flex-wrap: wrap;
      justify-content: flex-start;
    }
  }

  @media print {
    .topbar,
    .floating-toolbar,
    .sidebar,
    .statusbar,
    .notice {
      display: none !important;
    }

    .viewer-workspace {
      display: block;
    }

    .document-panel,
    .document-stage {
      display: block;
      overflow: visible;
      background: #fff;
      padding: 0;
    }

    .page-sheet {
      width: 100% !important;
      height: auto !important;
      box-shadow: none;
      overflow: visible;
    }

    .page-image {
      position: static;
      width: 100% !important;
      height: auto !important;
      transform: none !important;
    }
  }
</style>
