import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// Both right-side panes carry their own close button, so hiding one never depends on knowing
// which toolbar button or shortcut opened it. Joy could not hide the Details pane (plan 23).
const markup = (name: string) =>
  readFileSync(`src/lib/components/panes/${name}.svelte`, 'utf8').replace(
    /<script[\s\S]*?<\/script>/g,
    '',
  );

describe('pane close buttons', () => {
  it.each([
    ['DetailsPane', 'Close details pane'],
    ['PreviewPane', 'Close preview pane'],
  ])('%s has a labelled button wired to close', (name, label) => {
    const html = markup(name);
    expect(html).toContain(`aria-label="${label}"`);
    expect(html).toContain('onclick={pane.close}');
  });

  it('keeps the Details close button outside the scrolling body, so it stays in view', () => {
    const html = markup('DetailsPane');
    expect(html.indexOf('</header>')).toBeGreaterThan(-1);
    expect(html.indexOf('</header>')).toBeLessThan(html.indexOf('overflow-auto'));
  });
});
