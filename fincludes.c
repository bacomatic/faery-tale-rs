/* pre-compiled include files for fmain.c */

#include "exec/types.h"
#include "exec/memory.h"
#include "graphics/view.h"
#include "hardware/blit.h"
#include "hardware/custom.h"
#include "graphics/gfxmacros.h"
#include "graphics/copper.h"
#include "graphics/display.h"
#include "graphics/text.h"
#include "graphics/gfxbase.h"
#include "graphics/sprite.h"
#include "exec/devices.h"
#include "libraries/diskfont.h"
#include "libraries/dosextens.h"
#include "devices/input.h"
#include "devices/inputevent.h"
#include "devices/trackdisk.h"
#include "devices/audio.h"
#include "workbench/startup.h"

/* Forward declarations for Amiga library functions */
struct Library *OpenLibrary();
struct Layer_Info *NewLayerInfo();
struct TextFont *OpenFont();
PLANEPTR AllocRaster();
struct MsgPort *CreatePort();
struct IORequest *CreateExtIO();
APTR AllocMem();
struct Layer *CreateUpfrontLayer();

