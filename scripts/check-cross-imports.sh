#!/usr/bin/env bash
# check-cross-imports.sh — CI guardrail for the Karoowa open-core boundary.
#
# Fails (exit 1) if any source file under core/ references enterprise/.
# This prevents the OSS layer from accidentally depending on proprietary code.
#
# Enterprise code (enterprise/) is allowed to import from core/.
#
# Patterns checked (in .rs files under core/):
#   - use enterprise::
#   - use crate::enterprise::
#   - mod enterprise;
#   - path = "...enterprise..."  (in Cargo.toml-style path deps)
#   - Any other string "enterprise/" or "enterprise::" in source
#
# Usage: ./scripts/check-cross-imports.sh
# Returns: 0 if clean, 1 if cross-imports found.
#
# See decision D-012 in specs/strategy/03_decision_log.md
# See dev plan T1.0.4 in specs/development/dev_plan.md

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORE_DIR="$REPO_ROOT/core"

if [ ! -d "$CORE_DIR" ]; then
    echo "ERROR: core/ directory not found at $CORE_DIR"
    exit 1
fi

# Search for ACTUAL code imports of enterprise, not doc comments.
# Strategy: check .rs files for import-shaped patterns (use/mod/path) on
# non-comment lines, and .toml files for path references to enterprise.
#
# For .rs files: filter out lines that are purely comments (// or //!)
# before checking for enterprise references. This avoids false positives
# from doc comments that mention enterprise/ as documentation.
#
# For .toml files: any reference to enterprise in a path dep is a real import.

RS_MATCHES=$(grep -rn \
    -e 'use.*enterprise' \
    -e 'mod enterprise' \
    -e 'extern crate.*enterprise' \
    --include='*.rs' \
    "$CORE_DIR" 2>/dev/null \
    | grep -v '^\s*//' \
    | grep -v '//.*enterprise' \
    || true)

TOML_MATCHES=$(grep -rn \
    -e 'enterprise' \
    --include='*.toml' \
    "$CORE_DIR" 2>/dev/null || true)

MATCHES="${RS_MATCHES}${TOML_MATCHES}"

if [ -n "$MATCHES" ]; then
    echo "=========================================="
    echo "CROSS-IMPORT GUARDRAIL FAILED"
    echo "=========================================="
    echo ""
    echo "The following files under core/ reference enterprise/."
    echo "This is not allowed — the OSS layer must not depend on"
    echo "the proprietary enterprise layer."
    echo ""
    echo "Offending lines:"
    echo "$MATCHES"
    echo ""
    echo "Fix: move the dependency into enterprise/, or remove the import."
    echo "See: specs/strategy/02_technical_strategy.md (Open-core strategy)"
    echo "=========================================="
    exit 1
fi

echo "Cross-import guardrail: PASSED (no enterprise/ references in core/)"
exit 0
