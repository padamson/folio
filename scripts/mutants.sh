#!/usr/bin/env bash
#
# Run cargo-mutants locally over the lines changed in HEAD (or any base
# ref you choose), so the runtime is in the "useful while iterating"
# range rather than "leave on overnight."
#
# Usage:
#   ./scripts/mutants.sh                      # diff HEAD~1..HEAD
#   ./scripts/mutants.sh main                 # diff main..HEAD
#   ./scripts/mutants.sh 0bb7329              # diff <sha>..HEAD
#   ./scripts/mutants.sh HEAD~5               # diff last 5 commits
#   ./scripts/mutants.sh -- --jobs 4          # default base + extra cargo-mutants args
#   ./scripts/mutants.sh main --jobs 4        # explicit base + extra args
#
# The first non-dash argument is the base ref; anything else (and
# everything after the first dash-prefixed arg) passes through to
# cargo-mutants. See https://mutants.rs/ for the full CLI surface.
#
# Why `--in-diff`: an unscoped `cargo mutants` run grows linearly with
# codebase size and routinely runs many hours. `--in-diff` narrows
# mutation to just the lines in the supplied diff — typically seconds
# to minutes for a normal commit.
#
# Prerequisites: `cargo install cargo-mutants` (once per machine).
#
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Resolve the base ref: first non-dash positional arg, defaulting to
# HEAD~1. Anything starting with `-` is treated as a cargo-mutants arg.
if [[ $# -gt 0 && "$1" != -* && "$1" != "--" ]]; then
  BASE="$1"
  shift
else
  BASE="HEAD~1"
fi

# `--` separator is allowed for clarity; consume it so it doesn't pass
# through to cargo-mutants as a positional.
if [[ $# -gt 0 && "$1" == "--" ]]; then
  shift
fi

# Skip rather than error when the base ref doesn't resolve. On the repo's
# root commit there is no parent, so `git diff HEAD~1..HEAD` exits 128 and
# the CI mutation-testing-diff job fails on the very first push. Make it a
# clean no-op instead; normal runs resume once a parent exists.
if ! git rev-parse --verify --quiet "${BASE}^{commit}" >/dev/null; then
  echo "base ref '${BASE}' does not resolve (e.g. root commit has no parent) — skipping mutation testing."
  exit 0
fi

DIFF="$(mktemp -t mutants.XXXXXX.diff)"
trap 'rm -f "$DIFF"' EXIT

git diff "${BASE}..HEAD" > "$DIFF"

if [[ ! -s "$DIFF" ]]; then
  echo "no diff between ${BASE} and HEAD — nothing to mutate."
  echo "tip: commit your changes locally first, then re-run."
  exit 0
fi

echo "mutating changes in ${BASE}..HEAD ($(wc -l < "$DIFF") diff lines)"
exec cargo mutants --in-diff "$DIFF" "$@"
