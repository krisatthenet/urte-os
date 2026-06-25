# URTE OS Core

<img width="1168" height="784" alt="Myk5M" src="https://github.com/user-attachments/assets/b485a1ec-814f-41a6-9cde-f9900a378aa8" />


[![CI](https://github.com/krisatthenet/urte-os/actions/workflows/ci.yml/badge.svg)](https://github.com/krisatthenet/urte-os/actions/workflows/ci.yml)
[![Repo](https://img.shields.io/badge/GitHub-krisatthenet%2Furte--os-181717?logo=github)](https://github.com/krisatthenet/urte-os)
[![Last commit](https://img.shields.io/github/last-commit/krisatthenet/urte-os)](https://github.com/krisatthenet/urte-os/commits/master)
[![Rust](https://img.shields.io/badge/Rust-1.96-000000?logo=rust)](kernel-rs/)
[![POSIX](https://img.shields.io/badge/POSIX-IEEE%20Std%201003.1--2024-0033cc)](docs/URTE_POSIX_Kernel_Technical_Specification.pdf)
[![License: CC BY-NC-ND 4.0](https://img.shields.io/badge/License-CC%20BY--NC--ND%204.0-lightgrey)](LICENSE)

POSIX-conformant core of the operating system for the **Universal Regeneration
Therapy Ecosystem (URTE)**. The interface and structure follow
`URTE_POSIX_Kernel_Technical_Specification` (IEEE Std 1003.1-2024) and are
generated from the `urtecore` Capella MBSE model.

## Source of truth

> **Note:** the original `urtecore.json` in the project root is **not** model
> data — it is a saved HTML page from an online XML→JSON converter. The real,
> extracted model lives here: [`model/urtecore.model.json`](model/urtecore.model.json).

## Layout

```
urte-os/
├── model/urtecore.model.json   Clean JSON extracted from urtecore.capella
├── include/urte/               POSIX-conformant interface (headers)
│   ├── types.h                 Base types, scale levels, pipeline stages
│   ├── process.h               Extended PCB / process roles
│   ├── state_vector.h          Multi-scale state vectors (POSIX shm-backed)
│   ├── guardrail.h             Kernel guardrail enforcement
│   ├── channel.h               Typed scale-aware IPC (POSIX mq-backed)
│   ├── trl_pull.h              TRL Pull scheduling extension
│   └── syscalls.h              Single aggregate include point
├── kernel/
│   ├── urte_core.c             Core skeleton + boot/init + subsystems
│   └── urte_demo.c             Self-test / usage example
└── Makefile
```

## POSIX compliance model

- All standard POSIX interfaces behave exactly as specified; URTE additions are
  **additive and optional**.
- Every URTE call follows POSIX conventions: returns `-1` and sets `errno`
  (`EINVAL`, `ENOMEM`, `EPERM`, `EBADF`, `ESRCH`, `ENOSYS`, …).
- Handles returned by URTE calls are real file descriptors, usable with
  `mmap(2)`, `fstat(2)`, `close(2)`.
- When the `urte_core` module is not loaded the system is 100% POSIX
  conformant; `urte_core_available()` reports its status.

## Model → OS mapping

| Capella model element                         | OS core construct |
|-----------------------------------------------|-------------------|
| Operational Entity hierarchy (Ecosystem→AI core) | `urte_scale_level_t` (`types.h`) |
| OA activity chain (Sensing → … → therapy delivery mitigation) | `urte_stage_t` + `k_pipeline[]` (`urte_core.c`) |
| Fabrication flow (Crawling→…→Dissemination)   | actuation path behind guardrails |
| System / Logical / Physical components        | subsystem init (`urte_core_init`) |
| EPBS Configuration Item "System" (SystemCI)   | bootable core image |
| Human actors (Biomathematician, Ethics, …)    | `urte_proc_role_t`, Ethics Board veto |

## Build

```sh
make            # builds urte_demo against include/urte
make test       # runs the demo (prints scale levels + pipeline + guardrail demo)
make clean
```

Requires a C11 compiler (`gcc`/`clang`) and a POSIX environment with POSIX
message queues — **Linux or WSL** (links `-lrt -lpthread`). macOS does not
implement POSIX message queues, and native Windows has none; use WSL there.

## Status

Interface layer: **complete**.

- **IPC channels** (`urte_channel_*`): **implemented** over POSIX message queues
  (`mq_open`/`mq_send`/`mq_receive`/`mq_close`). A per-channel registry keeps the
  scale/TRL/guardrail metadata that the queue itself does not carry, and the
  guardrail mask (`URTE_GR_SIZE`/`URTE_GR_TRL`/`URTE_GR_SCALE`) is enforced on
  every send. Channel names follow POSIX mq naming (leading `/`).
- Remaining subsystem mechanics (scheduling, state-vector memory mapping,
  LSM/guardrail hooks) are stubbed with `ENOSYS`/`TODO` and ready for
  kernel-side implementation.

## License

Copyright © 2026 Kristupas Gelžinis and Olek Suchodolski. All rights reserved except as granted below.

This work — source code, models, and documentation — is licensed under the
**Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International**
license (CC BY-NC-ND 4.0). You may share it with attribution, but **commercial
use** and **distribution of modified versions** are not permitted without
explicit written permission. Full text in [`LICENSE`](LICENSE); summary at
<https://creativecommons.org/licenses/by-nc-nd/4.0/>.
