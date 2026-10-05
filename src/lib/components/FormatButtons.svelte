<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { FormatKind } from "$lib/formats";

  /* Les neuf actions de formatage, partagées entre la barre d'outils du mode
     Split et la barre flottante du mode Zen. */
  interface Props {
    onFormat: (kind: FormatKind) => void;
  }
  let { onFormat }: Props = $props();
</script>

<button class="icon-btn" onclick={() => onFormat("bold")} title="Gras">
  <Icon name="bold" size={16} width={1.6} />
</button>
<button class="icon-btn" onclick={() => onFormat("italic")} title="Italique">
  <Icon name="italic" size={16} />
</button>
<button class="icon-btn" onclick={() => onFormat("link")} title="Lien">
  <Icon name="link" size={16} />
</button>
<button class="icon-btn" onclick={() => onFormat("code")} title="Code inline">
  <Icon name="code" size={16} />
</button>
<button class="icon-btn" onclick={() => onFormat("codeblock")} title="Bloc de code — ```langage">
  <Icon name="code-block" size={16} width={1.4} />
</button>
<button class="icon-btn" onclick={() => onFormat("strike")} title="Barré">
  <Icon name="strike" size={16} width={1.4} />
</button>
<button class="icon-btn" onclick={() => onFormat("mark")} title="Surligner">
  <Icon name="mark" size={16} width={1.4} />
</button>
<button class="icon-btn" onclick={() => onFormat("list")} title="Liste">
  <Icon name="list" size={16} />
</button>
<button class="icon-btn" onclick={() => onFormat("quote")} title="Citation">
  <Icon name="quote" size={16} />
</button>
