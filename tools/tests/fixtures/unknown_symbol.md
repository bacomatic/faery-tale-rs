# Test Subsystem — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c
> Cross-refs: [RESEARCH §1](../../docs/RESEARCH.md#1-core-data-structures)

## Overview
Uses a symbol not registered anywhere.
## Symbols
None.

## sample_function

Source: `fmain.c:1-10`
Called by: `entry point`
Calls: `none`

```pseudo
def sample_function(x: int) -> int:
    """Uses an undefined name."""
    return undefined_global + x
```
