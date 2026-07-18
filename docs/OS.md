# Kabootar OS

Kabootar OS är en **sandboxad kernel** inbyggd i språket — Lager 2 i dual-layer-arkitekturen. Målet är ett fullständigt eget operativsystem med egen display server, minneshanterare och schemaläggare.

## Filosofi

- **Enkelt nu** — virtuellt filsystem, processer, fönster, display
- **Utbyggbart** — syscall-tabell, VFS-snapshot, cooperative scheduler
- **Konkurrenskraftigt mål** — samma API från REPL, WASM och native desktop shell
- **Ärlig scope** — använd `kabootar_reality()` och `os_sauce_honesty()` för att se sandbox vs native vs stub

## Desktop & utseende

kOS ska **kännas igen** för Windows-användare — samma mentala modell (skrivbord, fönster, aktivitetsfält, filutforskare, inställningar) — men byggas med **nyare teknik** än klassisk Win32/GDI/DWM.

| Windows-lik familiaritet | Modern Kabootar-stack |
|--------------------------|------------------------|
| Skrivbord + ikoner | kDOM + KSS, vektorlayout, GPU-compositor |
| Aktivitetsfält / Start | `kbrowser`-flikar + shell i **`lib/kos/`** (Kabootar, inte Rust-UI) |
| Fönster (min/max/stäng, snap) | `os_window_*` + compositor-lager, vsync ([Våg D5](ROADMAP.md)) |
| Filutforskare (träd, sökväg) | VFS (`kabootar://vfs/…`) + `os_mount` mot host |
| Inställningar (kategorier) | Kv8-app i VFS, samma render-pipeline som resten |
| Mörkt/ljust tema | KSS design tokens + `kb_theme()` |
| Notifieringar / systemfält | compositor overlay + `os_haptic_*` för feedback |

**Designprinciper:**

1. **Bekant, inte kopia** — layout och flöden som Windows 10/11, men eget Kabootar-branding; inga Microsoft-tillgångar eller varumärken.
2. **All UI i språket** — shell, Start, Explorer och Settings som `.kab` + KML/KSS/Kv8; Rust endast display server, input och drivrutiner.
3. **Compositor-first** — acrylic/Mica-liknande lager (blur + transparens via GPU), rundade hörn, 60–120 Hz, spring-animationer ([sauce/haptic](OS.md#hemliga-såsen--9-konkurrensstrategier)).
4. **En pipeline** — samma `kdom` → layout → paint som appar och `kbrowser`; inget separat legacy-widget-set.
5. **Progressiv polish** — fungerande shell först (fönster + taskbar + VFS), visuell finish i [ROADMAP G12](ROADMAP.md).

```
Användare (bekant Windows-UX)
        ↓
lib/kos/shell.kab  — taskbar, Start, bootKosDesktop (+ theme)
lib/kos/windows.kab — multi-fönster, snap, Alt+Tab
        ↓
kbrowser + kDOM/KSS/Kv8  — appar, Explorer, Settings
        ↓
os_window_* / GPU compositor  — presentation (winit + wgpu)
```

`kabootar shell` monterar kOS-skrivbordet via `bootKosDesktop()` → `kb_mount`/`kb_paint` (se [ROADMAP G12](ROADMAP.md) shell mount ✅ subset); tunn HTML-fallback om mount misslyckas.

Se [RENDERING.md](RENDERING.md), [BROWSER.md](BROWSER.md) och [ROADMAP.md — G12](ROADMAP.md).

### Kapabilitetstier

| Tier | Betydelse |
|------|-----------|
| `native` | Körs på riktigt i Kabootar-processen (t.ex. SQL, HTTP, stdlib) |
| `sandbox` | Modellerat API — simulerat OS/beteende (t.ex. VFS, sauce-strategier) |
| `partial` | Fungerar i vanliga fall — luckor kvar (bytecode, Kv8, drivrutiner) |
| `stub` | API finns — inte produktionsbeteende (t.ex. `os_compat_run`, WebRTC) |
| `early` | Finns men inte moget ekosystem (paket, community) |

```kabootar
kabootar_reality()           // helhetsrapport
feature_tier("os_sauce_compat") // en funktion
os_sauce_honesty()           // 9 strategier med tier + reality-text
```

## Kernel

```kabootar
os_info();    // kabootar-kernel 2.1.0
os_caps();    // aktiva kapabiliteter
os_syscalls(); // info, read, write, spawn, paint, present, sleep
```

### Kapabiliteter

| Kapabilitet | Status | Beskrivning |
|-------------|--------|-------------|
| `vfs` | ✅ | Virtuellt filsystem |
| `sandbox` | ✅ | Isolerad runtime per session |
| `modules` | ✅ | `import "name"` |
| `process-table` | ✅ | `os_spawn`, processlista |
| `window-manager` | ✅ | `os_window_create`, `os_window_bind` |
| `display-server` | ✅ | `os_display_register` |
| `memory-manager` | ✅ | `os_mem_*`; systems: `@manual` + `import "os/mem"` / `os/display_buf` |
| `scheduler` | ✅ | `os_sched_enqueue` |
| `syscalls` | ✅ | `os_syscall("read", path)` m.fl. |
| `vfs-persist` | ✅ | `os_vfs_save`, `os_vfs_load` (KVF1-format) |
| `device-manager` | ✅ | `os_dev_list`, `os_dev_open`, `os_dev_ioctl` |
| `gpu-driver` | ✅ | `os_gpu_info`, framebuffer + wgpu |
| `net-driver` | ✅ | `os_net_interfaces`, TCP sockets |
| `usb-driver` | ✅ | HID, mass storage, serial |
| `audio-driver` | ✅ | PCM output/input |
| `permissions` | ✅ | Kapabilitetsbaserad åtkomstkontroll per process |
| `hotplug` | ✅ | `os_hotplug_register`, `os_hotplug_poll`, `kb_poll_hotplug` |
| `host-bridge` | ✅ | Fil-baserad PCM/USB-fallback |
| `native-hw` | ✅ | cpal + serialport + hidapi + nusb (`--features hw`) |
| `vfs-extended` | ✅ | rename, copy, mount, metadata (KVF2) |
| `net-tcp-full` | ✅ | listen/accept, UDP, poll |
| `memory-safe` | ✅ | guarded heap + VM stack limit |
| `bytecode-optimize` | ✅ | constant fold + peephole + DCE |
| `ring0-kcore` | ✅ | microkernel IPC, HAL, CFS scheduler, dispatcher |
| `ring0-mm` | ✅ | VMM, pager, cache coherence, heap allocator |
| `ring0-io` | ✅ | driver framework, PnP, IRQ, DMA |
| `ring0-fs-stack` | ✅ | journal (WAL), block I/O scheduler, page cache |
| `ring0-netstack` | ✅ | Ethernet/IP/TCP/UDP layers + traffic control |
| `ring3-userland` | ✅ | init, shell, libc, posix/wsl subsystems |
| `crosscut-security` | ✅ | SRM, ACL, capabilities, sandbox hooks |
| `crosscut-log` | ✅ | ring buffer, ETW-style tracing |
| `crosscut-power` | ✅ | ACPI C/P-states, suspend/resume |
| `sauce-ai-composer` | ✅ | predictive prefetch, contextual UI |
| `sauce-zero-setup` | ✅ | 90s install, NFC zero-touch |
| `sauce-state-sep` | ✅ | OS/apps/data partitions, golden restore |
| `sauce-seamless` | ✅ | ultrasonic pairing, clipboard handoff |
| `sauce-energy-core` | ✅ | wall-power background scheduling |
| `sauce-haptic-ui` | ✅ | spring physics, danger haptics |
| `sauce-compat-god` | ✅ | APK/EXE/Linux32 syscall translation |
| `sauce-privacy` | ✅ | RAM panic encrypt, differential telemetry |
| `sauce-community-updates` | ✅ | beta/stable/classic channel swap |

| `kv8-engine` | ✅ | Kv8 DOM/CSS/JS-subset runtime |

## Kv8 engine

Kabootar's own engine for UI in the language — see [KV8.md](KV8.md).

```kabootar
let ctx = kv8_create();
kv8_run_ui(ctx, "<main>Hello</main>", "main { padding: 24px; }");
kv8_paint(ctx, 1280, 720);
```

## Hemliga såsen — 9 konkurrensstrategier

Sandbox-modell av de icke-tekniska differentierarna mot Windows, macOS, Linux och ChromeOS.

| # | Strategi | Modul | API |
|---|----------|-------|-----|
| 1 | AI-kompositör | `sauce/ai_composer` | `os_ai_prefetch`, `os_ai_record`, `os_ai_context_menu` |
| 2 | Tears-of-Joy setup | `sauce/setup` | `os_setup_nfc` |
| 3 | State separation | `sauce/state_sep` | `os_recovery_restore` |
| 4 | Sömlöst ekosystem | `sauce/seamless` | `os_seamless_pair`, `os_seamless_clipboard_*` |
| 5 | Batteri-evigt | `sauce/energy` | `os_energy_schedule` |
| 6 | Känslomässigt UI | `sauce/haptic` | `os_haptic_danger` |
| 7 | Kompatibilitets-gud | `sauce/compat` | `os_compat_run` |
| 8 | Privacy by design | `sauce/privacy` | `os_privacy_panic`, `os_privacy_telemetry` |
| 9 | Community-uppdateringar | `sauce/updates` | `os_update_channel`, `os_update_rollback` |

```kabootar
os_sauce_map();                              // status för alla 9
os_ai_prefetch();                            // ["spotify","mail","teams",...]
os_setup_nfc("phone-token");                 // Wi-Fi, språk, tidszon, tema
os_recovery_restore();                       // ≤2000 ms golden image (measured)
os_seamless_pair(19000);                     // ultraljudsparning
os_seamless_clipboard_push("text");          // mobil → desktop
os_energy_schedule("backup", true);          // pausas på batteri
os_haptic_danger("/system/kernel");          // röd glow + block
os_compat_run("windows", "CreateFileW", []);  // 99% native
os_privacy_panic();                          // krypterar RAM
os_update_channel("classic");                // växla utan omstart
```

## OS-marknadsföring — 20 punkter (sandbox-modell)

Säkra inkrement som inte krockar med Kabootar-arkitekturen. Fullständiga OS-löften (BIOS-fri boot, MPK-mikro-VM, haptisk hårdvara) modelleras som API/stub där det är rimligt.

| # | Punkt | Status | API / modul |
|---|-------|--------|-------------|
| 1 | Självläkande systempartition | ✅ | `os_recovery_restore()` — mäter verklig tid, cap 2000 ms |
| 2 | Noll-krasch-kärna | 🔶 | Kabootar minnessäkerhet + `memory-safe` heap (full formell garanti = språk) |
| 3 | KV8 snabbare än native | 🔶 | `kv8_*` + `kv8_opt_info()` — delad runtime, ingen kernel-delning |
| 4 | Prediktiv AI-schemaläggare | ✅ | `os_ai_prefetch()` → `os_sched_enqueue` per app |
| 5 | Universell app-kompatibilitet | 🔶 | `os_compat_run` — exe/apk/linux32/deb/app (99% stub) |
| 6 | Nanosekunds-kontextväxling | ✅ | `os_context_switch` → `elapsed_ns` |
| 7 | Batteri-evigt läge | ✅ | `os_energy_battery`, `display_present` 1 Hz-gate |
| 8 | Blixtsnabb start | ✅ | `os_boot_ms()` |
| 9 | Versionerade filsystem | ✅ | `os_vfs_save`, `os_snapshot_list` |
| 10 | Kvantsäker kryptering | 🔶 | `crypto_kyber_encapsulate` (Kyber768-stub) |
| 11 | Hårdvarusandlådor | 🔶 | `os_mm_map`, permissions — full MPK = framtida hw |
| 12 | Haptisk UI | ✅ | `os_haptic_danger` |
| 13 | Noll-telemetri | ✅ | `telemetry_enabled: false`, `os_privacy_telemetry_enable` |
| 14 | Enhets-mesh | ✅ | `os_seamless_*`, `os_seamless_list` |
| 15 | Hot-swappable drivrutiner | ✅ | `os_driver_register`, `os_driver_unregister` |
| 16 | Avbrottsfria uppdateringar | ✅ | `os_update_channel` → `swap_ms` |
| 17 | Federerad sökning | ✅ | `os_search` — VFS + DocAI |
| 18 | Eco-Governor | ✅ | `os_eco_mode` + ACPI C-state |
| 19 | AI-debugger | ✅ | `os_debug_suggest` |
| 20 | Vertikal integration | ✅ | `os_pkg_install`, `registry_install`, samma språk |

```kabootar
os_features_info();                          // boot_ms, battery, telemetry, snapshots
os_boot_ms();                                // tid sedan processstart
os_search("kernel");                         // vfs + docs
os_eco_mode(true);                           // throttla + C-state
os_snapshot_list();                          // sparade VFS-versioner
os_pkg_install("demo@1.0");                  // registry + /apps/*.kv8
os_driver_unregister(2);                     // hot-unload driver
crypto_kyber_encapsulate(pubkey_bytes);      // post-quantum stub
```

## Kärnarkitektur (7 delar + 4 tvärgående)

Kabootar OS modellerar en fullständig kernel-stack i sandbox — från Ring 0 till Ring 3.

### Del 1 — Ring 0 (kcore)

| Komponent | Modul | API |
|-----------|-------|-----|
| Mikrokärna (IPC) | `kcore/microkernel` | `os_ipc_send`, `os_ipc_recv` |
| Executive | `kcore/executive` | (objekt/I/O via executive) |
| HAL | `kcore/hal` | `os_kcore_info()` → `arch` |
| Scheduler (CFS) | `kcore/sched` | `os_sched_tick` |
| Dispatcher | `kcore/dispatcher` | `os_context_switch` |

### Del 2 — Minneshantering (MMU)

| Komponent | Modul | API |
|-----------|-------|-----|
| VMM (sidtabeller) | `mm/vmm` | `os_mm_map`, `os_mm_translate`, `os_mm_fault`, `os_mm_mmap`, COW |
| Pager (swap) | `mm/pager` | `os_mm_stats` |
| Cache-koherens | `mm/cache` | (stat i `os_mm_stats`) |
| Allokator | `mm/allocator` | `os_mem_alloc` (befintlig) |

### Del 3 — Process & trådar

| Komponent | Modul | API |
|-----------|-------|-----|
| PID-tabell | `process.rs` | `os_spawn`, `os_processes` |
| Trådpool | `proc2/thread` | `os_thread_spawn` |
| Signaler | `proc2/signal` | `os_signal_send` |
| Jobb-objekt | `proc2/job` | `os_job_create` |

### Del 4 — I/O & enheter

| Komponent | Modul | API |
|-----------|-------|-----|
| Driver framework | `iosys/framework` | `os_driver_register` |
| PnP | `iosys/pnp` | `os_pnp_discover` |
| IRQ | `iosys/irq` | `os_irq_poll` |
| DMA | `iosys/dma` | (stat i `os_architecture`) |

### Del 5 — Filsystem & lagring

| Komponent | Modul | API |
|-----------|-------|-----|
| VFS | `vfs.rs` | `os_read`, `os_write`, … |
| Journal (WAL) | `fsys/journal` | `os_journal_append` / `commit` / `replay` / `checkpoint` |
| ACL | `xcut/security` | `os_acl_grant` / `check` / `revoke` (path-ACL på VFS) |
| Block I/O | `fsys/block_io` | (kö i `os_architecture`) |
| Page cache | `fsys/page_cache` | (stat i `os_architecture`) |

### Del 6 — Nätverksstack

| Komponent | Modul | API |
|-----------|-------|-----|
| NIC-drivrutin | `drivers/net.rs` | `os_net_listen`, `os_net_poll` |
| Protokollstack | `netstack/layers` | `os_netstack_send` |
| Sockets | `drivers/net.rs` | `os_dev_ioctl(..., "connect")` |
| Traffic control | `netstack/qos` | QoS-klass i paketheader |

### Del 7 — Användarutrymme (Ring 3)

| Komponent | Modul | API |
|-----------|-------|-----|
| Init | `ring3/init` | (pid 1, stat i `os_architecture`) |
| Shell | `ring3/shell` | `os_shell("echo hi")` |
| Libc | `ring3/libc` | `os_libc_open` |
| Subsystem | `ring3/subsystem` | posix + wsl-kompatibilitet |

### Tvärgående system

| System | Modul | API |
|--------|-------|-----|
| Säkerhet (SRM/ACL) | `xcut/security` + `permissions.rs` | `os_perm_*` |
| Felhantering | `xcut/error` | `os_watchdog_ping` |
| Loggning | `xcut/log` | `os_log_drain` |
| Ström (ACPI) | `xcut/power` | `os_power_suspend` |

```kabootar
os_architecture();                    // alla 7 delar + tvärgående
os_kcore_info();                      // ring, arch, scheduler, ipc
os_ipc_send(1, 1, "ping");
os_ipc_recv(1);
os_sched_tick();
os_sched_yield();
os_irq_raise(0, "timer");
os_sched_preempt();
os_context_switch(1, 2);
os_mm_map(1, 4096, 7);
os_mm_translate(1, 4096);
os_mm_fault(1, 131072);
os_mm_mmap(1, 196608, 8192, 7);
os_mm_cow_share(1, 2, 196608);
os_mm_cow_break(2, 196608);
os_thread_spawn(1, "worker");
os_signal_send(1, 15);
os_driver_register("my-drv", "1.0");
os_pnp_discover("usb", "046d", "c52b");
os_journal_append("/path", "data");
os_journal_commit();
os_journal_replay();
os_acl_grant("uid:1", "/path", "read");
os_acl_check("uid:1", "/path", "read");
os_netstack_send("tcp", "payload");
os_netstack_info();
os_display_monitors();
os_display_vsync("fifo");
os_shell("echo hello");
os_libc_open("/dev/null");
os_log_drain(16);
os_watchdog_ping();
os_power_suspend();
```


```kabootar
os_mkdir("/apps");
os_write("/apps/note.txt", "Kabootar OS");
os_read("/apps/note.txt");
os_stat("/apps/note.txt");     // [file, size, mtime, readonly]
os_stat("/apps");              // [dir, entries, 0, false]
os_list("/apps");              // [note.txt]
os_exists("/apps/note.txt");   // true
os_delete("/apps/note.txt");
os_rename("/apps/old.txt", "/apps/new.txt");
os_copy("/apps/template.txt", "/apps/copy.txt");
os_mount("/host", "C:/kabootar-data");   // kräver perm:admin
os_mounts();                              // [{ vfs, host }, ...]
os_unmount("/host");
```

`os_write` kräver att föräldrakatalogen finns — skapa med `os_mkdir` först.

Host-mount mappar ett VFS-prefix till ett värd-katalog (läs/skriv via `std::fs`).

### Persistens (KVF2)

Spara och ladda hela VFS till disk (host-filsystem):

```kabootar
os_vfs_save("/tmp/kabootar-snapshot.kvf");
os_vfs_load("/tmp/kabootar-snapshot.kvf");
```

## Processer och fönster

```kabootar
let pid = os_spawn("my-app");
let win = os_window_create("My App", 1280, 800);
os_window_bind(win, 1);              // koppla till browser-flik 1
os_display_register(win, "Desktop", 1280, 800);
```

## Minne och scheduler

```kabootar
let region = os_mem_alloc(8192, "heap");
os_mem_write(region, 0, [1, 2, 3, 4]);
os_mem_read(region, 0, 4);           // byte-array
os_mem_free(region);                 // wipe + frigör
os_mem_stats();                      // [regions, bytes, limit]
os_sched_enqueue("render-tick");
```

Minnesregioner har **guard bytes** (0xDE) före/efter payload — korruption detekteras vid read/write.

Bytecode-VM har stack-gräns (8192) — `Bytecode stack overflow` vid överflöde.

## Syscalls

Enhetlig ingång till kernel-tjänster:

```kabootar
os_syscall("info");
os_syscall("read", "/apps/note.txt");
os_syscall("write", "/apps/log.txt", "line");
os_syscall("spawn", "worker");
os_syscall("dev_list");
os_syscall("gpu_info");
os_syscall("net_ifaces");
os_syscall("usb_list");
os_syscall("audio_list");
```

## Drivrutiner (GPU, nät, USB, ljud)

Kabootar OS har en **device manager** med registrerade drivrutiner:

| Drivrutin | Enhet | Beskrivning |
|-----------|-------|-------------|
| `gpu-driver` | `gpu-0` | Framebuffer + wgpu-upload när `--features gpu` |
| `net-driver` | `net-eth0` | NIC + TCP-sockets (host bridge på native) |
| `usb-driver` | `usb-hid-0`, `usb-ms-0`, `usb-serial-0` | HID, mass storage, serial |
| `audio-driver` | `audio-out-0`, `audio-in-0` | PCM output/input |

```kabootar
os_dev_list();
let gpu = os_dev_open("gpu-0");
os_dev_ioctl(gpu, "set_mode", 1920, 1080);
os_dev_ioctl(gpu, "present", 1280);

os_net_interfaces();
let nic = os_dev_open("net-eth0");
let sock = os_dev_ioctl(nic, "connect", "example.com", 80);
let listener = os_net_listen("0.0.0.0", 8080);
let client = os_net_accept(listener);
let events = os_net_poll([listener, client]);
let udp = os_net_udp_bind("0.0.0.0", 9);
os_dev_ioctl(nic, "udp_send", udp, "127.0.0.1", 9, [1, 2, 3]);
os_dev_ioctl(nic, "udp_recv", udp, 64);
```

**Net ioctl:** `connect`, `send`, `recv`, `close`, `listen`, `accept`, `poll`, `udp_bind`, `udp_send`, `udp_recv`

os_usb_devices();
let kb = os_dev_open("usb-hid-0");
os_dev_ioctl(kb, "transfer", "in");

os_audio_devices();
let spk = os_dev_open("audio-out-0");
os_dev_ioctl(spk, "write", [0, 1000, -1000, 0]);
```

## Permissions (kapabiliteter)

Åtkomst styrs per **process** (`os_subject` / `os_set_subject`). Init (pid 1) har full åtkomst.

```kabootar
let worker = os_spawn("worker");       // ärv förälders kapabiliteter
os_perm_clear(worker);                 // sandbox utan rättigheter
os_perm_grant(worker, "device:gpu-0");
os_perm_grant(worker, "vfs:read:/apps");
os_perm_grant(worker, "net:connect");
os_set_subject(worker);
os_dev_open("gpu-0");
```

| Kapabilitet | Betydelse |
|-------------|-----------|
| `device:gpu-0` | Öppna GPU-enhet |
| `device:usb-*` | USB-enheter (prefix-match) |
| `device-ioctl:net-eth0:connect` | Specifik ioctl |
| `vfs:read:/apps` | Läsa under `/apps` |
| `vfs:write:/data` | Skriva under `/data` |
| `net:connect` | TCP-anslutning |
| `hotplug:register` | Registrera USB hotplug |
| `perm:admin` | `os_perm_grant` / `os_perm_revoke` |
| `*` | Alla rättigheter (endast init standard) |

```kabootar
os_perm_check(worker, "device:gpu-0");
os_perm_list(worker);
os_perm_revoke(worker, "net:connect");
```

## Hotplug

```kabootar
os_hotplug_register("Acme", "Webcam", "hid");
let events = os_hotplug_poll();   // [{ action, device_id, kind, name, vendor }]
kb_poll_hotplug();                // samma bus — för browser-appar
```

## Host bridge (native)

Vidarebefordra ljud/USB till värd-OS via filer (fallback) eller **riktig hårdvara** med `--features hw`:

```bash
cargo run --no-default-features --features docai,codai,hw -- ...
# eller med shell: --features docai,codai,shell,hw
export KABOOTAR_HW=1          # default på native med hw
export KABOOTAR_HW=0          # tvinga simulerade enheter
```

| Backend | Bibliotek | Enhets-ID |
|---------|-----------|-----------|
| **Ljud ut/in** | cpal (WASAPI/ALSA/CoreAudio) | `host-audio-out-0`, `host-audio-in-0` |
| **USB serial (CDC)** | serialport | `host-usb-serial-COM3` (Windows) / `host-usb-serial-ttyUSB0` (Linux) |
| **USB HID** | hidapi | `host-usb-hid-0`, `host-usb-hid-1`, … |
| **USB (full enum)** | nusb | `host-usb-vvvv-pppp-bNaM` (vid/pid + bus + address) |

```kabootar
os_hw_refresh();              // skanna om host-enheter
os_host_info();               // { hw_enabled, audio_backend: "cpal", usb_backend: "serialport+hidapi+nusb", ... }
os_audio_devices();           // inkl. host-audio-out-*
let spk = os_dev_open("host-audio-out-0");
os_dev_ioctl(spk, "write", [0, 1000, -1000, 0]);

let ports = os_usb_devices();
let ser = os_dev_open("host-usb-serial-COM3");   // om COM-port finns
os_dev_ioctl(ser, "transfer", "out", "AT\r\n");

let hid = os_dev_open("host-usb-hid-0");
os_dev_ioctl(hid, "transfer", "in");             // läs HID-rapport (64 byte)
os_dev_ioctl(hid, "transfer", "out", [0, 1, 2]); // skriv HID-rapport

// nusb: öppna valfri icke-mass-storage-enhet (kräver claim_interface på Windows)
let dev = os_dev_open("host-usb-046d-c52b-b1a3");  // exempel-ID från os_usb_devices()
// Control transfer: [bmRequestType, bRequest, wValue_lo, wValue_hi, wIndex_lo, wIndex_hi, ...payload]
os_dev_ioctl(dev, "transfer", "control", [0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 18]);
```

**USB transfer-endpoints**

| Endpoint | Backend | Beskrivning |
|----------|---------|-------------|
| `"in"` | serial, HID | Läs bytes/rapport (timeout, tom vektor om inget data) |
| `"out"` | serial, HID | Skriv bytes/rapport |
| `"control"` | nusb | USB control transfer (bmRequest + payload) |

Mass storage från nusb är **endast enumeration** — öppning returnerar fel (kräver OS-mount). Virtuell `usb-ms-0` simulerar sektorer.

Fil-bridge (äldre fallback):

```bash
export KABOOTAR_HOST_BRIDGE=1
export KABOOTAR_HOST_AUDIO=/tmp/kabootar-out.pcm
export KABOOTAR_HOST_USB=/tmp/kabootar-serial.bin
```

## Arkitektur

```
src/runtime/os/
  kernel.rs     # Metadata och kapabiliteter
  subsys.rs     # KernelSubsystems — registry för alla delar
  kcore/        # Del 1: microkernel, executive, HAL, scheduler, dispatcher
  mm/           # Del 2: VMM, pager, cache, allocator
  proc2/        # Del 3: thread pool, signals, job objects
  iosys/        # Del 4: driver framework, PnP, IRQ, DMA
  fsys/         # Del 5: journal, block I/O, page cache
  netstack/     # Del 6: protocol layers, traffic control
  ring3/        # Del 7: init, shell, libc, subsystems
  xcut/         # Tvärgående: security, error, log, power
  sauce/        # 9 konkurrensstrategier (AI, setup, seamless, …)
  os_api.rs     # Natives: os_architecture, os_ipc_*, os_mm_*, …
  sauce_api.rs  # Natives: os_sauce_map, os_ai_*, os_compat_run, …
  vfs.rs        # Virtuellt filsystem
  process.rs    # Processtabell
  window.rs     # Fönsterhanterare
  display.rs    # Display server
  memory.rs     # Minneshanterare (guarded heap)
  scheduler.rs  # Cooperative scheduler (legacy)
  syscall.rs    # Syscall-tabell
  persist.rs    # VFS snapshot (KVF1/KVF2)
  drivers/      # GPU, net, USB, audio drivers + device manager
  permissions.rs
  hotplug.rs
  host_bridge.rs
  mod.rs        # OsHandle och inbyggda API:n
```

## Native desktop

```bash
cargo run --no-default-features --features docai,codai,shell -- shell
# GPU-accelerated presentation (wgpu):
cargo run --no-default-features --features docai,codai,shell,gpu -- shell
```

Startar Kabootar OS i ett riktigt fönster med pixel-compositor (winit + softbuffer).

Se [PLATFORM.md](PLATFORM.md), [RENDERING.md](RENDERING.md) och [ROADMAP.md](ROADMAP.md).
