/* fmain.c */
int open_all(void);
int close_all(void);
int read_sample(void);
int doorfind(int x, int y, unsigned long keytype);
void main(int argc,char argv[]);
int xfer(int xtest, int ytest, int flag);
int find_place(int flag);
int load_actors(void);
int set_encounter(int i, int spread);
int checkdead(long i, long dtype);
int load_carrier(int n);
int revive(int new);
int screen_size(long x);
int setmood(int now);
int gen_mini(void);
int pagechange(void);
int add_device(void);
int wrap_device(void);
int print_options(void);
int propt(int j, int pena);
int do_option(int hit);
int get_turtle(void);
int gomenu(int mode);
int set_options(void);
int load_all(void);
int load_new_region(void);
int effect(int num, long speed);
int mod1save(void);
