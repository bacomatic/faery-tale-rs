/* fmain2.c */
int dohit(long i, long j, long fc, int wt);
int aftermath(void);
int proxcheck(int x, int y, int i);
int nearest_fig(int constraint, int dist);
int calc_dist(long a, long b);
int move_figure(int fig, int dir, int dist);
int drawcompass(int dir);
int fade_page(int r, int g, int b, int limit, unsigned short *colors);
int colorplay(void);
int ppick(void);
int print(unsigned char *str);
int print_cont(unsigned char *str);
int extract(unsigned char *start);
int announce_container(unsigned char *s);
int announce_treasure(unsigned char *s);
int name(void);
int map_message(void);
int message_off(void);
int fade_down(void);
int fade_normal(void);
int stillscreen(void);
int shape_read(void);
int read_shapes(long num);
int load_track_range(int f_block, int b_count, void *buffer, int dr);
int motor_off(void);
int seekn(void);
int prep(int slot);
int load_next(void);
int read_score(void);
int copypage(unsigned char *br1, unsigned char *br2, int x, int y);
int flipscan(void);
int skipint(void);
unsigned char *into_chip(unsigned char *oldpointer, long size);
int witch_fx(struct fpage *fp);
int do_objects(void);
int leave_item(int i, int object);
int change_object(long id, long flag);
int set_objects(struct object *list, int length, long f);
int copy_protect_junk(void);
int locktest(unsigned char *name, long access);
int cpytest(void);
int waitnewdisk(void);
int savegame(int hit);
int saveload(unsigned char *buffer, long length);
int move_extent(int e, int x, int y);
int rescue(void);
int win_colors(void);
int day_fade(void);
int do_tactic(long i, long tactic);
int eat(int amt);
int set_loc(void);
