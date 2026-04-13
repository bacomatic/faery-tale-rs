# Discovery: narr.asm Complete Message Database

**Status**: complete
**Investigated**: 2025-01-27
**Requested by**: orchestrator
**Prompt summary**: Extract ALL messages from narr.asm: placard texts, speech entries, event entries, place names, inside place names, princess names, brother names.

## File Structure Overview

narr.asm (519 lines) is the complete text database. It contains:
- _event_msg (lines 11-58): 39 sequential null-terminated event messages
- _question (lines 63-82): 8 riddle questions for game intro
- _place_tbl / _place_msg (lines 86-197): outdoor location table + 27 messages
- _inside_tbl / _inside_msg (lines 117-226): indoor location table + 23 messages
- _placard_text (lines 235-347): 20 story placard screens
- _speeches (lines 351-518): 61 NPC dialogue entries

See detailed tables in agent response text (too large for file with shell escaping issues).

## Refinement Log
- 2025-01-27: Complete extraction. Full tables in agent response.
