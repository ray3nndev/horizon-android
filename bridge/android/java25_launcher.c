/*
 * Vyron Android Java 25 process launcher.
 *
 * This is a small PIE executable packaged in the APK's native-library directory
 * under a .so suffix. Horizon invokes it as a separate process for each exporter
 * job; it loads the Android-native JRE's libjli.so and forwards Java's argv.
 *
 * This file is original Vyron code and does not copy the Pojav/Amethyst launcher.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef int JLI_Launch_fn(
    int argc, char **argv,
    int jargc, const char **jargv,
    int appclassc, const char **appclassv,
    const char *fullversion,
    const char *dotversion,
    const char *pname,
    const char *lname,
    int javaargs,
    int cpwildcard,
    int javaw,
    int ergo
);

static int fail(const char *message) {
    fprintf(stderr, "Vyron Java 25 launcher: %s\n", message);
    return 127;
}

int main(int argc, char **argv) {
    const char *jre_home = getenv("HORIZON_JRE_HOME");
    const char *jli_path = getenv("HORIZON_JLI_LIBRARY");
    if (!jre_home || !*jre_home || !jli_path || !*jli_path) {
        return fail("HORIZON_JRE_HOME and HORIZON_JLI_LIBRARY must be set");
    }

    char java_path[PATH_MAX];
    if (snprintf(java_path, sizeof(java_path), "%s/bin/java", jre_home) >= (int)sizeof(java_path)) {
        return fail("JRE path is too long");
    }

    char library_path[PATH_MAX * 3];
    int path_len = snprintf(library_path, sizeof(library_path),
        "%s/lib:%s/lib/jli:%s/lib/server", jre_home, jre_home, jre_home);
    if (path_len < 0 || path_len >= (int)sizeof(library_path)) {
        return fail("JRE library path is too long");
    }
    const char *existing = getenv("LD_LIBRARY_PATH");
    if (existing && *existing) {
        size_t used = (size_t)path_len;
        if (used + strlen(existing) + 2 >= sizeof(library_path)) {
            return fail("combined library path is too long");
        }
        library_path[used++] = ':';
        strcpy(library_path + used, existing);
    }
    if (setenv("LD_LIBRARY_PATH", library_path, 1) != 0) {
        return fail("could not set LD_LIBRARY_PATH");
    }

    void *jli = dlopen(jli_path, RTLD_NOW | RTLD_GLOBAL);
    if (!jli) {
        fprintf(stderr, "Vyron Java 25 launcher: dlopen(%s): %s\n", jli_path, dlerror());
        return 127;
    }
    dlerror();
    JLI_Launch_fn *launch = (JLI_Launch_fn *)dlsym(jli, "JLI_Launch");
    const char *symbol_error = dlerror();
    if (symbol_error || !launch) {
        fprintf(stderr, "Vyron Java 25 launcher: JLI_Launch: %s\n",
                symbol_error ? symbol_error : "symbol not found");
        return 127;
    }

    if (argc < 1) return fail("missing argv[0]");
    argv[0] = java_path;
    return launch(argc, argv, 0, NULL, 0, NULL,
                  "25.0.0", "25", "java", "openjdk",
                  0, 1, 0, 0);
}
