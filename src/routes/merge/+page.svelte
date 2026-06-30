<script lang="ts">
  import { goto } from "$app/navigation";
  import { dndzone } from "svelte-dnd-action";
  import { flip } from "svelte/animate";
  import { onMount } from "svelte";
  import { pickPdfs, mergePdfs } from "$lib/api";
  import {
    fileNameFromPath,
    listenForFileDrops,
    splitPdfPaths,
  } from "$lib/fileDrop";
  import { takeMergeDrop } from "$lib/stores/droppedFiles";
  import DropOverlay from "$lib/components/DropOverlay.svelte";
  import { setDoc } from "$lib/stores/document.svelte";

  interface FileItem {
    id: string;
    path: string;
    name: string;
  }

  let files = $state<FileItem[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let dragActive = $state(false);
  const flipMs = 160;

  let nextId = 0;

  function fileItem(path: string): FileItem {
    return {
      id: `f${nextId++}`,
      path,
      name: fileNameFromPath(path),
    };
  }

  function addPaths(paths: string[]) {
    if (busy) return;

    const { pdfs, rejected } = splitPdfPaths(paths);

    if (pdfs.length > 0) {
      files = [...files, ...pdfs.map(fileItem)];
    }

    if (rejected.length > 0) {
      error =
        pdfs.length > 0
          ? `Skipped ${rejected.length} non-PDF file${rejected.length === 1 ? "" : "s"}.`
          : "Drop PDF files to add them.";
    } else {
      error = null;
    }
  }

  async function addFiles() {
    error = null;
    const picked = await pickPdfs();
    addPaths(picked);
  }

  function remove(id: string) {
    files = files.filter((f) => f.id !== id);
  }

  function onConsider(e: CustomEvent<{ items: FileItem[] }>) {
    files = e.detail.items;
  }
  function onFinalize(e: CustomEvent<{ items: FileItem[] }>) {
    files = e.detail.items;
  }

  async function merge() {
    if (files.length < 2) {
      error = "Add at least two PDFs to merge.";
      return;
    }
    busy = true;
    error = null;
    try {
      const result = await mergePdfs(files.map((f) => f.path));
      await setDoc(result);
      // Hand off to Organize so the user can review, reorder, and save a copy.
      await goto("/organize");
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    const pendingPaths = takeMergeDrop();
    if (pendingPaths.length > 0) addPaths(pendingPaths);

    let disposed = false;
    let unlisten: (() => void) | undefined;

    void listenForFileDrops({
      onEnter: () => {
        dragActive = true;
      },
      onOver: () => {
        dragActive = true;
      },
      onLeave: () => {
        dragActive = false;
      },
      onDrop: ({ paths }) => {
        dragActive = false;
        addPaths(paths);
      },
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
      } else {
        unlisten = cleanup;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<div class:drop-active={dragActive} class="screen">
  <header class="toolbar">
    <button class="btn btn-ghost" onclick={() => goto("/")}>← Home</button>
    <div class="title"><strong>Merge PDFs</strong></div>
    <div class="spacer"></div>
    <button class="btn" onclick={addFiles} disabled={busy}>+ Add PDFs</button>
    <button class="btn btn-primary" onclick={merge} disabled={busy || files.length < 2}>
      {busy ? "Merging…" : "Merge & review"}
    </button>
  </header>

  {#if error}
    <div class="banner err" role="alert">{error}</div>
  {/if}

  <div class="body">
    {#if files.length === 0}
      <div class="empty">
        <div class="empty-icon">⧉</div>
        <h2>Combine PDFs into one</h2>
        <p class="muted">
          Add two or more PDFs, drag to put them in the order you want, then
          merge.
        </p>
        <button class="btn btn-primary" onclick={addFiles}>+ Add PDFs</button>
      </div>
    {:else}
      <p class="hint muted">Drag to reorder. They'll be combined top to bottom.</p>
      <ul
        class="list"
        use:dndzone={{ items: files, flipDurationMs: flipMs }}
        onconsider={onConsider}
        onfinalize={onFinalize}
      >
        {#each files as f, i (f.id)}
          <li class="row" animate:flip={{ duration: flipMs }}>
            <span class="grip" aria-hidden="true">⠿</span>
            <span class="idx">{i + 1}</span>
            <span class="name">{f.name}</span>
            <button class="icon-btn danger" title="Remove" onclick={() => remove(f.id)}>✕</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if dragActive}
    <DropOverlay detail="Files will be added to the merge list." />
  {/if}
</div>

<style>
  .screen {
    position: relative;
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
  .spacer {
    flex: 1;
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.2rem;
  }
  .drop-active .body {
    background: var(--primary-soft);
  }
  .empty {
    max-width: 30rem;
    margin: 8vh auto 0;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.7rem;
  }
  .empty-icon {
    font-size: 3rem;
    color: var(--primary);
  }
  .hint {
    margin: 0 0 0.8rem;
    font-size: 0.9rem;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-width: 40rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.7rem 0.9rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .grip {
    cursor: grab;
    color: var(--text-soft);
  }
  .idx {
    font-weight: 700;
    color: var(--text-soft);
    width: 1.4rem;
    text-align: center;
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .icon-btn {
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: 6px;
    width: 30px;
    height: 30px;
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
  .banner.err {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
