#!/bin/sh
# Test stack operations: list, export, import

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$PROJECT_DIR/bin/memdeck-tui"
PASS=0
FAIL=0
TMPDIR="${TMPDIR:-/tmp}/memdeck-test-$$"
mkdir -p "$TMPDIR"

export MEMDECK_DATA="$PROJECT_DIR/data"

assert_eq() {
    desc="$1"; expected="$2"; actual="$3"
    if [ "$expected" = "$actual" ]; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        echo "FAIL: $desc"
        echo "  expected: $expected"
        echo "  actual:   $actual"
    fi
}

assert_contains() {
    desc="$1"; needle="$2"; haystack="$3"
    if echo "$haystack" | grep -q "$needle"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        echo "FAIL: $desc (output does not contain '$needle')"
    fi
}

assert_ok() {
    desc="$1"; shift
    if "$@" >/dev/null 2>&1; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        echo "FAIL: $desc (command failed)"
    fi
}

echo "=== Stack Tests ==="

# Test listing stacks
output=$("$BIN" list 2>&1)
assert_contains "list includes aronson" "Aronson" "$output"
assert_contains "list includes mnemonica" "Mnemonica" "$output"
assert_contains "list shows built-in" "built-in" "$output"

# Test export
output=$("$BIN" export "Aronson" 2>&1)
assert_contains "export has header" "# Aronson" "$output"
assert_contains "export has JS at pos 1" "1	JS" "$output"
assert_contains "export has 10H at pos 52" "52	10H" "$output"

output=$("$BIN" export "Mnemonica" 2>&1)
assert_contains "mnemonica export has 4C" "1	4C" "$output"
assert_contains "mnemonica export has KS" "52	KS" "$output"

# Test export nonexistent
"$BIN" export "NonExistent" >/dev/null 2>&1 && {
    FAIL=$((FAIL + 1))
    echo "FAIL: export nonexistent should fail"
} || PASS=$((PASS + 1))

# Test round-trip: export then validate
"$BIN" export "Aronson" > "$TMPDIR/exported.tsv" 2>/dev/null
assert_ok "validate exported stack" "$BIN" validate "$TMPDIR/exported.tsv"

# Test that each built-in stack has exactly 52 entries
for stack in aronson mnemonica memorandum; do
    count=$(grep -v '^#' "$PROJECT_DIR/data/stacks/$stack.tsv" | grep -c '[0-9]')
    assert_eq "$stack has 52 entries" "52" "$count"
done

# Cleanup
rm -rf "$TMPDIR"

echo "Stack tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
