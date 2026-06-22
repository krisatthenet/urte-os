/*
 * urte/guardrail.h - Kernel guardrail enforcement interface (spec ch. 7).
 *
 * Guardrail checks are performed IN ADDITION to standard POSIX permission
 * checks (uid/gid, capabilities, LSM). A process that passes POSIX checks may
 * still be denied here. Guardrails cannot be bypassed from user space.
 */
#ifndef URTE_GUARDRAIL_H
#define URTE_GUARDRAIL_H

#include <urte/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Categories of intervention subject to guardrail review. */
typedef enum urte_intervention_kind {
    URTE_IV_OBSERVE = 0,     /* read-only, no actuation        */
    URTE_IV_SIMULATE,        /* digital twin only              */
    URTE_IV_ACTUATE,         /* physical deployment / swarm    */
    URTE_IV_RELEASE          /* irreversible environmental act */
} urte_intervention_kind_t;

struct urte_intervention_request {
    urte_intervention_kind_t kind;
    urte_scale_level_t       scale;
    double                   magnitude;     /* normalized 0..1 */
    const char              *location;      /* free-form locus */
    const void              *payload;
    size_t                   payload_len;
};

struct urte_guardrail_result {
    urte_decision_t decision;
    int             reason_code;            /* planetary-boundary / ethics id */
    char            reason[128];            /* human-readable explanation     */
    bool            ethics_board_required;
};

/*
 * Synchronously evaluate a proposed intervention against the policies bound to
 * the target state vector. Returns 0 when the evaluation completes (decision is
 * in *result); -1/errno (EBADF, EINVAL, EACCES) when it cannot be performed.
 */
int urte_guardrail_check(urte_fd_t sv_fd,
                         const struct urte_intervention_request *req,
                         struct urte_guardrail_result *result);

/* Emergency pause of physical deployment (kill-switch path, spec ch. 7). */
int urte_intervention_pause(urte_fd_t sv_fd, int flags);

#ifdef __cplusplus
}
#endif

#endif /* URTE_GUARDRAIL_H */
