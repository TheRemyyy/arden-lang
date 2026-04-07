#!/usr/bin/env bash
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}" || exit 1

echo "========================================"
echo "   Arden Example Smoke Runner (Linux)"
echo "========================================"
echo
COMPILER_INPUT="${ARDEN_COMPILER_PATH:-${REPO_ROOT}/target/release/arden}"
if [[ "${COMPILER_INPUT}" = /* ]]; then
  COMPILER="${COMPILER_INPUT}"
else
  COMPILER="${REPO_ROOT}/${COMPILER_INPUT}"
fi

echo "[1/5] Preparing compiler..."

if [[ "${CI_SKIP_COMPILER_BUILD:-0}" != "1" ]]; then
  if ! cargo build --release; then
    echo "Build failed!"
    exit 1
  fi
fi

if [[ ! -x "${COMPILER}" ]]; then
  echo "Compiler binary not found or not executable at ${COMPILER}"
  exit 1
fi

PASS_COUNT=0
FAIL_COUNT=0

echo
echo "[2/5] Running single-file examples..."
echo

shopt -s nullglob
for file in "${REPO_ROOT}"/examples/*.arden; do
  echo "----------------------------------------"
  echo "Testing ${file}..."
  if "${COMPILER}" run "${file}"; then
    echo "[PASS] $(basename "${file}")"
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    echo "[FAIL] $(basename "${file}")"
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi
done

run_project_test() {
  local project_name="$1"
  local title="$2"
  local project_dir="${REPO_ROOT}/examples/${project_name}"

  echo
  echo "${title}"
  echo

  if [[ -f "${project_dir}/arden.toml" ]]; then
    if (cd "${project_dir}" && "${COMPILER}" run); then
      echo "[PASS] ${project_name}"
      PASS_COUNT=$((PASS_COUNT + 1))
    else
      echo "[FAIL] ${project_name}"
      FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
  else
    echo "${project_name} not found, skipping..."
  fi
}

run_project_test "starter_project" "[3/5] Testing starter project..."
run_project_test "nested_package_project" "[4/5] Testing nested package project..."
run_project_test "minimal_project" "[5/5] Testing minimal project..."

echo
echo "========================================"
echo "Test Summary"
echo "========================================"
echo "Passed: ${PASS_COUNT}"
echo "Failed: ${FAIL_COUNT}"

if [[ ${FAIL_COUNT} -eq 0 ]]; then
  echo "ALL TESTS PASSED"
  exit 0
else
  echo "SOME TESTS FAILED"
  exit 1
fi
