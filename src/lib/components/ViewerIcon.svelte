<script lang="ts">
  export type IconName =
    | "chevron-left"
    | "chevron-right"
    | "cursor"
    | "document"
    | "fit-width"
    | "folder-open"
    | "hand"
    | "home"
    | "merge"
    | "organize"
    | "pages"
    | "panel-left"
    | "panel-right"
    | "print"
    | "rotate"
    | "save-copy"
    | "search"
    | "sign"
    | "split"
    | "viewer"
    | "zoom-in"
    | "zoom-out";

  let {
    name,
    size = 20,
    title,
  }: { name: IconName; size?: number; title?: string } = $props();
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.9"
  stroke-linecap="round"
  stroke-linejoin="round"
  xmlns="http://www.w3.org/2000/svg"
  aria-hidden={title ? undefined : "true"}
  role={title ? "img" : undefined}
>
  {#if title}<title>{title}</title>{/if}

  {#if name === "chevron-left"}
    <path d="M14.5 6.5 9 12l5.5 5.5" />
  {:else if name === "chevron-right"}
    <path d="M9.5 6.5 15 12l-5.5 5.5" />
  {:else if name === "cursor"}
    <path d="M6 4.5v14.2l3.7-3.4 2.4 5.2 2.5-1.1-2.5-5.1h5.2L6 4.5Z" />
  {:else if name === "document"}
    <path d="M7 3.5h6.4L18 8v12.5H7V3.5Z" />
    <path d="M13 3.7v4.8h4.8" />
    <path d="M9.6 12h5M9.6 15h5M9.6 18h3.2" />
  {:else if name === "fit-width"}
    <rect x="5" y="4" width="14" height="16" rx="2" />
    <path d="M8 12h8M8 12l2.2-2.2M8 12l2.2 2.2M16 12l-2.2-2.2M16 12l-2.2 2.2" />
  {:else if name === "folder-open"}
    <path d="M3.5 8.5V18a2 2 0 0 0 2 2h13a2 2 0 0 0 2-2v-7.2a2 2 0 0 0-2-2H12L9.8 6.5H5.5a2 2 0 0 0-2 2Z" />
    <path d="M5.2 18.8 7 11h13" />
  {:else if name === "hand"}
    <path d="M8 12.1V5.8a1.3 1.3 0 0 1 2.6 0v5.4M10.6 11V4.9a1.3 1.3 0 0 1 2.6 0v6M13.2 11.3V6.2a1.3 1.3 0 0 1 2.6 0v6.5M15.8 12.6V9a1.25 1.25 0 0 1 2.5 0v5.7c0 4-2.3 6.3-6 6.3H11c-2.1 0-3.3-.9-4.5-2.5l-2.2-3a1.35 1.35 0 0 1 2.1-1.7L8 15.4" />
  {:else if name === "home"}
    <path d="M4.5 11.5 12 5l7.5 6.5" />
    <path d="M6.5 10.8V20h11v-9.2" />
    <path d="M10 20v-5h4v5" />
  {:else if name === "merge"}
    <rect x="4" y="4" width="7" height="8" rx="1.5" />
    <rect x="13" y="12" width="7" height="8" rx="1.5" />
    <path d="M9.5 14.5H13M13 14.5l-2-2M13 14.5l-2 2" />
  {:else if name === "organize"}
    <rect x="5" y="4" width="6" height="7" rx="1.3" />
    <rect x="13" y="4" width="6" height="7" rx="1.3" />
    <rect x="5" y="13" width="6" height="7" rx="1.3" />
    <rect x="13" y="13" width="6" height="7" rx="1.3" />
  {:else if name === "pages"}
    <path d="M8 5.5h9.5v13H8v-13Z" />
    <path d="M5.5 8v13H15" />
    <path d="M10.5 9.5H15M10.5 12.5H15M10.5 15.5h3" />
  {:else if name === "panel-left"}
    <rect x="4" y="5" width="16" height="14" rx="2" />
    <path d="M9 5v14M15 9l-3 3 3 3" />
  {:else if name === "panel-right"}
    <rect x="4" y="5" width="16" height="14" rx="2" />
    <path d="M15 5v14M9 9l3 3-3 3" />
  {:else if name === "print"}
    <path d="M7 8V4h10v4" />
    <path d="M7 17H5.5A2.5 2.5 0 0 1 3 14.5V11a2.5 2.5 0 0 1 2.5-2.5h13A2.5 2.5 0 0 1 21 11v3.5a2.5 2.5 0 0 1-2.5 2.5H17" />
    <rect x="7" y="14" width="10" height="6" rx="1" />
    <path d="M17.6 11.2h.1" />
  {:else if name === "rotate"}
    <path d="M18.8 8.8A7 7 0 1 0 19 15" />
    <path d="M18.8 8.8V4.7M18.8 8.8h-4.1" />
  {:else if name === "save-copy"}
    <path d="M8 6.5V4h9.5v12.5H15" />
    <rect x="5" y="7.5" width="10" height="12.5" rx="1.5" />
    <path d="M10 10.8v5.8M10 16.6l-2.2-2.2M10 16.6l2.2-2.2" />
  {:else if name === "search"}
    <circle cx="10.7" cy="10.7" r="5.7" />
    <path d="m15 15 4.4 4.4" />
  {:else if name === "sign"}
    <path d="M5 18.5c3.4-5.6 5.6-8.9 6.7-10 1.2-1.2 2.7-.8 2.8.7.2 2.7-3.1 5.1-1.8 6.4.9.9 2.9-.4 4.2-1.9" />
    <path d="M4.5 20h15" />
  {:else if name === "split"}
    <path d="M8 4.5h8v6.2a4 4 0 0 1-8 0V4.5Z" />
    <path d="M4 19.5h6M14 19.5h6M12 14.5v5M12 19.5h0" />
    <path d="M8 8h8" />
  {:else if name === "viewer"}
    <path d="M4 12s2.9-5.5 8-5.5S20 12 20 12s-2.9 5.5-8 5.5S4 12 4 12Z" />
    <circle cx="12" cy="12" r="2.5" />
  {:else if name === "zoom-in"}
    <circle cx="10.5" cy="10.5" r="5.5" />
    <path d="M10.5 8v5M8 10.5h5M14.7 14.7l4.3 4.3" />
  {:else if name === "zoom-out"}
    <circle cx="10.5" cy="10.5" r="5.5" />
    <path d="M8 10.5h5M14.7 14.7l4.3 4.3" />
  {/if}
</svg>

<style>
  .icon {
    display: block;
    flex: 0 0 auto;
  }
</style>
