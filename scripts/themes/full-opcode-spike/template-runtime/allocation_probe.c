// macOS malloc interposition used only in allocation-count runs. The timing binaries
// execute without this library. Totals apply to the marked interval on the probe thread.
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
typedef struct { uint64_t allocations, reallocations, frees, requested_bytes; } Counts;
static _Thread_local int enabled;
static _Thread_local Counts counts;
void template_probe_begin(void) { counts = (Counts){0}; enabled = 1; }
void template_probe_end(Counts *result) { enabled = 0; *result = counts; }
static void *count_malloc(size_t n) {
    void *p = malloc(n);
    if (enabled && p) { counts.allocations++; counts.requested_bytes += n; }
    return p;
}
static void *count_calloc(size_t n, size_t width) {
    void *p = calloc(n, width);
    if (enabled && p) { counts.allocations++; counts.requested_bytes += n * width; }
    return p;
}
static void *count_realloc(void *old, size_t n) {
    void *p = realloc(old,n);
    if (enabled && p) { counts.reallocations++; counts.requested_bytes += n; }
    return p;
}
static void count_free(void *p) { if (enabled && p) counts.frees++; free(p); }
#define WRAP(replacement, original) \
    __attribute__((used,section("__DATA,__interpose"))) static const struct {const void *new_fn, *old_fn;} wrap_##original = {replacement,original};
WRAP(count_malloc,malloc)
WRAP(count_calloc,calloc)
WRAP(count_realloc,realloc)
WRAP(count_free,free)
