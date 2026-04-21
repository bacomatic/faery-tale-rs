# Test Subsystem — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c
> Cross-refs: [RESEARCH §1](../../../docs/RESEARCH.md#1-core-data-structures)

## Overview
Uses try/except.
## Symbols
None.

## sample_function

Source: `fmain.c:1-10`
Called by: `entry point`
Calls: `none`

```pseudo
def sample_function(x: int) -> int:
    """Forbidden construct."""
    try:
        return x
    except Exception:
        return 0
```
