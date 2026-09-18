<script lang="ts">
  /* Icônes tracées, grille 16, trait 1.5 — jamais d'emoji : elles doivent se
     recolorer avec currentColor et rester nettes à toute taille. */
  interface Props {
    name: string;
    size?: number;
    width?: number;
  }
  let { name, size = 16, width = 1.5 }: Props = $props();

  const paths: Record<string, string> = {
    "file": "M4 2.5h5l3 3v8a1 1 0 01-1 1H5a1 1 0 01-1-1z M9 2.5v3h3",
    "folder": "M2.4 5.2a1 1 0 011-1h2.9l1.3 1.6h5a1 1 0 011 1v5.2a1 1 0 01-1 1H3.4a1 1 0 01-1-1z",
    "chevron-right": "M6 4l4 4-4 4",
    "chevron-down": "M4 6l4 4 4-4",
    "chevron-up": "M4 10l4-4 4 4",
    "search": "M10.2 10.2l3 3",
    "plus": "M8 3.8v8.4M3.8 8h8.4",
    "close": "M3.5 3.5l9 9M12.5 3.5l-9 9",
    "minimise": "M2.5 8h11",
    "undo": "M3.5 8.5h6.2a3 3 0 010 6H7 M3.5 8.5l2.6-2.6M3.5 8.5l2.6 2.6",
    "redo": "M12.5 8.5H6.3a3 3 0 000 6H9 M12.5 8.5L9.9 5.9M12.5 8.5L9.9 11.1",
    "bold": "M5 3h4.2a2.4 2.4 0 010 4.8H5z M5 7.8h4.8a2.6 2.6 0 010 5.2H5z",
    "italic": "M10.5 3H7M9 13H5.5M9.8 3L6.2 13",
    "link": "M6.4 9.6l3.2-3.2 M7.2 4.6l1-1a2.7 2.7 0 013.8 3.8l-1 1 M8.8 11.4l-1 1a2.7 2.7 0 01-3.8-3.8l1-1",
    "code": "M6 4.8L2.8 8 6 11.2M10 4.8L13.2 8 10 11.2",
    "list": "M6 4.2h7.2M6 8h7.2M6 11.8h7.2",
    "quote": "M3.4 3.6v8.8M6.6 6h6.2M6.6 10h4.2",
    "view-editor": "M4.6 6.4h4.2M4.6 8.6h5.6M4.6 10.8h3",
    "view-split": "M8 3.4v9.2",
    "zen": "M6.2 2.8H2.8V6.2M9.8 2.8h3.4V6.2M9.8 13.2h3.4V9.8M6.2 13.2H2.8V9.8",
    "zen-exit": "M2.6 6.4V2.8h3.6M13.4 6.4V2.8H9.8M13.4 9.6v3.6H9.8M2.6 9.6v3.6h3.6",
    "moon": "M12.6 9.4A5 5 0 016.6 3.4 5 5 0 108 13a5 5 0 004.6-3.6z",
    "arrow-left": "M12.5 8h-9M7 3.5L3 8l4 4.5",
    "arrow-right": "M3.5 8h9M9 3.5L13 8l-4 4.5",
    "measure": "M2.6 4.4h10.8M2.6 8h10.8M2.6 11.6h7.4",
    "settings": "M8 5.4a2.6 2.6 0 100 5.2 2.6 2.6 0 000-5.2z M8 2.8v1.5M8 11.7v1.5M2.8 8h1.5M11.7 8h1.5M4.3 4.3l1.1 1.1M10.6 10.6l1.1 1.1M11.7 4.3l-1.1 1.1M5.4 10.6l-1.1 1.1",
    "save": "M3.4 2.8h6.2l3 3v7.4a1 1 0 01-1 1H3.4a1 1 0 01-1-1V3.8a1 1 0 011-1z M5.4 8.6h5.2M5.4 11h3.4",
    "sync": "M3 6.5A4.2 4.2 0 019.6 4.2l2 1.6 M11.6 3.2v2.6H9 M13 9.5A4.2 4.2 0 016.4 11.8l-2-1.6 M4.4 12.8v-2.6H7",
    "clock": "M8 4.2v3.8l2.6 1.6",
    "wordmark": "M2.5 12.5V3.5l3.2 4 3.2-4v9 M12 3.5v9M12 12.5l1.8-2M12 12.5l-1.8-2",
  };

  const circles: Record<string, [number, number, number][]> = {
    "search": [[7.2, 7.2, 4]],
    "view-preview": [[8, 8, 1.7]],
    "clock": [[8, 8, 5.6]],
    "sun": [[8, 8, 3]],
  };

  const rects: Record<string, [number, number, number, number, number][]> = {
    "maximise": [[3, 3, 10, 10, 1.4]],
    "view-editor": [[2.6, 3.4, 10.8, 9.2, 1.5]],
    "view-split": [[2.6, 3.4, 10.8, 9.2, 1.5]],
    "table": [[2.8, 3.4, 10.4, 9.2, 1.5]],
    "sidebar": [[2.6, 3.4, 10.8, 9.2, 1.5]],
    "width-centered": [[2.6, 3.4, 10.8, 9.2, 1.5]],
    "width-full": [[2.6, 3.4, 10.8, 9.2, 1.5]],
  };

  const extra: Record<string, string> = {
    "view-preview": "M1.6 8S3.8 4.2 8 4.2 14.4 8 14.4 8 12.2 11.8 8 11.8 1.6 8 1.6 8z",
    "table": "M2.8 7h10.4M7.2 3.4v9.2",
    "sidebar": "M6.4 3.4v9.2",
    "width-centered": "M5.8 6h4.4M5.8 8.4h4.4M5.8 10.8h2.8",
    "width-full": "M4.4 6h7.2M4.4 8.4h7.2M4.4 10.8h4.6",
    "list-dots": "M3.1 4.2h.02M3.1 8h.02M3.1 11.8h.02",
    "sun": "M8 1.6v1.5M8 12.9v1.5M1.6 8h1.5M12.9 8h1.5M3.5 3.5l1 1M11.5 11.5l1 1M12.5 3.5l-1 1M4.5 11.5l-1 1",
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width={width}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each rects[name] ?? [] as [x, y, w, h, r]}
    <rect {x} {y} width={w} height={h} rx={r} />
  {/each}
  {#each circles[name] ?? [] as [cx, cy, r]}
    <circle {cx} {cy} {r} />
  {/each}
  {#if paths[name]}
    <path d={paths[name]} />
  {/if}
  {#if extra[name]}
    <path d={extra[name]} />
  {/if}
</svg>
