#include "exec/types.h"
#include "exec/memory.h"
#include "exec/io.h"
#include "graphics/display.h"
#include "graphics/gfxbase.h"
#include "graphics/gfx.h"

#define FOURCC(a,b,c,d) (((long)(a)<<24)|((long)(b)<<16)|((long)(c)<<8)|(long)(d))

#define IFF_FORM	FOURCC('F','O','R','M')
#define IFF_ILBM	FOURCC('I','L','B','M')
#define IFF_BMHD	FOURCC('B','M','H','D')
#define IFF_CMAP	FOURCC('C','M','A','P')
#define IFF_GRAB	FOURCC('G','R','A','B')
#define IFF_BODY	FOURCC('B','O','D','Y')
#define IFF_CAMG	FOURCC('C','A','M','G')
#define IFF_CRNG	FOURCC('C','R','N','G')
#define IFF_DEST	FOURCC('D','E','S','T')

long file_length;
long myfile;
long header;
long blocklength;

#define	cmpNone		0
#define	cmpByteRun1	1

typedef struct {
	short	width, height;
	short	xpic, ypic;
	UBYTE	nPlanes;
	UBYTE	masking;
	UBYTE	compression;
	UBYTE	pad1;
	short	transcolor;
	short	xAspect, yAspect;
	short	pageWidth, pageHeight;
} BitMapHeader;

BitMapHeader bmhd;

typedef struct {
	unsigned char colors[32][3];
} ColorMap;

/* ColorMap cmap; */

typedef struct {
	short xgrab, ygrab;
} GrabPoint;

/* GrabPoint grab; */

typedef struct {
	UBYTE	depth;
	UBYTE	pad1;
	UBYTE	planePick;
	UBYTE	planeOnOff;
	UBYTE	planeMask;
} DestMerge;

/* ReadHeader()
{	Read(myfile,&header,4);
	file_length -= 4;
}

ReadLength()
{	Read(myfile,&blocklength,4);
	file_length = file_length - 4 - blocklength;
} */

extern char *GfxBase;
char	*plane0, *plane1, *plane2, *plane3, *plane4;
long	bytecount;
char	*packdata;
extern struct ViewPort *vp;
extern char *shape_mem;
char	compress;

/* char *AllocMem(); */

#ifdef blarg
unpackpic(filename,bitmap) char *filename; struct BitMap *bitmap;
{	myfile = Open(filename,1005);
	if (myfile == 0)
	{	printf("couldn't get file\n"); return FALSE; }

	ReadHeader();
	if (header != IFF_FORM)
	{	printf("Unrecognizable format\n"); Close(myfile); return FALSE; }
	ReadLength(); file_length = blocklength;

	while (file_length)
	{	ReadHeader();
		if (header == IFF_ILBM) ;
		else if (header == IFF_BMHD)
		{	ReadLength(); Read(myfile,&bmhd,blocklength); }
		else if (header == IFF_CMAP)
		{	ReadLength(); Read(myfile,&cmap,blocklength); }
		else if (header == IFF_CAMG || header == IFF_CRNG || header == IFF_DEST)
		{	ReadLength(); Seek(myfile,blocklength,0); }
		else if (header == IFF_GRAB)
		{	ReadLength(); Read(myfile,&grab,blocklength); }
		else if (header == IFF_BODY)
		{	int i;
			char *membuffer;

			bytecount = (bmhd.width+7)/8;
			if (bytecount & 1) bytecount++;
			ReadLength();
			membuffer = AllocMem(blocklength,0);
			if (membuffer) { Read(myfile,membuffer,blocklength); }
			packdata = membuffer;
			plane0 = (char *)(bitmap->Planes[0]);
			plane1 = (char *)(bitmap->Planes[1]);
			plane2 = (char *)(bitmap->Planes[2]);
			plane3 = (char *)(bitmap->Planes[3]);
			plane4 = (char *)(bitmap->Planes[4]);
			compress = bmhd.compression;
			for (i=0; i<bmhd.height; i++)
			{	unpack_line(plane0); plane0 += 40;
				unpack_line(plane1); plane1 += 40;
				unpack_line(plane2); plane2 += 40;
				unpack_line(plane3); plane3 += 40;
				unpack_line(plane4); plane4 += 40;
			}
			FreeMem(membuffer,blocklength);
			break;
		}
		else
		{	printf("Unrecognizable format\n"); Close(myfile); return FALSE; }
	}
	Close(myfile);
	return TRUE;
}
#endif

int unpackbrush(char *filename, struct BitMap *bitmap, short x, short y)
{	int bitoffset = (x + (bitmap->BytesPerRow)*y);

	myfile = Open(filename,1005);
	if (myfile == 0) {	return FALSE; }

	ReadHeader();
	if (header != IFF_FORM)
	{	Close(myfile); return FALSE; }
	ReadLength(); file_length = blocklength;

	while (file_length)
	{	ReadHeader();
		if (header == IFF_ILBM) ;
		else if (header == IFF_BMHD)
		{	ReadLength(); Read(myfile,&bmhd,blocklength); }
		else if (header == IFF_CAMG || header == IFF_CRNG || header == IFF_DEST
			|| header == IFF_CMAP || header == IFF_GRAB)
		{	ReadLength(); Seek(myfile,blocklength,0); }
		else if (header == IFF_BODY)
		{	int i;

			packdata = shape_mem;
			bytecount = ((bmhd.width+15)/8) & 0xfffe;

			ReadLength();

			Read(myfile,packdata,blocklength);

			plane0 = (char *)(bitmap->Planes[0])+bitoffset;
			plane1 = (char *)(bitmap->Planes[1])+bitoffset;
			plane2 = (char *)(bitmap->Planes[2])+bitoffset;
			plane3 = (char *)(bitmap->Planes[3])+bitoffset;
			plane4 = (char *)(bitmap->Planes[4])+bitoffset;
			compress = bmhd.compression;
			for (i=0; i<bmhd.height; i++)
			{	if (bitmap->Depth > 0) unpack_line(plane0); plane0+=bitmap->BytesPerRow;
				if (bitmap->Depth > 1) unpack_line(plane1); plane1+=bitmap->BytesPerRow;
				if (bitmap->Depth > 2) unpack_line(plane2); plane2+=bitmap->BytesPerRow;
				if (bitmap->Depth > 3) unpack_line(plane3); plane3+=bitmap->BytesPerRow;
				if (bitmap->Depth > 4) unpack_line(plane4); plane4+=bitmap->BytesPerRow;
			}
			break;
		}
		else { Close(myfile); return FALSE; }
	}
	Close(myfile);
	return TRUE;
}

#ifdef blarg
erasebrush(bitmap,x,y) struct BitMap *bitmap; short x,y;
{	int bitoffset = (x + 40*y);
	int i;
	bytecount = (bmhd.width+7)/8;

	plane0 = (char *)(bitmap->Planes[0])+bitoffset;
	plane1 = (char *)(bitmap->Planes[1])+bitoffset;
	plane2 = (char *)(bitmap->Planes[2])+bitoffset;
	plane3 = (char *)(bitmap->Planes[3])+bitoffset;
	plane4 = (char *)(bitmap->Planes[4])+bitoffset;
	for (i=0; i<bmhd.height; i++)
	{	if (bitmap->Depth > 0) erase_line(plane0); plane0 += 40;
		if (bitmap->Depth > 1) erase_line(plane1); plane1 += 40;
		if (bitmap->Depth > 2) erase_line(plane2); plane2 += 40;
		if (bitmap->Depth > 3) erase_line(plane3); plane3 += 40;
		if (bitmap->Depth > 4) erase_line(plane4); plane4 += 40;
	}
}

erase_line(dest) char *dest;
{	int j;
	for (j=0; j<bytecount; j++) *dest++ = 0;
}
#endif

/* unpack_line(dest) char *dest;
{	short j, upc;
	if (bmhd.compression == 0) unpack_line1(dest); else unpack_line2(dest);
}
*/

/* unpack_line(dest) char *dest;
{	short j, upc;
	if (bmhd.compression == 0)
	{	for (j=0; j<bytecount; j++) *dest++ = *packdata++; }
	else for (j=0; j<bytecount; )
	{	upc = *packdata++;
		if (upc >= 0)
		{	upc +=1; j += upc;
			while (upc--) *dest++ = *packdata++;
		}
		else if (upc != -128)
		{	upc = 1-upc; j += upc;
			while (upc--) *dest++ = *packdata;
			packdata++;
		}
	}
}	
*/

/* fade_map(level) int level;
{	unsigned red, green, blue, i;
	for (i=0; i < 32; i++)
	{	red   = (level * cmap.colors[i][0])/(16*160);
		green = (level * cmap.colors[i][1])/(16*160);
		blue  = (level * cmap.colors[i][2])/(16*160);
		SetRGB4(vp,i,red,green,blue);
	}
}

low_fade(level) int level;
{	unsigned red, green, blue, i;
	for (i=0; i < 16; i++)
	{	red   = (level * cmap.colors[i][0])/(16*160);
		green = (level * cmap.colors[i][1])/(16*160);
		blue  = (level * cmap.colors[i][2])/(16*160);
		SetRGB4(vp,i,red,green,blue);
		SetRGB4(vp,i+16,red,green,blue);
	}
}

high_fade(level) int level;
{	unsigned red, green, blue, i;
	for (i=0; i < 16; i++)
	{	red   = 15 - (level * (255 - cmap.colors[i][0]) ) / (16*160);
		green = 15 - (level * (255 - cmap.colors[i][1]) ) / (16*160);
		blue  = 15 - (level * (255 - cmap.colors[i][2]) ) / (16*160);
		SetRGB4(vp,i+16,red,green,blue);
	}
}

*/
