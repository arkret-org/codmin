#!/usr/bin/env bash
# Smoke test: hit every health endpoint exposed by the example stack
# and confirm a 200 response within a budget.
#
# This is the minimum useful nightly assertion: "the docker-compose stack
# came up and every service is reachable on its published port". It does
# NOT exercise OAuth strands / federation / any business logic — that's the
# job of the playwright e2e suite, which can run against the same stack.
#
# Exit codes:
#   0 — every probed endpoint returned 200 within budget
#   3 — at least one endpoint failed; failing services are listed on stderr
#
# Usage:
#   ./examples/smoke.sh                   # default 30 attempts per probe @ 1s
#   SMOKE_ATTEMPT_BUDGET=60 ./smoke.sh    # bump per-probe attempts

set -euo pipefail

ATTEMPT_BUDGET="${SMOKE_ATTEMPT_BUDGET:-30}"
EXAMPLES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

probe() {
    local name="$1"
    local url="$2"
    local attempt
    for ((attempt = 1; attempt <= ATTEMPT_BUDGET; attempt++)); do
        local code
        local tls_options=()
        if [[ "${name}" == "coland" ]]; then
            tls_options=(--cacert "${EXAMPLES_DIR}/.local/tls/ca.crt")
        fi
        code="$(curl -fsS "${tls_options[@]}" -o /dev/null -m 2 -w '%{http_code}' "${url}" || echo "000")"
        if [[ "${code}" == "200" ]]; then
            echo "[example-stack/smoke] ${name}: 200 OK (after ${attempt} attempt(s))"
            return 0
        fi
        sleep 1
    done
    echo "[example-stack/smoke] ${name}: never returned 200 (${url})" >&2
    return 1
}

# Endpoint matrix kept in lock-step with docker-compose.example-stack.yaml
# port mapping. Update both when adding a service.
probes=(
    "coauth http://127.0.0.1:57080/health"
    "coland https://127.0.0.1:58787/health"
    "floria http://127.0.0.1:55000/ready"
    "codmin http://127.0.0.1:58200/healthz"
)

failures=0
failed_names=()
for entry in "${probes[@]}"; do
    name="${entry%% *}"
    url="${entry#* }"
    if ! probe "${name}" "${url}"; then
        failures=$((failures + 1))
        failed_names+=("${name}")
    fi
done

if (( failures > 0 )); then
    echo "[example-stack/smoke] ${failures} probe(s) failed: ${failed_names[*]}" >&2
    exit 3
fi

echo "[example-stack/smoke] all ${#probes[@]} probes returned 200"
