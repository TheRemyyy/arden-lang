#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <portable-archive>" >&2
  exit 1
fi

ARCHIVE_PATH="$1"
if [[ ! -f "${ARCHIVE_PATH}" ]]; then
  echo "portable archive not found: ${ARCHIVE_PATH}" >&2
  exit 1
fi

SMOKE_STEP_TIMEOUT_SECONDS="${SMOKE_STEP_TIMEOUT_SECONDS:-600}"

run_with_timeout() {
  local timeout_seconds="$1"
  shift
  python3 - "$timeout_seconds" "$@" <<'PY'
import subprocess
import sys

timeout = float(sys.argv[1])
cmd = sys.argv[2:]

try:
    completed = subprocess.run(
        cmd,
        check=False,
        text=True,
        capture_output=True,
        timeout=timeout,
    )
except subprocess.TimeoutExpired as exc:
    if exc.stdout:
        sys.stdout.write(exc.stdout)
    if exc.stderr:
        sys.stderr.write(exc.stderr)
    sys.stderr.write(
        f"error: smoke step timed out after {int(timeout)}s: {' '.join(cmd)}\n"
    )
    sys.exit(124)

if completed.stdout:
    sys.stdout.write(completed.stdout)
if completed.stderr:
    sys.stderr.write(completed.stderr)
sys.exit(completed.returncode)
PY
}

TEMP_ROOT_RAW="$(mktemp -d)"
TEMP_ROOT="$(cd "${TEMP_ROOT_RAW}" && pwd -P)"
HOME_DIR="${TEMP_ROOT}/home"
WORK_DIR="${TEMP_ROOT}/work"
BASE_PATH="/usr/bin:/bin"

mkdir -p "${HOME_DIR}" "${WORK_DIR}"
tar -xzf "${ARCHIVE_PATH}" -C "${TEMP_ROOT}"
trap 'rm -rf "${TEMP_ROOT_RAW}"' EXIT

BUNDLE_DIR="$(find "${TEMP_ROOT}" -mindepth 1 -maxdepth 1 -type d -name 'arden-*-portable' | head -n 1)"
if [[ -z "${BUNDLE_DIR}" ]]; then
  echo "portable bundle directory not found after extraction" >&2
  exit 1
fi
BUNDLE_DIR="$(cd "${BUNDLE_DIR}" && pwd -P)"

chmod +x "${BUNDLE_DIR}/arden"
if [[ -f "${BUNDLE_DIR}/install.sh" ]]; then
  chmod +x "${BUNDLE_DIR}/install.sh"
else
  echo "install script not found in bundle: ${BUNDLE_DIR}/install.sh" >&2
  exit 1
fi

echo "[smoke] verifying bundled launcher"
run_with_timeout "${SMOKE_STEP_TIMEOUT_SECONDS}" env -i \
  HOME="${HOME_DIR}" \
  PATH="${BASE_PATH}" \
  LLVM_SYS_221_PREFIX= \
  LLVM_SYS_211_PREFIX= \
  LLVM_CONFIG_PATH= \
  LIBRARY_PATH= \
  LD_LIBRARY_PATH= \
  DYLD_LIBRARY_PATH= \
  "${BUNDLE_DIR}/arden" --version

cat > "${WORK_DIR}/hello.arden" <<'EOF'
import std.io.*;

function main(): None {
    println("Hello from portable Arden!");
    return None;
}
EOF

echo "[smoke] running portable hello-world"
RUN_OUTPUT="$(run_with_timeout "${SMOKE_STEP_TIMEOUT_SECONDS}" env -i \
  HOME="${HOME_DIR}" \
  PATH="${BASE_PATH}" \
  LLVM_SYS_221_PREFIX= \
  LLVM_SYS_211_PREFIX= \
  LLVM_CONFIG_PATH= \
  LIBRARY_PATH= \
  LD_LIBRARY_PATH= \
  DYLD_LIBRARY_PATH= \
  "${BUNDLE_DIR}/arden" run "${WORK_DIR}/hello.arden")"
printf '%s\n' "${RUN_OUTPUT}"
grep -F "Hello from portable Arden!" <<< "${RUN_OUTPUT}" >/dev/null

echo "[smoke] installing launcher into isolated HOME"
run_with_timeout "${SMOKE_STEP_TIMEOUT_SECONDS}" env -i \
  HOME="${HOME_DIR}" \
  PATH="${BASE_PATH}" \
  LLVM_SYS_221_PREFIX= \
  LLVM_SYS_211_PREFIX= \
  LLVM_CONFIG_PATH= \
  LIBRARY_PATH= \
  LD_LIBRARY_PATH= \
  DYLD_LIBRARY_PATH= \
  "${BUNDLE_DIR}/install.sh"

echo "[smoke] validating installed launcher"
run_with_timeout "${SMOKE_STEP_TIMEOUT_SECONDS}" env -i \
  HOME="${HOME_DIR}" \
  PATH="${HOME_DIR}/.local/bin:${BASE_PATH}" \
  LLVM_SYS_221_PREFIX= \
  LLVM_SYS_211_PREFIX= \
  LLVM_CONFIG_PATH= \
  LIBRARY_PATH= \
  LD_LIBRARY_PATH= \
  DYLD_LIBRARY_PATH= \
  arden --version
