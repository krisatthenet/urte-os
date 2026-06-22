/*
 * urte/syscalls.h - Aggregated URTE OS core interface.
 *
 * Single include point for the optional URTE extensions provided by the
 * urte_core module. When the module is not loaded the system is fully POSIX
 * conformant; including this header alone changes nothing until the calls are
 * actually invoked. Every call follows POSIX conventions: -1 return + errno.
 */
#ifndef URTE_SYSCALLS_H
#define URTE_SYSCALLS_H

#include <urte/types.h>
#include <urte/process.h>
#include <urte/state_vector.h>
#include <urte/guardrail.h>
#include <urte/channel.h>
#include <urte/trl_pull.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Library/module version. */
#define URTE_CORE_VERSION_MAJOR 1
#define URTE_CORE_VERSION_MINOR 0

/* Returns true if the urte_core module is loaded and extensions are live. */
bool urte_core_available(void);

#ifdef __cplusplus
}
#endif

#endif /* URTE_SYSCALLS_H */
