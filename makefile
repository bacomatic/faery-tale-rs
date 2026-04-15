; -pa = ANSI compiler mode
; -qf = output errors and warnings to AztecC.Err
CFLAGS = -pa -qf

OBJS = fmain.o fsubs.o narr.o fmain2.o iffsubs.o gdriver.o makebitmap.o hdrive.o

AINC = AZTEC:incl_asm

.asm.o:
	as >asm.out -i$(AINC) $*.asm -o $@

.c.pre:
	cc $(CFLAGS) -ho $@ $*.c

.c.o:
	cc $(CFLAGS) -hi fincludes.pre -o $@ $*.c

fmain: fincludes.pre $(OBJS)
	ln +c -o fmain $(OBJS) -lc

clean:
	@DELETE #?.o #?.pre fmain asm.out AztecC.Err QUIET
