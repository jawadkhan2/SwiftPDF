<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { doc } from "$lib/stores/document.svelte";
  import {
    renderPage,
    toDataUrl,
    saveSignedPdf,
    editedName,
    type RenderResult,
    type Stamp,
  } from "$lib/api";
  import SignatureModal from "$lib/components/SignatureModal.svelte";

  type ShapeKind = "check" | "x" | "circle" | "square" | "line" | "dot";

  type EditorStamp =
    | {
        id: string;
        page: number;
        type: "text";
        fx: number;
        fy: number;
        fontFrac: number;
        text: string;
        color: [number, number, number];
      }
    | {
        id: string;
        page: number;
        type: "image";
        fx: number;
        fy: number;
        fw: number;
        fh: number;
        aspect: number; // natural width / height of the image
        dataUrl: string;
      }
    | {
        id: string;
        page: number;
        type: "shape";
        fx: number;
        fy: number;
        fw: number;
        fh: number;
        aspect: number; // kept at 1 → square box on screen
        shape: ShapeKind;
        color: [number, number, number];
      };

  const current = $derived(doc.current);

  let pageIndex = $state(0);
  let render = $state<RenderResult | null>(null);
  let loading = $state(false);
  let stamps = $state<EditorStamp[]>([]);
  let selectedId = $state<string | null>(null);
  let showSigModal = $state(false);
  let showShapeMenu = $state(false);
  let saving = $state(false);
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  // Context menu (right click) + where a queued signature should land.
  let ctxMenu = $state<{ x: number; y: number; fx: number; fy: number } | null>(
    null,
  );
  let pendingPos: { fx: number; fy: number } | null = null;
  // Newly added text we want to focus + select once the DOM exists.
  let focusId = $state<string | null>(null);

  let pageEl = $state<HTMLDivElement | null>(null);
  let stageEl = $state<HTMLDivElement | null>(null);
  let pageW = $state(1);
  let pageH = $state(1);

  const colors: [number, number, number][] = [
    [17, 17, 17],
    [37, 99, 246],
    [206, 42, 42],
  ];

  const shapeChoices: { kind: ShapeKind; label: string }[] = [
    { kind: "check", label: "Check" },
    { kind: "x", label: "Cross" },
    { kind: "circle", label: "Circle" },
    { kind: "square", label: "Square" },
    { kind: "line", label: "Line" },
    { kind: "dot", label: "Dot" },
  ];

  let nextId = 0;
  const newId = () => `s${nextId++}`;
  const clamp = (v: number, lo: number, hi: number) =>
    Math.min(hi, Math.max(lo, v));

  function measure() {
    if (pageEl) {
      pageW = pageEl.clientWidth;
      pageH = pageEl.clientHeight;
    }
  }

  onMount(() => {
    if (!doc.current) {
      goto("/");
      return;
    }
    void loadPage(0);

    const ro = new ResizeObserver(measure);
    if (pageEl) ro.observe(pageEl);
    return () => ro.disconnect();
  });

  async function loadPage(i: number) {
    if (!current) return;
    loading = true;
    selectedId = null;
    try {
      render = await renderPage(current.doc_id, i, 1500);
      pageIndex = i;
      queueMicrotask(measure);
    } finally {
      loading = false;
    }
  }

  const visible = $derived(stamps.filter((s) => s.page === pageIndex));

  // Focus + select a freshly added text box so typing replaces the placeholder.
  $effect(() => {
    if (!focusId) return;
    const id = focusId;
    queueMicrotask(() => {
      const el = document.getElementById(`tb-${id}`);
      if (el) {
        el.focus();
        const r = document.createRange();
        r.selectNodeContents(el);
        const sel = getSelection();
        sel?.removeAllRanges();
        sel?.addRange(r);
      }
    });
    focusId = null;
  });

  // --- Placement helpers ----------------------------------------------------
  // Fraction (0..1) of the page that is currently centered in the viewport, so
  // new items appear where the user is looking instead of at a fixed corner.
  function viewportCenterFrac(): { fx: number; fy: number } {
    if (pageEl && stageEl) {
      const pr = pageEl.getBoundingClientRect();
      const sr = stageEl.getBoundingClientRect();
      const cx = sr.left + sr.width / 2;
      const cy = sr.top + sr.height / 2;
      return {
        fx: clamp((cx - pr.left) / pr.width, 0.02, 0.95),
        fy: clamp((cy - pr.top) / pr.height, 0.02, 0.95),
      };
    }
    return { fx: 0.4, fy: 0.45 };
  }

  // --- Adding items ---------------------------------------------------------
  function addText(pos?: { fx: number; fy: number }) {
    const c = pos ?? viewportCenterFrac();
    // Default to a 16px label at the current display height.
    const fontFrac = pageH ? clamp(16 / pageH, 0.012, 0.12) : 0.02;
    const s: EditorStamp = {
      id: newId(),
      page: pageIndex,
      type: "text",
      fx: c.fx,
      fy: c.fy,
      fontFrac,
      text: "Type here",
      color: colors[0],
    };
    stamps = [...stamps, s];
    selectedId = s.id;
    focusId = s.id;
  }

  function addShape(kind: ShapeKind, pos?: { fx: number; fy: number }) {
    showShapeMenu = false;
    const c = pos ?? viewportCenterFrac();
    const fw = 0.1;
    const fh = pageH ? (fw * pageW) / pageH : fw; // square box on screen
    const s: EditorStamp = {
      id: newId(),
      page: pageIndex,
      type: "shape",
      fx: clamp(c.fx - fw / 2, 0, 0.95),
      fy: clamp(c.fy - fh / 2, 0, 0.95),
      fw,
      fh,
      aspect: 1,
      shape: kind,
      color: colors[0],
    };
    stamps = [...stamps, s];
    selectedId = s.id;
  }

  function onSignaturePicked(dataUrl: string) {
    showSigModal = false;
    const pos = pendingPos ?? viewportCenterFrac();
    pendingPos = null;
    const img = new Image();
    img.onload = () => {
      const aspect = img.naturalWidth / Math.max(img.naturalHeight, 1);
      const fw = 0.28;
      // Preserve the image's aspect within the page box (uses display aspect).
      const fh = (fw * (pageW / pageH)) / aspect;
      const s: EditorStamp = {
        id: newId(),
        page: pageIndex,
        type: "image",
        fx: clamp(pos.fx - fw / 2, 0, 0.95),
        fy: clamp(pos.fy - fh / 2, 0, 0.95),
        fw,
        fh,
        aspect,
        dataUrl,
      };
      stamps = [...stamps, s];
      selectedId = s.id;
    };
    img.src = dataUrl;
  }

  function deleteStamp(id: string) {
    stamps = stamps.filter((s) => s.id !== id);
    if (selectedId === id) selectedId = null;
  }

  function setText(id: string, value: string) {
    const s = stamps.find((x) => x.id === id);
    if (s && s.type === "text") s.text = value;
  }

  function setColor(id: string, c: [number, number, number]) {
    const s = stamps.find((x) => x.id === id);
    if (s && (s.type === "text" || s.type === "shape")) s.color = c;
  }

  function bumpSize(id: string, delta: number) {
    const s = stamps.find((x) => x.id === id);
    if (s && s.type === "text")
      s.fontFrac = clamp(s.fontFrac + delta, 0.012, 0.12);
  }

  // Uncontrolled contenteditable: seed the text once. We never write the value
  // back into the DOM, which is what caused the caret to reset to the start and
  // the first run of characters to come out reversed.
  function textInit(node: HTMLElement, value: string) {
    node.textContent = value;
  }

  // --- Context menu (right click) ------------------------------------------
  function onContext(e: MouseEvent) {
    e.preventDefault();
    if (!pageEl) return;
    const r = pageEl.getBoundingClientRect();
    ctxMenu = {
      x: e.clientX,
      y: e.clientY,
      fx: clamp((e.clientX - r.left) / r.width, 0, 0.98),
      fy: clamp((e.clientY - r.top) / r.height, 0, 0.98),
    };
  }

  function ctxAddText() {
    if (ctxMenu) addText({ fx: ctxMenu.fx, fy: ctxMenu.fy });
    ctxMenu = null;
  }

  function ctxAddSignature() {
    if (ctxMenu) pendingPos = { fx: ctxMenu.fx, fy: ctxMenu.fy };
    ctxMenu = null;
    showSigModal = true;
  }

  function ctxAddShape(kind: ShapeKind) {
    if (ctxMenu) addShape(kind, { fx: ctxMenu.fx, fy: ctxMenu.fy });
    ctxMenu = null;
  }

  // --- Drag to move ---------------------------------------------------------
  let drag: { id: string; dx: number; dy: number } | null = null;

  function startMove(e: PointerEvent, s: EditorStamp) {
    if (!pageEl) return;
    const r = pageEl.getBoundingClientRect();
    const px = (e.clientX - r.left) / r.width;
    const py = (e.clientY - r.top) / r.height;
    drag = { id: s.id, dx: px - s.fx, dy: py - s.fy };
    selectedId = s.id;
    (e.target as Element).setPointerCapture(e.pointerId);
    e.preventDefault();
    e.stopPropagation();
  }

  // --- Resize (images + shapes) --------------------------------------------
  let resize: { id: string; startX: number; startFw: number } | null = null;

  function startResize(e: PointerEvent, s: EditorStamp) {
    if (s.type !== "image" && s.type !== "shape") return;
    resize = { id: s.id, startX: e.clientX, startFw: s.fw };
    (e.target as Element).setPointerCapture(e.pointerId);
    e.preventDefault();
    e.stopPropagation();
  }

  function onPagePointerMove(e: PointerEvent) {
    if (!pageEl) return;
    const r = pageEl.getBoundingClientRect();
    if (drag) {
      const px = (e.clientX - r.left) / r.width;
      const py = (e.clientY - r.top) / r.height;
      const s = stamps.find((x) => x.id === drag!.id);
      if (s) {
        s.fx = clamp(px - drag.dx, 0, 0.99);
        s.fy = clamp(py - drag.dy, 0, 0.99);
      }
    } else if (resize) {
      const s = stamps.find((x) => x.id === resize!.id);
      if (s && (s.type === "image" || s.type === "shape")) {
        const dw = (e.clientX - resize.startX) / r.width;
        s.fw = clamp(resize.startFw + dw, 0.04, 1);
        s.fh = (s.fw * (pageW / pageH)) / s.aspect;
      }
    }
  }

  function endPointer() {
    drag = null;
    resize = null;
  }

  // --- Shapes ---------------------------------------------------------------
  // Single source of truth for a shape's geometry: rendered inline as SVG for
  // the on-screen overlay and rasterized from the same markup at save time.
  function shapeSvg(
    kind: ShapeKind,
    color: [number, number, number],
    size: string | number = "100%",
  ): string {
    const col = `rgb(${color[0]},${color[1]},${color[2]})`;
    const sw = 9;
    const stroke = `stroke="${col}" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round" fill="none"`;
    let body = "";
    switch (kind) {
      case "check":
        body = `<polyline points="18,53 40,74 82,26" ${stroke}/>`;
        break;
      case "x":
        body = `<path d="M24 24 L76 76 M76 24 L24 76" ${stroke}/>`;
        break;
      case "circle":
        body = `<circle cx="50" cy="50" r="40" ${stroke}/>`;
        break;
      case "square":
        body = `<rect x="12" y="12" width="76" height="76" rx="4" ${stroke}/>`;
        break;
      case "line":
        body = `<line x1="12" y1="50" x2="88" y2="50" ${stroke}/>`;
        break;
      case "dot":
        body = `<circle cx="50" cy="50" r="34" fill="${col}"/>`;
        break;
    }
    return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="${size}" height="${size}">${body}</svg>`;
  }

  function shapeToPng(
    kind: ShapeKind,
    color: [number, number, number],
  ): Promise<string> {
    return new Promise((resolve, reject) => {
      const px = 600;
      const svg = shapeSvg(kind, color, px);
      const url = "data:image/svg+xml;base64," + btoa(svg);
      const img = new Image();
      img.onload = () => {
        const cv = document.createElement("canvas");
        cv.width = px;
        cv.height = px;
        const ctx = cv.getContext("2d");
        if (!ctx) return reject("no 2d context");
        ctx.drawImage(img, 0, 0, px, px);
        resolve(cv.toDataURL("image/png"));
      };
      img.onerror = () => reject("shape render failed");
      img.src = url;
    });
  }

  // --- Save -----------------------------------------------------------------
  async function save() {
    if (!current) return;
    const payload: Stamp[] = [];
    for (const s of stamps) {
      if (s.type === "text") {
        if (!s.text.trim()) continue;
        payload.push({
          kind: "Text",
          page: s.page,
          fx: s.fx,
          fy: s.fy,
          fh: s.fontFrac,
          text: s.text,
          color: s.color,
        });
      } else if (s.type === "image") {
        payload.push({
          kind: "Image",
          page: s.page,
          fx: s.fx,
          fy: s.fy,
          fw: s.fw,
          fh: s.fh,
          png_base64: s.dataUrl.split(",")[1] ?? "",
        });
      } else {
        // Shape → rasterize to a PNG and reuse the image stamp pipeline.
        const dataUrl = await shapeToPng(s.shape, s.color);
        payload.push({
          kind: "Image",
          page: s.page,
          fx: s.fx,
          fy: s.fy,
          fw: s.fw,
          fh: s.fh,
          png_base64: dataUrl.split(",")[1] ?? "",
        });
      }
    }
    if (payload.length === 0) {
      toast = { kind: "err", msg: "Add some text, a shape or a signature first." };
      return;
    }
    saving = true;
    toast = null;
    try {
      const path = await saveSignedPdf(
        current.doc_id,
        payload,
        editedName(current.source_name),
      );
      if (path) toast = { kind: "ok", msg: `Saved a copy to ${path}` };
    } catch (e) {
      toast = { kind: "err", msg: String(e) };
    } finally {
      saving = false;
    }
  }
</script>

<div class="screen">
  <header class="toolbar">
    <button class="btn btn-ghost" onclick={() => goto("/")}>← Home</button>
    <div class="title">
      <strong>Fill &amp; Sign</strong>
      {#if current}<span class="muted">{current.source_name}</span>{/if}
    </div>
    <div class="spacer"></div>
    <button class="btn" onclick={() => addText()}>+ Text</button>
    <div class="shape-pick">
      <button class="btn" onclick={() => (showShapeMenu = !showShapeMenu)}
        >◆ Shape ▾</button
      >
      {#if showShapeMenu}
        <div class="shape-menu">
          {#each shapeChoices as sc (sc.kind)}
            <button
              class="shape-opt"
              title={sc.label}
              onclick={() => addShape(sc.kind)}
            >
              {@html shapeSvg(sc.kind, colors[0], 22)}
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <button
      class="btn"
      onclick={() => {
        pendingPos = null;
        showSigModal = true;
      }}>✎ Signature</button
    >
    <button class="btn btn-primary" onclick={save} disabled={saving}>
      {saving ? "Saving…" : "Save a copy"}
    </button>
  </header>

  {#if toast}
    <div class="banner {toast.kind}" role="status">{toast.msg}</div>
  {/if}

  {#if current}
    <div class="stage" bind:this={stageEl}>
      <div class="page-wrap">
        {#if render}
          <div
            class="page"
            bind:this={pageEl}
            role="application"
            aria-label="Page editor — drag items to position them"
            style={`aspect-ratio: ${render.width_px} / ${render.height_px};`}
            onpointerdown={() => {
              selectedId = null;
              ctxMenu = null;
              showShapeMenu = false;
            }}
            onpointermove={onPagePointerMove}
            onpointerup={endPointer}
            onpointerleave={endPointer}
            oncontextmenu={onContext}
          >
            <img
              class="bg"
              src={toDataUrl(render)}
              alt={`Page ${pageIndex + 1}`}
              draggable="false"
            />

            {#each visible as s (s.id)}
              <div
                class="stamp"
                class:selected={selectedId === s.id}
                style={`left:${s.fx * 100}%; top:${s.fy * 100}%;` +
                  (s.type === "image" || s.type === "shape"
                    ? `width:${s.fw * 100}%; height:${s.fh * 100}%;`
                    : "")}
                onpointerdown={(e) => {
                  e.stopPropagation();
                  selectedId = s.id;
                }}
                role="button"
                tabindex="0"
              >
                <!-- Fixed-layout control bar: grip + delete never overlap,
                     even when the content is a single character. -->
                <div class="bar" contenteditable="false">
                  <span
                    class="grip"
                    role="button"
                    tabindex="-1"
                    aria-label="Move"
                    title="Drag to move"
                    onpointerdown={(e) => startMove(e, s)}>⠿</span
                  >
                  <button
                    class="del"
                    title="Remove"
                    onpointerdown={(e) => {
                      e.stopPropagation();
                      deleteStamp(s.id);
                    }}>✕</button
                  >
                </div>

                {#if s.type === "text"}
                  <div
                    id={`tb-${s.id}`}
                    class="text-body"
                    contenteditable="true"
                    use:textInit={s.text}
                    style={`font-size:${s.fontFrac * pageH}px; color: rgb(${s.color[0]},${s.color[1]},${s.color[2]});`}
                    oninput={(e) =>
                      setText(s.id, (e.currentTarget as HTMLElement).innerText)}
                    onfocus={() => (selectedId = s.id)}
                  ></div>

                  {#if selectedId === s.id}
                    <div class="text-tools" contenteditable="false">
                      <button
                        onpointerdown={(e) => {
                          e.stopPropagation();
                          bumpSize(s.id, -0.005);
                        }}>A−</button
                      >
                      <button
                        onpointerdown={(e) => {
                          e.stopPropagation();
                          bumpSize(s.id, 0.005);
                        }}>A+</button
                      >
                      {#each colors as c (c.join())}
                        <button
                          class="swatch"
                          style={`background: rgb(${c[0]},${c[1]},${c[2]})`}
                          aria-label="text colour"
                          onpointerdown={(e) => {
                            e.stopPropagation();
                            setColor(s.id, c);
                          }}
                        ></button>
                      {/each}
                    </div>
                  {/if}
                {:else if s.type === "image"}
                  <img class="sig" src={s.dataUrl} alt="signature" draggable="false" />
                  {#if selectedId === s.id}
                    <span
                      class="resize"
                      role="button"
                      tabindex="-1"
                      aria-label="Resize"
                      title="Drag to resize"
                      onpointerdown={(e) => startResize(e, s)}
                    ></span>
                  {/if}
                {:else}
                  <div class="shape-box">{@html shapeSvg(s.shape, s.color)}</div>
                  {#if selectedId === s.id}
                    <div class="text-tools" contenteditable="false">
                      {#each colors as c (c.join())}
                        <button
                          class="swatch"
                          style={`background: rgb(${c[0]},${c[1]},${c[2]})`}
                          aria-label="shape colour"
                          onpointerdown={(e) => {
                            e.stopPropagation();
                            setColor(s.id, c);
                          }}
                        ></button>
                      {/each}
                    </div>
                    <span
                      class="resize"
                      role="button"
                      tabindex="-1"
                      aria-label="Resize"
                      title="Drag to resize"
                      onpointerdown={(e) => startResize(e, s)}
                    ></span>
                  {/if}
                {/if}
              </div>
            {/each}
          </div>
        {:else if loading}
          <div class="placeholder muted">Loading page…</div>
        {/if}
      </div>

      {#if current.page_count > 1}
        <div class="pager">
          <button
            class="btn btn-ghost"
            disabled={pageIndex === 0 || loading}
            onclick={() => loadPage(pageIndex - 1)}>‹ Prev</button
          >
          <span class="muted">Page {pageIndex + 1} of {current.page_count}</span>
          <button
            class="btn btn-ghost"
            disabled={pageIndex >= current.page_count - 1 || loading}
            onclick={() => loadPage(pageIndex + 1)}>Next ›</button
          >
        </div>
      {/if}

      <p class="hint muted">
        Add text, a shape or a signature, drag to position, then “Save a copy”.
        Right-click the page to drop an item where you click. Your original file
        is never changed.
      </p>
    </div>
  {/if}
</div>

{#if ctxMenu}
  <!-- backdrop closes the menu on any outside click -->
  <div
    class="ctx-backdrop"
    role="presentation"
    onpointerdown={() => (ctxMenu = null)}
    oncontextmenu={(e) => {
      e.preventDefault();
      ctxMenu = null;
    }}
  ></div>
  <div class="ctx-menu" style={`left:${ctxMenu.x}px; top:${ctxMenu.y}px;`}>
    <button onclick={ctxAddText}>Add text here</button>
    <button onclick={ctxAddSignature}>Add signature here</button>
    <div class="ctx-sep"></div>
    <div class="ctx-shapes">
      {#each shapeChoices as sc (sc.kind)}
        <button
          class="shape-opt"
          title={`Add ${sc.label}`}
          onclick={() => ctxAddShape(sc.kind)}
        >
          {@html shapeSvg(sc.kind, colors[0], 20)}
        </button>
      {/each}
    </div>
  </div>
{/if}

{#if showSigModal}
  <SignatureModal
    onpick={onSignaturePicked}
    onclose={() => {
      showSigModal = false;
      pendingPos = null;
    }}
  />
{/if}

<style>
  .screen {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 1.2rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .title {
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .spacer {
    flex: 1;
  }
  .shape-pick {
    position: relative;
  }
  .shape-menu {
    position: absolute;
    top: calc(100% + 0.3rem);
    right: 0;
    z-index: 30;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.25rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.35rem;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.16);
  }
  .shape-opt {
    width: 2.2rem;
    height: 2.2rem;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 6px;
    cursor: pointer;
  }
  .shape-opt:hover {
    background: var(--border);
  }
  .stage {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1rem;
    gap: 0.8rem;
  }
  .pager {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .page-wrap {
    width: 100%;
    display: flex;
    justify-content: center;
  }
  .page {
    position: relative;
    width: min(800px, 100%);
    background: #fff;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.16);
    user-select: none;
  }
  .bg {
    display: block;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .placeholder {
    padding: 4rem;
  }
  .stamp {
    position: absolute;
    min-width: 1rem;
    min-height: 1rem;
  }
  .stamp.selected {
    outline: 1.5px dashed var(--primary);
    outline-offset: 2px;
  }
  .bar {
    position: absolute;
    top: -1.7rem;
    left: 0;
    display: flex;
    gap: 0.25rem;
    align-items: center;
    opacity: 0;
    pointer-events: none;
  }
  .stamp:hover .bar,
  .stamp.selected .bar {
    opacity: 1;
    pointer-events: auto;
  }
  .grip {
    cursor: grab;
    background: var(--primary);
    color: #fff;
    border-radius: 4px;
    padding: 0 0.35rem;
    font-size: 0.8rem;
    line-height: 1.3rem;
    height: 1.3rem;
    touch-action: none;
  }
  .del {
    border: none;
    background: var(--danger);
    color: #fff;
    border-radius: 4px;
    width: 1.3rem;
    height: 1.3rem;
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0;
  }
  .text-body {
    white-space: nowrap;
    outline: none;
    line-height: 1.1;
    cursor: text;
    padding: 1px 2px;
    min-width: 0.5rem;
  }
  .text-tools {
    position: absolute;
    bottom: -2.4rem;
    left: 0;
    display: flex;
    gap: 0.25rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.2rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
    z-index: 5;
  }
  .text-tools button {
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 4px;
    min-width: 1.6rem;
    height: 1.6rem;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .swatch {
    width: 1.6rem;
    padding: 0;
  }
  .sig {
    width: 100%;
    height: 100%;
    object-fit: fill;
    pointer-events: none;
  }
  .shape-box {
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .shape-box :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .resize {
    position: absolute;
    right: -7px;
    bottom: -7px;
    width: 14px;
    height: 14px;
    background: var(--primary);
    border: 2px solid #fff;
    border-radius: 50%;
    cursor: nwse-resize;
    touch-action: none;
  }
  .hint {
    font-size: 0.85rem;
    text-align: center;
    max-width: 32rem;
  }
  .banner {
    margin: 0.8rem 1.2rem 0;
    padding: 0.7rem 1rem;
    border-radius: var(--radius-sm);
    align-self: stretch;
  }
  .banner.ok {
    background: #e6f4ec;
    color: var(--success);
  }
  .banner.err {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .ctx-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .ctx-menu {
    position: fixed;
    z-index: 41;
    min-width: 11rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.3rem;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
  }
  .ctx-menu > button {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    padding: 0.45rem 0.6rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
  }
  .ctx-menu > button:hover {
    background: var(--border);
  }
  .ctx-sep {
    height: 1px;
    background: var(--border);
    margin: 0.3rem 0;
  }
  .ctx-shapes {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 0.2rem;
  }
  .ctx-shapes .shape-opt {
    width: 100%;
    height: 1.9rem;
  }
</style>
