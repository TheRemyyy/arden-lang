#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <portable-archive>" >&2
  exit 1
fi

ARCHIVE_PATH="$1"
TEMP_ROOT="$(mktemp -d)"
HOME_DIR="${TEMP_ROOT}/home"
WORK_DIR="${TEMP_ROOT}/work"

mkdir -p "${HOME_DIR}" "${WORK_DIR}"
tar -xzf "${ARCHIVE_PATH}" -C "${TEMP_ROOT}"

BUNDLE_DIR="$(find "${TEMP_ROOT}" -mindepth 1 -maxdepth 1 -type d -name 'arden-*-portable' | head -n 1)"
if [[ -z "${BUNDLE_DIR}" ]]; then
  echo "portable bundle directory not found after extraction" >&2
  exit 1
fi

chmod +x "${BUNDLE_DIR}/arden"
if [[ -f "${BUNDLE_DIR}/install.sh" ]]; then
  chmod +x "${BUNDLE_DIR}/install.sh"
fi

"${BUNDLE_DIR}/arden" --version

cat > "${WORK_DIR}/hello.arden" <<'EOF'
import std.io.*;

function main(): None {
    println("Hello from portable Arden!");
    return None;
}
EOF

RUN_OUTPUT="$("${BUNDLE_DIR}/arden" run "${WORK_DIR}/hello.arden")"
printf '%s\n' "${RUN_OUTPUT}"
grep -F "Hello from portable Arden!" <<< "${RUN_OUTPUT}" >/dev/null

HOME="${HOME_DIR}" "${BUNDLE_DIR}/install.sh"
PATH="${HOME_DIR}/.local/bin:${PATH}" arden --version
