/* The camera portal bridge of a build without its system libraries (GIO 2.56, PipeWire 0.3): no
 * portal is present and no request starts, so the camera reports itself unavailable instead of the
 * build failing. build.rs compiles it and prints a warning when pkg-config does not find one. */
#include <stddef.h>
#include <stdint.h>

void *ferro_linux_portal_begin(void) { return NULL; }
int ferro_linux_portal_poll(void *r, int32_t *fd, uint32_t *node, char *error, uint32_t capacity) {
    (void)r; (void)node;
    if (fd != NULL) *fd = -1;
    if (error != NULL && capacity > 0) error[0] = '\0';
    return 3;
}
void ferro_linux_portal_release(void *r) { (void)r; }
int ferro_linux_portal_present(void) { return 0; }
