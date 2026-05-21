#!/usr/bin/env bash
# Creates a set of sandbox git repositories to showcase gitover's features:
# clean repos, dirty files, upstream ahead/behind, diverged branches,
# detached HEAD, and a merge conflict.
#
# Usage:  ./sandbox/create-repos.sh [<base-dir>]
#
#   <base-dir>  Root directory where sandbox repos are created.
#               Defaults to temporary directory so repos live outside the
#               project's own git tree.
#               (avoids editor "dubious ownership" warnings from editors
#               that auto-discover nested git repos).
#
# Re-running the script wipes and recreates all repos cleanly.
#
set -euo pipefail

# Resolve base dir: use argument if given, otherwise ~/gitover-sandbox.
SANDBOX="${1:-$(mktemp -d -t gitover-sandbox)}"
mkdir -p "$SANDBOX"
SANDBOX="$(cd "$SANDBOX" && pwd)"   # canonicalize to absolute path

# ── helpers ───────────────────────────────────────────────────────────────────

identity() {   # set a throwaway git identity inside repo $1
    git -C "$1" config user.email "sandbox@gitover.local"
    git -C "$1" config user.name  "Sandbox"
}

commit() {     # git add -A + commit with message "$2" inside repo "$1"
    git -C "$1" add -A
    git -C "$1" commit -m "$2" -q
}

# ── clean up previous run ─────────────────────────────────────────────────────

for d in repo-01 repo-01.origin repo-02 repo-03 repo-03.origin \
         repo-04 repo-04.origin repo-05 repo-05.origin \
         repo-06 repo-07 _tmp; do
    rm -rf "$SANDBOX/$d"
done

echo "Creating demo repos in $SANDBOX …"
echo ""

# ── repo-01: clean, fully in sync with upstream ────────────────────────────
# Shows:  clean status, ↑0 ↓0 on both upstream and trunk columns

echo "  repo-01 — clean, in sync with upstream"
git init --bare  "$SANDBOX/repo-01.origin" -b main -q
git clone        "$SANDBOX/repo-01.origin" "$SANDBOX/repo-01" -q
identity         "$SANDBOX/repo-01"
echo "# Alpha" > "$SANDBOX/repo-01/README.md"
commit           "$SANDBOX/repo-01" "initial commit"
git -C           "$SANDBOX/repo-01" push origin main -q

# ── repo-02: staged + modified + deleted + untracked ────────────────────────
# Shows:  S / M / D / U counts in the Status column; no upstream configured

echo "  repo-02 — staged + modified + deleted + untracked files"
git init                   "$SANDBOX/repo-02" -b main -q
identity                   "$SANDBOX/repo-02"
echo "# Beta"            > "$SANDBOX/repo-02/README.md"
echo "hello world"       > "$SANDBOX/repo-02/hello.txt"
echo "to be deleted"     > "$SANDBOX/repo-02/remove-me.txt"
echo "original content"  > "$SANDBOX/repo-02/config.txt"
commit                     "$SANDBOX/repo-02" "initial commit"
# produce each status kind
echo "modified"            >> "$SANDBOX/repo-02/hello.txt"   # M – modified
rm                            "$SANDBOX/repo-02/remove-me.txt" # D – deleted
echo "staged change"       >> "$SANDBOX/repo-02/config.txt"
git -C "$SANDBOX/repo-02"  add config.txt                      # S – staged
echo "brand new file"      >  "$SANDBOX/repo-02/new-file.txt"  # U – untracked

# ── repo-03: 3 commits ahead of upstream ───────────────────────────────────
# Shows:  ↑3 ↓0 in the upstream column; trunk column in sync

echo "  repo-03 — 3 commits ahead of upstream"
git init --bare  "$SANDBOX/repo-03.origin" -b main -q
git clone        "$SANDBOX/repo-03.origin" "$SANDBOX/repo-03" -q
identity         "$SANDBOX/repo-03"
echo "# Gamma" > "$SANDBOX/repo-03/README.md"
commit           "$SANDBOX/repo-03" "initial commit"
git -C           "$SANDBOX/repo-03" push origin main -q
for i in 1 2 3; do
    echo "local work $i" > "$SANDBOX/repo-03/work-$i.md"
    commit "$SANDBOX/repo-03" "local commit $i (not pushed)"
done

# ── repo-04: 2 commits behind upstream ────────────────────────────────────
# Shows:  ↑0 ↓2 in the upstream column

echo "  repo-04 — 2 commits behind upstream"
git init --bare  "$SANDBOX/repo-04.origin" -b main -q
git clone        "$SANDBOX/repo-04.origin" "$SANDBOX/repo-04" -q
identity         "$SANDBOX/repo-04"
echo "# Delta" > "$SANDBOX/repo-04/README.md"
commit           "$SANDBOX/repo-04" "initial commit"
git -C           "$SANDBOX/repo-04" push origin main -q
# advance the remote via a temporary clone
git clone        "$SANDBOX/repo-04.origin" "$SANDBOX/_tmp" -q
identity         "$SANDBOX/_tmp"
for i in 1 2; do
    echo "remote fix $i" > "$SANDBOX/_tmp/fix-$i.md"
    commit "$SANDBOX/_tmp" "remote fix $i"
done
git -C "$SANDBOX/_tmp" push origin main -q
rm -rf "$SANDBOX/_tmp"
git -C "$SANDBOX/repo-04" fetch -q   # see the behind count, don't pull

# ── repo-05: feature branch, 2 commits ahead of trunk ───────────────────
# Shows:  non-main branch, ↑0 ↓0 upstream, ↑2 ↓0 trunk (origin/main)

echo "  repo-05 — feature branch, 2 commits ahead of trunk"
git init --bare       "$SANDBOX/repo-05.origin" -b main -q
git clone             "$SANDBOX/repo-05.origin" "$SANDBOX/repo-05" -q
identity              "$SANDBOX/repo-05"
echo "# Epsilon"    > "$SANDBOX/repo-05/README.md"
commit                "$SANDBOX/repo-05" "initial commit"
git -C                "$SANDBOX/repo-05" push origin main -q
git -C                "$SANDBOX/repo-05" checkout -b feature/new-ui -q
echo "ui component" > "$SANDBOX/repo-05/ui.md"
commit                "$SANDBOX/repo-05" "add ui component"
echo "ui tests"     > "$SANDBOX/repo-05/ui-tests.md"
commit                "$SANDBOX/repo-05" "add ui tests"
git -C                "$SANDBOX/repo-05" push --set-upstream origin feature/new-ui -q

# ── repo-06: detached HEAD ──────────────────────────────────────────────────
# Shows:  branch column displays "detached <sha8>"

echo "  repo-06 — detached HEAD"
git init             "$SANDBOX/repo-06" -b main -q
identity             "$SANDBOX/repo-06"
echo "# Zeta v1"   > "$SANDBOX/repo-06/README.md"
commit               "$SANDBOX/repo-06" "v1 – will detach here"
echo "v2 content" >> "$SANDBOX/repo-06/README.md"
commit               "$SANDBOX/repo-06" "v2"
echo "v3 content" >> "$SANDBOX/repo-06/README.md"
commit               "$SANDBOX/repo-06" "v3 (HEAD)"
DETACH_SHA=$(git -C  "$SANDBOX/repo-06" rev-list --max-parents=0 HEAD)
git -C               "$SANDBOX/repo-06" checkout "$DETACH_SHA" -q 2>/dev/null

# ── repo-07: merge conflict ──────────────────────────────────────────────────
# Shows:  C count in the Status column

echo "  repo-07 — merge conflict on shared.txt"
git init               "$SANDBOX/repo-07" -b main -q
identity               "$SANDBOX/repo-07"
echo "shared line"   > "$SANDBOX/repo-07/shared.txt"
echo "other file"    > "$SANDBOX/repo-07/other.txt"
commit                 "$SANDBOX/repo-07" "initial commit"
git -C                 "$SANDBOX/repo-07" checkout -b branch-a -q
echo "branch-a edit" > "$SANDBOX/repo-07/shared.txt"
commit                 "$SANDBOX/repo-07" "branch-a: edit shared.txt"
git -C                 "$SANDBOX/repo-07" checkout main -q
echo "main edit"    > "$SANDBOX/repo-07/shared.txt"
commit                "$SANDBOX/repo-07" "main: edit shared.txt"
# merge → leaves shared.txt in conflict state
git -C                "$SANDBOX/repo-07" merge branch-a 2>/dev/null || true

echo ""
echo "Done. Add these paths to gitover with the 'A' key:"
echo ""
echo "  $SANDBOX/repo-01 : clean, in sync with upstream"
echo "  $SANDBOX/repo-02 : S+M+D+U in status column"
echo "  $SANDBOX/repo-03 : ↑3 ↓0 (3 commits ahead of upstream)"
echo "  $SANDBOX/repo-04 : ↑0 ↓2 (2 commits behind upstream)"
echo "  $SANDBOX/repo-05 : feature branch, ↑2 ahead of trunk"
echo "  $SANDBOX/repo-06 : detached HEAD"
echo "  $SANDBOX/repo-07 : merge conflict"
