#!/bin/sh
# Fails when the frontend module graph is incomplete.
#
# The app ships plain ES modules: the browser loads frontend/js/main.js and
# follows every relative import from there. There is no bundler, so nothing
# checks the graph at compile time. A missing file surfaces only in the
# browser, as a blank page.
#
# That is exactly what happened once: the build context was prepared with an
# unanchored exclusion for the top-level data directory, which also deleted
# frontend/js/data/ because the pattern matched any path segment named "data".
# The image built fine and shipped without those 14 modules, so every screen
# went blank. This script is the guard: run it on the build context and the
# build stops instead.
#
# Usage: scripts/verify-frontend-graph.sh [frontend-dir]
set -eu

frontend_dir="${1:-frontend}"
js_dir="$frontend_dir/js"

for required in "$frontend_dir/index.html" "$js_dir/main.js" "$frontend_dir/styles/index.css"; do
    if [ ! -f "$required" ]; then
        echo "FAIL: $required is missing from the build context." >&2
        exit 1
    fi
done

missing=0
checked=0

# Collect every relative specifier in every module. Query strings and fragments
# are stripped: "./controllers.js?v=3" is the same file as "./controllers.js".
for file in $(find "$js_dir" -type f -name '*.js' | sort); do
    dir=$(dirname "$file")
    specifiers=$(
        grep -ohE "from '[.]{1,2}/[^']*'" "$file" 2>/dev/null \
            | sed -e "s/^from '//" -e "s/'$//" \
            | sed -e 's/[?#].*$//' \
            | sort -u || true
    )

    for specifier in $specifiers; do
        checked=$((checked + 1))
        if [ ! -f "$dir/$specifier" ]; then
            echo "FAIL: $file imports '$specifier', which is missing." >&2
            missing=$((missing + 1))
        fi
    done
done

if [ "$missing" -gt 0 ]; then
    echo "" >&2
    echo "The build context lost $missing module(s) the app imports at runtime." >&2
    echo "Rebuild the context from a clean checkout, for example:" >&2
    echo "  git archive HEAD | tar x -C <build-dir>" >&2
    echo "and prefix every exclusion with '/' so it only matches the repo root." >&2
    exit 1
fi

echo "Frontend module graph OK: $checked relative imports resolve."
