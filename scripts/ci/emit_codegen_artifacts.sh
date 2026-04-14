#!/usr/bin/env bash
set -euo pipefail

COMPILER_PATH="${1:-${ARDEN_COMPILER_PATH:-}}"
SOURCE_PATH="${2:-${ARDEN_FAILURE_SOURCE:-}}"
CONTEXT="${3:-${ARDEN_FAILURE_CONTEXT:-ci-failure}}"
OUTPUT_ROOT="${4:-${ARDEN_FAILURE_OUTPUT_ROOT:-}}"

resolve_compiler() {
  local hint="$1"
  if [[ -n "${hint}" && -x "${hint}" ]]; then
    printf '%s\n' "${hint}"
    return 0
  fi
  if [[ -x "target/release/arden" ]]; then
    printf '%s\n' "$(pwd)/target/release/arden"
    return 0
  fi
  if [[ -x "target/debug/arden" ]]; then
    printf '%s\n' "$(pwd)/target/debug/arden"
    return 0
  fi
  return 1
}

resolve_abs_path() {
  local p="$1"
  local d b
  d="$(cd "$(dirname "${p}")" && pwd -P)"
  b="$(basename "${p}")"
  printf '%s/%s\n' "${d}" "${b}"
}

resolve_llc() {
  if command -v llc >/dev/null 2>&1; then
    command -v llc
    return 0
  fi
  if [[ -n "${LLVM_SYS_221_PREFIX:-}" && -x "${LLVM_SYS_221_PREFIX}/bin/llc" ]]; then
    printf '%s\n' "${LLVM_SYS_221_PREFIX}/bin/llc"
    return 0
  fi
  return 1
}

sanitize_stem() {
  printf '%s' "$1" | sed -E 's/[^A-Za-z0-9._-]/_/g'
}

if [[ -z "${OUTPUT_ROOT}" ]]; then
  if [[ -n "${GITHUB_WORKSPACE:-}" ]]; then
    OUTPUT_ROOT="${GITHUB_WORKSPACE}/artifacts/ci-failure"
  else
    OUTPUT_ROOT="$(pwd)/artifacts/ci-failure"
  fi
fi

if ! COMPILER="$(resolve_compiler "${COMPILER_PATH}")"; then
  echo "::warning::No arden compiler binary found (checked hint + target/release + target/debug). Skipping LLVM dump."
  exit 0
fi

LLC_BIN=""
if LLC_BIN="$(resolve_llc)"; then
  :
else
  echo "::warning::llc was not found on PATH or under LLVM_SYS_221_PREFIX; object emission will be skipped."
fi

mkdir -p "${OUTPUT_ROOT}/${CONTEXT}"

sources=()
if [[ -n "${SOURCE_PATH}" ]]; then
  sources+=("${SOURCE_PATH}")
elif [[ -n "${ARDEN_FAILURE_SOURCES:-}" ]]; then
  while IFS=';' read -r src; do
    [[ -n "${src}" ]] && sources+=("${src}")
  done <<<"${ARDEN_FAILURE_SOURCES}"
else
  sources+=(
    "examples/single_file/stdlib_and_system/18_file_io/18_file_io.arden"
    "examples/demos/demo_notes/demo_notes.arden"
  )
fi

attempted=0
for src in "${sources[@]}"; do
  [[ -z "${src}" ]] && continue
  resolved="${src}"
  if [[ ! "${resolved}" = /* ]]; then
    resolved="$(pwd)/${resolved}"
  fi
  if [[ ! -f "${resolved}" ]]; then
    echo "::warning::Skipping missing source: ${src}"
    continue
  fi
  resolved="$(resolve_abs_path "${resolved}")"
  attempted=1

  label="${resolved}"
  if [[ -n "${GITHUB_WORKSPACE:-}" ]]; then
    ws="$(resolve_abs_path "${GITHUB_WORKSPACE}")"
    if [[ "${label}" == "${ws}"* ]]; then
      label="${label#${ws}/}"
    fi
  fi
  stem="$(sanitize_stem "$(printf '%s' "${label}" | sed 's#[/\\]#__#g')")"
  ll_path="${OUTPUT_ROOT}/${CONTEXT}/${stem}.ll"
  obj_path="${OUTPUT_ROOT}/${CONTEXT}/${stem}.obj"
  log_path="${OUTPUT_ROOT}/${CONTEXT}/${stem}.log.txt"
  meta_path="${OUTPUT_ROOT}/${CONTEXT}/${stem}.meta.txt"

  echo
  echo "[ci-dump] source: ${resolved}"
  echo "[ci-dump] llvm:   ${ll_path}"
  echo "[ci-dump] object: ${obj_path}"

  {
    echo "context=${CONTEXT}"
    echo "source=${resolved}"
    echo "compiler=${COMPILER}"
    echo "llc=${LLC_BIN}"
    date -u +"timestamp_utc=%Y-%m-%dT%H:%M:%SZ"
  } >"${meta_path}"

  if ! "${COMPILER}" compile "${resolved}" --emit-llvm "${ll_path}" 2>&1 | tee -a "${log_path}"; then
    echo "::warning::Failed to emit LLVM for ${resolved}"
    continue
  fi
  echo "[ci-dump] generated .ll => ${ll_path}"

  if [[ -n "${LLC_BIN}" ]]; then
    if "${LLC_BIN}" "${ll_path}" -filetype=obj -o "${obj_path}" 2>&1 | tee -a "${log_path}"; then
      echo "[ci-dump] generated .obj => ${obj_path}"
    else
      echo "::warning::llc failed for ${resolved}"
    fi
  fi
done

if [[ "${attempted}" -eq 0 ]]; then
  echo "::warning::No valid source files were available for LLVM/object dump."
fi

echo "[ci-dump] output root: ${OUTPUT_ROOT}/${CONTEXT}"
exit 0
