#!/usr/bin/env bash
set -euo pipefail

sdk_root="${SDKROOT:-}"
if [[ -z "$sdk_root" ]]; then
  sdk_root="$(xcrun --sdk macosx --show-sdk-path)"
fi
sdk_version="${MACOSX_DEPLOYMENT_TARGET:-}"
if [[ -z "$sdk_version" ]]; then
  sdk_version="$(xcrun --sdk macosx --show-sdk-version)"
fi

linker_bin="$(command -v ld64.lld || command -v ld.lld || command -v lld)"
if [[ -z "$linker_bin" ]]; then
  printf '%s\n' "error: ld64.lld not found in PATH" >&2
  exit 1
fi

append_driver_payload() {
  local payload="$1"
  local old_ifs="$IFS"
  local part
  IFS=','
  read -r -a parts <<< "$payload"
  IFS="$old_ifs"
  for part in "${parts[@]}"; do
    [[ -n "$part" ]] && forwarded_args+=("$part")
  done
}

forwarded_args=(
  "-arch" "x86_64"
  "-platform_version" "macos" "$sdk_version" "$sdk_version"
  "-syslibroot" "$sdk_root"
  "-dead_strip"
  "-demangle"
  "-adhoc_codesign"
)
skip_next=0
for arg in "$@"; do
  if [[ "$skip_next" -eq 1 ]]; then
    skip_next=0
    continue
  fi
  case "$arg" in
    -fuse-ld=*|-mmacosx-version-min=*)
      ;;
    -arch)
      skip_next=1
      ;;
    -Wl,*)
      append_driver_payload "${arg#-Wl,}"
      ;;
    *)
      forwarded_args+=("$arg")
      ;;
  esac
done
forwarded_args+=("-lSystem")

exec "$linker_bin" "${forwarded_args[@]}"
