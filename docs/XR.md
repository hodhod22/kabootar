# Kabootar XR (`import "game/xr"`)

**Status:** GP6n ✅ **subset** — Kab API + host FFI probe/stub; real HMD present is optional when an OpenXR loader is installed.

## Environments

| Env | Behavior |
|-----|----------|
| `KABOOTAR_XR_STUB=1` | Force stub backend (`openxr-stub` / stub trampolines). CI default path. |
| `KABOOTAR_XR_HAND_TRACKING=1` | Enable EXT hand tracking surface (`openxr-stub` or `openxr-ext`). |
| `KABOOTAR_XR_LOADER=…` | Optional path hint for `openxr_loader` (native). |
| No loader + no stub | Descriptor / emulated paths; hand locate errors unless tracking env set. |

## Kab API (high level)

```kabootar
import "game/xr"

let xr = xrBindHeadset(createXrSession("vr"), true)
xr = xrBegin(xr)

// Controllers / WebXR-style input sources
let sources = xrInputSources(xr)   // left + right; may include .hand when tracking live
let grip = xrGetInputPose(sources[1], "grip")
xrInjectInputEvent("select", "right")
let evs = xrPollInputEvents(xr)

// XR_EXT_hand_tracking
let st = xrHandTrackingStatus()
// st.trackers.left/right — synth or created handles
// st.createPath — stub-xrCreateHandTrackerEXT | xrCreateHandTrackerEXT | …
// st.locatePath — stub-xrLocateHandJointsEXT | xrLocateHandJointsEXT | …
let loc = xrLocateHandJoints("left")   // FFI create/locate then synth-fill 26 joints
let hand = xrHandJoints("left")        // live-buffer or stub joints
```

## Host contract (`src/runtime/game/xr_ffi.rs`)

| Kab / native | Role |
|--------------|------|
| `xr_host_info` / `xrInfo` | Loader/WebXR probe bag |
| `xr_bind_headset` | Bind session; may resolve OpenXR procs |
| `xrCreateHandTrackerEXT` | Resolved or **stub trampoline**; stores handles in `HandJointBuffers` |
| `xrLocateHandJointsEXT` | Resolved or stub; always synth-fills joint buffers for CI |
| `xrEndFrame` | Resolved or stub trampoline for frame submit accounting |
| `xr_host_present` | Present / compositor handoff descriptor |

**Stub trampolines** return success with synthetic handles so CI never needs a real runtime. When the loader resolves a real proc, `extResolved` / `extCreateResolved` are true and `ffiMode` is `direct` (may still synth-fill poses without a live tracker).

## Related

- [GAME.md](GAME.md) — game pillar overview  
- [ROADMAP.md](ROADMAP.md) — GP6n checkpoint  
- Tests: `tests/game_gp6_gp7.rs` (`xr_hand_joints_and_input_profiles`, `xr_input_sources_select_and_poses`)
