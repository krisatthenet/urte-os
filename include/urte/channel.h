/*
 * urte/channel.h - Typed, scale-aware IPC channels (spec ch. 5).
 *
 * URTE Channels are a thin extension over POSIX message queues (mq_overview(7)):
 * every message carries a scale level and is guardrail-checked on send. The
 * naming and oflag semantics mirror mq_open(3) (O_CREAT, O_RDWR, O_NONBLOCK).
 */
#ifndef URTE_CHANNEL_H
#define URTE_CHANNEL_H

#include <urte/types.h>
#include <fcntl.h>   /* O_CREAT, O_RDWR, O_NONBLOCK */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Guardrail mask bits applied on every send (spec ch. 7). A zero mask disables
 * per-message checking. Channel names must follow POSIX mq naming: a leading
 * '/' followed by up to NAME_MAX characters and no further '/' (mq_overview(7)).
 * Link the implementation with -lrt (and -lpthread for the registry lock).
 */
#define URTE_GR_NONE        0x0000u
#define URTE_GR_SCALE       0x0001u  /* reject messages above channel scale   */
#define URTE_GR_TRL         0x0002u  /* reject senders below required_trl     */
#define URTE_GR_SIZE        0x0004u  /* reject messages exceeding msgsize     */
#define URTE_GR_ALL         (URTE_GR_SCALE | URTE_GR_TRL | URTE_GR_SIZE)

struct urte_channel_attr {
    urte_scale_level_t scale_level;
    urte_trl_t         required_trl;     /* min TRL of sender */
    uint32_t           guardrail_mask;   /* checks applied per message */
    long               maxmsg;           /* as POSIX mq_attr.mq_maxmsg */
    long               msgsize;          /* as POSIX mq_attr.mq_msgsize */
};

/* Open/create a channel. Returns POSIX-usable descriptor, or -1/errno. */
urte_fd_t urte_channel_open(const char *name, int oflag,
                            const struct urte_channel_attr *attr);

/*
 * Send a message. The kernel runs the guardrail mask before enqueueing.
 * Returns 0 on success; -1 with errno (EAGAIN, EPERM, EACCES, EMSGSIZE).
 */
int urte_channel_send(urte_fd_t ch, const void *msg, size_t len,
                      unsigned int prio);

/* Receive a message; semantics follow mq_receive(3). */
ssize_t urte_channel_recv(urte_fd_t ch, void *buf, size_t len,
                          unsigned int *prio);

int urte_channel_close(urte_fd_t ch);   /* equivalent to close(2) */

#ifdef __cplusplus
}
#endif

#endif /* URTE_CHANNEL_H */
