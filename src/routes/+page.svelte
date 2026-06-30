<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { pickPdf, openPdf } from "$lib/api";
  import {
    listenForFileDrops,
    splitPdfPaths,
    type DropPosition,
  } from "$lib/fileDrop";
  import AppIcon from "$lib/components/AppIcon.svelte";
  import { queueMergeDrop } from "$lib/stores/droppedFiles";
  import { setDoc } from "$lib/stores/document.svelte";
  import DropOverlay from "$lib/components/DropOverlay.svelte";
  import ViewerIcon, { type IconName } from "$lib/components/ViewerIcon.svelte";
  import { updater } from "$lib/updater.svelte";

  let busy = $state(false);
  let error = $state<string | null>(null);
  let dragActive = $state(false);
  // How many PDFs are being dragged, and which tool tile the cursor is over —
  // used to tell the user exactly what will happen when they drop.
  let dragFileCount = $state(0);
  let dropTargetRoute = $state<string | null>(null);

  // Open a single PDF, register it, and navigate to the chosen tool screen.
  async function openAndGo(route: string, droppedPath?: string) {
    error = null;
    const path = droppedPath ?? (await pickPdf());
    if (!path) return; // user cancelled
    busy = true;
    try {
      const result = await openPdf(path);
      await setDoc(result);
      await goto(route);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  // Which tool tile (if any) sits under the cursor at this screen position.
  function tileRouteAt(position?: DropPosition): string | null {
    if (!position) return null;
    const x = position.x / window.devicePixelRatio;
    const y = position.y / window.devicePixelRatio;
    const target = document.elementFromPoint(x, y);
    const tile = target?.closest<HTMLElement>("[data-drop-route]");
    return tile?.dataset.dropRoute ?? null;
  }

  async function handleDroppedFiles(paths: string[], position?: DropPosition) {
    if (busy) return;

    const { pdfs, rejected } = splitPdfPaths(paths);
    if (pdfs.length === 0) {
      error = rejected.length > 0 ? "Drop PDF files to start." : null;
      return;
    }

    const route = tileRouteAt(position) ?? "/view";
    if (route === "/merge" || pdfs.length > 1) {
      queueMergeDrop(pdfs);
      await goto("/merge");
      return;
    }

    await openAndGo(route, pdfs[0]);
  }

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void listenForFileDrops({
      onEnter: (paths) => {
        dragActive = true;
        dragFileCount = splitPdfPaths(paths).pdfs.length;
      },
      onOver: (position) => {
        dragActive = true;
        dropTargetRoute = tileRouteAt(position);
      },
      onLeave: () => {
        dragActive = false;
        dropTargetRoute = null;
      },
      onDrop: async ({ paths, position }) => {
        dragActive = false;
        dropTargetRoute = null;
        dragFileCount = 0;
        await handleDroppedFiles(paths, position);
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

  const tiles = [
    {
      key: "view",
      title: "Open PDF",
      desc: "Read, zoom, browse pages, and use PDF tools",
      icon: "viewer",
      route: "/view",
      featured: true,
      action: () => openAndGo("/view"),
    },
    {
      key: "organize",
      title: "Organize pages",
      desc: "Reorder, rotate, or delete pages",
      icon: "organize",
      route: "/organize",
      featured: false,
      action: () => openAndGo("/organize"),
    },
    {
      key: "sign",
      title: "Fill & Sign",
      desc: "Type into forms and add your signature",
      icon: "sign",
      route: "/sign",
      featured: false,
      action: () => openAndGo("/sign"),
    },
    {
      key: "merge",
      title: "Merge PDFs",
      desc: "Combine several files into one",
      icon: "merge",
      route: "/merge",
      featured: false,
      action: () => goto("/merge"),
    },
    {
      key: "split",
      title: "Split / extract",
      desc: "Pull out pages into a new file",
      icon: "split",
      route: "/split",
      featured: false,
      action: () => openAndGo("/split"),
    },
  ] satisfies {
    key: string;
    title: string;
    desc: string;
    icon: IconName;
    route: string;
    featured: boolean;
    action: () => void | Promise<void>;
  }[];

  const targetTile = $derived(
    tiles.find((t) => t.route === dropTargetRoute) ?? null,
  );
  const dropTitle = $derived(
    dragFileCount > 1
      ? "Merge PDFs"
      : (targetTile?.title ?? "Drop a PDF"),
  );
  const dropDetail = $derived(
    dragFileCount > 1
      ? `Drop to combine ${dragFileCount} files into one`
      : targetTile
        ? `Drop to open in ${targetTile.title}`
        : "Drop onto a tool to choose what happens",
  );
</script>

<main class:drop-active={dragActive} class="home">
  <header class="hero">
    <div class="logo"><AppIcon size={68} /></div>
    <h1>SwiftPDF</h1>
    <p class="muted">
      Edit your PDFs right on your computer. Nothing is uploaded — your files
      stay private.
    </p>
  </header>

  {#if error}
    <div class="banner error" role="alert">{error}</div>
  {/if}

  <section class="tiles" aria-busy={busy}>
    {#each tiles as tile (tile.key)}
      <button
        class="tile"
        class:featured={tile.featured}
        class:drop-target={dragActive &&
          (dragFileCount > 1
            ? tile.route === "/merge"
            : tile.route === dropTargetRoute)}
        data-drop-route={tile.route}
        onclick={tile.action}
        disabled={busy}
      >
        <span class="tile-icon" aria-hidden="true">
          <ViewerIcon name={tile.icon} size={30} />
        </span>
        <span class="tile-title">{tile.title}</span>
        <span class="tile-desc">{tile.desc}</span>
      </button>
    {/each}
  </section>

  <p class="hint muted">Pick what you'd like to do — we'll guide you.</p>

  <button
    class="update-link"
    onclick={() => updater.check(true)}
    disabled={updater.busy}
  >
    {updater.phase === "checking" ? "Checking…" : "Check for updates"}
  </button>

  {#if dragActive}
    <DropOverlay title={dropTitle} detail={dropDetail} />
  {/if}
</main>

<style>
  .home {
    position: relative;
    max-width: 760px;
    margin: 0 auto;
    padding: 5vh 1.5rem 2rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    min-height: 100%;
  }
  .hero {
    text-align: center;
    margin-bottom: 2rem;
  }
  .logo {
    display: inline-flex;
    line-height: 0;
  }
  .hero h1 {
    font-size: 2.4rem;
    margin: 0.4rem 0 0.6rem;
  }
  .hero p {
    max-width: 30rem;
    margin: 0 auto;
    line-height: 1.5;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
    width: 100%;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    gap: 0.35rem;
    padding: 1.4rem 1.4rem 1.5rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    transition: transform 0.08s, box-shadow 0.15s, border-color 0.15s;
  }
  .tile.featured {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    column-gap: 0.9rem;
    align-items: center;
    background: #f9fbff;
  }
  .tile.featured .tile-icon {
    grid-row: span 2;
  }
  .tile:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
    border-color: var(--primary);
  }
  .drop-active .tile {
    border-color: #c3d2f2;
  }
  .tile.drop-target {
    border-color: var(--primary);
    background: var(--primary-soft);
    box-shadow: 0 0 0 3px rgba(37, 99, 246, 0.25), var(--shadow-lg);
    transform: translateY(-2px);
  }
  .tile:disabled {
    opacity: 0.6;
  }
  .tile-icon {
    width: 2.25rem;
    height: 2.25rem;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--primary-soft);
    color: var(--primary);
  }
  .tile-title {
    font-size: 1.2rem;
    font-weight: 700;
  }
  .tile-desc {
    color: var(--text-soft);
    font-size: 0.95rem;
  }
  .hint {
    margin-top: 1.5rem;
  }
  .update-link {
    margin-top: 0.6rem;
    background: none;
    border: none;
    color: var(--text-soft);
    font-size: 0.85rem;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .update-link:hover:not(:disabled) {
    color: var(--primary);
  }
  .update-link:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .banner {
    width: 100%;
    padding: 0.8rem 1rem;
    border-radius: var(--radius-sm);
    margin-bottom: 1rem;
  }
  .banner.error {
    background: var(--danger-soft);
    color: var(--danger);
  }
</style>
