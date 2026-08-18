# kOS

kOS är Kabootars **enda operativsystem**: kernel (VFS, process, sched, mem) och skrivbord (shell, fönster, Explorer) i `lib/kos/`.

OS:et **skrivs** i Kabootar (`.kab`). Det **kör** appar i **vilket språk som helst** som landar i **Kv8 + kDOM + kstyle**. Kabootar är ett gästspråk bland flera.

Plan: [ROADMAP.md](ROADMAP.md). Syscalls/host: [docs/OS.md](../../docs/OS.md). Läsare: [../kbrowser/README.md](../kbrowser/README.md).

## Kontrakt

```
Gästspråk  →  Kv8 (script) + kDOM (träd/events) + kstyle (layout)
                    ↓
              kOS-fönster, VFS, Start, input
                    ↓
              kbrowser paint  →  compositor (host)
```

Inget OS-API ska kräva att appen är `.kab`. `os_*` är host-syscalls; produktpolicy ligger i `import "kos/…"`.

## Moduler

```kabootar
import "kos"              // aggregator → vfs
import "kos/vfs"          // read, write, exists, list, mkdir, remove, stat
import "kos/vfs_policy"   // /apps-sandbox: canWrite, writeFile, ensureAppsRoot
import "kos/mount"        // mount, unmount, mounts
import "kos/async"        // readAsync, writeAsync, awaitAll
import "kos/sched"        // enqueue, tick, schedYield, preempt
import "kos/sched_policy" // runFairTick, runRoundRobin
import "kos/process"      // spawn, list
import "kos/process_policy" // spawnSandbox, spawnAppsWorker, canKill
import "kos/kernel"       // info, caps
import "kos/mem"          // @manual MemBox (owned_*)
import "kos/display_buf"  // @manual framebuffer
import "kos/shell"        // skrivbord, Start, bootKosDesktop
import "kos/boot"         // bootKosSession, handleShellClick
import "kos/launch"       // launchApp, drainKosEvents
import "kos/windows"      // openWindow, snapWindow, Alt+Tab
import "kos/explorer"     // VFS-utforskare
import "kos/theme"        // applyKosTheme
```

## Kärna

```kabootar
import "kos/vfs"
import "kos/process"
import "kos/kernel"

mkdir("/tmp")
write("/tmp/hello.txt", "kOS")
read("/tmp/hello.txt")
let pid = spawn("worker")
info()
caps()
```

Policy (Kab, inte Rust):

```kabootar
import "kos/vfs_policy"
import "kos/sched_policy"
import "kos/process_policy"

ensureAppsRoot()
writeFile("/apps/hello.app", "Hello")
canWrite("/etc/passwd")   // false
runRoundRobin(["a", "b"], 2)
spawnAppsWorker("job")
```

Async: `import "kos/async"` + `await` (`readAsync` / `awaitAll`).

`@manual`: `import "kos/mem"` och `kos/display_buf` — se [OWNERSHIP.md](../../docs/OWNERSHIP.md).

## Skrivbord

Windows-lik modell (skrivbord, taskbar, Start, fönster, Explorer), modern kDOM/kstyle — inte Win32.

```kabootar
import "kos/boot"

let shell = bootKosSession()   // seed /apps, theme, Start, kb_mount + kb_paint
handleShellClick(shell, x, y)  // kb_click → drain → remount
```

`kabootar shell` kör samma väg. Host ger pixlar och input; UI är kDOM.

| Del | Modul |
|-----|--------|
| Taskbar / Start | `kos/shell` |
| Appar från `/apps` | `kos/launch` |
| Multi-fönster, snap, Alt+Tab | `kos/windows` |
| Filer | `kos/explorer` |
| Tema | `kos/theme` |

## Appar (alla språk)

En app i `/apps` är en kDOM-yta + valfritt Kv8 + kstyle. Shell, Explorer och Settings använder samma tre lager.

## Tester

```bash
cargo test --test os_lib
cargo test --test kos_lib
cargo run --bin kabootar -- examples/os_smoke.kab
cargo run --bin kabootar -- examples/kos_shell_mount_smoke.kab
```

CI: `kos_lib` i `self-host.yml`.
