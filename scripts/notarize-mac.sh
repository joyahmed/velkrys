#!/usr/bin/env bash
# Turns an ad-hoc signed Velkrys .dmg into one Apple has notarized, so it opens with no warning.
#   bash scripts/notarize-mac.sh <input.dmg> <work-folder>
# Output: <work-folder>/Velkrys_<version>_universal_notarized.dmg. Publishes nothing.
#
# Needs Xcode signed in to team AXMUG2K6X2 with access to the cloud-managed Developer ID
# certificate. Apple keeps the private key, so this runs here, not on the GitHub Mac runner.
# The work folder is kept: if Apple is slow, run the same line again to resume.
set -euo pipefail

[[ "$(uname -s)" == "Darwin" ]] || { echo "notarize-mac.sh: macOS only" >&2; exit 1; }
dmg=${1:?usage: notarize-mac.sh <input.dmg> <work-folder>}
work=${2:?usage: notarize-mac.sh <input.dmg> <work-folder>}
repo="$(cd "$(dirname "$0")/.." && pwd)"
team=AXMUG2K6X2
mkdir -p "$work"
archive=$work/Velkrys.xcarchive

if [[ ! -d "$archive" ]]; then
  mount=$(mktemp -d)
  hdiutil attach -quiet -nobrowse -readonly -mountpoint "$mount" "$dmg"
  mkdir -p "$archive/Products/Applications"
  ditto "$mount/Velkrys.app" "$archive/Products/Applications/Velkrys.app"
  hdiutil detach -quiet "$mount"
  app=$archive/Products/Applications/Velkrys.app
  version=$(plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")
  bundle_id=$(plutil -extract CFBundleIdentifier raw "$app/Contents/Info.plist")
  # The export keeps the hardened runtime and entitlements of this ad-hoc signature.
  codesign --force --options runtime --entitlements "$repo/src-tauri/Entitlements.mac.plist" --sign - "$app"
  cat > "$archive/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>ApplicationProperties</key><dict>
    <key>ApplicationPath</key><string>Applications/Velkrys.app</string>
    <key>CFBundleIdentifier</key><string>$bundle_id</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>Team</key><string>$team</string>
  </dict>
  <key>ArchiveVersion</key><integer>2</integer>
  <key>Name</key><string>Velkrys</string>
</dict></plist>
PLIST
  cat > "$work/export.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>method</key><string>developer-id</string>
  <key>teamID</key><string>$team</string>
  <key>signingStyle</key><string>automatic</string>
  <key>destination</key><string>upload</string>
</dict></plist>
PLIST
  xcodebuild -exportArchive -archivePath "$archive" -exportOptionsPlist "$work/export.plist" \
    -exportPath "$work/up" -allowProvisioningUpdates >"$work/upload.log" 2>&1
fi

tries=0
rm -rf "$work/out"
until xcodebuild -exportNotarizedApp -archivePath "$archive" -exportPath "$work/out" >"$work/notary.log" 2>&1; do
  tries=$((tries + 1))
  if ((tries >= 120)); then
    echo "Apple has not answered after 60 minutes. Run the same line again later." >&2
    exit 1
  fi
  rm -rf "$work/out"
  sleep 30
done

app=$work/out/Velkrys.app
version=$(plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")
codesign --verify --deep --strict "$app"
xcrun stapler validate "$app"
spctl --assess --type execute --verbose=2 "$app" 2>&1 | grep -q "Notarized Developer ID" || {
  echo "Gatekeeper does not accept the app - do not publish." >&2
  exit 1
}
stage=$(mktemp -d)
ditto "$app" "$stage/Velkrys.app"
ln -s /Applications "$stage/Applications"
result=$work/Velkrys_${version}_universal_notarized.dmg
hdiutil create -quiet -volname Velkrys -srcfolder "$stage" -ov -format UDZO "$result"
rm -rf "$stage"
shasum -a 256 "$result"
