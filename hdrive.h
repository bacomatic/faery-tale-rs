/* hdrive.c */
int AllocDiskIO(void);
void FreeDiskIO(void);
void WaitDiskIO(int num);
void InvalidDiskIO(int num);
int CheckDiskIO(int num);
int IsReadDiskIO(int num);
void WaitLastDiskIO(void);
void InvalidLastDiskIO(void);
int CheckLastDiskIO(void);
int IsReadLastDiskIO(void);
void load_track_range(short f_block, short b_count, APTR buffer, short dr);
void motor_off(void);
BOOL IsHardDrive(void);
