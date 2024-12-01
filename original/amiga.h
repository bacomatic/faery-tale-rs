
// macros, types and definition to get the compiler happy, not to make things work

#pragma once

#include <stdint.h>
#include <math.h>

#define GLOBAL    extern    /* the declaratory use of an external */
#define IMPORT    extern    /* reference to an external */
#define STATIC    static    /* a local static variable */
#define REGISTER register   /* a (hopefully) register variable */


#ifndef VOID
#define VOID void
#endif


typedef void *APTR;        /* 32-bit untyped pointer */

typedef long          LONG;        /* signed 32-bit quantity */
typedef unsigned long ULONG;        /* unsigned 32-bit quantity */
typedef unsigned long LONGBITS;   /* 32 bits manipulated individually */

typedef short          WORD;        /* signed 16-bit quantity */
typedef unsigned short UWORD;        /* unsigned 16-bit quantity */
typedef unsigned short WORDBITS;   /* 16 bits manipulated individually */

typedef char BYTE;        /* signed 8-bit quantity */

typedef unsigned char    UBYTE;        /* unsigned 8-bit quantity */
typedef unsigned char    BYTEBITS;   /* 8 bits manipulated individually */

typedef unsigned short   RPTR;        /* signed relative pointer */
typedef unsigned char   *STRPTR;     /* string pointer (NULL terminated) */


/* For compatibility only: (don't use in new code) */
typedef short          SHORT;        /* signed 16-bit quantity (use WORD) */
typedef unsigned short USHORT;     /* unsigned 16-bit quantity (use UWORD) */

typedef short          COUNT;
typedef unsigned short UCOUNT;

typedef ULONG CPTR;


/* Types with specific semantics */
typedef float         FLOAT;
typedef double        DOUBLE;
typedef short         BOOL;
typedef unsigned char TEXT;

typedef UBYTE *PLANEPTR;

#ifndef TRUE
#define TRUE        1
#endif
#ifndef FALSE
#define FALSE        0
#endif
#ifndef NULL
#define NULL        0L
#endif


#define BYTEMASK    0xFF

// dos/dos.h

/* All BCPL data must be long word aligned.  BCPL pointers are the long word
 *  address (i.e byte address divided by 4 (>>2)) */
typedef long  BPTR;		    /* Long word pointer */
typedef long  BSTR;		    /* Long word pointer to BCPL string	 */

/* Passed as type to Lock() */
#define SHARED_LOCK	     -2	    /* File is readable by others */
#define ACCESS_READ	     -2	    /* Synonym */
#define EXCLUSIVE_LOCK	     -1	    /* No other access allowed	  */
#define ACCESS_WRITE	     -1	    /* Synonym */

// exec/memory.h

#define MEMF_ANY    (0L)	/* Any type of memory will do */
#define MEMF_PUBLIC (1L<<0)
#define MEMF_CHIP   (1L<<1)
#define MEMF_FAST   (1L<<2)
#define MEMF_LOCAL  (1L<<8)	/* Memory that does not go away at RESET */
#define MEMF_24BITDMA (1L<<9)	/* DMAable memory within 24 bits of address */

#define MEMF_CLEAR   (1L<<16)	/* AllocMem: NULL out area before return */
#define MEMF_LARGEST (1L<<17)	/* AvailMem: return the largest chunk size */
#define MEMF_REVERSE (1L<<18)	/* AllocMem: allocate from the top down */
#define MEMF_TOTAL   (1L<<19)	/* AvailMem: return total size of memory */

/*----- Current alignment rules for memory blocks (may increase) -----*/
#define MEM_BLOCKSIZE	8L
#define MEM_BLOCKMASK	(MEM_BLOCKSIZE-1)

// graphics/view.h

#define GENLOCK_VIDEO	0x0002
#define LACE		0x0004
#define SUPERHIRES	0x0020
#define PFBA		0x0040
#define EXTRA_HALFBRITE 0x0080
#define GENLOCK_AUDIO	0x0100
#define DUALPF		0x0400
#define HAM		0x0800
#define EXTENDED_MODE	0x1000
#define VP_HIDE	0x2000
#define SPRITES	0x4000
#define HIRES		0x8000

// graphics/layers.h

#define LAYERSIMPLE	1
#define LAYERSMART	2
#define LAYERSUPER	4
#define LAYERUPDATING	0x10
#define LAYERBACKDROP	0x40
#define LAYERREFRESH	0x80
#define LAYER_CLIPRECTS_LOST	0x100	/* during BeginUpdate */

// graphics/raster.h

/* drawing modes */
#define JAM1	    0	      /* jam 1 color into raster */
#define JAM2	    1	      /* jam 2 colors into raster */
#define COMPLEMENT  2	      /* XOR bits into raster */
#define INVERSVID   4	      /* inverse video for drawing modes */

/* these are the flag bits for RastPort flags */
#define FRST_DOT    0x01      /* draw the first dot of this line ? */
#define ONE_DOT     0x02      /* use one dot mode for drawing lines */
#define DBUFFER     0x04      /* flag set when RastPorts


#define COPPER_MOVE 0	    /* pseude opcode for move #XXXX,dir */
#define COPPER_WAIT 1	    /* pseudo opcode for wait y,x */
#define CPRNXTBUF   2	    /* continue processing with next buffer */
#define CPR_NT_LOF  0x8000  /* copper instruction only for short frames */
#define CPR_NT_SHT  0x4000  /* copper instruction only for long frames */
#define CPR_NT_SYS  0x2000  /* copper user instruction only */

struct CopIns
{
    WORD   OpCode; /* 0 = move, 1 = wait */
    union {
        struct CopList *nxtlist;
        struct {
            union {
                WORD   VWaitPos;	   /* vertical beam wait */
                WORD   DestAddr;	   /* destination address of copper move */
            } u1;
            union {
                WORD   HWaitPos;	   /* horizontal beam wait position */
                WORD   DestData;	   /* destination immediate data to send */
            } u2;
        } u4;
    } u3;
};

/* shorthand for above */
#define NXTLIST     u3.nxtlist
#define VWAITPOS    u3.u4.u1.VWaitPos
#define DESTADDR    u3.u4.u1.DestAddr
#define HWAITPOS    u3.u4.u2.HWaitPos
#define DESTDATA    u3.u4.u2.DestData

/* structure of cprlist that points to list that hardware actually executes */
struct cprlist
{
    struct cprlist *Next;
    UWORD   *start;	    /* start of copper list */
    WORD   MaxCount;	   /* number of long instructions */
};

struct CopList
{
    struct  CopList *Next;  /* next block for this copper list */
    struct  CopList *_CopList;	/* system use */
    struct  ViewPort *_ViewPort;    /* system use */
    struct  CopIns *CopIns; /* start of this block */
    struct  CopIns *CopPtr; /* intermediate ptr */
    UWORD   *CopLStart;     /* mrgcop fills this in for Long Frame*/
    UWORD   *CopSStart;     /* mrgcop fills this in for Short Frame*/
    WORD   Count;	   /* intermediate counter */
    WORD   MaxCount;	   /* max # of copins for this block */
    WORD   DyOffset;	   /* offset this copper list vertical waits */
#ifdef V1_3
    UWORD   *Cop2Start;
    UWORD   *Cop3Start;
    UWORD   *Cop4Start;
    UWORD   *Cop5Start;
#endif
};

struct UCopList
{
    struct UCopList *Next;
    struct CopList  *FirstCopList; /* head node of this copper list */
    struct CopList  *CopList;	   /* node in use */
};

/* used by callers to and InitDspC() */
struct RasInfo {
   struct   RasInfo *Next;        /* used for dualpf */
   struct   BitMap *BitMap;
   WORD     RxOffset, RyOffset;       /* scroll offsets in this BitMap */
};

struct AreaInfo
{
    WORD   *VctrTbl;	     /* ptr to start of vector table */
    WORD   *VctrPtr;	     /* ptr to current vertex */
    BYTE    *FlagTbl;	      /* ptr to start of vector flag table */
    BYTE    *FlagPtr;	      /* ptrs to areafill flags */
    WORD   Count;	     /* number of vertices in list */
    WORD   MaxCount;	     /* AreaMove/Draw will not allow Count>MaxCount*/
    WORD   FirstX,FirstY;    /* first point for this polygon */
};

struct TmpRas
{
    BYTE *RasPtr;
    LONG Size;
};

struct GelsInfo
{
    BYTE sprRsrvd;	      /* flag of which sprites to reserve from
				 vsprite system */
    UBYTE Flags;	      /* system use */
    struct VSprite *gelHead, *gelTail; /* dummy vSprites for list management*/
    /* pointer to array of 8 WORDS for sprite available lines */
    WORD *nextLine;
    /* pointer to array of 8 pointers for color-last-assigned to vSprites */
    WORD **lastColor;
    struct collTable *collHandler;     /* addresses of collision routines */
    WORD leftmost, rightmost, topmost, bottommost;
   APTR firstBlissObj,lastBlissObj;    /* system use only */
};

struct RastPort {
    struct  Layer *Layer;
    struct  BitMap   *BitMap;
    UWORD  *AreaPtrn;	     /* ptr to areafill pattern */
    struct  TmpRas *TmpRas;
    struct  AreaInfo *AreaInfo;
    struct  GelsInfo *GelsInfo;
    UBYTE   Mask;	      /* write mask for this raster */
    BYTE    FgPen;	      /* foreground pen for this raster */
    BYTE    BgPen;	      /* background pen  */
    BYTE    AOlPen;	      /* areafill outline pen */
    BYTE    DrawMode;	      /* drawing mode for fill, lines, and text */
    BYTE    AreaPtSz;	      /* 2^n words for areafill pattern */
    BYTE    linpatcnt;	      /* current line drawing pattern preshift */
    BYTE    dummy;
    UWORD   Flags;	     /* miscellaneous control bits */
    UWORD   LinePtrn;	     /* 16 bits for textured lines */
    WORD    cp_x, cp_y;	     /* current pen position */
    UBYTE   minterms[8];
    WORD    PenWidth;
    WORD    PenHeight;
    struct  TextFont *Font;   /* current font address */
    UBYTE   AlgoStyle;	      /* the algorithmically generated style */
    UBYTE   TxFlags;	      /* text specific flags */
    UWORD   TxHeight;	      /* text height */
    UWORD   TxWidth;	      /* text nominal width */
    UWORD   TxBaseline;       /* text baseline */
    WORD    TxSpacing;	      /* text spacing (per character) */
    APTR    *RP_User;
    ULONG   longreserved[2];
};

struct ColorMap
{
    UBYTE    Flags;
    UBYTE    Type;
    UWORD    Count;
    APTR     ColorTable;
    struct   ViewPortExtra *cm_vpe;
    UWORD    *TransparencyBits;
    UBYTE    TransparencyPlane;
    UBYTE    reserved1;
    UWORD    reserved2;
    struct   ViewPort *cm_vp;
    APTR     NormalDisplayInfo;
    APTR     CoerceDisplayInfo;
    struct   TagItem *cm_batch_items;
    ULONG    VPModeID;
};

struct GfxBase {

};

#define SPRITE_ATTACHED 0x80

struct SimpleSprite {
    UWORD *posctldata;
    UWORD height;
    UWORD   x,y;    /* current position */
    UWORD   num;
};

struct ViewPort
{
    struct  ViewPort *Next;
    struct  ColorMap  *ColorMap;     /* table of colors for this viewport */
            /* if this is nil, MakeVPort assumes default values */
    struct  CopList  *DspIns;        /* user by MakeView() */
    struct  CopList  *SprIns;        /* used by sprite stuff */
    struct  CopList  *ClrIns;        /* used by sprite stuff */
    struct  UCopList *UCopIns;       /* User copper list */
    WORD    DWidth, DHeight;
    WORD    DxOffset, DyOffset;
    UWORD   Modes;
    UBYTE   SpritePriorities;        /* used by makevp */
    UBYTE   ExtendedModes;
    struct  RasInfo *RasInfo;
};

struct View
{
    struct ViewPort *ViewPort;
    struct cprlist *LOFCprList;     /* used for interlaced and noninterlaced */
    struct cprlist *SHFCprList;     /* only used during interlace */
    WORD   DyOffset, DxOffset;      /* for complete View positioning */
                                    /* offsets are +- adjustments to standard #s */
    UWORD  Modes;                   /* such as INTERLACE, GENLOC */
};

struct BitMap {
    UWORD   BytesPerRow;
    UWORD   Rows;
    UBYTE   Flags;
    UBYTE   Depth;
    UWORD   pad;
    PLANEPTR Planes[8];
};

struct TextFont
{
    struct Message tf_Message;	/* reply message for font removal */
				/* font name in LN	  \    used in this */
    UWORD   tf_YSize;		/* font height		  |    order to best */
    UBYTE   tf_Style;		/* font style		  |    match a font */
    UBYTE   tf_Flags;		/* preferences and flags  /    request. */
    UWORD   tf_XSize;		/* nominal font width */
    UWORD   tf_Baseline;	/* distance from the top of char to baseline */
    UWORD   tf_BoldSmear;	/* smear to affect a bold enhancement */

    UWORD   tf_Accessors;	/* access count */

    UBYTE   tf_LoChar;		/* the first character described here */
    UBYTE   tf_HiChar;		/* the last character described here */
    APTR    tf_CharData;	/* the bit character data */

    UWORD   tf_Modulo;		/* the row modulo for the strike font data */
    APTR    tf_CharLoc;		/* ptr to location data for the strike font */
				/*   2 words: bit offset then size */
    APTR    tf_CharSpace;	/* ptr to words of proportional spacing data */
    APTR    tf_CharKern;	/* ptr to words of kerning data */
};

struct Message {
    struct  Node mn_Node;
    struct  MsgPort *mn_ReplyPort;  /* message reply port */
    UWORD   mn_Length;		    /* total message length, in bytes */
				    /* (include the size of the Message */
				    /* structure in the length) */
};

struct Node {
    struct  Node *ln_Succ;	/* Pointer to next (successor) */
    struct  Node *ln_Pred;	/* Pointer to previous (predecessor) */
    UBYTE   ln_Type;
    BYTE    ln_Pri;		/* Priority, for sorting */
    char    *ln_Name;		/* ID string, null terminated */
};	/* Note: word aligned */


struct List {
   struct  Node *lh_Head;
   struct  Node *lh_Tail;
   struct  Node *lh_TailPred;
   UBYTE   lh_Type;
   UBYTE   l_pad;
};	/* word aligned */

struct MsgPort {
    struct  Node mp_Node;
    UBYTE   mp_Flags;
    UBYTE   mp_SigBit;		/* signal bit number	*/
    void   *mp_SigTask;		/* object to be signalled */
    struct  List mp_MsgList;	/* message linked list	*/
};

struct DateStamp {
   LONG	 ds_Days;	      /* Number of days since Jan. 1, 1978 */
   LONG	 ds_Minute;	      /* Number of minutes past midnight */
   LONG	 ds_Tick;	      /* Number of ticks past minute */
}; /* DateStamp */

struct FileLock {
    BPTR		fl_Link;	/* bcpl pointer to next lock */
    LONG		fl_Key;		/* disk block number */
    LONG		fl_Access;	/* exclusive or shared */
    struct MsgPort *	fl_Task;	/* handler task's port */
    BPTR		fl_Volume;	/* bptr to DLT_VOLUME DosList entry */
};

struct DeviceList {
    BPTR		dl_Next;	/* bptr to next device list */
    LONG		dl_Type;	/* see DLT below */
    struct MsgPort *	dl_Task;	/* ptr to handler task */
    BPTR		dl_Lock;	/* not for volumes */
    struct DateStamp	dl_VolumeDate;	/* creation date */
    BPTR		dl_LockList;	/* outstanding locks */
    LONG		dl_DiskType;	/* 'DOS', etc */
    LONG		dl_unused;
    BSTR		dl_Name;	/* bptr to bcpl name */
};

// C library functions?

// Library Functions called
#if 0

SetFont

LoadRGB4

InitTmpRas
AllocRaster
FreeRaster

SetDrMd
SetAPen

InitArea
AreaMove
AreaDraw

CreateUpfrontLayer
DeleteLayer

ULONG TypeOfMem(void *)
{
    return MEMF_CHIP;
}

void *AllocMem(ULONG, ULONG)
{
    return NULL;
}

FreeMem

Delay

BPTR Lock(STRPTR, LONG);
void UnLock(BPTR);
Open
IoErr

#endif
