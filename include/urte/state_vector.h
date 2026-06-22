/*
 * urte/state_vector.h - Multi-scale state vector interface (spec ch. 4).
 *
 * State vectors are exposed to user space as POSIX shared memory: the urte_fd_t
 * returned by urte_state_vector_create() is a valid POSIX file descriptor and
 * may be passed to mmap(2), fstat(2) and close(2) unchanged.
 */
#ifndef URTE_STATE_VECTOR_H
#define URTE_STATE_VECTOR_H

#include <urte/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Creation flags. */
#define URTE_SV_SHARED            0x0001  /* mappable by authorized processes  */
#define URTE_SV_GUARDRAIL_ENFORCED 0x0002 /* every write checked by guardrails */

/* Resilience / tipping-point metadata (spec: R = -lambda_dom). */
struct urte_resilience {
    double lambda_dom;       /* dominant eigenvalue; resilience R = -lambda_dom */
    double variance;         /* early-warning: rising variance                  */
    double lag1_autocorr;    /* early-warning: critical slowing down            */
};

/* Attributes for state vector creation. */
struct urte_state_vector_attr {
    urte_scale_level_t level;
    size_t             dimension;     /* element count of the state vector */
    struct urte_resilience resilience;
    urte_fd_t          guardrail_policy; /* policy handle, or -1 for default */
    const char        *name;          /* optional; NULL => anonymous        */
};

/*
 * Create a multi-scale state vector region. Returns a POSIX-usable descriptor
 * on success, or -1 with errno set (EINVAL, ENOMEM, EPERM, EEXIST).
 */
urte_fd_t urte_state_vector_create(const struct urte_state_vector_attr *attr,
                                   int flags);

/* Read current resilience metadata. 0 on success, -1/errno on failure. */
int urte_resilience_query(urte_fd_t sv_fd, struct urte_resilience *out);

/*
 * Request a controlled cross-scale transition (e.g. TISSUE -> ECOSYSTEM).
 * Validates coupling constraints and guardrails. Errors: EINVAL, EPERM, EBUSY.
 */
struct urte_transition_attr {
    double coupling_strength;
    int    validation_mode;
};
int urte_scale_transition(urte_fd_t sv_fd,
                          urte_scale_level_t new_level,
                          const struct urte_transition_attr *attr);

#ifdef __cplusplus
}
#endif

#endif /* URTE_STATE_VECTOR_H */
