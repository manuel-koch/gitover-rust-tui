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
         repo-06 repo-07 repo-08 repo-08.origin \
         repo-09 repo-09.origin _tmp; do
    rm -rf "$SANDBOX/$d"
done

echo "Creating demo repos in $SANDBOX …"
echo ""

# ── repo-01: clean, fully in sync with upstream; 2 remote-only branches ─────
# Shows:  clean status, ↑0 ↓0 on both upstream and trunk columns,
#         remote branches feature/login and feature/dashboard not yet checked out

echo "  repo-01 — clean, in sync with upstream, 2 remote-only branches"
git init --bare  "$SANDBOX/repo-01.origin" -b main -q
git clone        "$SANDBOX/repo-01.origin" "$SANDBOX/repo-01" -q
identity         "$SANDBOX/repo-01"
echo "# Alpha" > "$SANDBOX/repo-01/README.md"
commit           "$SANDBOX/repo-01" "initial commit"
git -C           "$SANDBOX/repo-01" push origin main -q
# Push two branches to origin via a temp clone (no local branch in repo-01)
git clone        "$SANDBOX/repo-01.origin" "$SANDBOX/_tmp" -q
identity         "$SANDBOX/_tmp"
git -C           "$SANDBOX/_tmp" checkout -b feature/login -q
echo "login page" > "$SANDBOX/_tmp/login.txt"
commit           "$SANDBOX/_tmp" "feat: add login page"
git -C           "$SANDBOX/_tmp" push origin feature/login -q
git -C           "$SANDBOX/_tmp" checkout main -q
git -C           "$SANDBOX/_tmp" checkout -b feature/dashboard -q
echo "dashboard" > "$SANDBOX/_tmp/dashboard.txt"
commit           "$SANDBOX/_tmp" "feat: add dashboard"
git -C           "$SANDBOX/_tmp" push origin feature/dashboard -q
rm -rf           "$SANDBOX/_tmp"
git -C           "$SANDBOX/repo-01" fetch -q   # make remote branches visible, no local checkout

# ── repo-02: staged + modified + deleted + untracked ────────────────────────
# Shows:  S / M / D / U counts in the Status column; no upstream configured

echo "  repo-02 — staged + modified + deleted + untracked files"
git init                   "$SANDBOX/repo-02" -b main -q
identity                   "$SANDBOX/repo-02"
echo "# Beta"            > "$SANDBOX/repo-02/README.md"
echo "hello world"       > "$SANDBOX/repo-02/hello.txt"
echo "to be deleted"     > "$SANDBOX/repo-02/remove-me.txt"
echo "original content"  > "$SANDBOX/repo-02/config.txt"
printf "line one\nline two\nline three\nline four\nline six\nline seven\nline eight\nline nine\n" > "$SANDBOX/repo-02/notes.txt"
printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde' \
    > "$SANDBOX/repo-02/image.png"                              # binary file (initial)
commit                     "$SANDBOX/repo-02" "initial commit"
# feature/metrics — 3 commits with mixed changes
git -C "$SANDBOX/repo-02" checkout -b feature/metrics -q
echo "collect cpu mem io"  >  "$SANDBOX/repo-02/metrics.txt"
echo "## Metrics"          >> "$SANDBOX/repo-02/README.md"
commit                        "$SANDBOX/repo-02" "feat: add metrics collector"
echo "store: file"         >  "$SANDBOX/repo-02/metrics-store.txt"
echo "flush interval: 10s" >> "$SANDBOX/repo-02/metrics.txt"
echo "metrics.path=/var/metrics" >> "$SANDBOX/repo-02/config.txt"
commit                        "$SANDBOX/repo-02" "feat: add metrics storage and config"
echo "# Metrics"           >  "$SANDBOX/repo-02/metrics-docs.md"
echo "export format: json" >> "$SANDBOX/repo-02/metrics.txt"
printf "line one\nline two\nline three\nline four\nline five\n" > "$SANDBOX/repo-02/notes.txt"
commit                        "$SANDBOX/repo-02" "docs: document metrics module and revise notes"
git -C "$SANDBOX/repo-02" checkout main -q

# feature/search — 3 commits with mixed changes
git -C "$SANDBOX/repo-02" checkout -b feature/search -q
echo "index: inverted"     >  "$SANDBOX/repo-02/search.txt"
echo "## Search"           >> "$SANDBOX/repo-02/README.md"
commit                        "$SANDBOX/repo-02" "feat: add search index"
echo "filter: stopwords"   >  "$SANDBOX/repo-02/search-filters.txt"
echo "max results: 100"    >> "$SANDBOX/repo-02/search.txt"
echo "search.cache=true"   >> "$SANDBOX/repo-02/config.txt"
commit                        "$SANDBOX/repo-02" "feat: add search filters and extend config"
echo "# Search"            >  "$SANDBOX/repo-02/search-docs.md"
echo "ranking: bm25"       >> "$SANDBOX/repo-02/search.txt"
echo "hello from search"   >> "$SANDBOX/repo-02/hello.txt"
commit                        "$SANDBOX/repo-02" "docs: document search module and update hello"
git -C "$SANDBOX/repo-02" checkout main -q

# produce each status kind
echo "modified"            >> "$SANDBOX/repo-02/hello.txt"   # M – modified
rm                            "$SANDBOX/repo-02/remove-me.txt" # D – deleted
echo "staged change"       >> "$SANDBOX/repo-02/config.txt"
git -C "$SANDBOX/repo-02"  add config.txt                      # S – staged
echo "brand new file"      >  "$SANDBOX/repo-02/new-file.txt"  # U – untracked
printf "line one\nline three\nline four\nline nine\nline ten\n" > "$SANDBOX/repo-02/notes.txt"  # M – line removed
printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x02\x00\x00\x00\x02\x08\x02\x00\x00\x00\x90wS\xde' \
    > "$SANDBOX/repo-02/image.png"                              # M – binary modified

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

# ── repo-08: merged and active branches ────────────────────────────────────
# Shows:  merged-feature with ✓ marker (ahead=0 vs trunk), active-feature without

echo "  repo-08 — merged-feature (✓ merged to trunk) + active-feature (still ahead)"
git init --bare  "$SANDBOX/repo-08.origin" -b main -q
git clone        "$SANDBOX/repo-08.origin" "$SANDBOX/repo-08" -q
identity         "$SANDBOX/repo-08"
echo "# Eta"  > "$SANDBOX/repo-08/README.md"
commit           "$SANDBOX/repo-08" "initial commit"
git -C           "$SANDBOX/repo-08" push origin main -q

# merged-feature: create, push, merge back to main via merge commit, then push main
git -C           "$SANDBOX/repo-08" checkout -b merged-feature -q
echo "feature"  > "$SANDBOX/repo-08/feature.txt"
commit           "$SANDBOX/repo-08" "feat: implement feature"
git -C           "$SANDBOX/repo-08" push origin merged-feature -q
git -C           "$SANDBOX/repo-08" checkout main -q
git -C           "$SANDBOX/repo-08" merge merged-feature --no-ff -m "Merge merged-feature into main" -q
git -C           "$SANDBOX/repo-08" push origin main -q
# Now merged-feature has ahead=0, behind=1 vs origin/main → is_merged=true

# active-feature: branched from current main, has 1 new commit not yet merged
git -C           "$SANDBOX/repo-08" checkout -b active-feature -q
echo "wip"  > "$SANDBOX/repo-08/wip.txt"
commit           "$SANDBOX/repo-08" "wip: start new feature"
git -C           "$SANDBOX/repo-08" push origin active-feature -q
git -C           "$SANDBOX/repo-08" checkout main -q
# active-feature has ahead=1 vs origin/main → is_merged=false

# ── repo-09: local branches never pushed to origin ─────────────────────────
# Shows:  current branch with no upstream (push available in both repo and branch
#         action menus); non-current branch also never pushed (push available in
#         branch action menu)

echo "  repo-09 — current + non-current local branches never pushed to origin"
git init --bare       "$SANDBOX/repo-09.origin" -b main -q
git clone             "$SANDBOX/repo-09.origin" "$SANDBOX/repo-09" -q
identity              "$SANDBOX/repo-09"
echo "# Theta"      > "$SANDBOX/repo-09/README.md"
commit                "$SANDBOX/repo-09" "initial commit"
git -C                "$SANDBOX/repo-09" push origin main -q

# draft-notes: a non-current branch, never pushed (no upstream)
git -C                "$SANDBOX/repo-09" checkout -b draft-notes -q
echo "draft note 1" > "$SANDBOX/repo-09/notes-1.md"
commit                "$SANDBOX/repo-09" "draft: first note"
echo "draft note 2" > "$SANDBOX/repo-09/notes-2.md"
commit                "$SANDBOX/repo-09" "draft: second note"

# feature/wip: current branch, never pushed (no upstream) — leave HEAD here
git -C                "$SANDBOX/repo-09" checkout main -q
git -C                "$SANDBOX/repo-09" checkout -b feature/wip -q
echo "wip change 1" > "$SANDBOX/repo-09/wip-1.md"
commit                "$SANDBOX/repo-09" "wip: first change"
echo "wip change 2" > "$SANDBOX/repo-09/wip-2.md"
commit                "$SANDBOX/repo-09" "wip: second change"
# HEAD = feature/wip, upstream = none → push shown in repo menu + branch menu

echo ""
echo "Done. Add these paths to gitover with the 'A' key:"
echo ""
echo "  $SANDBOX/repo-01 : clean, in sync with upstream; remote branches feature/login + feature/dashboard not checked out"
echo "  $SANDBOX/repo-02 : S+M+D+U in status column"
echo "  $SANDBOX/repo-03 : ↑3 ↓0 (3 commits ahead of upstream)"
echo "  $SANDBOX/repo-04 : ↑0 ↓2 (2 commits behind upstream)"
echo "  $SANDBOX/repo-05 : feature branch, ↑2 ahead of trunk"
echo "  $SANDBOX/repo-06 : detached HEAD"
echo "  $SANDBOX/repo-07 : merge conflict"
echo "  $SANDBOX/repo-08 : merged-feature (✓, ahead=0 vs trunk) + active-feature (↑1 vs trunk)"
echo "  $SANDBOX/repo-09 : feature/wip (current, never pushed) + draft-notes (non-current, never pushed)"
