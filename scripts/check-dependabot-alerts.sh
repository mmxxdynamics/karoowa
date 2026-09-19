#!/usr/bin/env bash
#
# Fail when GitHub holds an open Dependabot alert at or above a severity
# threshold that is not recorded as accepted risk.
#
# Why this exists: `cargo audit` and `cargo deny` both read only the RustSec
# database. An advisory carrying a GHSA but no RUSTSEC entry is invisible to
# both, by construction. That is not hypothetical — GHSA-vxx9-2994-q338
# (yamux, HIGH) has no RustSec advisory at all and sat open for 136 days while
# every build stayed green. See issue #59.
#
# Two failure modes are deliberately distinguished:
#
#   - alerts found       -> exit 1, listing them
#   - alerts unreadable  -> exit 1 by default (`--on-unreadable fail`)
#
# The second matters. PR #29 landed because a security job reported "0
# vulnerabilities" while the API call behind it was failing; a gate that
# cannot read must never look like a gate that read and found nothing. The
# caller may downgrade it to a warning for events where the GITHUB_TOKEN is
# structurally read-only (Dependabot-authored PRs, fork PRs) — see ci.yml.

set -euo pipefail

THRESHOLD="high"
ALLOWLIST=".github/advisory-allowlist.txt"
DENY_TOML="deny.toml"
AUDIT_TOML=".cargo/audit.toml"
ON_UNREADABLE="fail"
REPO="${GITHUB_REPOSITORY:-}"

usage() {
    cat <<'EOF'
Usage: check-dependabot-alerts.sh [options]

  --repo <owner/name>       Defaults to $GITHUB_REPOSITORY.
  --threshold <severity>    low | medium | high | critical. Default: high.
  --allowlist <path>        Default: .github/advisory-allowlist.txt
  --on-unreadable <mode>    fail | warn. What to do when the alerts API is not
                            readable with the available token. Default: fail.
  -h, --help

Requires the `gh` CLI with a token that can read Dependabot alerts. In Actions
that means `permissions: vulnerability-alerts: read`; `security-events: read`
is NOT sufficient and returns 403.
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --repo) REPO="$2"; shift 2 ;;
        --threshold) THRESHOLD="$2"; shift 2 ;;
        --allowlist) ALLOWLIST="$2"; shift 2 ;;
        --on-unreadable) ON_UNREADABLE="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
done

case "$ON_UNREADABLE" in
    fail|warn) ;;
    *) echo "--on-unreadable must be 'fail' or 'warn', got '${ON_UNREADABLE}'" >&2; exit 2 ;;
esac

if [ -z "$REPO" ]; then
    echo "no repository: pass --repo owner/name or set GITHUB_REPOSITORY" >&2
    exit 2
fi

# GitHub's REST severities. GraphQL says MODERATE where REST says medium;
# accept both so the ranking does not depend on which API produced the value.
severity_rank() {
    case "$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')" in
        critical) echo 4 ;;
        high) echo 3 ;;
        medium|moderate) echo 2 ;;
        low) echo 1 ;;
        *) echo 0 ;;
    esac
}

THRESHOLD_RANK="$(severity_rank "$THRESHOLD")"
if [ "$THRESHOLD_RANK" -eq 0 ]; then
    echo "unrecognised --threshold '${THRESHOLD}'" >&2
    exit 2
fi

# Emit a GitHub Actions annotation when running in Actions, a plain line
# otherwise, so the script is equally usable from a laptop.
annotate() {
    local level="$1" message="$2"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then
        printf '::%s::%s\n' "$level" "$message"
    else
        printf '%s: %s\n' "$level" "$message"
    fi
}

# ---------------------------------------------------------------------------
# 1. Allowlist, and its consistency with the two RUSTSEC ignore lists.
#
# This half needs no network and no token, so it runs first and always runs:
# a drifted allowlist is a defect whether or not the API is reachable.
# ---------------------------------------------------------------------------

if [ ! -f "$ALLOWLIST" ]; then
    echo "allowlist not found: ${ALLOWLIST}" >&2
    exit 2
fi

allow_ghsa=()
allow_rustsec=()
allow_reason=()
sync_errors=0

while IFS= read -r line; do
    entry="${line%%#*}"
    if [ "${line#*#}" = "$line" ]; then
        reason=""
    else
        reason="${line#*#}"
    fi
    # shellcheck disable=SC2086 # deliberate word-splitting of the two columns
    set -- $entry
    if [ $# -eq 0 ]; then
        continue
    fi
    if [ $# -ne 2 ]; then
        echo "malformed allowlist line (want '<GHSA id>  <RUSTSEC id|->  # reason'): ${line}" >&2
        exit 2
    fi
    case "$1" in
        GHSA-*) ;;
        *) echo "allowlist column 1 must be a GHSA id, got '$1'" >&2; exit 2 ;;
    esac
    case "$2" in
        -|RUSTSEC-*) ;;
        *) echo "allowlist column 2 must be a RUSTSEC id or '-', got '$2'" >&2; exit 2 ;;
    esac
    allow_ghsa+=("$1")
    allow_rustsec+=("$2")
    allow_reason+=("$(printf '%s' "$reason" | sed 's/^[[:space:]]*//')")
done < <(grep -v '^[[:space:]]*#' "$ALLOWLIST" | grep -v '^[[:space:]]*$' || true)

allow_count="${#allow_ghsa[@]}"
echo "Allowlist: ${allow_count} accepted-risk entries from ${ALLOWLIST}"

# Every RUSTSEC alias named in the allowlist must also be ignored by
# cargo-deny and cargo-audit. Without this the same advisory could be accepted
# here and enforced there — or, worse, quietly dropped from both.
i=0
while [ "$i" -lt "$allow_count" ]; do
    rustsec="${allow_rustsec[$i]}"
    if [ "$rustsec" != "-" ]; then
        for f in "$DENY_TOML" "$AUDIT_TOML"; do
            if [ ! -f "$f" ]; then
                annotate error "${f} not found; cannot verify allowlist sync"
                sync_errors=$((sync_errors + 1))
            elif ! grep -q "\"${rustsec}\"" "$f"; then
                annotate error "${allow_ghsa[$i]} is allowlisted as ${rustsec}, but ${rustsec} is not ignored in ${f} — the advisory lists have drifted"
                sync_errors=$((sync_errors + 1))
            fi
        done
    fi
    i=$((i + 1))
done

if [ "$sync_errors" -gt 0 ]; then
    echo "Allowlist is out of sync with the RUSTSEC ignore lists (${sync_errors} problem(s))." >&2
    exit 1
fi
echo "Allowlist is consistent with ${DENY_TOML} and ${AUDIT_TOML}."
echo

# ---------------------------------------------------------------------------
# 2. Open alerts.
# ---------------------------------------------------------------------------

stderr_file="$(mktemp)"
trap 'rm -f "$stderr_file"' EXIT

alerts=""
if ! alerts="$(
    gh api "repos/${REPO}/dependabot/alerts?state=open&per_page=100" --paginate \
        --jq '.[] | [
            (.number | tostring),
            .security_advisory.severity,
            .security_advisory.ghsa_id,
            .dependency.package.name,
            (.dependency.scope // "unknown"),
            .created_at,
            .html_url
        ] | @tsv' 2>"$stderr_file"
)"; then
    detail="$(tr '\n' ' ' <"$stderr_file")"
    if [ "$ON_UNREADABLE" = "warn" ]; then
        annotate warning "Dependabot alerts API not readable with this token, so this leg checked nothing. ${detail}"
        echo "This event runs with a read-only GITHUB_TOKEN, so the alerts leg is advisory here."
        echo "The same check runs unrestricted on push to main and on the daily schedule."
        exit 0
    fi
    annotate error "Dependabot alerts API not readable: ${detail}"
    cat >&2 <<'EOF'

The gate could not read the alerts API, so it cannot say whether the repo is
clean. That is a failure, not a pass — see PR #29, where a security job
reported zero vulnerabilities because the API call behind it had failed.

If this is a 403, the job is missing 'vulnerability-alerts: read'.
'security-events: read' does not cover this endpoint.
EOF
    exit 1
fi

blocking=0
allowed=0
below=0
matched_ghsa=""

echo "Open Dependabot alerts (threshold: ${THRESHOLD}):"
if [ -n "$alerts" ]; then
    while IFS=$'\t' read -r number severity ghsa pkg scope created url; do
        if [ -z "$number" ]; then
            continue
        fi
        # Record the match before the threshold test: an allowlisted advisory
        # that is open but below the threshold is still live, not stale.
        matched_ghsa="${matched_ghsa} ${ghsa}"

        rank="$(severity_rank "$severity")"
        if [ "$rank" -lt "$THRESHOLD_RANK" ]; then
            below=$((below + 1))
            printf '  below threshold  #%-4s %-8s %-22s %s\n' "$number" "$severity" "$pkg" "$ghsa"
            continue
        fi

        idx=""
        j=0
        while [ "$j" -lt "$allow_count" ]; do
            if [ "${allow_ghsa[$j]}" = "$ghsa" ]; then
                idx="$j"
                break
            fi
            j=$((j + 1))
        done

        if [ -n "$idx" ]; then
            allowed=$((allowed + 1))
            printf '  accepted risk    #%-4s %-8s %-22s %s  (%s)\n' \
                "$number" "$severity" "$pkg" "$ghsa" "${allow_reason[$idx]}"
        else
            blocking=$((blocking + 1))
            printf '  BLOCKING         #%-4s %-8s %-22s %s  scope=%s opened=%s\n' \
                "$number" "$severity" "$pkg" "$ghsa" "$scope" "$created"
            annotate error "Open ${severity} Dependabot alert #${number}: ${pkg} (${ghsa}) — ${url}"
        fi
    done <<<"$alerts"
else
    echo "  (none)"
fi

echo
echo "At or above '${THRESHOLD}': $((blocking + allowed))  (${allowed} accepted risk, ${blocking} blocking)"
echo "Below '${THRESHOLD}': ${below}"

# An allowlist entry with no matching open alert is stale: the alert was fixed
# or dismissed, and the accepted risk should be retired along with its RUSTSEC
# siblings. A warning, not a failure — a dismissed alert is no reason to block
# merges, but it should not be left to rot here either.
i=0
while [ "$i" -lt "$allow_count" ]; do
    case " ${matched_ghsa} " in
        *" ${allow_ghsa[$i]} "*) ;;
        *)
            also=""
            if [ "${allow_rustsec[$i]}" != "-" ]; then
                also=" (and ${allow_rustsec[$i]} from ${DENY_TOML} / ${AUDIT_TOML})"
            fi
            annotate warning "Allowlist entry ${allow_ghsa[$i]} matches no open alert — retire it from ${ALLOWLIST}${also}"
            ;;
    esac
    i=$((i + 1))
done

if [ "$blocking" -gt 0 ]; then
    echo
    echo "${blocking} open Dependabot alert(s) at or above '${THRESHOLD}' are not accepted risk." >&2
    echo "Fix them, or record them in ${ALLOWLIST} with a justification." >&2
    exit 1
fi

echo "No blocking Dependabot alerts."
