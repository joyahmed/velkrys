#!/usr/bin/env bash
# AppImage checks (what AppImageHub's test tripped on for v0.1.3):
#   1. every entry in the squashfs is readable by everyone (o+r), every folder and every
#      executable is enterable/runnable by everyone (o+x). `firejail --appimage` runs the app as
#      another user, so a 0770 AppRun.wrapped there is "Permission denied".
#   2. glibc floor: the highest GLIBC_x.y any ELF file inside needs is <= --max (default 2.35,
#      Ubuntu 22.04), same rule as linux-package-check.ts uses for the .deb.
# Modes are read with `unsquashfs -lls`, never by extracting: extraction rewrites folder modes.
# The AppImage is never executed.
#
#   bash scripts/linux-appimage-check.sh Velkrys_0.1.3_amd64.AppImage [--max 2.35]
# Needs: readelf, objdump (binutils), unsquashfs (squashfs-tools), file.
set -euo pipefail

img="${1:?usage: linux-appimage-check.sh <file.AppImage> [--max 2.35]}"
max="2.35"
if [[ "${2:-}" == "--max" ]]; then max="${3:?--max needs a version}"; fi
fail=0

# The squashfs starts right after the runtime ELF: section header offset + size * count.
hdr="$(readelf -h "$img")"
shoff=$(awk -F: '/Start of section headers/ {gsub(/[^0-9]/,"",$2); print $2}' <<<"$hdr")
shsize=$(awk -F: '/Size of section headers/ {gsub(/[^0-9]/,"",$2); print $2}' <<<"$hdr")
shnum=$(awk -F: '/Number of section headers/ {gsub(/[^0-9]/,"",$2); print $2}' <<<"$hdr")
offset=$((shoff + shsize * shnum))
echo "squashfs offset: $offset"

listing="$(unsquashfs -o "$offset" -lls "$img" | grep -E '^[-dlcbps][-rwxsStT]{9} ')"
total=$(wc -l <<<"$listing")
echo "entries: $total"
echo "launchers:"
grep -E ' squashfs-root/AppRun(\.wrapped)?$' <<<"$listing" || { echo "FAIL: no AppRun in image"; fail=1; }

# Not symlinks: others must read (char 8); folders and anything executable by owner (char 4)
# must also be o+x (char 10).
bad="$(awk '
  substr($1,1,1) == "l" { next }
  { o_r = substr($1,8,1); u_x = substr($1,4,1); o_x = substr($1,10,1); d = substr($1,1,1) }
  o_r != "r" { print; next }
  (d == "d" || u_x == "x" || u_x == "s") && o_x != "x" && o_x != "t" { print }
' <<<"$listing")"
if [[ -n "$bad" ]]; then
  echo "FAIL: $(wc -l <<<"$bad") entries not readable/runnable by everyone:"
  head -40 <<<"$bad"
  fail=1
else
  echo "ok: all $total entries readable by everyone, folders and executables o+x"
fi

# glibc floor across every ELF inside.
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
unsquashfs -q -n -o "$offset" -d "$tmp/root" "$img" >/dev/null
elfs=0
top=""
while IFS= read -r -d '' f; do
  [[ "$(head -c 4 "$f" | od -An -c | tr -d ' ')" == '177ELF' ]] || continue
  elfs=$((elfs + 1))
  v="$(objdump -T "$f" 2>/dev/null | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)+' | sed 's/GLIBC_//' | sort -V | tail -1 || true)"
  [[ -z "$v" ]] && continue
  if [[ -z "$top" || "$(printf '%s\n%s\n' "$top" "$v" | sort -V | tail -1)" == "$v" && "$v" != "$top" ]]; then
    top="$v"; topfile="${f#"$tmp/root/"}"
  fi
done < <(find "$tmp/root" -type f -print0)
echo "ELF files: $elfs"
if [[ -z "$top" ]]; then
  echo "FAIL: no GLIBC_ symbol found in any ELF (check is not measuring)"; fail=1
elif [[ "$(printf '%s\n%s\n' "$top" "$max" | sort -V | tail -1)" != "$max" ]]; then
  echo "FAIL: glibc: needs GLIBC_$top ($topfile), max allowed $max"; fail=1
else
  echo "ok: glibc: highest GLIBC_$top ($topfile), max allowed $max"
fi

exit "$fail"
