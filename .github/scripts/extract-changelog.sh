#!/usr/bin/env bash
set -euo pipefail

version="${GITHUB_REF_NAME#v}"
changelog="CHANGELOG.md"
output="release-notes.md"

if [[ ! -f "$changelog" ]]; then
  echo "::error::CHANGELOG.md was not found."
  exit 1
fi

# Prefer the versioned section; use Unreleased for a tag whose section has not
# been promoted yet. Stop at the next level-two heading.
if grep -Fxq "## [$version]" "$changelog"; then
  heading="## [$version]"
else
  heading="## [Unreleased]"
  echo "::notice::No changelog section for $version; using [Unreleased]."
fi

awk -v heading="$heading" '
  $0 == heading { found = 1; next }
  found && /^## / { exit }
  found { print }
' "$changelog" > "$output"

if [[ ! -s "$output" ]] || ! grep -q '[^[:space:]]' "$output"; then
  echo "::error::The selected changelog section ($heading) is empty. Add release notes before publishing."
  exit 1
fi

echo "Release notes prepared from $heading: $output"
