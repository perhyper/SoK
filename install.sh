#!/bin/sh

set -eu

repository="${SOK_REPOSITORY:-perhyper/SoK}"
source_ref="${SOK_REF:-main}"
source_url="${SOK_SOURCE_URL:-}"
codeload_url="${SOK_CODELOAD_URL:-https://codeload.github.com}"
build_target_dir="${SOK_BUILD_TARGET_DIR:-}"

tmp_dir=""
staging_dir=""
backup_dir=""
install_dir=""

usage() {
  cat <<'EOF'
Build and install Structure of Knowledge from a Git ref.

Usage:
  install.sh [--ref REF]

Options:
  --ref REF      Install a branch, tag, or commit (default: main).
  -h, --help     Show this help.

Environment:
  CODEX_HOME     Codex home directory (default: $HOME/.codex).
  SOK_REF        Git ref to install; overridden by --ref.
  SOK_REPOSITORY GitHub repository (default: perhyper/SoK).

Requirements:
  A stable Rust toolchain with Cargo, curl, and tar.
EOF
}

die() {
  printf 'sok installer: %s\n' "$*" >&2
  exit 1
}

cleanup() {
  if [ -n "$staging_dir" ] && [ -d "$staging_dir" ]; then
    rm -rf "$staging_dir"
  fi
  if [ -n "$backup_dir" ] &&
    { [ -e "$backup_dir" ] || [ -L "$backup_dir" ]; }; then
    if [ -n "$install_dir" ] && [ ! -e "$install_dir" ]; then
      mv "$backup_dir" "$install_dir" 2>/dev/null || true
    else
      rm -rf "$backup_dir"
    fi
  fi
  if [ -n "$tmp_dir" ] && [ -d "$tmp_dir" ]; then
    rm -rf "$tmp_dir"
  fi
}

trap cleanup EXIT
trap 'exit 1' HUP INT TERM

while [ "$#" -gt 0 ]; do
  case "$1" in
    --ref)
      [ "$#" -ge 2 ] || die "--ref requires a value"
      source_ref="$2"
      shift 2
      ;;
    --ref=*)
      source_ref="${1#*=}"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

for command_name in cargo curl tar awk grep mktemp uname; do
  command -v "$command_name" >/dev/null 2>&1 ||
    die "required command not found: $command_name"
done

case "$(uname -s)" in
  Darwin | Linux)
    ;;
  *)
    die "unsupported operating system: $(uname -s); use the manual installation instructions"
    ;;
esac

case "$source_ref" in
  "" | -* | /* | *..*)
    die "invalid Git ref: $source_ref"
    ;;
esac

printf '%s\n' "$source_ref" |
  grep -Eq '^[0-9A-Za-z][0-9A-Za-z._/-]*$' ||
  die "invalid Git ref: $source_ref"

if [ -z "$source_url" ]; then
  source_url="$codeload_url/$repository/tar.gz/$source_ref"
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/sok-install.XXXXXX")"
source_archive="$tmp_dir/source.tar.gz"
source_root="$tmp_dir/source"

printf 'Downloading %s@%s\n' "$repository" "$source_ref"
curl -fsSL --retry 3 --connect-timeout 15 \
  "$source_url" \
  -o "$source_archive" ||
  die "could not download source from $source_url"

archive_paths_are_safe() {
  tar -tzf "$1" |
    awk '
      /^\// { unsafe = 1 }
      {
        count = split($0, parts, "/")
        for (part_index = 1; part_index <= count; part_index++) {
          if (parts[part_index] == "..") {
            unsafe = 1
          }
        }
      }
      END { exit unsafe }
    '
}

archive_paths_are_safe "$source_archive" ||
  die "unsafe path found in the source archive"

mkdir -p "$source_root"
tar -xzf "$source_archive" --strip-components=1 -C "$source_root"

cli_manifest="$source_root/cli/Cargo.toml"
skill_source="$source_root/structure-of-knowledge"

[ -f "$cli_manifest" ] ||
  die "the selected ref does not contain cli/Cargo.toml"
[ -f "$source_root/cli/Cargo.lock" ] ||
  die "the selected ref does not contain cli/Cargo.lock"
[ -f "$skill_source/SKILL.md" ] ||
  die "the selected ref does not contain structure-of-knowledge/SKILL.md"

if [ -z "$build_target_dir" ]; then
  build_target_dir="$tmp_dir/target"
fi

printf 'Building the Rust CLI\n'
CARGO_TARGET_DIR="$build_target_dir" \
  cargo build --locked --release --manifest-path "$cli_manifest" --bin sok

cli_source="$build_target_dir/release/sok"
[ -f "$cli_source" ] ||
  die "Cargo completed without producing the sok executable"

crate_version="$(
  awk -F '"' '
    /^version = "[^"]+"/ {
      print $2
      exit
    }
  ' "$cli_manifest"
)"
[ -n "$crate_version" ] ||
  die "could not read the CLI version from cli/Cargo.toml"

expected_cli_version="sok $crate_version (rust)"
actual_cli_version="$("$cli_source" --version)" ||
  die "the built CLI could not be executed"
[ "$actual_cli_version" = "$expected_cli_version" ] ||
  die "CLI version mismatch: expected '$expected_cli_version', got '$actual_cli_version'"

if [ -n "${CODEX_HOME:-}" ]; then
  codex_home="$CODEX_HOME"
else
  [ -n "${HOME:-}" ] || die "HOME or CODEX_HOME must be set"
  codex_home="$HOME/.codex"
fi

skills_dir="$codex_home/skills"
install_dir="$skills_dir/structure-of-knowledge"
mkdir -p "$skills_dir"
staging_dir="$(mktemp -d "$skills_dir/.structure-of-knowledge.XXXXXX")"

tar -C "$skill_source" -cf - . | tar -C "$staging_dir" -xf -
mkdir -p "$staging_dir/bin"
cp "$cli_source" "$staging_dir/bin/sok"
chmod 0755 "$staging_dir/bin/sok"

if [ -e "$install_dir" ] || [ -L "$install_dir" ]; then
  backup_dir="$(mktemp -d "$skills_dir/.structure-of-knowledge.backup.XXXXXX")"
  rmdir "$backup_dir"
  mv "$install_dir" "$backup_dir"
fi

mv "$staging_dir" "$install_dir"
staging_dir=""

if [ -n "$backup_dir" ]; then
  rm -rf "$backup_dir"
  backup_dir=""
fi

printf 'Installed SoK %s from %s@%s\n' "$crate_version" "$repository" "$source_ref"
printf 'Skill: %s\n' "$install_dir"
printf 'CLI:   %s\n' "$install_dir/bin/sok"
