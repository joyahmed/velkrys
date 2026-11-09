// Renames Trove to Velkrys across the tracked files (plan 28). Rules, not file lists, so the same
// script runs on any branch cut before the rename: run it there first, then merge.
//   bun scripts/rename-to-velkrys.ts
// A line holding the marker below is never rewritten, so a re-run on the renamed tree changes
// nothing. Only text is rewritten; a binary still moves when its path holds the old name.
import { execFileSync } from 'node:child_process';
import { lstatSync, readFileSync, writeFileSync } from 'node:fs';

export const KEEP_MARKER = 'rename-keep';

/** History: what happened under the old name stays written under it. */
const HISTORY = [
  /^docs\/plans\//,
  /^docs\/ai-memory\//,
  /^docs\/release-notes\/v[^/]*\.md$/,
  /^docs\/screenshots\//,
  /^PLAN\.md$/,
];

/** These must hold the old name to do their job. */
const OWN_FILES = new Set([
  'scripts/rename-to-velkrys.ts',
  'scripts/rename-to-velkrys.test.ts',
  'src-tauri/src/core/from_trove.rs',
]);

/** Names that belong to someone or something else, or are baked into a binary fixture. */
const PROTECTED = [
  '[Rr]ust[Rr]over',
  // the words typed in the Mac autocorrect check; a record of what happened
  'trovete(?:st)?',
  // public links and outside package names: they move only when the outside thing moves
  'joyahmed/tap/trove',
  'joyahmed/trove(?:-releases)?',
  'trove-releases',
  'zettabyte/trove',
  'ZettaByte\\.Trove',
  '--cask trove',
  // a real folder on Joy's disk, named in checklists and scripts
  '[Cc]:[\\\\/]+trove-test',
  // the same folder as a path crumb, its drive one array item earlier
  "'[Cc]:', 'trove-test'",
  // inside the bytes of tests/fixtures/archives, which this script does not rewrite
  'hello from trove',
  'trove 7z fixture',
  'trove zip method 9',
];

/** In docs/brand the word is also the common noun the icon is drawn around. */
const BRAND_NOUN = ['\\bthe trove\\b', '\\b[Aa] trove\\b'];

const NEW_NAME: Record<string, string> = {
  Trove: 'Velkrys',
  Troves: 'Velkrys',
  trove: 'velkrys',
  troves: 'velkrys',
  TROVE: 'VELKRYS',
  TROVES: 'VELKRYS',
};

// One pass: a protected name is matched first at its position and comes back unchanged.
const pattern = (keep: string[]) =>
  new RegExp([...keep, 'Troves?', 'troves?', 'TROVES?'].join('|'), 'g');
const PLAIN = pattern(PROTECTED);
const BRAND = pattern([...PROTECTED, ...BRAND_NOUN]);

const isBrand = (path: string) => path.startsWith('docs/brand/');

export const isSkipped = (path: string): boolean =>
  OWN_FILES.has(path) || HISTORY.some((rule) => rule.test(path));

export const renameLine = (line: string, path = ''): string =>
  line.includes(KEEP_MARKER)
    ? line
    : line.replace(isBrand(path) ? BRAND : PLAIN, (hit) => NEW_NAME[hit] ?? hit);

/** Line by line; a `\r` stays at the end of its line, so CRLF files keep their endings. */
export const renameText = (text: string, path = ''): string =>
  text
    .split('\n')
    .map((line) => renameLine(line, path))
    .join('\n');

export const renamedPath = (path: string): string =>
  path.replace(PLAIN, (hit) => NEW_NAME[hit] ?? hit);

/** Null for a binary, or for bytes that would not survive a trip through UTF-8. */
export const asText = (bytes: Buffer): string | null => {
  if (bytes.includes(0)) return null;
  const text = bytes.toString('utf8');
  return Buffer.from(text, 'utf8').equals(bytes) ? text : null;
};

const run = (program: string, args: string[]) =>
  execFileSync(program, args, { stdio: ['ignore', 'pipe', 'inherit'], encoding: 'utf8' });

const inChunks = (paths: string[], size: number): string[][] =>
  Array.from({ length: Math.ceil(paths.length / size) }, (_, i) =>
    paths.slice(i * size, (i + 1) * size),
  );

const main = () => {
  const tracked = run('git', ['ls-files', '-z']).split('\0').filter(Boolean);
  const candidates = tracked.filter(
    (path) => !isSkipped(path) && lstatSync(path, { throwIfNoEntry: false })?.isFile(),
  );

  const changed: string[] = [];
  for (const path of candidates) {
    const text = asText(readFileSync(path));
    if (text === null) continue;
    const next = renameText(text, path);
    if (next === text) continue;
    writeFileSync(path, next);
    changed.push(path);
  }

  const moved = candidates.filter((path) => renamedPath(path) !== path);
  for (const path of moved) run('git', ['mv', path, renamedPath(path)]);

  const now = changed.map(renamedPath);
  for (const chunk of inChunks(now, 100))
    run('bunx', ['prettier', '--write', '--ignore-unknown', '--log-level', 'warn', ...chunk]);
  if (now.some((path) => path.endsWith('.rs')))
    run('cargo', ['fmt', '--manifest-path', 'src-tauri/Cargo.toml']);
  // the renamed package sorts elsewhere: let cargo put Cargo.lock back in order, no versions move
  if (now.includes('src-tauri/Cargo.lock'))
    run('cargo', ['update', '--offline', '--workspace', '--manifest-path', 'src-tauri/Cargo.toml']);

  console.log(`rename-to-velkrys: ${changed.length} file(s) rewritten, ${moved.length} moved`);
};

if (import.meta.main) main();
