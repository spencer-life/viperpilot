# Cargo, mise and Aube decision

Updated 2026-09-29.

## Decision

Keep Cargo for Rust dependencies/builds, rustup for the stable Rust toolchain, and the existing mise task entrypoints. Aube was assessed and not adopted. No Node frontend, package manifest, lockfile, workspace or runtime requirement was added.

The repository contains `Cargo.toml`/`Cargo.lock`, native Windows code and optional Rust UI previews. It has no Node package installation or npm lifecycle scripts to migrate. Its Cargo dependency build scripts remain managed by Cargo, not Aube's JavaScript build approvals. CI already calls the repository mise tasks.

## Assessment and limits

At public source checkpoint `7cb0440`, two isolated Aube 2.6.0 frozen-install attempts failed because no Node manifest/workspace existed. Those rejected commands are not install-speed benchmarks.

The specific Rust guide https://aube.sh/embedding/rust.html was reviewed. `aube::embed` lets a Rust host manage JavaScript dependencies and commands. It requires host runtime, initialization, output/cancellation and lifecycle/runtime integration. It does not provide mouse-profile persistence, HID/device protocol or native UI. The embedded library was **not added, compiled, linked or benchmarked**; the CLI failures do not establish its Windows library compatibility, binary size or runtime overhead.

For the existing Cargo workflow, isolated Linux wall-time measurements were:

| Workflow | Cold/first run | Warm median (three runs) |
| --- | ---: | ---: |
| Locked dependency fetch | 31.429 s | 0.163 s |
| Native release build | 4.167 s | 0.180 s |
| Core tests | 11.688 s | 0.217 s |
| Headless egui preview tests | 24.789 s | 0.287 s |

Cold fetch used an empty private Cargo cache; first builds/tests used separate empty output directories with dependencies already fetched. Rust/Cargo 1.98.1 and mise 2026.9.17 were used with six compilation jobs. No compiler-cache wrapper was active. The OS page cache and toolchain were not flushed. These are small local samples, not an Aube comparison, Windows UI timing, production tray performance or mouse validation. Detailed private-machine logs remain outside this sanitized repository.

Formatting, lint, 108 core tests, three preview tests and preview CI passed. See [the current status](quick-switch-status.md) for software validation and the open manual gates. Keep task entrypoints unchanged; reassess Aube only for a real JavaScript workload if one is separately proposed.

## Sources

- https://aube.sh/guide.html
- https://aube.sh/package-manager/lockfiles.html
- https://aube.sh/embedding/rust.html
- https://doc.rust-lang.org/cargo/guide/dependencies.html
