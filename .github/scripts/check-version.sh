#!/usr/bin/env bash
# Checks that every place the Hops version is declared agrees, then prints it.
#
# Usage: check-version.sh [expected-tag]
#   expected-tag  Optional release tag (e.g. v0.1.6) that must match the version.
#
# A half-done version bump would ship an installer and updater manifest whose
# versions disagree, so CI runs this before any release and on pull requests.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# First top-level "version" field of a JSON file (both files declare it once).
json_version() {
  sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1 | tr -d '\r'
}

package_json=$(json_version package.json)
tauri_conf=$(json_version src-tauri/tauri.conf.json)
# The [package] version is the first line that starts with `version =`.
cargo_toml=$(sed -n 's/^version = "\([^"]*\)".*/\1/p' src-tauri/Cargo.toml | head -n 1 | tr -d '\r')
# The version line right after `name = "hops"` in the lock file.
cargo_lock=$(awk '{ sub(/\r$/, "") } found { gsub(/^version = "|"$/, ""); print; exit } $0 == "name = \"hops\"" { found = 1 }' src-tauri/Cargo.lock)

version="$package_json"
status=0
for entry in \
  "package.json:$package_json" \
  "src-tauri/tauri.conf.json:$tauri_conf" \
  "src-tauri/Cargo.toml:$cargo_toml" \
  "src-tauri/Cargo.lock:$cargo_lock"; do
  file="${entry%%:*}"
  found="${entry#*:}"
  if [[ -z "$found" ]]; then
    echo "::error file=$file::Could not read the Hops version from $file." >&2
    status=1
  elif [[ "$found" != "$version" ]]; then
    echo "::error file=$file::$file has version $found, but package.json has $version." >&2
    status=1
  fi
done

if [[ $# -ge 1 && -n "$1" && "$1" != "v$version" ]]; then
  echo "::error::Release tag $1 does not match the app version (expected v$version)." >&2
  status=1
fi

if [[ $status -ne 0 ]]; then
  exit "$status"
fi

echo "$version"
