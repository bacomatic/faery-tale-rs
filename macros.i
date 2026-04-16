* macros.i - custom macros for Faery Tale Adventure assembly files
* Reconstructed from usage in MakeBitMap.asm

DECLARE		MACRO
		cseg
		public	_\1
_\1:
		ENDM

SaveM		MACRO
		movem.l	\1,-(sp)
		ENDM

RestoreM	MACRO
		movem.l	(sp)+,\1
		ENDM

CallGfx		MACRO
		xref	_LVO\1
		move.l	_GfxBase,a6
		jsr	_LVO\1(a6)
		ENDM
