<script lang="ts">
  import NameField from '$lib/components/ui/NameField.svelte';
  import { useRenameBox } from '$lib/hooks/use-rename-box.svelte';

  interface Props {
    /** grow with the text instead of filling the row — the horizontal list view's items are narrow */
    wide?: boolean;
    /** under an icon (the tile views): the text is centred, as Explorer's is */
    centered?: boolean;
  }
  let { wide = false, centered = false }: Props = $props();

  const box = useRenameBox();
</script>

<NameField
  bind:draft={box.draft}
  {wide}
  {centered}
  size={box.size}
  onkeydown={box.onkeydown}
  onblur={box.onblur}
  focus={box.focus}
/>
