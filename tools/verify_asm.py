#!/usr/bin/env python3
"""Assemble and execute 68000 code snippets to verify assembly logic.

Uses m68k-linux-gnu-as (GNU cross-assembler) to assemble Motorola 68k assembly
and machine68k (Musashi-based CPU emulator) to execute it step-by-step.

This tool helps verify understanding of 68k assembly instructions found in the
FTA source code (fsubs.asm, fsupp.asm, gdriver.asm, etc.) by assembling and
running small test snippets.

Prerequisites:
    sudo apt-get install binutils-m68k-linux-gnu
    pip install machine68k

Usage:
    # Inline assembly snippet
    python tools/verify_asm.py -c "moveq #42,d0; moveq #10,d1; add.l d1,d0"

    # From a file
    python tools/verify_asm.py snippet.s

    # Show specific registers and memory
    python tools/verify_asm.py -c "moveq #42,d0" --regs d0,d1 --mem 0x2000:16

    # Step-by-step trace
    python tools/verify_asm.py -c "moveq #42,d0; add.l d0,d0" --trace

    # Set initial register/memory state
    python tools/verify_asm.py -c "add.l d1,d0" --set-reg d0=100,d1=50
    python tools/verify_asm.py -c "move.w (a0),d0" --set-mem 0x2000=0xBEEF --set-reg a0=0x2000

    # Non-interactive JSON output for scripts
    python tools/verify_asm.py -c "moveq #5,d0" --json
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

# Check dependencies early
ASSEMBLER = "m68k-linux-gnu-as"
OBJCOPY = "m68k-linux-gnu-objcopy"

CODE_BASE = 0x1000      # Where code is loaded in emulated memory
DATA_BASE = 0x2000      # Default data area
STACK_TOP = 0xFFF0      # Initial stack pointer
MEM_SIZE = 64 * 1024    # 64KB emulated RAM


def check_dependencies():
    """Verify required tools are available."""
    errors = []
    if not shutil.which(ASSEMBLER):
        errors.append(
            f"{ASSEMBLER} not found. Install with: "
            "sudo apt-get install binutils-m68k-linux-gnu"
        )
    try:
        import machine68k  # noqa: F401
    except ImportError:
        errors.append("machine68k not found. Install with: pip install machine68k")
    if errors:
        for e in errors:
            print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)


def assemble(source_text):
    """Assemble 68k source text and return raw binary bytes.

    Uses GNU as with --register-prefix-optional so that Motorola-style
    register names (d0, a0) work without % prefixes.
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        src_path = os.path.join(tmpdir, "input.s")
        obj_path = os.path.join(tmpdir, "input.o")
        bin_path = os.path.join(tmpdir, "input.bin")

        # Wrap in .text section
        full_source = "    .text\n" + source_text + "\n"
        with open(src_path, "w") as f:
            f.write(full_source)

        # Assemble
        result = subprocess.run(
            [ASSEMBLER, "-m68000", "--register-prefix-optional",
             "-o", obj_path, src_path],
            capture_output=True, text=True
        )
        if result.returncode != 0:
            print("Assembly failed:", file=sys.stderr)
            print(result.stderr, file=sys.stderr)
            sys.exit(1)

        # Extract raw binary
        result = subprocess.run(
            [OBJCOPY, "-O", "binary", obj_path, bin_path],
            capture_output=True, text=True
        )
        if result.returncode != 0:
            print("objcopy failed:", file=sys.stderr)
            print(result.stderr, file=sys.stderr)
            sys.exit(1)

        with open(bin_path, "rb") as f:
            return f.read()


def parse_reg_value(s):
    """Parse 'reg=value' into (register_enum, int_value)."""
    from machine68k import Register
    name, val_str = s.split("=", 1)
    name = name.strip().upper()
    reg = getattr(Register, name, None)
    if reg is None:
        print(f"Unknown register: {name}", file=sys.stderr)
        print(f"Valid: D0-D7, A0-A7, SP, SR, PC", file=sys.stderr)
        sys.exit(1)
    value = int(val_str.strip(), 0)
    return reg, value


def parse_mem_spec(s):
    """Parse 'addr=value' for --set-mem or 'addr:len' for --mem."""
    if "=" in s:
        addr_str, val_str = s.split("=", 1)
        addr = int(addr_str.strip(), 0)
        value = int(val_str.strip(), 0)
        return addr, value
    elif ":" in s:
        addr_str, len_str = s.split(":", 1)
        addr = int(addr_str.strip(), 0)
        length = int(len_str.strip(), 0)
        return addr, length
    else:
        addr = int(s.strip(), 0)
        return addr, 16  # default 16 bytes


def read_all_regs(machine):
    """Read all data and address registers plus SR, PC."""
    from machine68k import Register
    regs = {}
    for i in range(8):
        regs[f"D{i}"] = machine.cpu.r_reg(getattr(Register, f"D{i}"))
    for i in range(8):
        regs[f"A{i}"] = machine.cpu.r_reg(getattr(Register, f"A{i}"))
    regs["PC"] = machine.cpu.r_reg(Register.PC)
    regs["SR"] = machine.cpu.r_reg(Register.SR)
    return regs


def format_sr(sr):
    """Format status register flags."""
    flags = []
    if sr & 0x10:
        flags.append("X")
    if sr & 0x08:
        flags.append("N")
    if sr & 0x04:
        flags.append("Z")
    if sr & 0x02:
        flags.append("V")
    if sr & 0x01:
        flags.append("C")
    return "".join(flags) if flags else "-"


def dump_memory(machine, addr, length):
    """Hex dump of memory region."""
    data = machine.mem.r_block(addr, length)
    lines = []
    for offset in range(0, len(data), 16):
        chunk = data[offset:offset + 16]
        hex_part = " ".join(f"{b:02x}" for b in chunk)
        ascii_part = "".join(chr(b) if 32 <= b < 127 else "." for b in chunk)
        lines.append(f"  0x{addr + offset:04x}: {hex_part:<48s} {ascii_part}")
    return "\n".join(lines)


def run_snippet(source_text, trace=False, reg_filter=None, mem_regions=None,
                init_regs=None, init_mem=None, json_output=False,
                max_steps=None):
    """Assemble, load, execute, and report results."""
    import machine68k
    from machine68k import CPUType, Register

    # Assemble
    code = assemble(source_text)
    if not code:
        print("Assembly produced empty output", file=sys.stderr)
        sys.exit(1)

    # Create machine and load code
    m = machine68k.Machine(CPUType.M68000, MEM_SIZE)
    m.mem.w_block(CODE_BASE, code)
    m.cpu.w_pc(CODE_BASE)
    m.cpu.w_reg(Register.SP, STACK_TOP)

    # Set initial register values
    if init_regs:
        for reg, value in init_regs:
            m.cpu.w_reg(reg, value & 0xFFFFFFFF)

    # Set initial memory values
    if init_mem:
        for addr, value in init_mem:
            if value <= 0xFF:
                m.mem.w8(addr, value)
            elif value <= 0xFFFF:
                m.mem.w16(addr, value)
            else:
                m.mem.w32(addr, value)

    code_end = CODE_BASE + len(code)
    step_limit = max_steps if max_steps else 1000  # safety limit

    # Disassemble only the code region for display
    disasm_lines = []
    pc = CODE_BASE
    while pc < code_end:
        nbytes, text = m.cpu.disassemble(pc)
        if nbytes <= 0:
            break
        disasm_lines.append((pc, nbytes, text))
        pc += nbytes

    if not json_output:
        print(f"=== 68000 Assembly Verification ===")
        print(f"Code loaded at 0x{CODE_BASE:04X}, {len(code)} bytes\n")
        print("Disassembly:")
        for pc, nbytes, text in disasm_lines:
            raw = m.mem.r_block(pc, nbytes)
            hex_str = " ".join(f"{b:02x}" for b in raw)
            print(f"  0x{pc:04x}: {hex_str:<12s} {text}")
        print()

    # Execute step-by-step, stopping when PC leaves code region
    trace_data = []
    steps_executed = 0
    if not json_output and trace:
        print("Execution trace:")

    while steps_executed < step_limit:
        cur_pc = m.cpu.r_reg(Register.PC)
        if cur_pc < CODE_BASE or cur_pc >= code_end:
            break
        _nbytes, text = m.cpu.disassemble(cur_pc)
        m.execute(1)
        steps_executed += 1
        regs_after = read_all_regs(m)
        sr_flags = format_sr(regs_after["SR"])

        if trace:
            step_info = {
                "step": steps_executed,
                "pc": cur_pc,
                "instruction": text,
                "regs": regs_after,
                "flags": sr_flags,
            }
            trace_data.append(step_info)

            if not json_output:
                reg_strs = []
                if reg_filter:
                    for rname in reg_filter:
                        val = regs_after.get(rname.upper(), 0)
                        reg_strs.append(f"{rname.upper()}={val}")
                else:
                    for rname in [f"D{j}" for j in range(8)] + [f"A{j}" for j in range(8)]:
                        val = regs_after[rname]
                        if val != 0 or rname in ("A7",):
                            reg_strs.append(f"{rname}={val}")
                print(f"  [{steps_executed}] 0x{cur_pc:04x}: {text}")
                print(f"       {' '.join(reg_strs)}  flags={sr_flags}")

    # Final state
    final_regs = read_all_regs(m)
    sr_flags = format_sr(final_regs["SR"])

    if json_output:
        output = {
            "code_bytes": len(code),
            "steps_executed": steps_executed,
            "disassembly": [
                {"pc": pc, "bytes": m.mem.r_block(pc, nb).hex(), "text": text}
                for pc, nb, text in disasm_lines
            ],
            "final_registers": final_regs,
            "flags": sr_flags,
        }
        if trace:
            output["trace"] = trace_data
        if mem_regions:
            output["memory"] = {}
            for addr, length in mem_regions:
                data = m.mem.r_block(addr, length)
                output["memory"][f"0x{addr:x}"] = data.hex()
        print(json.dumps(output, indent=2))
        return

    print(f"\n=== Final State ===")

    # Show registers
    if reg_filter:
        for rname in reg_filter:
            rname = rname.upper()
            val = final_regs.get(rname, 0)
            print(f"  {rname} = {val} (0x{val & 0xFFFFFFFF:08X})")
    else:
        # Show all non-zero data/address regs
        for prefix, count in [("D", 8), ("A", 8)]:
            line_parts = []
            for i in range(count):
                rname = f"{prefix}{i}"
                val = final_regs[rname]
                if val != 0 or rname == "A7":
                    line_parts.append(f"{rname}=0x{val & 0xFFFFFFFF:08X}")
            if line_parts:
                print(f"  {' '.join(line_parts)}")

    print(f"  SR=0x{final_regs['SR']:04X} flags={sr_flags}")
    print(f"  PC=0x{final_regs['PC']:04X}")

    # Show memory regions
    if mem_regions:
        print(f"\nMemory:")
        for addr, length in mem_regions:
            print(dump_memory(m, addr, length))


def normalize_inline_asm(text):
    """Convert semicolons to newlines and ensure proper indentation."""
    lines = []
    for line in text.split(";"):
        line = line.strip()
        if line:
            # Labels (ending with :) go at column 0, instructions get indented
            if line.endswith(":") or ":" in line.split()[0]:
                lines.append(line)
            else:
                lines.append("    " + line)
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Assemble and execute 68000 code snippets for verification.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""\
Examples:
  %(prog)s -c "moveq #42,d0; moveq #10,d1; add.l d1,d0"
  %(prog)s -c "moveq #42,d0" --regs d0 --trace
  %(prog)s -c "add.l d1,d0" --set-reg d0=100,d1=50
  %(prog)s -c "move.w (a0),d0" --set-mem 0x2000=0xBEEF --set-reg a0=0x2000
  %(prog)s snippet.s --trace --json
"""
    )
    parser.add_argument("file", nargs="?", help="Assembly source file")
    parser.add_argument("-c", "--code", help="Inline assembly (use ; as line separator)")
    parser.add_argument("--trace", action="store_true", help="Step-by-step execution trace")
    parser.add_argument("--regs", help="Comma-separated registers to display (e.g. d0,d1,a0)")
    parser.add_argument("--mem", action="append",
                        help="Memory region to dump: addr:len (e.g. 0x2000:16)")
    parser.add_argument("--set-reg", help="Initial register values: reg=val,... (e.g. d0=42,a0=0x2000)")
    parser.add_argument("--set-mem", action="append",
                        help="Initial memory value: addr=val (e.g. 0x2000=0xBEEF)")
    parser.add_argument("--json", action="store_true", help="JSON output for scripting")
    parser.add_argument("--steps", type=int, help="Max instructions to execute (default: stop at end of code)")

    args = parser.parse_args()

    if not args.file and not args.code:
        parser.error("Provide assembly via -c or as a file argument")

    check_dependencies()

    # Get source text
    if args.code:
        source = normalize_inline_asm(args.code)
    else:
        with open(args.file) as f:
            source = f.read()

    # Parse options
    reg_filter = [r.strip() for r in args.regs.split(",")] if args.regs else None

    init_regs = None
    if args.set_reg:
        init_regs = [parse_reg_value(s) for s in args.set_reg.split(",")]

    init_mem = None
    if args.set_mem:
        init_mem = [parse_mem_spec(s) for s in args.set_mem]

    mem_regions = None
    if args.mem:
        mem_regions = [parse_mem_spec(s) for s in args.mem]

    run_snippet(source, trace=args.trace, reg_filter=reg_filter,
                mem_regions=mem_regions, init_regs=init_regs,
                init_mem=init_mem, json_output=args.json,
                max_steps=args.steps)


if __name__ == "__main__":
    main()
