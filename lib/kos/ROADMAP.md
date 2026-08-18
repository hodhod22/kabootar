# kOS — roadmap

**kOS är det enda operativsystemet vi bygger.** Själva OS:et skrivs i Kabootar (`.kab`); Rust är tillfälliga `os_*`-syscalls tills de tunnas bort.

kOS är **inte** ett Kabootar-only-skrivbord. Det är en **allmän värd** för appar i **vilket språk som helst** så länge de kör mot **Kv8 + kDOM + kstyle**. Kabootar är ett gästspråk bland flera — samma fönster, VFS, Start och compositor.

`lib/os/` är borttagen. Kernel och skrivbord lever under `lib/kos/`. Syscalls heter `os_*`; moduler `import "kos/…"`.

**Byggordning:** kOS först, sedan [kbrowser](../kbrowser/ROADMAP.md). Skrivbordet målar via kbrowser; kbrowser navigerar `kabootar://` via VFS.

Se även [docs/OS.md](../../docs/OS.md) (host `os_*`) och [README.md](README.md) (produkt). Språk/self-host: [docs/ROADMAP.md](../../docs/ROADMAP.md). kbrowser: [../kbrowser/README.md](../kbrowser/README.md).

## Layout

```
lib/kos/
  vfs.kab, vfs_policy.kab, mount.kab, async.kab
  sched.kab, sched_policy.kab
  process.kab, process_policy.kab
  kernel.kab, mem.kab, display_buf.kab
  boot.kab, shell.kab, launch.kab, windows.kab, explorer.kab, theme.kab
```

| Import | Roll |
|--------|------|
| `kos/vfs` | read/write/list/mkdir — wrappers över `os_read`/`os_write` |
| `kos/vfs_policy` | `/apps`-sandbox (`canWrite`, `ensureAppsRoot`) |
| `kos/sched` / `kos/sched_policy` | enqueue/tick + fair/round-robin policy |
| `kos/process` / `kos/process_policy` | spawn + caps/`spawnSandbox` |
| `kos/mem` / `kos/display_buf` | `@manual` MemBox / framebuffer |
| `kos/boot` | `bootKosSession` / `handleShellClick` |
| `kos/shell` | skrivbord, taskbar, Start, `bootKosDesktop` |
| `kos/windows` | fönster, snap, Alt+Tab |
| `kos/explorer` | VFS-utforskare |
| `kos/theme` | KSS polish |

Gate: `cargo test --test os_lib` (kernel) + `cargo test --test kos_lib` (skrivbord). CI: `kos_lib` i `self-host.yml`.

---

## Regel

- Ny OS-logik skrivs **bara** i `.kab` under `lib/kos/`.
- Inga nya features i Rust. Host = syscall, sedan bootstrap, sedan bort.
- kbrowser är **inte** kOS — chrome och tabs hör hemma i `lib/kbrowser/`.
- Appar binder mot **Kv8 / kDOM / kstyle**, inte mot Kabootar-syntax. Inget OS-API som kräver `.kab` hos gästen.

## Gäster (alla språk)

UI-kontraktet är tre lager som alla språk delar:

| Lager | Roll för gästen |
|-------|-----------------|
| **Kv8** | script/eval (JS-subset och andra språk som kompilerar dit) |
| **kDOM** | träd, events, query — inte host-`document` |
| **kstyle** | layout + tema (KSS/CSS-object) |

kOS ger process, VFS, fönster och input. En app i `/apps` är en kDOM-yta + Kv8 + kstyle, oavsett källspråk. Shell/Explorer/Settings följer samma kontrakt.

---

## Kärna (tidigare K3 / H6d)

| Fas | Innehåll | Status |
|-----|----------|--------|
| **KOS-K1** | VFS + mount + `kos/sched` + `/apps` write-policy | ✅ subset |
| **KOS-K2** | Process-tabell + sandbox-caps (`spawnSandbox`, `canKill`) | ✅ subset |
| **KOS-K3** | Async VFS (`kos/async` + riktig `await`) | ✅ subset |
| **KOS-K4** | `@manual` mem + display_buf | ✅ subset |
| **KOS-K5** | Policy i Kab: `runRoundRobin`, `runFairTick`, `ensureAppsRoot` | ✅ subset (`h6d_os_policy_smoke`) |

**Nästa (kärna):**

- [ ] Fler VFS-operationer i Kab (rename/copy/metadata) ovanpå thin `os_*` — inte ny Rust-policy
- [ ] Sched-policy mot riktiga worker-callbacks (inte bara enqueue-namn)
- [ ] Process: kill-policy + cap-check före varje syscall-grupp
- [ ] Tunna bort Rust-process/VFS-policy tills host bara är diskbytes

---

## Skrivbord (tidigare K5 / G12)

Mål: Windows-lik mental modell, 2020+-känsla, all UI i Kabootar (kDOM/KSS) — se [OS.md desktop](../../docs/OS.md#desktop--utseende).

| Komponent | Innehåll | Modul |
|-----------|----------|--------|
| Shell | Taskbar, Start, systemfält | `kos/shell` |
| Fönster | min/max/stäng, snap, Alt+Tab | `kos/windows` |
| Explorer | `kabootar://vfs`, sökväg | `kos/explorer` + kbrowser |
| Settings | system/nät/skärm | Kv8-app i VFS (plan) |
| Tema | ljust/mörkt, accent | `kos/theme` |
| Boot | seed `/apps`, mount, paint | `kos/boot` |

| Fas | Innehåll | Status |
|-----|----------|--------|
| **G12.1** | Skrivbord + taskbar + ett fönster | ✅ subset (`buildShell` / `listApps`) |
| **G12.2** | Start + `/apps` → `openWindow` | ✅ subset (`launchApp`, `clickStartApp`, `drainKosEvents`, app-body) |
| **G12.3** | Explorer + `os_read`/`write`/`list` | ✅ subset |
| **G12.4** | Snap + multi-fönster + Alt+Tab | ✅ subset |
| **G12.5** | Tema/polish (inte full GPU-blur) | ✅ subset |
| **KOS-D1** | `bootKosDesktop` / `bootKosSession` + `kb_mount`/`kb_paint` | ✅ subset |
| **KOS-D2** | Host-klick → `kb_click` → drain → remount | ✅ subset |

**Nästa (skrivbord):**

- [ ] Settings-app i VFS (Kv8), samma pipeline som Explorer
- [ ] GPU blur / acrylic via compositor (host thin; policy i `.kab`)
- [ ] Spring-animationer för öppna/stäng/snap (Kab + vsync-syscall)
- [ ] Delete-gate: skapa app i `/apps` → Start → fönster → stäng utan Rust-UI

Beror på: kbrowser paint/nav ([kbrowser roadmap](../kbrowser/ROADMAP.md)), layout (Våg C), GPU compositor (Våg D5 — host).

---

## Rust → noll (OS-delen av H6)

| Fas | Mål | Delete-gate |
|-----|-----|-------------|
| **H6d** | OS-policy i `.kab` | ✅ subset — Rust = disk/net/GPU/hw + thin `os_*` |
| **KOS-H1** | Inga nya `os_*` produkt-API | pågående |
| **KOS-H2** | Init/shell-policy 100 % Kab | `kos/boot` + `kos/shell`; host bara fönster/pixels |
| **KOS-H3** | Bare-metal/boot (tidigare D7) | efter thin host — `.kab` + minimal laddare |

---

## Checkpoint

`cargo test --test os_lib` · `cargo test --test kos_lib` · smokes: `examples/os_*.kab`, `examples/kos_*.kab`, `examples/h6d_os_policy_smoke.kab`
