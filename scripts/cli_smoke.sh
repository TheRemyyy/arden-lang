#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
COMPILER_INPUT="${ARDEN_COMPILER_PATH:-${REPO_ROOT}/target/release/arden}"
if [[ "${COMPILER_INPUT}" = /* ]]; then
  COMPILER="${COMPILER_INPUT}"
else
  COMPILER="${REPO_ROOT}/${COMPILER_INPUT}"
fi

if [[ "${CI_SKIP_COMPILER_BUILD:-0}" != "1" ]]; then
  cargo build --release >/dev/null
fi

if [[ ! -x "${COMPILER}" ]]; then
  echo "Compiler binary not found or not executable: ${COMPILER}" >&2
  exit 127
fi

TMP_DIR_RAW="$(mktemp -d)"
TMP_DIR="$(cd "${TMP_DIR_RAW}" && pwd -P)"
trap 'rm -rf "${TMP_DIR_RAW}"' EXIT

platform_name() {
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
      echo "windows"
      ;;
    Darwin)
      echo "macos"
      ;;
    *)
      echo "linux"
      ;;
  esac
}

assert_output_artifact_exists() {
  local base_path="$1"
  local kind="$2"
  local platform
  platform="$(platform_name)"
  local candidates=()

  case "${kind}" in
    bin)
      candidates+=("${base_path}")
      [[ "${platform}" == "windows" ]] && candidates+=("${base_path}.exe")
      ;;
    shared)
      candidates+=("${base_path}")
      case "${platform}" in
        windows)
          candidates+=("${base_path}.dll" "${base_path}.lib")
          ;;
        macos)
          candidates+=("${base_path}.dylib")
          ;;
        *)
          candidates+=("${base_path}.so")
          ;;
      esac
      ;;
    static)
      candidates+=("${base_path}" "${base_path}.a" "${base_path}.lib")
      ;;
    *)
      echo "unknown artifact kind: ${kind}" >&2
      exit 1
      ;;
  esac

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -f "${candidate}" ]]; then
      return 0
    fi
  done

  echo "expected ${kind} artifact for ${platform}, but none of these files exist:" >&2
  for candidate in "${candidates[@]}"; do
    echo "  - ${candidate}" >&2
  done
  exit 1
}

PROJECT_DIR="${TMP_DIR}/sample_project"
UGLY_FILE="${TMP_DIR}/ugly.arden"
LINT_FILE="${TMP_DIR}/linty.arden"
TEST_FILE="${TMP_DIR}/sample_test.arden"
IGNORE_ESC_FILE="${TMP_DIR}/ignore_escape_test.arden"
ESCAPE_FILE="${TMP_DIR}/escapes.arden"
ESCAPE_OUT="${TMP_DIR}/escapes_bin"
ESCAPE_STDOUT="${TMP_DIR}/escapes.stdout"
EXAMPLE_STDOUT="${TMP_DIR}/example.stdout"
RANGE_FLOAT_FILE="${TMP_DIR}/range_float.arden"
RANGE_ZERO_RUNTIME_FILE="${TMP_DIR}/range_zero_runtime.arden"
HEADER_FILE="${TMP_DIR}/sample.h"
BINDINGS_FILE="${TMP_DIR}/bindings.arden"
OUT_FILE="${TMP_DIR}/ugly_bin"
NESTED_FIELD_FILE="${TMP_DIR}/nested_field_assign.arden"
NESTED_FIELD_OUT="${TMP_DIR}/nested_field_assign_bin"
SHARED_PROJECT="${TMP_DIR}/shared_project"
STATIC_PROJECT="${TMP_DIR}/static_project"
BORROW_ERR_OUT="${TMP_DIR}/borrow_err.out"
BORROW_USE_AFTER_MOVE_FILE="${TMP_DIR}/borrow_use_after_move.arden"
BORROW_MOVE_WHILE_BORROWED_FILE="${TMP_DIR}/borrow_move_while_borrowed.arden"
BORROW_DOUBLE_MUT_FILE="${TMP_DIR}/borrow_double_mut.arden"
BORROW_SCOPE_RELEASE_FILE="${TMP_DIR}/borrow_scope_release.arden"
BORROW_REBORROW_AFTER_SCOPE_FILE="${TMP_DIR}/borrow_reborrow_after_scope.arden"
BORROW_LAMBDA_MOVE_FILE="${TMP_DIR}/borrow_lambda_move.arden"
BORROW_COMPOUND_BORROWED_FILE="${TMP_DIR}/borrow_compound_borrowed.arden"
PROJECT_TYPECHECK_DIR="${TMP_DIR}/project_typecheck"
PROJECT_STDLIB_ALIAS_DIR="${TMP_DIR}/project_stdlib_alias"
PROJECT_TABLE_CFG_DIR="${TMP_DIR}/project_table_cfg"

"${COMPILER}" new sample_project --path "${PROJECT_DIR}" >/dev/null

pushd "${PROJECT_DIR}" >/dev/null
"${COMPILER}" info >/dev/null
"${COMPILER}" check >/dev/null
"${COMPILER}" run >/dev/null
popd >/dev/null

cat <<'EOF_FILE' > "${UGLY_FILE}"
// lead comment
import std.io.*;
function main(): None {println("ok");return None;}
EOF_FILE

cat <<'EOF_LINT' > "${LINT_FILE}"
import std.string.*;
import std.io.*;
import std.io.*;

function main(): None {
    println("ok");
    return None;
}
EOF_LINT

if "${COMPILER}" fmt --check "${UGLY_FILE}" >/dev/null 2>&1; then
  echo "fmt --check unexpectedly passed on unformatted source" >&2
  exit 1
fi

"${COMPILER}" lint "${LINT_FILE}" >/dev/null 2>&1 && {
  echo "lint unexpectedly passed on linty source" >&2
  exit 1
}
"${COMPILER}" fix "${LINT_FILE}" >/dev/null
"${COMPILER}" lint "${LINT_FILE}" >/dev/null
"${COMPILER}" fmt "${UGLY_FILE}" >/dev/null
"${COMPILER}" fmt --check "${UGLY_FILE}" >/dev/null
"${COMPILER}" lex "${UGLY_FILE}" >/dev/null
"${COMPILER}" parse "${UGLY_FILE}" >/dev/null
"${COMPILER}" compile "${UGLY_FILE}" -o "${OUT_FILE}" >/dev/null
"${OUT_FILE}" >/dev/null
"${COMPILER}" run "${UGLY_FILE}" >/dev/null
"${COMPILER}" bench "${UGLY_FILE}" --iterations 2 >/dev/null
"${COMPILER}" profile "${UGLY_FILE}" >/dev/null

cat <<'EOF_NESTED_FIELD' > "${NESTED_FIELD_FILE}"
class B {
    mut v: Integer;
    constructor(v: Integer) { this.v = v; }
}
class A {
    mut b: B;
    constructor(v: Integer) { this.b = B(v); }
}
function main(): None {
    mut a: A = A(7);
    a.b.v += 1;
    return None;
}
EOF_NESTED_FIELD
"${COMPILER}" compile "${NESTED_FIELD_FILE}" -o "${NESTED_FIELD_OUT}" --no-check >/dev/null

cat <<'EOF_TEST' > "${TEST_FILE}"
@Test
function sampleTest(): None {
    assert_eq(1, 1);
    return None;
}
EOF_TEST

"${COMPILER}" test --list --path "${TEST_FILE}" >/dev/null
"${COMPILER}" test --path "${TEST_FILE}" >/dev/null
"${COMPILER}" test --path "${REPO_ROOT}/examples/single_file/tooling_and_ffi/24_test_attributes/24_test_attributes.arden" >"${BORROW_ERR_OUT}" 2>&1
grep -q "8 passed;" "${BORROW_ERR_OUT}"
grep -q "2 ignored;" "${BORROW_ERR_OUT}"

cat <<'EOF_IGNORE_ESC' > "${IGNORE_ESC_FILE}"
@Test
@Ignore("c:\\tmp\\foo\nline2")
function skipped(): None {
    fail("should not run");
    return None;
}
EOF_IGNORE_ESC
"${COMPILER}" test --path "${IGNORE_ESC_FILE}" >"${BORROW_ERR_OUT}" 2>&1
grep -Fq 'c:\tmp\foo\nline2' "${BORROW_ERR_OUT}"
"${COMPILER}" test --list --path "${IGNORE_ESC_FILE}" >"${BORROW_ERR_OUT}" 2>&1
grep -Fq '(ignored: c:\\tmp\\foo\nline2)' "${BORROW_ERR_OUT}"

cat <<'EOF_ESCAPE' > "${ESCAPE_FILE}"
import std.io.*;
function main(): None {
    println("A\nB");
    println("X\tY");
    println("Q:\" Z");
    return None;
}
EOF_ESCAPE
"${COMPILER}" compile "${ESCAPE_FILE}" -o "${ESCAPE_OUT}" >/dev/null
"${ESCAPE_OUT}" > "${ESCAPE_STDOUT}"
python3 - <<'PY' "${ESCAPE_STDOUT}"
from pathlib import Path
import sys

actual = Path(sys.argv[1]).read_text().replace("\r\n", "\n")
expected = 'A\nB\nX\tY\nQ:" Z\n'
if actual != expected:
    raise SystemExit(
        f"escape stdout mismatch\nexpected={expected!r}\nactual={actual!r}"
    )
PY

"${COMPILER}" run "${REPO_ROOT}/examples/single_file/language_edges/35_visibility_enforcement/35_visibility_enforcement.arden" >"${EXAMPLE_STDOUT}"
grep -Fq 'Account: Standard, balance=150' "${EXAMPLE_STDOUT}"
grep -Fq 'Premium owner code=77' "${EXAMPLE_STDOUT}"
"${COMPILER}" run "${REPO_ROOT}/examples/single_file/language_edges/36_inheritance_extends/36_inheritance_extends.arden" >"${EXAMPLE_STDOUT}"
grep -Fq 'Animal(Buddy)' "${EXAMPLE_STDOUT}"
grep -Fq 'sound=woof' "${EXAMPLE_STDOUT}"
"${COMPILER}" run "${REPO_ROOT}/examples/single_file/language_edges/37_interfaces_contracts/37_interfaces_contracts.arden" >"${EXAMPLE_STDOUT}"
grep -Fq 'name=Arden Language' "${EXAMPLE_STDOUT}"
grep -Fq 'Book: Arden Language' "${EXAMPLE_STDOUT}"
"${COMPILER}" run "${REPO_ROOT}/examples/single_file/language_edges/38_import_aliases/38_import_aliases.arden" >"${EXAMPLE_STDOUT}"
grep -Fq 'abs=42, upper=ARDEN' "${EXAMPLE_STDOUT}"

cat <<'EOF_RANGE_FLOAT' > "${RANGE_FLOAT_FILE}"
import std.io.*;

function main(): None {
    r: Range<Float> = range(0.0, 3.0, 1.0);
    while (r.has_next()) {
        println(to_string(r.next()));
    }
    return None;
}
EOF_RANGE_FLOAT
"${COMPILER}" run "${RANGE_FLOAT_FILE}" > "${EXAMPLE_STDOUT}"
grep -Fq '0.000000' "${EXAMPLE_STDOUT}"
grep -Fq '1.000000' "${EXAMPLE_STDOUT}"
grep -Fq '2.000000' "${EXAMPLE_STDOUT}"

cat <<'EOF_RANGE_ZERO' > "${RANGE_ZERO_RUNTIME_FILE}"
function choose_zero(): Integer {
    return 0;
}

function main(): None {
    r: Range<Integer> = range(0, 3, choose_zero());
    return None;
}
EOF_RANGE_ZERO
if "${COMPILER}" run "${RANGE_ZERO_RUNTIME_FILE}" >"${BORROW_ERR_OUT}" 2>&1; then
  echo "range() unexpectedly accepted runtime zero step" >&2
  exit 1
fi
grep -q "Runtime error: range() step cannot be 0" "${BORROW_ERR_OUT}"

cat <<'EOF_HEADER' > "${HEADER_FILE}"
int add(int a, int b);
int puts(const char* msg);
EOF_HEADER

"${COMPILER}" bindgen "${HEADER_FILE}" --output "${BINDINGS_FILE}" >/dev/null
grep -q 'extern(c) function add(a: Integer, b: Integer): Integer;' "${BINDINGS_FILE}"
grep -q 'extern(c) function puts(msg: String): Integer;' "${BINDINGS_FILE}"

cat <<'EOF_BORROW_UAM' > "${BORROW_USE_AFTER_MOVE_FILE}"
import std.io.*;
function consume(owned s: String): None { return None; }
function main(): None {
    s: String = "x";
    consume(s);
    println(s);
    return None;
}
EOF_BORROW_UAM
if "${COMPILER}" check "${BORROW_USE_AFTER_MOVE_FILE}" >"${BORROW_ERR_OUT}" 2>&1; then
  echo "borrow check unexpectedly passed for use-after-move" >&2
  exit 1
fi
grep -q "Use of moved value 's'" "${BORROW_ERR_OUT}"

cat <<'EOF_BORROW_MWB' > "${BORROW_MOVE_WHILE_BORROWED_FILE}"
function consume(owned s: String): None { return None; }
function main(): None {
    s: String = "x";
    r: &String = &s;
    consume(s);
    return None;
}
EOF_BORROW_MWB
if "${COMPILER}" check "${BORROW_MOVE_WHILE_BORROWED_FILE}" >"${BORROW_ERR_OUT}" 2>&1; then
  echo "borrow check unexpectedly passed for move-while-borrowed" >&2
  exit 1
fi
grep -q "Cannot move 's' while borrowed" "${BORROW_ERR_OUT}"

cat <<'EOF_BORROW_DM' > "${BORROW_DOUBLE_MUT_FILE}"
function main(): None {
    mut x: Integer = 1;
    a: &mut Integer = &mut x;
    b: &mut Integer = &mut x;
    return None;
}
EOF_BORROW_DM
if "${COMPILER}" check "${BORROW_DOUBLE_MUT_FILE}" >"${BORROW_ERR_OUT}" 2>&1; then
  echo "borrow check unexpectedly passed for double mutable borrow" >&2
  exit 1
fi
grep -q "Cannot borrow 'x' while mutably borrowed" "${BORROW_ERR_OUT}"

cat <<'EOF_BORROW_SCOPE' > "${BORROW_SCOPE_RELEASE_FILE}"
function consume(owned s: String): None { return None; }
function main(): None {
    s: String = "x";
    if (true) {
        r: &String = &s;
    }
    consume(s);
    return None;
}
EOF_BORROW_SCOPE
"${COMPILER}" check "${BORROW_SCOPE_RELEASE_FILE}" >/dev/null

cat <<'EOF_BORROW_REBORROW' > "${BORROW_REBORROW_AFTER_SCOPE_FILE}"
function take_borrow(borrow s: String): None { return None; }
function main(): None {
    s: String = "x";
    if (true) {
        r: &String = &s;
        take_borrow(s);
    }
    take_borrow(s);
    return None;
}
EOF_BORROW_REBORROW
"${COMPILER}" check "${BORROW_REBORROW_AFTER_SCOPE_FILE}" >/dev/null

cat <<'EOF_BORROW_LAMBDA' > "${BORROW_LAMBDA_MOVE_FILE}"
import std.io.*;
function consume(owned s: String): None { return None; }
function main(): None {
    s: String = "x";
    f: () -> None = () => consume(s);
    println(s);
    return None;
}
EOF_BORROW_LAMBDA
if "${COMPILER}" check "${BORROW_LAMBDA_MOVE_FILE}" >"${BORROW_ERR_OUT}" 2>&1; then
  echo "borrow check unexpectedly passed for lambda capture move" >&2
  exit 1
fi
grep -q "Use of moved value 's'" "${BORROW_ERR_OUT}"

cat <<'EOF_BORROW_CA' > "${BORROW_COMPOUND_BORROWED_FILE}"
function main(): None {
    mut x: Integer = 10;
    r: &mut Integer = &mut x;
    x += 1;
    return None;
}
EOF_BORROW_CA
if "${COMPILER}" check "${BORROW_COMPOUND_BORROWED_FILE}" >"${BORROW_ERR_OUT}" 2>&1; then
  echo "borrow check unexpectedly passed for compound-assign on borrowed variable" >&2
  exit 1
fi
grep -q "Cannot assign to 'x' while mutably borrowed" "${BORROW_ERR_OUT}"

mkdir -p "${PROJECT_TYPECHECK_DIR}/src"
cat <<'EOF_PROJECT_CFG' > "${PROJECT_TYPECHECK_DIR}/arden.toml"
name = "project_typecheck"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden", "src/util.arden"]
output = "project_typecheck"
opt_level = "0"
EOF_PROJECT_CFG
cat <<'EOF_PROJECT_MAIN' > "${PROJECT_TYPECHECK_DIR}/src/main.arden"
import std.io.*;
import util.*;
function main(): None {
    println(to_string(helper()));
    return None;
}
EOF_PROJECT_MAIN
cat <<'EOF_PROJECT_UTIL' > "${PROJECT_TYPECHECK_DIR}/src/util.arden"
package util;
function helper(): Integer {
    return "bad";
}
EOF_PROJECT_UTIL
if (cd "${PROJECT_TYPECHECK_DIR}" && "${COMPILER}" check >"${BORROW_ERR_OUT}" 2>&1); then
  echo "project check unexpectedly passed despite cross-file type error" >&2
  exit 1
fi
grep -q "mismatch" "${BORROW_ERR_OUT}"

mkdir -p "${PROJECT_STDLIB_ALIAS_DIR}/src"
cat <<'EOF_ALIAS_CFG' > "${PROJECT_STDLIB_ALIAS_DIR}/arden.toml"
name = "project_stdlib_alias"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "project_stdlib_alias"
opt_level = "0"
EOF_ALIAS_CFG
cat <<'EOF_ALIAS_MAIN' > "${PROJECT_STDLIB_ALIAS_DIR}/src/main.arden"
import std.io as io;
import std.math as math;
function main(): None {
    io.println(to_string(math.abs(-1)));
    return None;
}
EOF_ALIAS_MAIN
(cd "${PROJECT_STDLIB_ALIAS_DIR}" && "${COMPILER}" check >/dev/null)

mkdir -p "${PROJECT_TABLE_CFG_DIR}/src"
cat <<'EOF_PROJECT_TABLE_CFG' > "${PROJECT_TABLE_CFG_DIR}/arden.toml"
[project]
name = "project_table_cfg"
version = "0.1.0"
entry = "src/main.arden"
files = ["src/main.arden"]
output = "project_table_cfg"
opt_level = "0"
EOF_PROJECT_TABLE_CFG
cat <<'EOF_PROJECT_TABLE_MAIN' > "${PROJECT_TABLE_CFG_DIR}/src/main.arden"
import std.io.*;
function main(): None {
    println("ok");
    return None;
}
EOF_PROJECT_TABLE_MAIN
(cd "${PROJECT_TABLE_CFG_DIR}" && "${COMPILER}" check >/dev/null)

python3 - <<'PY' "${COMPILER}" "${TMP_DIR}"
from pathlib import Path
import subprocess
import sys

compiler = sys.argv[1]
tmp_dir = Path(sys.argv[2])
single_root = tmp_dir / "single_regressions"
single_root.mkdir(parents=True, exist_ok=True)


def run_single(name: str, source: str, expect_ok: bool, required: list[str] | None = None) -> None:
    path = single_root / f"{name}.arden"
    path.write_text(source)
    proc = subprocess.run([compiler, "check", str(path)], capture_output=True, text=True)
    output = (proc.stdout or "") + (proc.stderr or "")
    ok = proc.returncode == 0
    if ok != expect_ok:
        raise SystemExit(
            f"[single:{name}] expected ok={expect_ok}, got rc={proc.returncode}\n{output}"
        )
    if required:
        for needle in required:
            if needle not in output:
                raise SystemExit(
                    f"[single:{name}] missing expected text: {needle!r}\n{output}"
                )


def run_single_error_count(name: str, source: str, needle: str, expected_count: int) -> None:
    path = single_root / f"{name}.arden"
    path.write_text(source)
    proc = subprocess.run([compiler, "check", str(path)], capture_output=True, text=True)
    output = (proc.stdout or "") + (proc.stderr or "")
    count = output.count(needle)
    if count != expected_count:
        raise SystemExit(
            f"[single:{name}] expected {expected_count} occurrences of {needle!r}, got {count}\n{output}"
        )


def run_compile_stdout(name: str, source: str, expected_stdout: str) -> None:
    path = single_root / f"{name}.arden"
    out_bin = single_root / f"{name}.bin"
    path.write_text(source)
    compile_proc = subprocess.run(
        [compiler, "compile", str(path), "-o", str(out_bin)],
        capture_output=True,
        text=True,
    )
    if compile_proc.returncode != 0:
        output = (compile_proc.stdout or "") + (compile_proc.stderr or "")
        raise SystemExit(f"[compile:{name}] expected compile success\n{output}")
    run_proc = subprocess.run([str(out_bin)], capture_output=True, text=True)
    got = (run_proc.stdout or "").strip()
    if got != expected_stdout:
        raise SystemExit(
            f"[runtime:{name}] expected stdout={expected_stdout!r}, got {got!r}"
        )


def run_fmt_preserves_prefix(name: str, source: str, prefix: str) -> None:
    path = single_root / f"{name}.arden"
    path.write_text(source)
    fmt = subprocess.run([compiler, "fmt", str(path)], capture_output=True, text=True)
    if fmt.returncode != 0:
        raise SystemExit(f"[fmt:{name}] fmt failed\n{fmt.stdout}{fmt.stderr}")
    formatted = path.read_text()
    if not formatted.startswith(prefix):
        raise SystemExit(
            f"[fmt:{name}] expected formatted output to start with {prefix!r}\n{formatted}"
        )
    run_single(name, formatted, True)


def run_lint(name: str, source: str, expect_ok: bool, forbidden: list[str] | None = None) -> None:
    path = single_root / f"{name}.arden"
    path.write_text(source)
    proc = subprocess.run([compiler, "lint", str(path)], capture_output=True, text=True)
    output = (proc.stdout or "") + (proc.stderr or "")
    ok = proc.returncode == 0
    if ok != expect_ok:
        raise SystemExit(
            f"[lint:{name}] expected ok={expect_ok}, got rc={proc.returncode}\n{output}"
        )
    if forbidden:
        for needle in forbidden:
            if needle in output:
                raise SystemExit(
                    f"[lint:{name}] unexpectedly contained text: {needle!r}\n{output}"
                )


def run_fix_then_check(name: str, source: str) -> None:
    path = single_root / f"{name}.arden"
    path.write_text(source)
    fix_proc = subprocess.run([compiler, "fix", str(path)], capture_output=True, text=True)
    if fix_proc.returncode != 0:
        output = (fix_proc.stdout or "") + (fix_proc.stderr or "")
        raise SystemExit(f"[fix:{name}] fix failed\n{output}")
    check_proc = subprocess.run([compiler, "check", str(path)], capture_output=True, text=True)
    if check_proc.returncode != 0:
        output = (check_proc.stdout or "") + (check_proc.stderr or "")
        raise SystemExit(f"[fix:{name}] check failed after fix\n{output}")


def run_fix_preserves_prefix(name: str, source: str, prefix: str) -> None:
    path = single_root / f"{name}.arden"
    path.write_text(source)
    fix_proc = subprocess.run([compiler, "fix", str(path)], capture_output=True, text=True)
    if fix_proc.returncode != 0:
        output = (fix_proc.stdout or "") + (fix_proc.stderr or "")
        raise SystemExit(f"[fix:{name}] fix failed\n{output}")
    fixed = path.read_text()
    if not fixed.startswith(prefix):
        raise SystemExit(
            f"[fix:{name}] expected fixed output to start with {prefix!r}\n{fixed}"
        )
    run_single(name, fixed, True)


def run_single_fmt_roundtrip(
    name: str, source: str, expect_ok: bool, required: list[str] | None = None
) -> None:
    path = single_root / f"{name}.arden"
    path.write_text(source)
    fmt = subprocess.run([compiler, "fmt", str(path)], capture_output=True, text=True)
    if fmt.returncode != 0:
        raise SystemExit(f"[fmt:{name}] fmt failed\n{fmt.stdout}{fmt.stderr}")
    run_single(name, path.read_text(), expect_ok, required)


def run_compile(name: str, source: str, expect_ok: bool) -> None:
    path = single_root / f"{name}.arden"
    out = single_root / f"{name}.bin"
    path.write_text(source)
    proc = subprocess.run(
        [compiler, "compile", str(path), "-o", str(out)],
        capture_output=True,
        text=True,
    )
    output = (proc.stdout or "") + (proc.stderr or "")
    ok = proc.returncode == 0
    if ok != expect_ok:
        raise SystemExit(
            f"[compile:{name}] expected ok={expect_ok}, got rc={proc.returncode}\n{output}"
        )


def run_project(name: str, files: dict[str, str], expect_ok: bool, required: list[str] | None = None) -> None:
    project_root = tmp_dir / f"project_{name}"
    src_root = project_root / "src"
    src_root.mkdir(parents=True, exist_ok=True)

    file_list = sorted(files.keys())
    toml_files = ", ".join([f"\"src/{f}\"" for f in file_list])
    (project_root / "arden.toml").write_text(
        "\n".join(
            [
                f"name = \"{name}\"",
                "version = \"0.1.0\"",
                "entry = \"src/main.arden\"",
                f"files = [{toml_files}]",
                f"output = \"{name}\"",
                "opt_level = \"0\"",
                "",
            ]
        )
    )
    for rel, content in files.items():
        (src_root / rel).write_text(content)

    proc = subprocess.run([compiler, "check"], cwd=project_root, capture_output=True, text=True)
    output = (proc.stdout or "") + (proc.stderr or "")
    ok = proc.returncode == 0
    if ok != expect_ok:
        raise SystemExit(
            f"[project:{name}] expected ok={expect_ok}, got rc={proc.returncode}\n{output}"
        )
    if required:
        for needle in required:
            if needle not in output:
                raise SystemExit(
                    f"[project:{name}] missing expected text: {needle!r}\n{output}"
                )


# Explicit regression bundle (historical bugfix paths from recent cycles)
run_single(
    "builtin_ctor_list_invalid",
    """
function main(): None {
    xs: List<Integer> = List<Integer>("bad", true, 5);
    return None;
}
""",
    False,
    ["expects 0 or 1 arguments"],
)
run_single(
    "builtin_ctor_map_set_invalid",
    """
function main(): None {
    m: Map<String, Integer> = Map<String, Integer>(1);
    s: Set<Integer> = Set<Integer>(1, 2);
    return None;
}
""",
    False,
    ["expects 0 arguments"],
)
run_single(
    "borrow_invalid_assign_keeps_state",
    """
function consume(owned s: String): None { return None; }
function main(): None {
    mut s: String = "a";
    r: &String = &s;
    s = "b";
    consume(s);
    return None;
}
""",
    False,
    ["Cannot assign to 's' while borrowed", "Cannot move 's' while borrowed"],
)
run_single(
    "stdlib_module_import_required",
    """
function main(): None {
    x: Float = Math.abs(-1.0);
    return None;
}
""",
    False,
    ["import std.math.*;"],
)
run_single(
    "alias_call_checks_nested_args",
    """
import std.io as io;
function main(): None {
    io.println(to_string(Math.abs(-3)));
    return None;
}
""",
    False,
    ["import std.math.*;"],
)
run_single(
    "constructor_visibility_modifier_rejected",
    """
class C {
    private constructor() { }
}
function main(): None { return None; }
""",
    False,
    ["Visibility modifiers are not supported on constructors"],
)
run_single(
    "wildcard_import_alias_rejected",
    """
import std.io.* as io;
function main(): None { return None; }
""",
    False,
    ["Cannot use alias with wildcard import"],
)
run_single(
    "private_class_construction_rejected",
    """
private class Secret {
    constructor() {}
}
function main(): None {
    s: Secret = Secret();
    return None;
}
""",
    False,
    ["Class 'Secret' is private"],
)
run_single(
    "private_class_signature_rejected",
    """
private class Secret { constructor() {} }
function take(s: Secret): None { return None; }
function main(): None { return None; }
""",
    False,
    ["Class 'Secret' is private"],
)
run_single(
    "private_base_extends_rejected",
    """
private class Base { constructor() {} }
class Child extends Base { constructor() {} }
function main(): None { return None; }
""",
    False,
    ["Class 'Base' is private"],
)
run_single(
    "private_class_interface_signature_rejected",
    """
private class Secret { constructor() {} }
interface I {
    function leak(s: Secret): None;
}
function main(): None { return None; }
""",
    False,
    ["Class 'Secret' is private"],
)
run_single(
    "list_index_codegen_no_panic",
    """
import std.io.*;
function main(): None {
    mut xs: List<Integer> = List<Integer>();
    xs.push(1);
    xs.push(2);
    xs.push(3);
    xs[0] += 1;
    xs.set(1, xs.get(0) + xs.get(2));
    println(to_string(xs[0]));
    return None;
}
""",
    True,
)
run_single(
    "local_shadow_std_print",
    """
function print(owned s: String): None { return None; }
function main(): None {
    s: String = "x";
    print(s);
    return None;
}
""",
    True,
)
run_single(
    "if_expr_branch_match_statement_with_semicolon",
    """
function main(): None {
    v: None = if (true) {
        match (1) {
            1 => { 1; }
            _ => { 2; }
        };
    } else {
        None;
    };
    return None;
}
""",
    True,
)
run_single(
    "uppercase_function_call_parses_as_call",
    """
function Foo(): Integer { return 7; }
function main(): None {
    x: Integer = Foo();
    return None;
}
""",
    True,
)
run_single(
    "forward_uppercase_function_call_parses_as_call",
    """
function main(): None {
    x: Integer = Foo();
    return None;
}
function Foo(): Integer { return 7; }
""",
    True,
)
run_single(
    "explicit_generic_function_call_parses",
    """
function id<T>(x: T): T { return x; }
function main(): None {
    x: Integer = id<Integer>(1);
    return None;
}
""",
    True,
)
run_single(
    "explicit_generic_non_generic_function_rejected",
    """
function f(x: Integer): Integer { return x; }
function main(): None {
    y: Integer = f<String>(1);
    return None;
}
""",
    False,
    ["is not generic"],
)
run_single(
    "explicit_generic_function_arity_mismatch_rejected",
    """
function id<T>(x: T): T { return x; }
function main(): None {
    y: Integer = id<Integer, String>(1);
    return None;
}
""",
    False,
    ["expects 1 type arguments"],
)
run_single(
    "explicit_generic_unknown_type_rejected",
    """
function id<T>(x: T): T { return x; }
function main(): None {
    y: Integer = id<Nope>(1);
    return None;
}
""",
    False,
    ["Unknown type: Nope"],
)
run_single(
    "explicit_generic_method_call_parses_and_typechecks",
    """
class C {
    function id<T>(x: T): T { return x; }
}
function main(): None {
    c: C = C();
    y: Integer = c.id<Integer>(1);
    return None;
}
""",
    True,
)
run_single(
    "explicit_generic_module_call_parses_and_typechecks",
    """
module M {
    function id<T>(x: T): T { return x; }
}
function main(): None {
    y: Integer = M.id<Integer>(1);
    return None;
}
""",
    True,
)
run_single(
    "method_call_with_expression_receiver_does_not_move_borrow_arg",
    """
import std.io.*;
class C {
    function use(borrow s: String): None { println(s); return None; }
}
function mk(): C { return C(); }
function main(): None {
    s: String = "x";
    mk().use(s);
    println(s);
    return None;
}
""",
    True,
)
run_single(
    "field_assignment_through_immutable_owner_rejected",
    """
class C {
    mut v: Integer;
    constructor() { this.v = 1; }
}
function main(): None {
    c: C = C();
    c.v = 2;
    return None;
}
""",
    False,
    ["Cannot assign to immutable variable 'c'"],
)
run_single(
    "index_assignment_through_immutable_owner_rejected",
    """
function main(): None {
    xs: List<Integer> = List<Integer>();
    xs.push(1);
    xs[0] = 2;
    return None;
}
""",
    False,
    ["Cannot assign to immutable variable 'xs'"],
)
run_single(
    "immutable_receiver_cannot_call_mutating_method",
    """
class C {
    mut v: Integer;
    constructor(v: Integer) { this.v = v; }
    function touch(): None { this.v += 1; return None; }
}
function main(): None {
    c: C = C(1);
    c.touch();
    return None;
}
""",
    False,
    ["Cannot mutably borrow immutable variable 'c'"],
)
run_compile_stdout(
    "match_expression_literal_runtime_selects_correct_arm",
    """
import std.io.*;
function main(): None {
    x: Integer = match (2) {
        1 => { 10; },
        2 => { 20; },
        _ => { 30; }
    };
    println(to_string(x));
    return None;
}
""",
    "20",
)
run_compile_stdout(
    "match_expression_boolean_runtime_selects_correct_arm",
    """
import std.io.*;
function main(): None {
    x: Integer = match (true) {
        true => { 7; },
        false => { 9; }
    };
    println(to_string(x));
    return None;
}
""",
    "7",
)
run_compile_stdout(
    "match_statement_string_runtime_selects_correct_arm",
    """
import std.io.*;
function main(): None {
    s: String = "b";
    match (s) {
        "a" => { println("A"); }
        "b" => { println("B"); }
        _ => { println("Z"); }
    }
    return None;
}
""",
    "B",
)
run_compile_stdout(
    "match_expression_string_runtime_selects_correct_arm",
    """
import std.io.*;
function main(): None {
    s: String = "b";
    x: String = match (s) {
        "a" => { "A"; },
        "b" => { "B"; },
        _ => { "Z"; }
    };
    println(x);
    return None;
}
""",
    "B",
)
run_single(
    "match_patterns_support_float_char_negative_literals",
    """
function main(): None {
    f: Float = 1.0;
    c: Char = 'a';
    i: Integer = -1;
    match (f) { 1.0 => { } _ => { } }
    match (c) { 'a' => { } _ => { } }
    match (i) { -1 => { } _ => { } }
    return None;
}
""",
    True,
)
run_compile(
    "match_statement_option_boolean_binding_codegen",
    """
import std.io.*;
function main(): None {
    o: Option<Boolean> = Option<Boolean>();
    match (o) {
        Some(v) => { if (v) { println("T"); } }
        None => { println("N"); }
    }
    return None;
}
""",
    True,
)
run_compile(
    "match_expression_option_boolean_binding_codegen",
    """
import std.io.*;
function main(): None {
    o: Option<Boolean> = Option<Boolean>();
    x: Boolean = match (o) {
        Some(v) => { v; },
        None => { false; }
    };
    println(to_string(x));
    return None;
}
""",
    True,
)
run_compile(
    "match_expression_result_boolean_binding_codegen",
    """
import std.io.*;
function main(): None {
    r: Result<Boolean, String> = Result<Boolean, String>();
    x: Boolean = match (r) {
        Ok(v) => { v; },
        Error(e) => { false; }
    };
    println(to_string(x));
    return None;
}
""",
    True,
)
run_single(
    "if_expression_condition_checks_missing_import",
    """
function main(): None {
    x: Integer = if (Math.abs(-1.0) > 0.0) { 1; } else { 2; };
    return None;
}
""",
    False,
    ["import std.math.*;"],
)
run_single(
    "async_block_checks_missing_import",
    """
function main(): None {
    t: Task<Integer> = async { return Math.abs(-1); };
    return None;
}
""",
    False,
    ["import std.math.*;"],
)
run_single(
    "if_expression_branch_checks_missing_import",
    """
function main(): None {
    x: Float = if (true) { Math.abs(-1.0); } else { 0.0; };
    return None;
}
""",
    False,
    ["import std.math.*;"],
)
run_single(
    "require_expression_checks_missing_import",
    """
function main(): None {
    require(Math.abs(-1.0) > 0.0, "x");
    return None;
}
""",
    False,
    ["import std.math.*;"],
)
run_single_error_count(
    "if_expression_single_undefined_error",
    """
function main(): None {
    x: Integer = if (true) { y; } else { 1; };
    return None;
}
""",
    "Undefined variable: y",
    1,
)
run_single_error_count(
    "match_expression_single_undefined_error",
    """
function main(): None {
    x: Integer = match (1) {
        1 => { y; },
        _ => { 0; }
    };
    return None;
}
""",
    "Undefined variable: y",
    1,
)
run_lint(
    "alias_imports_are_not_duplicates",
    """
import std.io as io;
import std.io as io2;
function main(): None {
    io.println("a");
    io2.println("b");
    return None;
}
""",
    True,
    ["[L001]"],
)
run_fix_then_check(
    "fix_import_with_trailing_comment_preserves_required_import",
    """
import std.string.*; // needed for Str.len
import std.io.*;
function main(): None {
    println(to_string(Str.len("abc")));
    return None;
}
""",
)
run_fix_then_check(
    "fix_import_with_block_comment_preserves_required_import",
    """
import std.string.*; /* needed for Str.len */
import std.io.*;
function main(): None {
    println(to_string(Str.len("abc")));
    return None;
}
""",
)
run_fix_preserves_prefix(
    "fix_preserves_shebang_prefix",
    """#!/usr/bin/env arden
import std.io.*;
function main(): None { println("ok"); return None; }
""",
    "#!/usr/bin/env arden\n",
)
run_single_fmt_roundtrip(
    "fmt_match_expr_statement_roundtrip",
    """
function main(): None {
    x: Integer = match (1) {
        1 => { (match (2) { 2 => { 3; }, _ => { 4; } }); },
        _ => { 0; }
    };
    return None;
}
""",
    True,
)
run_single_fmt_roundtrip(
    "fmt_preserves_shebang_roundtrip",
    """#!/usr/bin/env arden
import std.io.*;
function main(): None {
    println("ok");
    return None;
}
""",
    True,
)
run_fmt_preserves_prefix(
    "fmt_preserves_shebang_prefix",
    """#!/usr/bin/env arden
import std.io.*;
function main(): None {
    println("ok");
    return None;
}
""",
    "#!/usr/bin/env arden\n",
)
run_single(
    "async_borrow_capture_blocks_move",
    """
function take_borrow(borrow s: String): None { return None; }
function consume(owned s: String): None { return None; }
function main(): None {
    s: String = "x";
    t: Task<None> = async { take_borrow(s); return None; };
    consume(s);
    return None;
}
""",
    False,
    ["Cannot move 's' while borrowed"],
)
run_single(
    "async_mut_borrow_capture_blocks_assignment",
    """
function main(): None {
    mut x: Integer = 1;
    t: Task<None> = async {
        r: &mut Integer = &mut x;
        return None;
    };
    x += 1;
    return None;
}
""",
    False,
    ["Cannot assign to 'x' while"],
)
run_single(
    "async_mut_borrow_capture_blocks_immutable_borrow",
    """
function main(): None {
    mut x: Integer = 1;
    t: Task<None> = async {
        r: &mut Integer = &mut x;
        return None;
    };
    y: &Integer = &x;
    return None;
}
""",
    False,
    ["Cannot borrow 'x' while mutably borrowed"],
)
run_single_fmt_roundtrip(
    "fmt_if_expr_statement_roundtrip",
    """
function main(): None {
    mut x: Integer = 0;
    if (true) {
        x = 1;
    } else {
        x = 2;
    }
    return None;
}
""",
    True,
)
run_compile(
    "if_expr_lambda_then_else_codegen",
    """
function main(): None {
    f: (Integer) -> Integer = if (true) { (x: Integer) => x + 1; } else { (x: Integer) => x + 2; };
    y: Integer = f(1);
    return None;
}
""",
    True,
)
run_compile(
    "if_expr_lambda_local_codegen",
    """
function main(): None {
    f: (Integer) -> Integer = if (true) {
        g: (Integer) -> Integer = (x: Integer) => x + 1;
        g;
    } else {
        (x: Integer) => x + 2;
    };
    y: Integer = f(2);
    return None;
}
""",
    True,
)
run_project(
    "stdlib_alias_project_ok",
    {
        "main.arden": """
import std.io as io;
import std.math as math;
function main(): None {
    io.println(to_string(math.abs(-1)));
    return None;
}
""",
    },
    True,
)
run_project(
    "stdlib_alias_shadow_var_error",
    {
        "main.arden": """
import std.io as io;
function main(): None {
    io: Integer = 1;
    io.println("x");
    return None;
}
""",
    },
    False,
    ["Cannot call method on type Integer"],
)
run_project(
    "project_cross_file_type_error",
    {
        "main.arden": """
import std.io.*;
import util.*;
function main(): None {
    println(to_string(helper()));
    return None;
}
""",
        "util.arden": """
package util;
function helper(): Integer {
    return "bad";
}
""",
    },
    False,
    ["mismatch"],
)


# 100 generated smoke cases (50 pass + 50 fail)
bulk_root = tmp_dir / "bulk_100_cases"
bulk_root.mkdir(parents=True, exist_ok=True)
bulk_cases: list[tuple[str, str, bool, list[str] | None]] = []

for i in range(1, 101):
    if i % 2 == 1:
        src = f"""
import std.io.*;
function main(): None {{
    x: Integer = {i};
    y: Integer = x + 1;
    println(to_string(y));
    return None;
}}
"""
        bulk_cases.append((f"bulk_pass_{i:03d}", src, True, None))
    else:
        mode = i % 4
        if mode == 0:
            src = f"""
function main(): None {{
    x: Integer = "bad-{i}";
    return None;
}}
"""
            bulk_cases.append((f"bulk_fail_type_{i:03d}", src, False, ["Type mismatch"]))
        elif mode == 2:
            src = f"""
function main(): None {{
    x: Float = Math.abs(-1.0);
    return None;
}}
"""
            bulk_cases.append(
                (f"bulk_fail_import_{i:03d}", src, False, ["import std.math.*;"])
            )
        else:
            src = f"""
function take(owned s: String): None {{ return None; }}
function main(): None {{
    s: String = "x";
    r: &String = &s;
    take(s);
    return None;
}}
"""
            bulk_cases.append((f"bulk_fail_borrow_{i:03d}", src, False, ["while borrowed"]))

for name, src, expect_ok, required in bulk_cases:
    path = bulk_root / f"{name}.arden"
    path.write_text(src)
    proc = subprocess.run([compiler, "check", str(path)], capture_output=True, text=True)
    output = (proc.stdout or "") + (proc.stderr or "")
    ok = proc.returncode == 0
    if ok != expect_ok:
        raise SystemExit(
            f"[bulk:{name}] expected ok={expect_ok}, got rc={proc.returncode}\n{output}"
        )
    if required:
        for needle in required:
            if needle not in output:
                raise SystemExit(
                    f"[bulk:{name}] missing expected text: {needle!r}\n{output}"
                )
print("ci smoke regression bundle: explicit + 100 generated cases passed")
PY

"${COMPILER}" new shared_project --path "${SHARED_PROJECT}" >/dev/null
python3 - <<'PY' "${SHARED_PROJECT}/arden.toml"
from pathlib import Path
import sys

path = Path(sys.argv[1])
content = path.read_text()
content += '\noutput_kind = "shared"\n'
path.write_text(content)
PY
pushd "${SHARED_PROJECT}" >/dev/null
"${COMPILER}" build >/dev/null
assert_output_artifact_exists "${SHARED_PROJECT}/shared_project" "shared"
popd >/dev/null

"${COMPILER}" new static_project --path "${STATIC_PROJECT}" >/dev/null
python3 - <<'PY' "${STATIC_PROJECT}/arden.toml"
from pathlib import Path
import sys

path = Path(sys.argv[1])
content = path.read_text()
content += '\noutput_kind = "static"\n'
path.write_text(content)
PY
pushd "${STATIC_PROJECT}" >/dev/null
"${COMPILER}" build >/dev/null
assert_output_artifact_exists "${STATIC_PROJECT}/static_project" "static"
popd >/dev/null
