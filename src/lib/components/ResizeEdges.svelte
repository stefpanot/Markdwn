<script lang="ts">
  import { startResize, type Edge } from "$lib/window";

  /* Poignées invisibles sur les huit bords. Des <button> et non des <div> :
     un élément statique avec un gestionnaire de pointeur n'est pas accessible,
     et ceux-ci sont bel et bien actionnables. Hors de l'ordre de tabulation,
     puisqu'au clavier on passe par le menu système. */
  const edges: { dir: Edge; cls: string; label: string }[] = [
    { dir: "North", cls: "n", label: "Redimensionner vers le haut" },
    { dir: "South", cls: "s", label: "Redimensionner vers le bas" },
    { dir: "West", cls: "w", label: "Redimensionner vers la gauche" },
    { dir: "East", cls: "e", label: "Redimensionner vers la droite" },
    { dir: "NorthWest", cls: "nw", label: "Redimensionner en haut à gauche" },
    { dir: "NorthEast", cls: "ne", label: "Redimensionner en haut à droite" },
    { dir: "SouthWest", cls: "sw", label: "Redimensionner en bas à gauche" },
    { dir: "SouthEast", cls: "se", label: "Redimensionner en bas à droite" },
  ];
</script>

{#each edges as edge (edge.dir)}
  <button
    class="edge {edge.cls}"
    tabindex="-1"
    aria-label={edge.label}
    onpointerdown={(e) => {
      if (e.button === 0) startResize(edge.dir);
    }}
  ></button>
{/each}

<style>
  .edge {
    position: fixed;
    z-index: 40;
    background: none;
  }
  /* 5px : assez pour viser à la souris, assez peu pour ne rien voler au
     contenu — les coins passent devant les bords. */
  .n,
  .s {
    left: 6px;
    right: 6px;
    height: 5px;
    cursor: ns-resize;
  }
  .n {
    top: 0;
  }
  .s {
    bottom: 0;
  }
  .w,
  .e {
    top: 6px;
    bottom: 6px;
    width: 5px;
    cursor: ew-resize;
  }
  .w {
    left: 0;
  }
  .e {
    right: 0;
  }
  .nw,
  .ne,
  .sw,
  .se {
    width: 9px;
    height: 9px;
    z-index: 41;
  }
  .nw {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }
  .se {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
  }
  .ne {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }
  .sw {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }
</style>
