# Cargo, mise and Aube decision

Updated 2026-09-30.

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

Formatting, lint, 108 core tests, three preview tests and preview CI passed at that historical checkpoint. See [the current status](quick-switch-status.md) for software validation and the open manual gates. Preserve existing task entrypoints; reassess Aube only for a real JavaScript workload if one is separately proposed.

## Development commands

2026-09-30: add three reusable entrypoints to avoid skipping editor checks or
Windows-only code during Linux/WSL development. Existing CI tasks and tool pins
remain unchanged. Cargo/rustfmt/Clippy already cover Rust formatting and linting;
no additional formatter, dependency manager or global hook is needed.

| Purpose | Command | Scope |
| --- | --- | --- |
| Format Rust while editing | `mise run fmt` | Changes source formatting |
| Quick core checks | `mise run check` | Formatting verification, strict host Clippy, core tests |
| Full Linux/WSL software checks | `mise run verify-linux` | Core CI/build/CLI help, then headless editor lint/tests |
| Windows-only code lint from Linux/WSL | `mise run lint-windows-cross` | xwin Clippy for default and combined preview features |
| All Windows build checks from Linux/WSL | `mise run verify-windows-cross` | Formatting, cross-lint, production/native-preview/editor release builds |
| Headless checks on native Windows | `mise run ci-windows`, then `mise run ci-windows-preview` | Native Windows compilation/tests; select the PowerShell environment below |
| Workflow/security changes | `mise run security` | actionlint, fully redacted history secret scan, zizmor |
| Validate task definitions | `mise tasks validate` | Checks task references and configuration |

Run `verify-linux` and `verify-windows-cross` separately, in that order, for a
Linux/WSL pre-PR pass. Aggregates use sequential task calls, including xwin builds:
concurrent xwin tool setup previously failed with a shared symlink race. Build
outputs stay under repository `target/`; the native preview build also copies
its assets there. Checks never install or launch the app, open a GUI, enumerate
devices, change startup, or run hardware writes. `security` is separate because
it scans full history and needs its pinned Linux tools.

Host Clippy on Linux does not inspect `cfg(windows)` code. Cross-lint therefore
checks default and preview feature combinations separately and keeps the existing
Windows policy: compiler/default Clippy warnings denied, pedantic warnings
temporarily allowed. Cross-builds do not execute Windows tests or establish
UI/accessibility, hotkey, performance or mouse behavior.

With tools already prepared, disable automatic installation for **both** task
execution and nested `mise exec` using process-scoped
`MISE_TASK_RUN_AUTO_INSTALL=false MISE_EXEC_AUTO_INSTALL=false`. An outer
`--skip-tools` alone does not govern nested `mise exec`. Keep setup explicit:
`bootstrap` installs tools/targets and is not part of verification. Interactive
preview/measurement tasks require the owner-present Windows session; installation
and production tray gates remain blocked. See [manual handoff](manual-handoff.md).

For native Windows checks with Rust and mise already installed, match hosted CI
in the current PowerShell process (these assignments do not persist):

```powershell
$env:MISE_ENV = 'windows'
$env:MISE_ENABLE_TOOLS = ''
$env:MISE_TASK_RUN_AUTO_INSTALL = 'false'
$env:MISE_EXEC_AUTO_INSTALL = 'false'
mise run ci-windows
mise run ci-windows-preview
```

`MISE_ENV=windows` selects `mise.windows.toml`'s PowerShell task shell. The empty
tool selection prevents these native checks from installing the Linux/WSL cross
toolchain. Do not use that empty selection for xwin tasks, which need the pinned
cross-build tools.

Validation on 2026-09-30: both new verification aggregates passed on Linux/WSL
with automatic installation disabled. Core checks ran 129 library, 2 CLI, 3
capability and 6 catalog tests; preview checks ran 135 library, 11 editor, 2 CLI,
3 capability and 6 catalog tests. Both Windows cross-lint configurations and all
three release builds passed. All 33 locally discovered tasks validated with no
errors or warnings (including five global tasks); no GUI or device test ran.

## Sources

- https://aube.sh/guide.html
- https://aube.sh/package-manager/lockfiles.html
- https://aube.sh/embedding/rust.html
- https://doc.rust-lang.org/cargo/guide/dependencies.html
