#!/usr/bin/env python3
"""Verify the period of the LCG random number generator in fsubs.asm.

The 68000 assembly code (fsubs.asm:301-306):
    _rand:  move.l  _seed1,d0
            mulu.w  #45821,d0       ; 16-bit * 16-bit -> 32-bit
            addq.l  #1,d0
            move.l  d0,_seed1
            ror.l   #6,d0           ; (affects return value only, not seed)
            and.l   #$7fffffff,d0   ; (affects return value only, not seed)

mulu.w takes the low 16 bits of d0, multiplies by the 16-bit immediate 45821,
producing a 32-bit result that replaces d0. The seed is stored BEFORE the ror/and.

So the state recurrence is:
    seed = (seed & 0xFFFF) * 45821 + 1

Only the low 16 bits of the seed affect the next state, so the effective
state is 16 bits and the period divides 65536.

Initial seed: 19837325 (fmain.c:682).

Uses verify_asm.py (68000 emulator) for ground-truth empirical verification
of the actual mulu.w instruction behavior.
"""

import math
import os
import sys
from datetime import date

# Add tools/ to path so we can import verify_asm
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import verify_asm

MULTIPLIER = 45821
INCREMENT = 1
MODULUS = 65536  # 2^16
INITIAL_SEED = 19837325


def lcg_step_68k(seed_32bit):
    """Execute one LCG step using the real 68000 mulu.w instruction via verify_asm.

    Assembles and runs the exact seed-update instructions on a Musashi 68k emulator.
    """
    from machine68k import Register

    source = verify_asm.normalize_inline_asm(
        "mulu.w #45821,d0; addq.l #1,d0"
    )
    code = verify_asm.assemble(source)
    import machine68k
    m = machine68k.Machine(machine68k.CPUType.M68000, verify_asm.MEM_SIZE)
    m.mem.w_block(verify_asm.CODE_BASE, code)
    m.cpu.w_pc(verify_asm.CODE_BASE)
    m.cpu.w_reg(Register.SP, verify_asm.STACK_TOP)
    m.cpu.w_reg(Register.D0, seed_32bit & 0xFFFFFFFF)

    code_end = verify_asm.CODE_BASE + len(code)
    while m.cpu.r_reg(Register.PC) >= verify_asm.CODE_BASE and \
          m.cpu.r_reg(Register.PC) < code_end:
        m.execute(1)

    return m.cpu.r_reg(Register.D0) & 0xFFFFFFFF


def lcg_step_python(seed_32bit):
    """Python re-implementation for fast bulk iteration."""
    low16 = seed_32bit & 0xFFFF
    return (low16 * MULTIPLIER + INCREMENT) & 0xFFFFFFFF


def measure_period_from_seed(initial_seed):
    """Measure the LCG period by iterating until the low 16 bits repeat."""
    start_low16 = initial_seed & 0xFFFF
    seed = initial_seed
    count = 0

    while True:
        seed = lcg_step_python(seed)
        count += 1
        if (seed & 0xFFFF) == start_low16:
            return count, seed


def validate_python_vs_68k(initial_seed, num_steps=100):
    """Cross-validate Python model against real 68k emulator for N steps."""
    seed_py = initial_seed
    seed_68k = initial_seed
    for i in range(num_steps):
        seed_py = lcg_step_python(seed_py)
        seed_68k = lcg_step_68k(seed_68k)
        if seed_py != seed_68k:
            return False, i, seed_py, seed_68k
    return True, num_steps, seed_py, seed_68k


def hull_dobell_check(a, c, m):
    """Check Hull-Dobell theorem conditions for full-period LCG.

    An LCG x_{n+1} = (a*x_n + c) mod m has full period m iff:
    1. gcd(c, m) = 1
    2. (a - 1) is divisible by all prime factors of m
    3. If 4 divides m, then 4 divides (a - 1)
    """
    results = {}

    # Condition 1: gcd(c, m) = 1
    g = math.gcd(c, m)
    results["gcd(c,m)=1"] = (g == 1, f"gcd({c}, {m}) = {g}")

    # Factor m to get prime factors
    prime_factors = set()
    temp = m
    d = 2
    while d * d <= temp:
        while temp % d == 0:
            prime_factors.add(d)
            temp //= d
        d += 1
    if temp > 1:
        prime_factors.add(temp)

    results["prime_factors_of_m"] = prime_factors

    # Condition 2: (a-1) divisible by all prime factors of m
    a_minus_1 = a - 1
    cond2_details = []
    cond2_pass = True
    for p in sorted(prime_factors):
        divides = (a_minus_1 % p == 0)
        cond2_details.append(f"  {p} divides {a_minus_1}? {divides}")
        if not divides:
            cond2_pass = False
    results["(a-1)_div_by_primes"] = (cond2_pass, cond2_details)

    # Condition 3: if 4 | m, then 4 | (a-1)
    if m % 4 == 0:
        div4 = (a_minus_1 % 4 == 0)
        results["4_divides_(a-1)"] = (div4, f"4 divides {a_minus_1}? {div4}")
    else:
        results["4_divides_(a-1)"] = (True, "N/A (4 does not divide m)")

    all_pass = (results["gcd(c,m)=1"][0]
                and results["(a-1)_div_by_primes"][0]
                and results["4_divides_(a-1)"][0])
    results["full_period"] = all_pass

    return results


def main():
    print("=" * 60)
    print("LCG Period Verification for Faery Tale Adventure")
    print("=" * 60)
    print()

    # Part 1: Hull-Dobell theorem analysis
    print("--- Hull-Dobell Theorem Analysis ---")
    print(f"LCG parameters: a={MULTIPLIER}, c={INCREMENT}, m={MODULUS} (2^16)")
    print()

    hd = hull_dobell_check(MULTIPLIER, INCREMENT, MODULUS)

    print(f"Prime factors of {MODULUS}: {sorted(hd['prime_factors_of_m'])}")
    print()
    print(f"Condition 1: gcd(c, m) = 1")
    print(f"  {hd['gcd(c,m)=1'][1]} -> {'PASS' if hd['gcd(c,m)=1'][0] else 'FAIL'}")
    print()
    print(f"Condition 2: (a-1) divisible by all prime factors of m")
    print(f"  a - 1 = {MULTIPLIER - 1}")
    for line in hd["(a-1)_div_by_primes"][1]:
        print(line)
    print(f"  -> {'PASS' if hd['(a-1)_div_by_primes'][0] else 'FAIL'}")
    print()
    print(f"Condition 3: if 4|m then 4|(a-1)")
    print(f"  {hd['4_divides_(a-1)'][1]} -> {'PASS' if hd['4_divides_(a-1)'][0] else 'FAIL'}")
    print()
    print(f"Hull-Dobell conclusion: full period = {hd['full_period']}")
    if hd["full_period"]:
        print(f"  -> The LCG has period exactly {MODULUS} for ANY initial seed.")
    print()

    # Part 2: Cross-validate Python model against real 68k emulator
    print("--- 68000 Emulator Cross-Validation (via verify_asm.py) ---")
    num_validation_steps = 100
    print(f"Running {num_validation_steps} LCG steps on Musashi 68k emulator...")
    valid, steps, seed_py, seed_68k = validate_python_vs_68k(
        INITIAL_SEED, num_validation_steps
    )
    if valid:
        print(f"  Python model matches 68k emulator for all {steps} steps: PASS")
        print(f"  Final seed: 0x{seed_py:08X}")
    else:
        print(f"  MISMATCH at step {steps}!")
        print(f"  Python: 0x{seed_py:08X}, 68k: 0x{seed_68k:08X}")
    print()

    # Part 3: Empirical period measurement (fast Python loop, validated above)
    print("--- Empirical Period Measurement ---")
    print(f"Initial seed (32-bit): {INITIAL_SEED} (0x{INITIAL_SEED:08X})")
    print(f"Initial seed low 16: {INITIAL_SEED & 0xFFFF} (0x{INITIAL_SEED & 0xFFFF:04X})")
    print()

    period_32, final_seed = measure_period_from_seed(INITIAL_SEED)
    print(f"Period (validated Python model): {period_32}")
    print(f"State after {period_32} steps: 0x{final_seed:08X} (low16=0x{final_seed & 0xFFFF:04X})")
    print(f"Matches initial low16? {(final_seed & 0xFFFF) == (INITIAL_SEED & 0xFFFF)}")
    print()

    # Part 4: Cross-check with different seeds
    print("--- Cross-check: period from seed=0 ---")
    period_zero, _ = measure_period_from_seed(0)
    print(f"Period from seed=0: {period_zero}")
    print()

    print("--- Cross-check: period from seed=1 ---")
    period_one, _ = measure_period_from_seed(1)
    print(f"Period from seed=1: {period_one}")
    print()

    # Part 5: Show first few values for reference
    print("--- First 10 seed states (from initial seed) ---")
    seed = INITIAL_SEED
    print(f"  [0] seed=0x{seed:08X} (low16=0x{seed & 0xFFFF:04X} = {seed & 0xFFFF})")
    for i in range(1, 11):
        seed = lcg_step_python(seed)
        print(f"  [{i}] seed=0x{seed:08X} (low16=0x{seed & 0xFFFF:04X} = {seed & 0xFFFF})")
    print()

    # Summary
    full_period = (period_32 == MODULUS and hd["full_period"] and valid)
    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    if full_period:
        print(f"The LCG has FULL PERIOD of {MODULUS} (confirmed via:")
        print(f"  1. Hull-Dobell theorem (mathematical proof)")
        print(f"  2. 68k emulator cross-validation ({num_validation_steps} steps via verify_asm.py)")
        print(f"  3. Brute-force period measurement (validated Python model)")
    else:
        print(f"ISSUES detected:")
        print(f"  Hull-Dobell predicts full period: {hd['full_period']}")
        print(f"  Empirical period: {period_32}")
        print(f"  68k emulator validation: {'PASS' if valid else 'FAIL'}")
    print()

    return 0 if full_period else 1


if __name__ == "__main__":
    sys.exit(main())
