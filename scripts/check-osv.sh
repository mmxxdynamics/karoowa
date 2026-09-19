#!/usr/bin/env bash
#
# Scan Cargo.lock against OSV and fail on findings that RustSec does not carry.
#
# Division of labour, deliberately strict so that no advisory is ignored in two
# places at once:
#
#   cargo-audit / cargo-deny   authority for everything with a RUSTSEC id.
#                              Ignores live in .cargo/audit.toml and deny.toml.
#   this script                authority for everything else. OSV aggregates
#                              RustSec *and* the GitHub Advisory Database, so
#                              "in OSV, no RUSTSEC id" is exactly the blind
#                              spot that the other two cannot see.
#
# A finding whose alias set contains any RUSTSEC id is therefore skipped here:
# cargo-audit already decides it, under the ignore list that is already
# reviewed. Suppressing it a second time in a third file is how ignore lists
# drift apart.
#
# The remaining findings — GHSA-only — are checked against
# .github/advisory-allowlist.txt. Today that is exactly one: GHSA-vxx9-2994-q338
# (yamux, HIGH), which has no RustSec advisory and which `cargo audit` has been
# green on for its entire 136-day life. See issue #59.
#
# Why this exists alongside the Dependabot alerts job, which finds the same
# advisory: this leg needs no GitHub token. On Dependabot-authored and fork
# PRs the alerts API is not reliably readable, and those are precisely the PRs
# that change dependency versions. Without this job those PRs would be checked
# by RustSec alone.

set -euo pipefail

LOCKFILE="Cargo.lock"
ALLOWLIST=".github/advisory-allowlist.txt"
OSV_SCANNER="osv-scanner"

usage() {
    cat <<'EOF'
Usage: check-osv.sh [options]

  --lockfile <path>    Default: Cargo.lock
  --allowlist <path>   Default: .github/advisory-allowlist.txt
  --osv-scanner <bin>  Default: osv-scanner (must be on PATH)
  -h, --help
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --lockfile) LOCKFILE="$2"; shift 2 ;;
        --allowlist) ALLOWLIST="$2"; shift 2 ;;
        --osv-scanner) OSV_SCANNER="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
done

for f in "$LOCKFILE" "$ALLOWLIST"; do
    if [ ! -f "$f" ]; then
        echo "not found: ${f}" >&2
        exit 2
    fi
done

annotate() {
    local level="$1" message="$2"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then
        printf '::%s::%s\n' "$level" "$message"
    else
        printf '%s: %s\n' "$level" "$message"
    fi
}

report="$(mktemp)"
trap 'rm -f "$report"' EXIT

# osv-scanner exits 1 when it finds something and 0 when it does not; both are
# successful scans. Anything else is a real error (network, bad arguments, no
# packages found) and must not be mistaken for a clean result.
rc=0
"$OSV_SCANNER" --lockfile "$LOCKFILE" --format json --output-file "$report" || rc=$?
if [ "$rc" -gt 1 ]; then
    annotate error "osv-scanner failed with exit ${rc}; the OSV leg checked nothing"
    exit 1
fi

# The same GHSA-keyed allowlist the Dependabot alerts job uses. That job owns
# validating it and asserting it stays in sync with the RUSTSEC ignore lists,
# so here we only need the ids.
# `|| true` because grep exits 1 on an allowlist that is entirely comments,
# and under `set -o pipefail` that would abort the script before it printed
# anything — a silent red that looks identical to a finding.
allow_ids="$(grep -v '^[[:space:]]*#' "$ALLOWLIST" | awk 'NF {print $1}' || true)"

blocking=0
allowed=0
delegated=0

echo "OSV findings for ${LOCKFILE}:"
while IFS=$'\t' read -r pkg version ids aliases severity; do
    if [ -z "$pkg" ]; then
        continue
    fi

    case ",${aliases}," in
        *,RUSTSEC-*)
            delegated=$((delegated + 1))
            printf '  rustsec-covered  %-22s %-12s %s\n' "$pkg" "$version" "$ids"
            continue
            ;;
    esac

    hit=""
    for id in ${ids//,/ }; do
        if printf '%s\n' "$allow_ids" | grep -qx "$id"; then
            hit="$id"
            break
        fi
    done

    if [ -n "$hit" ]; then
        allowed=$((allowed + 1))
        printf '  accepted risk    %-22s %-12s %s\n' "$pkg" "$version" "$ids"
    else
        blocking=$((blocking + 1))
        printf '  BLOCKING         %-22s %-12s %s  cvss=%s\n' "$pkg" "$version" "$ids" "${severity:-n/a}"
        annotate error "OSV advisory with no RustSec entry: ${pkg} ${version} — ${ids} (https://osv.dev/${ids%%,*})"
    fi
done < <(
    jq -r '
        .results[]?.packages[]?
        | .package.name as $p
        | .package.version as $v
        | .groups[]?
        | [$p, $v, (.ids | join(",")), (.aliases | join(",")), (.max_severity // "")]
        | @tsv
    ' "$report"
)

echo
echo "Delegated to cargo-audit / cargo-deny (has a RUSTSEC id): ${delegated}"
echo "GHSA-only, accepted risk: ${allowed}"
echo "GHSA-only, blocking: ${blocking}"

if [ "$blocking" -gt 0 ]; then
    echo
    echo "${blocking} OSV finding(s) have no RustSec entry and are not accepted risk." >&2
    echo "No RustSec entry means cargo-audit and cargo-deny cannot see them at all." >&2
    echo "Fix them, or record them in ${ALLOWLIST} with a justification." >&2
    exit 1
fi

echo "No blocking OSV findings."
