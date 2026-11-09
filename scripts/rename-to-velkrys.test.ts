import { expect, test } from 'bun:test';
import {
  KEEP_MARKER,
  asText,
  isSkipped,
  renameLine,
  renameText,
  renamedPath,
} from './rename-to-velkrys';

test('every case of the name maps, plurals included', () => {
  expect(renameLine('Trove trove TROVE')).toBe('Velkrys velkrys VELKRYS');
  expect(renameLine('quit_troves running Troves')).toBe('quit_velkrys running Velkrys');
  expect(renameLine('__TROVE_PLATFORM__ TrovePlatform troveItem')).toBe(
    '__VELKRYS_PLATFORM__ VelkrysPlatform velkrysItem',
  );
  expect(renameLine('com.joyahmed.trove.desktop')).toBe('com.joyahmed.velkrys.desktop');
});

test('outside names stay', () => {
  const kept = [
    'rustrover RustRover',
    'typed trovetest and trovete',
    'https://github.com/joyahmed/trove',
    'https://github.com/joyahmed/trove-releases/releases/latest/download/latest.json',
    'the trove-releases repo',
    'brew install joyahmed/tap/trove',
    'scoop install zettabyte/trove',
    'winget install ZettaByte.Trove',
    'brew install --cask trove',
    String.raw`C:\trove-test\report.docx`,
    String.raw`C:\\trove-test\\report.docx`,
    'C:/trove-test',
    "['C:', 'trove-test', 'a.zip']",
    'hello from trove',
    'trove 7z fixture',
    'trove zip method 9',
  ];
  for (const line of kept) expect(renameLine(line)).toBe(line);
});

test('a protected name next to the real one leaves only the real one changed', () => {
  expect(renameLine('Trove lives at github.com/joyahmed/trove')).toBe(
    'Velkrys lives at github.com/joyahmed/trove',
  );
  expect(renameLine('com.joyahmed.trove and TroveTest')).toBe(
    'com.joyahmed.velkrys and VelkrysTest',
  );
});

test('the common noun stays in docs/brand and only there', () => {
  const brand = 'docs/brand/icon.py';
  expect(renameLine('Trove icon, the trove, a trove holds. Not a trove.', brand)).toBe(
    'Velkrys icon, the trove, a trove holds. Not a trove.',
  );
  expect(renameLine('docs/brand/trove.svg', brand)).toBe('docs/brand/velkrys.svg');
  expect(renameLine('the trove window')).toBe('the velkrys window');
});

test('a marked line is never rewritten, and the file keeps its line endings', () => {
  const marked = `'trove:home': HOME, // ${KEEP_MARKER}`;
  expect(renameLine(marked)).toBe(marked);
  expect(renameText(`Trove\r\n${marked}\r\ntrove\n`)).toBe(`Velkrys\r\n${marked}\r\nvelkrys\n`);
});

test('a second run changes nothing', () => {
  const once = renameText('Trove at github.com/joyahmed/trove\nTROVE_T\n');
  expect(renameText(once)).toBe(once);
});

test('history and the files that need the old name are skipped', () => {
  for (const path of [
    'docs/plans/28-rename-to-velkrys.md',
    'docs/ai-memory/session.md',
    'docs/release-notes/v0.1.1.md',
    'docs/screenshots/a/b.png',
    'PLAN.md',
    'scripts/rename-to-velkrys.ts',
    'scripts/rename-to-velkrys.test.ts',
    'src-tauri/src/core/from_trove.rs',
  ])
    expect(isSkipped(path)).toBe(true);
  for (const path of [
    'README.md',
    'docs/JOY-CHECKLIST.md',
    'docs/release-notes-template.md',
    'docs/install/linux.md',
    'src-tauri/Cargo.lock',
  ])
    expect(isSkipped(path)).toBe(false);
});

test('paths move by the same map', () => {
  expect(renamedPath('docs/brand/trove-icon-1024.png')).toBe('docs/brand/velkrys-icon-1024.png');
  expect(renamedPath('src-tauri/linux/com.joyahmed.trove.metainfo.xml')).toBe(
    'src-tauri/linux/com.joyahmed.velkrys.metainfo.xml',
  );
  expect(renamedPath('src/lib/a.ts')).toBe('src/lib/a.ts');
});

test('binaries and bytes that are not UTF-8 are left alone', () => {
  expect(asText(Buffer.from('Trove\n'))).toBe('Trove\n');
  expect(asText(Buffer.from([0x54, 0x00, 0x72]))).toBeNull();
  expect(asText(Buffer.from([0x54, 0xff, 0x72]))).toBeNull();
});
