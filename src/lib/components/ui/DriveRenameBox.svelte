<!-- The name box shown in place of a drive's label while it is being renamed (sidebar row or
     This PC tile). All behaviour is in `useDriveRenameBox`. -->
<script lang="ts">
  import NameField from '$lib/components/ui/NameField.svelte';
  import { useDriveRenameBox } from '$lib/hooks/use-drive-rename-box.svelte';

  interface Props {
    /** under an icon, as on a tile */
    centered?: boolean;
  }
  let { centered = false }: Props = $props();

  const box = useDriveRenameBox();
</script>

<NameField
  bind:draft={box.draft}
  {centered}
  onkeydown={box.onkeydown}
  onblur={box.onblur}
  focus={box.focus}
/>
