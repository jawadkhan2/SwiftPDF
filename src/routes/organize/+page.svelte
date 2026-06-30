<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { dndzone } from "svelte-dnd-action";
  import { flip } from "svelte/animate";
  import Thumbnail from "$lib/components/Thumbnail.svelte";
  import { doc } from "$lib/stores/document.svelte";
  import { saveBuiltPdf, editedName, type PagePlan } from "$lib/api";

  // One editable page in the working set. `page` is the index into the original
  // document; `rotation` is the desired ABSOLUTE rotation in the output.
  interface Item {
    id: string;
    page: number;
    rotation: number;
  }

  const current = $derived(doc.current);

  let items = $state<Item[]>([]);
  let selected = $state<Set<string>>(new Set());
  let history = $state<Item[][]>([]);
  let saving = $state(false);
  let toast = $state<{ kind: "ok" | "err"; msg: string } | null>(null);

  const flipMs = 180;

  onMount(() => {
    if (!doc.current) {
      goto("/");
      return;
    }
    // Seed the working set: one item per original page, rotation = its baked-in
    // rotation (so "no change" round-trips identically).
    items = doc.current.pages.map((p, i) => ({
      id: `p${i}`,
      page: i,
      rotation: p.rotation,
    }));
  });

  // Original (baked-in) rotation for a page, used to compute the CSS preview delta.
  function baseRotation(page: number): number {
    return current?.pages[page]?.rotation ?? 0;
  }

  function snapshot() {
    // Deep-copy current items onto the undo stack (cap history depth).
    history = [...history, items.map((it) => ({ ...it }))].slice(-50);
  }

  function undo() {
    const prev = history.at(-1);
    if (!prev) return;
    items = prev;
    history = history.slice(0, -1);
    selected = new Set();
  }

  function toggleSelect(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected = next;
  }

  function rotateItem(id: string) {
    snapshot();
    items = items.map((it) =>
      it.id === id ? { ...it, rotation: (it.rotation + 90) % 360 } : it,
    );
  }

  function deleteItem(id: string) {
    if (items.length <= 1) {
      toast = { kind: "err", msg: "A document needs at least one page." };
      return;
    }
    snapshot();
    items = items.filter((it) => it.id !== id);
    const next = new Set(selected);
    next.delete(id);
    selected = next;
  }

  function rotateSelected() {
    if (selected.size === 0) return;
    snapshot();
    items = items.map((it) =>
      selected.has(it.id) ? { ...it, rotation: (it.rotation + 90) % 360 } : it,
    );
  }

  function deleteSelected() {
    if (selected.size === 0) return;
    if (selected.size >= items.length) {
      toast = { kind: "err", msg: "You can't delete every page." };
      return;
    }
    snapshot();
    items = items.filter((it) => !selected.has(it.id));
    selected = new Set();
  }

  // svelte-dnd-action handlers.
  function onConsider(e: CustomEvent<{ items: Item[] }>) {
    items = e.detail.items;
  }
  function onFinalize(e: CustomEvent<{ items: Item[] }>) {
    snapshot();
    items = e.detail.items;
  }

  async function save() {
    if (!current) return;
    saving = true;
    toast = null;
    try {
      const plan: PagePlan[] = items.map((it) => ({
        page: it.page,
        rotation: it.rotation,
      }));
      const savedPath = await saveBuiltPdf(
        current.doc_id,
        plan,
        editedName(current.source_name),
      );
      if (savedPath) {
        const name = savedPath.split(/[\\/]/).pop() ?? savedPath;
        toast = { kind: "ok", msg: `Saved as ${name}` };
      }
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
      <strong>Organize pages</strong>
      {#if current}
        <span class="muted">{current.source_name} · {items.length} pages</span>
      {/if}
    </div>
    <div class="spacer"></div>
    {#if selected.size > 0}
      <span class="selcount">{selected.size} selected</span>
      <button class="btn" onclick={rotateSelected}>⟳ Rotate</button>
      <button class="btn btn-danger" onclick={deleteSelected}>🗑 Delete</button>
    {/if}
    <button class="btn" onclick={undo} disabled={history.length === 0}>↶ Undo</button>
    <button class="btn btn-primary" onclick={save} disabled={saving}>
      {saving ? "Saving…" : "Save a copy"}
    </button>
  </header>

  <div class="hint">
    <span class="hint-icon" aria-hidden="true">↕</span>
    <span>
      Drag pages to reorder · click a page to select · use the ⟳ and 🗑 buttons
      per page. Your original file is never changed.
    </span>
  </div>

  {#if toast}
    <div class="banner {toast.kind}" role="status">{toast.msg}</div>
  {/if}

  {#if current}
    <section
      class="grid"
      use:dndzone={{ items, flipDurationMs: flipMs, dropTargetStyle: {} }}
      onconsider={onConsider}
      onfinalize={onFinalize}
    >
      {#each items as item (item.id)}
        <div class="card" animate:flip={{ duration: flipMs }}>
          <button
            class="select-area"
            class:selected={selected.has(item.id)}
            onclick={() => toggleSelect(item.id)}
            aria-pressed={selected.has(item.id)}
            title="Click to select"
          >
            <Thumbnail
              docId={current.doc_id}
              page={item.page}
              rotate={item.rotation - baseRotation(item.page)}
            />
          </button>
          <div class="card-bar">
            <span class="pagenum">{items.indexOf(item) + 1}</span>
            <div class="card-actions">
              <button
                class="icon-btn"
                title="Rotate this page"
                onclick={() => rotateItem(item.id)}>⟳</button
              >
              <button
                class="icon-btn danger"
                title="Delete this page"
                onclick={() => deleteItem(item.id)}>🗑</button
              >
            </div>
          </div>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .screen {
    height: 100%;
    min-height: 0;
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
    line-height: 1.3;
  }
  .spacer {
    flex: 1;
  }
  .selcount {
    color: var(--text-soft);
    font-size: 0.9rem;
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    margin: 0;
    padding: 0.6rem 1.2rem;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
    color: var(--text-soft);
    font-size: 0.88rem;
  }
  .hint-icon {
    display: inline-grid;
    place-items: center;
    width: 1.4rem;
    height: 1.4rem;
    flex: 0 0 auto;
    border-radius: 6px;
    background: var(--primary-soft);
    color: var(--primary);
    font-weight: 700;
  }
  .grid {
    flex: 1;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 1rem;
    padding: 1.4rem 1.2rem 2rem;
    align-content: start;
  }
  .card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .select-area {
    border: 2px solid transparent;
    border-radius: 8px;
    padding: 0.3rem;
    background: var(--surface);
    cursor: pointer;
  }
  .select-area.selected {
    border-color: var(--primary);
    background: var(--primary-soft);
  }
  .card-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .pagenum {
    font-size: 0.85rem;
    color: var(--text-soft);
    font-weight: 600;
  }
  .card-actions {
    display: flex;
    gap: 0.3rem;
  }
  .icon-btn {
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 6px;
    width: 28px;
    height: 28px;
    font-size: 0.9rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .icon-btn:hover {
    background: var(--surface-2);
  }
  .icon-btn.danger:hover {
    background: var(--danger-soft);
    color: var(--danger);
    border-color: var(--danger);
  }
  .banner {
    margin: 0.8rem 1.2rem 0;
    padding: 0.7rem 1rem;
    border-radius: var(--radius-sm);
  }
  .banner.ok {
    background: #e6f4ec;
    color: var(--success);
  }
  .banner.err {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
