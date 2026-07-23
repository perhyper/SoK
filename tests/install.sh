#!/bin/sh

set -eu

script_dir="$(CDPATH= cd "$(dirname "$0")" && pwd)"
repository_root="$(CDPATH= cd "$script_dir/.." && pwd)"
version="$(
  awk -F '"' '
    /^version = "[^"]+"/ {
      print $2
      exit
    }
  ' "$repository_root/cli/Cargo.toml"
)"
[ -n "$version" ]

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/sok-installer-test.XXXXXX")"

cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

fixture_root="$tmp_dir/SoK-test"
source_archive="$tmp_dir/source.tar.gz"
mkdir -p "$fixture_root"

tar -C "$repository_root" \
  --exclude='./.git' \
  --exclude='./.private' \
  --exclude='./.crack' \
  --exclude='./cli/target' \
  --exclude='./dist' \
  -cf - . |
  tar -C "$fixture_root" -xf -
tar -C "$tmp_dir" -czf "$source_archive" SoK-test

codex_home="$tmp_dir/codex-home"
CODEX_HOME="$codex_home" \
  SOK_SOURCE_URL="file://$source_archive" \
  SOK_BUILD_TARGET_DIR="$repository_root/cli/target" \
  sh "$repository_root/install.sh" --ref test-ref

installed="$codex_home/skills/structure-of-knowledge"
test -f "$installed/SKILL.md"
test -x "$installed/bin/sok"
test "$("$installed/bin/sok" --version)" = "sok $version (rust)"
test ! -e "$installed/Cargo.toml"
test ! -e "$installed/Cargo.lock"
test ! -e "$installed/src"
test ! -e "$installed/target"

touch "$installed/stale-file"
CODEX_HOME="$codex_home" \
  SOK_REF="environment-ref" \
  SOK_SOURCE_URL="file://$source_archive" \
  SOK_BUILD_TARGET_DIR="$repository_root/cli/target" \
  sh "$repository_root/install.sh" --ref test-ref
test ! -e "$installed/stale-file"

bad_fixture_root="$tmp_dir/SoK-bad"
bad_source_archive="$tmp_dir/bad-source.tar.gz"
mkdir -p "$bad_fixture_root"
tar -C "$tmp_dir" -czf "$bad_source_archive" SoK-bad

bad_codex_home="$tmp_dir/bad-codex-home"
if CODEX_HOME="$bad_codex_home" \
  SOK_SOURCE_URL="file://$bad_source_archive" \
  sh "$repository_root/install.sh" --ref test-ref >/dev/null 2>&1; then
  printf 'installer accepted an incomplete source archive\n' >&2
  exit 1
fi
test ! -e "$bad_codex_home/skills/structure-of-knowledge"

printf 'Installer integration test passed.\n'
