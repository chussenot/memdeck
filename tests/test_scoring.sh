#!/bin/sh
# Test scoring and progress operations

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$PROJECT_DIR/bin/memdeck-tui"
PASS=0
FAIL=0
TMPDIR="${TMPDIR:-/tmp}/memdeck-test-$$"
mkdir -p "$TMPDIR"

# Use temp dir for user data to avoid polluting real data
export XDG_DATA_HOME="$TMPDIR/xdg"
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

echo "=== Scoring Tests ==="

# Test initial stats (no progress file)
output=$("$BIN" stats 2>&1)
assert_contains "initial sessions 0" "Sessions:.*0" "$output"
assert_contains "initial correct 0" "Correct:.*0" "$output"
assert_contains "initial streak 0" "Day Streak:.*0" "$output"
assert_contains "initial last Never" "Never" "$output"

# Test reset command
"$BIN" reset-progress >/dev/null 2>&1
output=$("$BIN" stats 2>&1)
assert_contains "reset sessions 0" "Sessions:.*0" "$output"

# Test progress file format
mkdir -p "$TMPDIR/xdg/memdeck"
cat > "$TMPDIR/xdg/memdeck/progress.dat" <<'EOF'
total_sessions=5
total_correct=42
total_incorrect=8
best_score=84
current_streak=3
best_streak=7
last_date=2026-03-31
err_0=2
err_5=3
cor_0=10
cor_5=7
EOF

output=$("$BIN" stats 2>&1)
assert_contains "loaded sessions" "Sessions:.*5" "$output"
assert_contains "loaded correct" "Correct:.*42" "$output"
assert_contains "loaded incorrect" "Incorrect:.*8" "$output"
assert_contains "loaded accuracy" "84%" "$output"
assert_contains "loaded streak" "Day Streak:.*3" "$output"
assert_contains "loaded last date" "2026-03-31" "$output"

# Test help command
output=$("$BIN" help 2>&1)
assert_contains "help shows usage" "Usage" "$output"
assert_contains "help shows validate" "validate" "$output"
assert_contains "help shows export" "export" "$output"

# Cleanup
rm -rf "$TMPDIR"

echo "Scoring tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
