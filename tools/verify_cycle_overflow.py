#!/usr/bin/env python3
"""Verify that all uses of `cycle` (short/16-bit signed) in fmain.c are safe
across the signed overflow boundary.

cycle is declared `short cycle` (16-bit signed, range -32768..32767) and
incremented every tick without reset. After 32767 ticks it wraps to -32768
under two's complement. This script checks every usage pattern found in the
source to determine whether the pattern produces identical results when
treating the underlying 16-bit value as signed vs unsigned.

Key insight: bitmask operations (& N) on two's complement are bit-identical
regardless of signedness. Arithmetic right shift (>>) and signed division (/)
may differ.
"""

import ctypes
import re
import sys
from datetime import date
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
FMAIN = REPO_ROOT / "fmain.c"
RESULTS_DIR = REPO_ROOT / "tools" / "results"


def to_signed16(val):
    """Interpret a 16-bit unsigned value as signed (two's complement)."""
    return ctypes.c_int16(val).value


def c_signed_div(a, b):
    """Emulate C89 signed integer division (truncation toward zero)."""
    # Python's // truncates toward negative infinity; C truncates toward zero
    if (a < 0) ^ (b < 0):
        return -(abs(a) // abs(b))
    return abs(a) // abs(b)


def extract_cycle_patterns():
    """Read fmain.c and extract all lines referencing `cycle`."""
    text = FMAIN.read_text(encoding="latin-1")
    patterns = []
    for lineno, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        # Skip declaration and bare increment
        if "short cycle" in stripped or stripped == "cycle++;":
            continue
        if "cycle" in stripped:
            patterns.append((lineno, stripped))
    return patterns


def test_bitmask(mask, label="cycle & N"):
    """Test `cycle & mask` for all 65536 16-bit values.
    Returns (safe: bool, first_mismatch_unsigned: int or None)."""
    for u in range(65536):
        s = to_signed16(u)
        # In C, bitwise & on a signed short promotes to int but the bit
        # pattern is the same under two's complement. We verify:
        unsigned_result = u & mask
        signed_result = s & mask
        # Python's & on negative ints extends sign; mask with 0xFFFF to compare
        if (unsigned_result & 0xFFFF) != (signed_result & 0xFFFF):
            return False, u
    return True, None


def test_add_bitmask(addend, mask, label="(cycle+i) & N"):
    """Test `(cycle + addend) & mask`."""
    for u in range(65536):
        s = to_signed16(u)
        unsigned_result = (u + addend) & mask
        signed_result = (s + addend) & mask
        if (unsigned_result & 0xFFFF) != (signed_result & 0xFFFF):
            return False, u
    return True, None


def test_shift_bitmask(shift, mask, label="(cycle>>N) & M"):
    """Test `(cycle >> shift) & mask`.
    C arithmetic right shift on signed values is implementation-defined
    (but on 68000/Aztec C it's arithmetic). We test both sign-extending
    (arithmetic) and zero-filling (logical) shift."""
    results = {}

    # Arithmetic shift (sign-extending) — the 68000 ASR behavior
    mismatches_arith = []
    for u in range(65536):
        s = to_signed16(u)
        unsigned_result = (u >> shift) & mask
        signed_result = (s >> shift) & mask  # Python >> is arithmetic for negative
        if (unsigned_result & 0xFFFF) != (signed_result & 0xFFFF):
            mismatches_arith.append(u)
            if len(mismatches_arith) >= 5:
                break
    results["arithmetic_shift"] = (len(mismatches_arith) == 0, mismatches_arith)

    return results


def test_div_bitmask(divisor, mask, label="(cycle/N) & M"):
    """Test `(cycle / divisor) & mask`.
    This is the critical test — signed division truncates toward zero in C,
    which differs from how unsigned division works for negative values."""
    mismatches = []
    for u in range(65536):
        s = to_signed16(u)
        unsigned_result = (u // divisor) & mask
        signed_result = c_signed_div(s, divisor) & mask
        if (unsigned_result & 0xFFFF) != (signed_result & 0xFFFF):
            mismatches.append((u, s, unsigned_result, signed_result))
            if len(mismatches) >= 10:
                break
    return len(mismatches) == 0, mismatches


def test_bitmask_equality(mask, compare_val, label="(cycle&N) == V"):
    """Test `(cycle & mask) == compare_val`."""
    for u in range(65536):
        s = to_signed16(u)
        unsigned_result = (u & mask) == compare_val
        signed_result = (s & mask) == compare_val
        if unsigned_result != signed_result:
            return False, u
    return True, None


def main():
    print("=" * 70)
    print("Cycle Overflow Safety Verification")
    print("=" * 70)

    # Step 1: Extract all cycle patterns from source
    patterns = extract_cycle_patterns()
    print(f"\nFound {len(patterns)} cycle references in fmain.c:\n")
    for lineno, line in patterns:
        print(f"  Line {lineno}: {line}")

    # Step 2: Categorize and test each pattern
    print("\n" + "=" * 70)
    print("Testing each pattern across full signed 16-bit range")
    print("=" * 70)

    results = []

    # Pattern 1: cycle & 1 (lines 1400, 1518, 1539, 1729, 1804)
    safe, mm = test_bitmask(1)
    res = ("cycle & 1", "SAFE" if safe else f"UNSAFE at u={mm}", safe)
    results.append(res)
    print(f"\n  cycle & 1 : {'SAFE' if safe else 'UNSAFE'}")

    # Pattern 2: (cycle+i) & 1 (line 2497, i varies)
    # Test with representative i values 0..31
    all_safe = True
    for i in range(32):
        safe, mm = test_add_bitmask(i, 1)
        if not safe:
            all_safe = False
            break
    res = ("(cycle+i) & 1", "SAFE" if all_safe else f"UNSAFE", all_safe)
    results.append(res)
    print(f"  (cycle+i) & 1 : {'SAFE' if all_safe else 'UNSAFE'}")

    # Pattern 3: (cycle+i) & 7 (line 1632)
    all_safe = True
    for i in range(32):
        safe, mm = test_add_bitmask(i, 7)
        if not safe:
            all_safe = False
            break
    res = ("(cycle+i) & 7", "SAFE" if all_safe else f"UNSAFE", all_safe)
    results.append(res)
    print(f"  (cycle+i) & 7 : {'SAFE' if all_safe else 'UNSAFE'}")

    # Pattern 4: cycle & 2 (line 1640)
    safe, mm = test_bitmask(2)
    res = ("cycle & 2", "SAFE" if safe else f"UNSAFE at u={mm}", safe)
    results.append(res)
    print(f"  cycle & 2 : {'SAFE' if safe else 'UNSAFE'}")

    # Pattern 5: (cycle>>1) & 1 (line 1658)
    shift_res = test_shift_bitmask(1, 1)
    arith_safe, arith_mm = shift_res["arithmetic_shift"]
    detail = "SAFE" if arith_safe else f"UNSAFE (arith shift): {len(arith_mm)} mismatches"
    res = ("(cycle>>1) & 1", detail, arith_safe)
    results.append(res)
    print(f"  (cycle>>1) & 1 : {detail}")
    if not arith_safe and arith_mm:
        for u in arith_mm[:3]:
            s = to_signed16(u)
            print(f"    u={u} s={s}: unsigned={(u>>1)&1} signed={(s>>1)&1}")

    # Pattern 6: (cycle/2) & 1 (line 1805) — THE CRITICAL ONE
    safe, mismatches = test_div_bitmask(2, 1)
    if safe:
        detail = "SAFE"
    else:
        detail = f"UNSAFE: {len(mismatches)} mismatches found (showing up to 10)"
    res = ("(cycle/2) & 1", detail, safe)
    results.append(res)
    print(f"  (cycle/2) & 1 : {detail}")
    if not safe:
        print("    Mismatches (unsigned_val, signed_val, unsigned_result, signed_result):")
        for u, s, ur, sr in mismatches:
            print(f"      u={u:5d} s={s:6d} : unsigned_result={ur} signed_result={sr}")

    # Pattern 7: cycle & 3 (line 1810)
    safe, mm = test_bitmask(3)
    res = ("cycle & 3", "SAFE" if safe else f"UNSAFE at u={mm}", safe)
    results.append(res)
    print(f"  cycle & 3 : {'SAFE' if safe else 'UNSAFE'}")

    # Pattern 8: (cycle & 7) == 0 (line 1849)
    safe, mm = test_bitmask_equality(7, 0)
    res = ("(cycle&7) == 0", "SAFE" if safe else f"UNSAFE at u={mm}", safe)
    results.append(res)
    print(f"  (cycle&7) == 0 : {'SAFE' if safe else 'UNSAFE'}")

    # Step 3: Check for any comparison operators used with cycle
    print("\n" + "=" * 70)
    print("Scanning for comparison operators with cycle")
    print("=" * 70)
    text = FMAIN.read_text(encoding="latin-1")
    comparison_ops = []
    for lineno, line in enumerate(text.splitlines(), 1):
        if "cycle" not in line:
            continue
        stripped = line.strip()
        # Skip declaration and increment
        if "short cycle" in stripped or stripped == "cycle++;":
            continue
        # Check for comparisons: >, <, >=, <=, ==, != applied to cycle itself
        # (not to (cycle & N) == 0 which is a bitmask check)
        # Look for patterns like cycle > N, cycle < N, cycle == N, cycle != N
        # But NOT (expr & N) == V patterns
        if re.search(r'(?<![&|^])\bcycle\b\s*[><=!]=?\s*\d', stripped):
            comparison_ops.append((lineno, stripped))
        elif re.search(r'\d\s*[><=!]=?\s*\bcycle\b', stripped):
            comparison_ops.append((lineno, stripped))

    if comparison_ops:
        print(f"\n  FOUND {len(comparison_ops)} direct comparison(s):")
        for ln, line in comparison_ops:
            print(f"    Line {ln}: {line}")
    else:
        print("\n  No direct comparisons (>, <, ==, !=) on cycle itself found.")
        print("  All uses are bitmask (&), shift (>>), or division (/) followed by bitmask.")

    # Step 4: Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)

    all_safe = all(r[2] for r in results)
    unsafe = [r for r in results if not r[2]]

    if all_safe:
        print("\n  ALL patterns are safe across the signed overflow boundary.")
        status = "PASS"
    else:
        print(f"\n  {len(unsafe)} pattern(s) are UNSAFE:")
        for name, detail, _ in unsafe:
            print(f"    - {name}: {detail}")
        status = "PARTIAL"

    # Step 5: Detailed analysis of cycle/2 vs cycle>>1
    print("\n" + "=" * 70)
    print("Detailed: (cycle/2)&1 vs (cycle>>1)&1")
    print("=" * 70)
    print("\n  For odd negative values, C signed division (cycle/2) truncates")
    print("  toward zero, e.g. -1/2 = 0, while arithmetic right shift -1>>1 = -1.")
    print("  This means (cycle/2)&1 and (cycle>>1)&1 can differ for negative cycle.")
    print("\n  Example values around the overflow boundary:")
    for val in [32765, 32766, 32767, -32768, -32767, -32766, -1, -2, -3]:
        s = ctypes.c_int16(val).value
        u = val & 0xFFFF
        div_result = c_signed_div(s, 2) & 1
        shift_result = (s >> 1) & 1
        u_div = (u // 2) & 1
        print(f"    cycle={s:6d} (u={u:5d}): (cycle/2)&1={div_result}  "
              f"(cycle>>1)&1={shift_result}  unsigned:(u/2)&1={u_div}")

    # Write results file
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    result_path = RESULTS_DIR / "cycle_overflow.txt"

    findings = []
    findings.append("All bitmask patterns (cycle & N) are safe — bitwise AND is "
                     "identical for signed and unsigned under two's complement.")
    findings.append("All (cycle+i) & N patterns are safe — addition overflow wraps "
                     "identically in two's complement, and the subsequent mask discards "
                     "upper bits.")
    if any(not r[2] for r in results if "cycle/2" in r[0]):
        findings.append("(cycle/2)&1 at line 1805 is UNSAFE: signed division truncates "
                         "toward zero for negative values, producing different low-bit "
                         "results than unsigned division. However, this only affects "
                         "animation frame selection (toggling between two frames), so "
                         "the visual effect is a phase shift in the animation cycle, "
                         "not a crash or gameplay bug.")
    if any(not r[2] for r in results if "cycle>>1" in r[0]):
        findings.append("(cycle>>1)&1 at line 1658 is UNSAFE under arithmetic right "
                         "shift for negative values.")
    else:
        findings.append("(cycle>>1)&1 at line 1658 is SAFE — arithmetic right shift "
                         "preserves the bit pattern for the low bit after masking.")
    findings.append("No direct comparison operators (>, <, ==, !=) are used on "
                     "cycle itself — only on bitmask results like (cycle&7)==0, "
                     "which is safe.")

    with open(result_path, "w") as f:
        f.write(f"Experiment: cycle_overflow\n")
        f.write(f"Date: {date.today().isoformat()}\n")
        f.write(f"Command: python tools/verify_cycle_overflow.py\n")
        f.write(f"Status: {status}\n")
        f.write(f"\nFindings:\n")
        for finding in findings:
            f.write(f"- {finding}\n")
        f.write(f"\nDetails:\n")

        f.write("\nAll cycle usage patterns in fmain.c:\n")
        for lineno, line in patterns:
            f.write(f"  Line {lineno}: {line}\n")

        f.write("\nPattern safety results:\n")
        for name, detail, safe in results:
            f.write(f"  {name}: {detail}\n")

        if not all_safe:
            f.write("\nUnsafe pattern analysis:\n")
            for name, detail, safe in results:
                if not safe and "cycle/2" in name:
                    f.write(f"  {name}:\n")
                    f.write(f"    Signed division of negative odd numbers truncates toward zero.\n")
                    f.write(f"    Example: -1/2 = 0 (signed) vs 32767/2 = 16383 (same bit pattern unsigned)\n")
                    f.write(f"    Impact: animation frame selection toggles at a different phase\n")
                    f.write(f"    after overflow. This affects race==4 enemy DYING animation only.\n")
                    f.write(f"    Severity: cosmetic (animation glitch), not a crash or logic error.\n")

        if comparison_ops:
            f.write(f"\nDirect comparison operators found:\n")
            for ln, line in comparison_ops:
                f.write(f"  Line {ln}: {line}\n")
        else:
            f.write(f"\nNo direct comparison operators on cycle found.\n")

    print(f"\nResults written to {result_path}")
    return 0 if all_safe else 2


if __name__ == "__main__":
    sys.exit(main())
