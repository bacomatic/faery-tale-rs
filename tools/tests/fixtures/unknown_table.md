# Test Subsystem — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c
> Cross-refs: [RESEARCH §1](../../../reference/RESEARCH.md#1-core-data-structures)

## Overview
Calls a table that isn't registered.
## Symbols
None.

## sample_function

Source: `fmain.c:1-10`
Called by: `entry point`
Calls: `TABLE:this_does_not_exist`

```pseudo
def sample_function(x: int) -> int:
    """Uses an unregistered table ref."""
    return x
```
