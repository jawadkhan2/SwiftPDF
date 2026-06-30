<script lang="ts">
  let {
    size = 40,
    title = "SwiftPDF",
  }: { size?: number; title?: string } = $props();
</script>

<img
  class="app-icon"
  src="/app-icon.png"
  width={size}
  height={size}
  alt={title}
  draggable="false"
/>

<style>
  .app-icon {
    display: block;
    border-radius: 8px;
  }
</style>
