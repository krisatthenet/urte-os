/*
 * urte/process.h - Extended process model (spec ch. 3).
 *
 * The URTE Process Control Block embeds the standard POSIX task fields and adds
 * scale/TRL/guardrail metadata. Standard POSIX process semantics (fork, execve,
 * wait, signals) are unchanged; the extension fields default to inert values.
 */
#ifndef URTE_PROCESS_H
#define URTE_PROCESS_H

#include <urte/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/* URTE process roles (spec: Regen Agent / Digital Twin / Guardrail Monitor). */
typedef enum urte_proc_role {
    URTE_ROLE_POSIX = 0,        /* ordinary POSIX process (default) */
    URTE_ROLE_REGEN_AGENT,
    URTE_ROLE_DIGITAL_TWIN,
    URTE_ROLE_GUARDRAIL_MONITOR
} urte_proc_role_t;

/* Public, read-only view of the URTE-specific PCB extension. */
struct urte_proc_info {
    pid_t              pid;
    pid_t              ppid;
    urte_proc_role_t   role;
    urte_scale_level_t scale_level;
    urte_trl_t         trl_level;
    bool               is_trl_pull_source;  /* clinical revenue generator */
    urte_fd_t          state_vector;        /* associated u_s, or -1 */
};

/* Promote the calling POSIX process to a URTE role. 0 / -1+errno (EPERM). */
int urte_proc_set_role(urte_proc_role_t role,
                       urte_scale_level_t scale,
                       urte_trl_t trl);

/* Query URTE metadata for a pid. 0 / -1+errno (ESRCH). */
int urte_proc_info(pid_t pid, struct urte_proc_info *out);

#ifdef __cplusplus
}
#endif

#endif /* URTE_PROCESS_H */
