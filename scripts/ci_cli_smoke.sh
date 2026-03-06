#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
COMPILER="${REPO_ROOT}/target/release/apex-compiler"

cargo build --release >/dev/null

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

PROJECT_DIR="${TMP_DIR}/sample_project"
UGLY_FILE="${TMP_DIR}/ugly.apex"
LINT_FILE="${TMP_DIR}/linty.apex"
TEST_FILE="${TMP_DIR}/sample_test.apex"
HEADER_FILE="${TMP_DIR}/sample.h"
BINDINGS_FILE="${TMP_DIR}/bindings.apex"
OUT_FILE="${TMP_DIR}/ugly_bin"
NESTED_FIELD_FILE="${TMP_DIR}/nested_field_assign.apex"
NESTED_FIELD_OUT="${TMP_DIR}/nested_field_assign_bin"
SHARED_PROJECT="${TMP_DIR}/shared_project"
STATIC_PROJECT="${TMP_DIR}/static_project"
BORROW_ERR_OUT="${TMP_DIR}/borrow_err.out"
BORROW_USE_AFTER_MOVE_FILE="${TMP_DIR}/borrow_use_after_move.apex"
BORROW_MOVE_WHILE_BORROWED_FILE="${TMP_DIR}/borrow_move_while_borrowed.apex"
BORROW_DOUBLE_MUT_FILE="${TMP_DIR}/borrow_double_mut.apex"
BORROW_SCOPE_RELEASE_FILE="${TMP_DIR}/borrow_scope_release.apex"
BORROW_REBORROW_AFTER_SCOPE_FILE="${TMP_DIR}/borrow_reborrow_after_scope.apex"
BORROW_LAMBDA_MOVE_FILE="${TMP_DIR}/borrow_lambda_move.apex"
BORROW_COMPOUND_BORROWED_FILE="${TMP_DIR}/borrow_compound_borrowed.apex"
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
cat <<'EOF_PROJECT_CFG' > "${PROJECT_TYPECHECK_DIR}/apex.toml"
name = "project_typecheck"
version = "0.1.0"
entry = "src/main.apex"
files = ["src/main.apex", "src/util.apex"]
output = "project_typecheck"
opt_level = "0"
EOF_PROJECT_CFG
cat <<'EOF_PROJECT_MAIN' > "${PROJECT_TYPECHECK_DIR}/src/main.apex"
import std.io.*;
import util.*;
function main(): None {
    println(to_string(helper()));
    return None;
}
EOF_PROJECT_MAIN
cat <<'EOF_PROJECT_UTIL' > "${PROJECT_TYPECHECK_DIR}/src/util.apex"
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
cat <<'EOF_ALIAS_CFG' > "${PROJECT_STDLIB_ALIAS_DIR}/apex.toml"
name = "project_stdlib_alias"
version = "0.1.0"
entry = "src/main.apex"
files = ["src/main.apex"]
output = "project_stdlib_alias"
opt_level = "0"
EOF_ALIAS_CFG
cat <<'EOF_ALIAS_MAIN' > "${PROJECT_STDLIB_ALIAS_DIR}/src/main.apex"
import std.io as io;
import std.math as math;
function main(): None {
    io.println(to_string(math.abs(-1)));
    return None;
}
EOF_ALIAS_MAIN
(cd "${PROJECT_STDLIB_ALIAS_DIR}" && "${COMPILER}" check >/dev/null)

mkdir -p "${PROJECT_TABLE_CFG_DIR}/src"
cat <<'EOF_PROJECT_TABLE_CFG' > "${PROJECT_TABLE_CFG_DIR}/apex.toml"
[project]
name = "project_table_cfg"
version = "0.1.0"
entry = "src/main.apex"
files = ["src/main.apex"]
output = "project_table_cfg"
opt_level = "0"
EOF_PROJECT_TABLE_CFG
cat <<'EOF_PROJECT_TABLE_MAIN' > "${PROJECT_TABLE_CFG_DIR}/src/main.apex"
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
    path = single_root / f"{name}.apex"
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


def run_project(name: str, files: dict[str, str], expect_ok: bool, required: list[str] | None = None) -> None:
    project_root = tmp_dir / f"project_{name}"
    src_root = project_root / "src"
    src_root.mkdir(parents=True, exist_ok=True)

    file_list = sorted(files.keys())
    toml_files = ", ".join([f"\"src/{f}\"" for f in file_list])
    (project_root / "apex.toml").write_text(
        "\n".join(
            [
                f"name = \"{name}\"",
                "version = \"0.1.0\"",
                "entry = \"src/main.apex\"",
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
run_project(
    "stdlib_alias_project_ok",
    {
        "main.apex": """
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
        "main.apex": """
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
        "main.apex": """
import std.io.*;
import util.*;
function main(): None {
    println(to_string(helper()));
    return None;
}
""",
        "util.apex": """
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
    path = bulk_root / f"{name}.apex"
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
python3 - <<'PY' "${SHARED_PROJECT}/apex.toml"
from pathlib import Path
import sys

path = Path(sys.argv[1])
content = path.read_text()
content += '\noutput_kind = "shared"\n'
path.write_text(content)
PY
pushd "${SHARED_PROJECT}" >/dev/null
"${COMPILER}" build >/dev/null
test -f "${SHARED_PROJECT}/shared_project"
popd >/dev/null

"${COMPILER}" new static_project --path "${STATIC_PROJECT}" >/dev/null
python3 - <<'PY' "${STATIC_PROJECT}/apex.toml"
from pathlib import Path
import sys

path = Path(sys.argv[1])
content = path.read_text()
content += '\noutput_kind = "static"\n'
path.write_text(content)
PY
pushd "${STATIC_PROJECT}" >/dev/null
"${COMPILER}" build >/dev/null
test -f "${STATIC_PROJECT}/static_project"
popd >/dev/null
