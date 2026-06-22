/*
 * urte/types.h - Base types for the URTE OS core.
 *
 * Conforms to IEEE Std 1003.1-2024 (POSIX.1). All URTE additions are layered
 * on top of standard POSIX types and never redefine them. Standard processes
 * that do not include this header see a 100% POSIX-conformant system.
 *
 * Derived from the urtecore Capella MBSE model (see model/urtecore.model.json).
 */
#ifndef URTE_TYPES_H
#define URTE_TYPES_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <sys/types.h>   /* POSIX: pid_t, uid_t, gid_t, size_t, ssize_t */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Scale levels (model: Operational Entity hierarchy Ecosystem -> ... -> AI core).
 * Numbering matches the Technical Specification: 0 = Molecular ... 6 = Planetary.
 */
typedef enum urte_scale_level {
    URTE_SCALE_MOLECULAR = 0,
    URTE_SCALE_CELLULAR  = 1,
    URTE_SCALE_TISSUE    = 2,
    URTE_SCALE_ORGAN     = 3,
    URTE_SCALE_ECOSYSTEM = 4,
    URTE_SCALE_BIOME     = 5,
    URTE_SCALE_PLANETARY = 6,
    URTE_SCALE__COUNT
} urte_scale_level_t;

/* Technology Readiness Level (TRL Pull scheduling, spec ch. 6). */
typedef uint8_t  urte_trl_t;        /* 1..9 per ISO 16290 */

/* Opaque handle returned by URTE syscalls; usable with POSIX mmap/close/fstat. */
typedef int      urte_fd_t;

/* Guardrail decision codes (spec ch. 7). */
typedef enum urte_decision {
    URTE_DECISION_ALLOW          = 0,
    URTE_DECISION_DENY           = 1,
    URTE_DECISION_ETHICS_VETO    = 2,   /* Ethics Board veto required */
    URTE_DECISION_PAUSE_REQUIRED = 3
} urte_decision_t;

/* Functional stages of the model's operational pipeline (OA activities). */
typedef enum urte_stage {
    URTE_STAGE_SENSING = 0,
    URTE_STAGE_DATA_GATHERING,
    URTE_STAGE_MEMORIZATION,
    URTE_STAGE_STRING_COMPARISON,
    URTE_STAGE_OPERATIONAL_STATUS_CHECK,
    URTE_STAGE_HYPOTHESIS_STATEMENT,
    URTE_STAGE_MEASURE_DEFINITION,
    URTE_STAGE_MEASURE_GATHERING,
    URTE_STAGE_MEASURE_COMPOSE,
    URTE_STAGE_MEASURE_SELECTOR_CHECK,
    URTE_STAGE_REACTOR_THERAPY_ASSUMPTION,
    URTE_STAGE_THERAPY_DELIVERY_MITIGATION,
    URTE_STAGE__COUNT
} urte_stage_t;

#ifdef __cplusplus
}
#endif

#endif /* URTE_TYPES_H */
