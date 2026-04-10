#!/usr/bin/env bash
set -euo pipefail

find_first_existing() {
  local candidate
  for candidate in "$@"; do
    if [[ -e "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

collect_latest_gcc_dir() {
  local root
  local match
  for root in /usr/lib/gcc /usr/lib64/gcc; do
    [[ -d "$root" ]] || continue
    match="$(find "$root" -mindepth 2 -maxdepth 2 -type d 2>/dev/null | sort -Vr | head -n 1 || true)"
    if [[ -n "$match" ]]; then
      printf '%s\n' "$match"
      return 0
    fi
  done
  return 1
}

append_driver_payload() {
  local payload="$1"
  local old_ifs="$IFS"
  local part
  IFS=','
  read -r -a parts <<< "$payload"
  IFS="$old_ifs"
  for part in "${parts[@]}"; do
    case "$part" in
      "") ;;
      -z) ;;
      noexecstack|relro|now) forwarded_args+=("-z" "$part") ;;
      *) forwarded_args+=("$part") ;;
    esac
  done
}

gcc_dir="$(collect_latest_gcc_dir || true)"
dynamic_linker="$(find_first_existing /lib64/ld-linux-x86-64.so.2 /lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 || true)"
crt_entry="$(find_first_existing /usr/lib/x86_64-linux-gnu/Scrt1.o /usr/lib64/Scrt1.o /usr/lib/x86_64-linux-gnu/crt1.o /usr/lib64/crt1.o || true)"
crti_path="$(find_first_existing /usr/lib/x86_64-linux-gnu/crti.o /usr/lib64/crti.o || true)"
crtn_path="$(find_first_existing /usr/lib/x86_64-linux-gnu/crtn.o /usr/lib64/crtn.o || true)"
crtbegin_path="$(find_first_existing "${gcc_dir}/crtbeginS.o" "${gcc_dir}/crtbegin.o" || true)"
crtend_path="$(find_first_existing "${gcc_dir}/crtendS.o" "${gcc_dir}/crtend.o" || true)"

forwarded_args=()
linking_binary=1
for arg in "$@"; do
  case "$arg" in
    -m64|-m32) ;;
    -nodefaultlibs) ;;
    -B*) ;;
    -fuse-ld=*) ;;
    -shared)
      linking_binary=0
      forwarded_args+=("--shared")
      ;;
    -Wl,*)
      append_driver_payload "${arg#-Wl,}"
      ;;
    *)
      forwarded_args+=("$arg")
      ;;
  esac
done

for lib_dir in /usr/lib64 /lib64 /usr/lib /lib /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu; do
  if [[ -d "$lib_dir" ]]; then
    forwarded_args=("-L" "$lib_dir" "${forwarded_args[@]}")
  fi
done
if [[ -n "$gcc_dir" ]]; then
  forwarded_args=("-L" "$gcc_dir" "${forwarded_args[@]}")
fi

prefix_args=("--thread-count=$(nproc)" "--build-id" "--as-needed")
suffix_args=()

if [[ "$linking_binary" -eq 1 ]]; then
  [[ -n "$dynamic_linker" ]] && prefix_args+=("--dynamic-linker" "$dynamic_linker")
  [[ -n "$crt_entry" ]] && prefix_args+=("$crt_entry")
  [[ -n "$crti_path" ]] && prefix_args+=("$crti_path")
  [[ -n "$crtbegin_path" ]] && prefix_args+=("$crtbegin_path")
  [[ -n "$crtend_path" ]] && suffix_args+=("$crtend_path")
  [[ -n "$crtn_path" ]] && suffix_args+=("$crtn_path")
fi

exec mold "${prefix_args[@]}" "${forwarded_args[@]}" "${suffix_args[@]}"
