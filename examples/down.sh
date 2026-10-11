#!/usr/bin/env bash
# Tear down the codmin example stack and drop volumes.
#
# Usage:
#   ./examples/down.sh             # docker compose down -v
#   ./examples/down.sh --keep-vol  # keep the postgres volume around

set -euo pipefail

EXAMPLES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${EXAMPLES_DIR}/docker-compose.example-stack.yaml"

KEEP_VOL=0
for arg in "$@"; do
    case "${arg}" in
        --keep-vol) KEEP_VOL=1 ;;
        -h|--help)
            sed -n '2,8p' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            echo "[example-stack/down] unknown flag: ${arg}" >&2
            exit 2
            ;;
    esac
done

if [[ ! -f "${COMPOSE_FILE}" ]]; then
    echo "[example-stack/down] missing ${COMPOSE_FILE}" >&2
    exit 2
fi

if (( KEEP_VOL == 1 )); then
    echo "[example-stack/down] docker compose down (keeping volumes)"
    docker compose -f "${COMPOSE_FILE}" down
else
    echo "[example-stack/down] docker compose down -v"
    docker compose -f "${COMPOSE_FILE}" down -v
fi

echo "[example-stack/down] done"
