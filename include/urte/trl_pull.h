/*
 * urte/trl_pull.h - TRL Pull scheduling extension (spec ch. 6).
 *
 * Layered on POSIX scheduling (sched_setscheduler(2), SCHED_FIFO/RR/OTHER).
 * A high-maturity (clinical) process notifies the kernel of generated
 * revenue/data; the scheduler boosts priority and reserves bandwidth for the
 * lower-TRL modules that depend on it. POSIX scheduling of the caller itself
 * is never altered.
 */
#ifndef URTE_TRL_PULL_H
#define URTE_TRL_PULL_H

#include <urte/types.h>

#ifdef __cplusplus
extern "C" {
#endif

#define URTE_TRL_REVENUE  0x01
#define URTE_TRL_DATA     0x02

/*
 * Notify the scheduler that source_pid produced revenue/data units.
 * Returns 0 on success; -1/errno (ESRCH, EPERM).
 */
int urte_trl_pull_notify(pid_t source_pid,
                         uint64_t revenue_or_data_units,
                         uint32_t flags);

/* Register dependent as accelerated when source advances. 0 / -1+errno. */
int urte_trl_pull_link(pid_t source_pid, pid_t dependent_pid);

#ifdef __cplusplus
}
#endif

#endif /* URTE_TRL_PULL_H */
