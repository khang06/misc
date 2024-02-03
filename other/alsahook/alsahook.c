#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>

#define PERIOD_TIME_US 1333 // 1.333ms or 64/48000
#define PERIOD_COUNT 2

typedef struct _snd_pcm_t snd_pcm_t;
typedef struct _snd_pcm_hw_params_t snd_pcm_hw_params_t;

#define RESOLVE_SYM(x)  if (!RESOLVE_SYM_HIDDEN(x)) { \
                            RESOLVE_SYM_HIDDEN(x) = dlsym(RTLD_NEXT, #x); \
                            if (!RESOLVE_SYM_HIDDEN(x)) { \
                                fprintf(stderr, "alsahook: failed to resolve"#x"\n"); \
                                exit(1); \
                            } \
                        }
#define RESOLVE_SYM_HIDDEN(x) orig_##x

static int(*orig_snd_pcm_hw_params_set_period_time_near)(snd_pcm_t *pcm, snd_pcm_hw_params_t *params, unsigned int *val, int *dir) = 0;
int snd_pcm_hw_params_set_period_time_near(snd_pcm_t *pcm, snd_pcm_hw_params_t *params, unsigned int *val, int *dir) {
    RESOLVE_SYM(snd_pcm_hw_params_set_period_time_near)
    fprintf(stderr, "alsahook: intercepting snd_pcm_hw_params_set_period_time_near (orig %u, new %u)\n", *val, PERIOD_TIME_US);
    *val = PERIOD_TIME_US;
    return orig_snd_pcm_hw_params_set_period_time_near(pcm, params, val, dir);
}

static int(*orig_snd_pcm_hw_params_set_periods_near)(snd_pcm_t *pcm, snd_pcm_hw_params_t *params, unsigned int *val, int *dir) = 0;
int snd_pcm_hw_params_set_periods_near(snd_pcm_t *pcm, snd_pcm_hw_params_t *params, unsigned int *val, int *dir) {
    RESOLVE_SYM(snd_pcm_hw_params_set_periods_near)
    fprintf(stderr, "alsahook: intercepting snd_pcm_hw_params_set_periods_near (orig %u, new %u)\n", *val, PERIOD_COUNT);
    *val = PERIOD_COUNT;
    return orig_snd_pcm_hw_params_set_periods_near(pcm, params, val, dir);
}
