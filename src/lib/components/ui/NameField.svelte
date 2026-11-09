<!-- The inline name box: the one text input behind every rename in place (files, drives). It owns the
     look and stops clicks reaching the row; what Enter, Escape and blur do is the caller's hook. -->
<script lang="ts">
  import type { Action } from 'svelte/action';

  interface Props {
    draft: string;
    /** grow with the text instead of filling the row — the horizontal list view's items are narrow */
    wide?: boolean;
    /** under an icon (the tile views): the text is centred, as Explorer's is */
    centered?: boolean;
    /** columns of a box that grows with the text (`wide` only) */
    size?: number;
    onkeydown: (e: KeyboardEvent) => void;
    onblur: () => void;
    /** `use:` action run on mount: focus, select, scroll into view */
    focus: Action<HTMLInputElement>;
  }
  let {
    draft = $bindable(),
    wide = false,
    centered = false,
    size = undefined,
    onkeydown,
    onblur,
    focus,
  }: Props = $props();
</script>

<!-- clicks inside the field must not reach the row's select handler; the input owns the keyboard -->
<span class={wide ? 'shrink-0' : 'flex-1'}>
  <input
    class="rounded-control border-accent bg-field h-6 border px-1 text-xs outline-none {wide
      ? ''
      : 'w-full'} {centered ? 'text-center' : ''}"
    type="text"
    autocorrect="off"
    autocapitalize="off"
    autocomplete="off"
    spellcheck="false"
    size={wide ? size : undefined}
    bind:value={draft}
    onclick={(e) => e.stopPropagation()}
    ondblclick={(e) => e.stopPropagation()}
    {onkeydown}
    {onblur}
    use:focus
  />
</span>
