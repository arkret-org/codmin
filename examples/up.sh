#!/usr/bin/env bash
# Bring up the codmin example stack (postgres + coauth + coland + floria + codmin).
#
# Usage:
#   ./examples/up.sh                 # build images, generate config, bring stack up
#   ./examples/up.sh --no-build      # reuse existing images
#   ./examples/up.sh --regen-config  # regenerate examples/coauth-config.yaml
#
# Image overrides honour the same env vars the compose file reads:
#   COAUTH_IMAGE  COLAND_IMAGE  FLORIA_IMAGE  CODMIN_IMAGE
#
# Exit codes:
#   0 - every service reported healthy within budget
#   2 - invocation / pre-flight error
#   3 - compose up failed; recent logs are dumped to stderr

set -euo pipefail

EXAMPLES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${EXAMPLES_DIR}/docker-compose.example-stack.yaml"
GENERATED_CONFIG="${EXAMPLES_DIR}/coauth-config.yaml"
COAUTH_IMAGE_REF="${COAUTH_IMAGE:-coauth:dev}"

NO_BUILD=0
REGEN_CONFIG=0

for arg in "$@"; do
    case "${arg}" in
        --no-build) NO_BUILD=1 ;;
        --regen-config) REGEN_CONFIG=1 ;;
        -h|--help)
            sed -n '2,15p' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            echo "[example-stack/up] unknown flag: ${arg}" >&2
            exit 2
            ;;
    esac
done

if [[ ! -f "${COMPOSE_FILE}" ]]; then
    echo "[example-stack/up] missing ${COMPOSE_FILE}" >&2
    exit 2
fi

if ! command -v docker >/dev/null 2>&1; then
    echo "[example-stack/up] docker not on PATH" >&2
    exit 2
fi

if ! docker info >/dev/null 2>&1; then
    echo "[example-stack/up] docker daemon not reachable (start Docker Desktop / dockerd)" >&2
    exit 2
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "[example-stack/up] python3 not on PATH; required for coauth config rewrite" >&2
    exit 2
fi

if ! command -v openssl >/dev/null 2>&1; then
    echo "[example-stack/up] openssl not on PATH; required for verified development TLS" >&2
    exit 2
fi

UMBRELLA_DIR="$(cd "${EXAMPLES_DIR}/../.." && pwd)"
for sibling in arkret-rust-sdk arkret-spec coauth coland floria codmin cotest garth bridges; do
    if [[ ! -d "${UMBRELLA_DIR}/${sibling}" ]]; then
        echo "[example-stack/up] missing sibling ${UMBRELLA_DIR}/${sibling}" >&2
        echo "[example-stack/up] umbrella layout expected: arkret/{arkret-rust-sdk,arkret-spec,coauth,coland,floria,codmin,cotest,garth}/" >&2
        exit 2
    fi
done

if [[ ! -f "${UMBRELLA_DIR}/../savfox-ai/savfox/Cargo.toml" ]]; then
    echo "[example-stack/up] missing Savfox build context ${UMBRELLA_DIR}/../savfox-ai/savfox" >&2
    exit 2
fi

if (( NO_BUILD == 0 )); then
    echo "[example-stack/up] building coauth image first (needed for config gen)"
    docker compose -f "${COMPOSE_FILE}" build coauth || {
        echo "[example-stack/up] coauth build failed" >&2
        exit 3
    }
fi

if [[ ! -f "${GENERATED_CONFIG}" ]] || (( REGEN_CONFIG == 1 )); then
    echo "[example-stack/up] generating ${GENERATED_CONFIG} via ephemeral coauth"
    docker run --rm "${COAUTH_IMAGE_REF}" config generate > "${GENERATED_CONFIG}.raw" || {
        echo "[example-stack/up] coauth config generate failed (image=${COAUTH_IMAGE_REF})" >&2
        rm -f "${GENERATED_CONFIG}.raw"
        exit 3
    }

    python3 - "${GENERATED_CONFIG}.raw" "${GENERATED_CONFIG}" <<'PY'
import pathlib
import re
import sys

src = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
out_path = pathlib.Path(sys.argv[2])

src, secret_sections = re.subn(
    r"(?m)^secrets:\n(?:^[ \t]+.*\n|^\n)*",
    "secrets:\n"
    "  backend: encrypted_file\n"
    "  path: /var/lib/coauth/keystore.v1\n"
    "  master_key_file: /run/secrets/master-key\n",
    src,
    count=1,
)
if secret_sections != 1:
    sys.exit("FATAL: no generated secrets section to configure the durable container KeyStore")

src = re.sub(
    r"^(\s*uri:).*$",
    r"\1 postgresql://arkret:arkret@postgres:5432/coauth",
    src,
    count=1,
    flags=re.MULTILINE,
)

src = re.sub(
    r"^(\s*public_base_url:).*$",
    r"\1 http://coauth:7080/",
    src,
    count=1,
    flags=re.MULTILINE,
)
src = re.sub(
    r"^(\s*issuer:).*$",
    r"\1 http://coauth:7080/",
    src,
    count=1,
    flags=re.MULTILINE,
)

listeners_replacement = (
    "  listeners:\n"
    "  - name: web\n"
    "    resources:\n"
    "    - name: discovery\n"
    "    - name: human\n"
    "    - name: oauth\n"
    "    - name: restapi\n"
    "    - name: assets\n"
    "    - name: adminapi\n"
    "    - name: health\n"
    "    binds:\n"
    "    - address: '0.0.0.0:7080'\n"
    "    proxy_protocol: false\n"
)

http_match = re.search(r"^http:\s*$", src, flags=re.MULTILINE)
if not http_match:
    sys.exit("FATAL: no `http:` section in generated config")
http_start = http_match.end() + 1

listeners_match = re.search(r"^  listeners:\s*$", src[http_start:], flags=re.MULTILINE)
if not listeners_match:
    sys.exit("FATAL: no `http.listeners:` key in generated config")
listeners_start = http_start + listeners_match.start()

lines = src[listeners_start:].splitlines(keepends=True)
offset = len(lines[0])
for line in lines[1:]:
    if line and not line[0].isspace():
        break
    if len(line) > 2 and line[0] == " " and line[1] == " " and line[2].isalpha():
        break
    offset += len(line)

tail_start = listeners_start + offset
src = src[:listeners_start] + listeners_replacement + src[tail_start:]

out_path.write_text(src, encoding="utf-8")
PY
    rm -f "${GENERATED_CONFIG}.raw"
    echo "[example-stack/up] wrote $(wc -l < "${GENERATED_CONFIG}") line config"
else
    echo "[example-stack/up] reusing existing ${GENERATED_CONFIG} (--regen-config to refresh)"
fi

if (( NO_BUILD == 0 )); then
    # Each Rust/Dioxus image can use the full compiler budget. Avoid running
    # several independent release builds at once on a cold CI runner.
    for service in coland floria codmin; do
        echo "[example-stack/up] building ${service} image"
        docker compose -f "${COMPOSE_FILE}" build "${service}" || {
            echo "[example-stack/up] ${service} image build failed" >&2
            exit 3
        }
    done
fi

python3 "${EXAMPLES_DIR}/prepare-tls.py"

echo "[example-stack/up] initializing separate durable service KeyStores (existing keys are retained)"
docker compose -f "${COMPOSE_FILE}" run --rm --no-deps coland-keystore-init

echo "[example-stack/up] preparing service databases"
docker compose -f "${COMPOSE_FILE}" up -d --wait postgres
docker compose -f "${COMPOSE_FILE}" exec -T postgres \
    psql -v ON_ERROR_STOP=1 -U arkret -d arkret < "${EXAMPLES_DIR}/init-databases.sql"

keystore_state=0
docker compose -f "${COMPOSE_FILE}" run --rm --no-deps \
    --entrypoint /bin/sh coland-keystore-init -ec \
    'if test -e /var/lib/coauth/keystore.v1; then exit 0; else exit 42; fi' || keystore_state=$?
if (( keystore_state != 0 && keystore_state != 42 )); then
    echo "[example-stack/up] could not inspect the durable Coauth KeyStore" >&2
    exit 3
fi
if (( keystore_state == 42 )); then
    echo "[example-stack/up] explicitly provisioning the empty Coauth KeyStore"
    if ! docker compose -f "${COMPOSE_FILE}" up -d --wait coauth-provision; then
        docker compose -f "${COMPOSE_FILE}" logs --tail=120 coauth-provision >&2 || true
        exit 3
    fi
    docker compose -f "${COMPOSE_FILE}" stop coauth-provision
    docker compose -f "${COMPOSE_FILE}" rm --force coauth-provision
fi

echo "[example-stack/up] docker compose up -d --wait"
if ! docker compose -f "${COMPOSE_FILE}" up -d --wait; then
    echo "[example-stack/up] compose up failed; dumping recent logs" >&2
    docker compose -f "${COMPOSE_FILE}" ps >&2 || true
    docker compose -f "${COMPOSE_FILE}" logs --tail=120 >&2 || true
    exit 3
fi

cat <<'EOF'
[example-stack/up] stack is up.

  postgres : localhost:55432   (user=arkret pass=arkret db=arkret)
  coauth   : http://localhost:57080/health
  coland   : https://127.0.0.1:58787/health (CA: examples/.local/tls/ca.crt)
  floria   : http://localhost:55000/ready
  codmin   : http://localhost:58200/healthz

Run ./examples/smoke.sh to verify each /health returns 200.
Run ./examples/down.sh to tear down + drop volumes.
EOF
