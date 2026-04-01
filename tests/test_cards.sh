#!/bin/sh
# Test card parsing and validation via the validate command

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$PROJECT_DIR/bin/memdeck-tui"
PASS=0
FAIL=0
TMPDIR="${TMPDIR:-/tmp}/memdeck-test-$$"
mkdir -p "$TMPDIR"

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

assert_ok() {
    desc="$1"; shift
    if "$@" >/dev/null 2>&1; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        echo "FAIL: $desc (command failed)"
    fi
}

assert_fail() {
    desc="$1"; shift
    if "$@" >/dev/null 2>&1; then
        FAIL=$((FAIL + 1))
        echo "FAIL: $desc (command should have failed)"
    else
        PASS=$((PASS + 1))
    fi
}

echo "=== Card Tests ==="

# Test valid stack files
assert_ok "validate aronson" "$BIN" validate "$PROJECT_DIR/data/stacks/aronson.tsv"
assert_ok "validate mnemonica" "$BIN" validate "$PROJECT_DIR/data/stacks/mnemonica.tsv"
assert_ok "validate example" "$BIN" validate "$PROJECT_DIR/data/stacks/example-custom.tsv"

# Test invalid: missing cards
cat > "$TMPDIR/short.tsv" <<'EOF'
1	AS
2	2S
3	3S
EOF
assert_fail "reject short stack" "$BIN" validate "$TMPDIR/short.tsv"

# Test invalid: duplicate card
cat > "$TMPDIR/dup-card.tsv" <<'EOF'
1	AS
2	AS
3	3S
4	4S
5	5S
6	6S
7	7S
8	8S
9	9S
10	10S
11	JS
12	QS
13	KS
14	AH
15	2H
16	3H
17	4H
18	5H
19	6H
20	7H
21	8H
22	9H
23	10H
24	JH
25	QH
26	KH
27	AC
28	2C
29	3C
30	4C
31	5C
32	6C
33	7C
34	8C
35	9C
36	10C
37	JC
38	QC
39	KC
40	AD
41	2D
42	3D
43	4D
44	5D
45	6D
46	7D
47	8D
48	9D
49	10D
50	JD
51	QD
52	KD
EOF
assert_fail "reject duplicate card" "$BIN" validate "$TMPDIR/dup-card.tsv"

# Test invalid: bad card code
cat > "$TMPDIR/bad-card.tsv" <<'EOF'
1	XY
EOF
assert_fail "reject bad card code" "$BIN" validate "$TMPDIR/bad-card.tsv"

# Test: empty file
cat > "$TMPDIR/empty.tsv" <<'EOF'
# just a comment
EOF
assert_fail "reject empty stack" "$BIN" validate "$TMPDIR/empty.tsv"

# Test: various card formats (case insensitive)
cat > "$TMPDIR/case.tsv" <<'EOF'
1	as
2	2h
3	10c
4	kd
5	js
6	qh
7	3c
8	4d
9	5s
10	6h
11	7c
12	8d
13	9s
14	ah
15	2c
16	3d
17	4s
18	5h
19	6c
20	7d
21	8s
22	9h
23	10s
24	jh
25	qc
26	kc
27	ad
28	2s
29	3h
30	4c
31	5d
32	6s
33	7h
34	8c
35	9d
36	10h
37	jc
38	qd
39	ks
40	ac
41	2d
42	3s
43	4h
44	5c
45	6d
46	7s
47	8h
48	9c
49	10d
50	jd
51	qs
52	kh
EOF
assert_ok "accept lowercase cards" "$BIN" validate "$TMPDIR/case.tsv"

# Cleanup
rm -rf "$TMPDIR"

echo "Card tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
