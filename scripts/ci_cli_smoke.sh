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
