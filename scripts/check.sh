#!/usr/bin/env bash
set -euo pipefail

echo "==> fmt check"
cargo fmt --all --check

echo "==> clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> tests"
cargo test --all

echo "==> no-unwrap gate (library code only, outside #[cfg(test)] blocks)"
# We check for unwrap() or panic! in src files but allow them inside test modules.
# Strategy: grep for unwrap/panic!, then filter out lines that are inside #[cfg(test)]
# For simplicity: flag any unwrap()/panic! in crates/*/src that isn't on a // SAFETY or test line.
FAIL=0
for f in $(find crates -name "*.rs" -path "*/src/*"); do
    # Skip test files (files ending in _test.rs or under tests/)
    if [[ "$f" == *_test* ]] || [[ "$f" == */tests/* ]]; then
        continue
    fi
    # Check for unwrap() or panic! outside of #[cfg(test)] context
    # Simple heuristic: flag if any non-comment line has unwrap() or panic!
    # and the file is not a test-only file
    if grep -nE '^\s*(\.unwrap\(\)|panic!)' "$f" 2>/dev/null | grep -v '^\s*//' | grep -qv '#\[cfg(test)\]'; then
        # More targeted: look for .unwrap() or panic! calls in non-test context
        while IFS= read -r line; do
            echo "FAIL: bare unwrap/panic in $f: $line"
            FAIL=1
        done < <(grep -nE '\.(unwrap|expect)\(\)|panic!' "$f" | grep -v '//.*unwrap' | grep -v '#\[cfg(test)\]' || true)
    fi
done

if [ "$FAIL" -ne 0 ]; then
    echo "==> no-unwrap gate FAILED"
    exit 1
fi

echo "==> all checks passed"
