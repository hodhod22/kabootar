# Simulation / robotics (`import "sim"`)

Kab-first **digital twin** module: articulated bodies, hinge/slider joints, joint-space ODE, forward kinematics, sensor stubs, GP7 editor mapping, and **live teleop** — one runtime with game + STEM.

## Quick start

```kab
import "sim"
import "sim/robot"
import "sim/teleop"

let arm = createArm3(defaultArmParams())
arm = setArmTargets(arm, 0.6, 0.8, -0.4)
arm = simulateArm(arm, 1.0 / 60.0, 120)
let ee = endEffector(arm)
let enc = readEncoders(arm)
let imu = readImu(arm)
let root = worldToEditor(arm)   // digital twin in editor scene graph

// Live teleop session (joint / IK / Learn)
let tele = bindArmEditor(arm, "/sim/arm_params.json")
tele = teleopSetJoint(tele, "j1", 0.8, 60)
tele = teleopPlaceEe(tele, 1.2, 0.4, 0.0, 60)
tele = setLearnParam(tele, "kp", 5.0)
```

## API (MVP)

| Area | Functions |
|------|-----------|
| World | `createWorld`, `addBody`, `addJoint`, `step`, `stepN`, `applyLiveParams` |
| Bodies | `createBody`, `createFixedBase`, `findBody` |
| Joints | `createHinge`, `createSlider`, `setJointTarget`, `findJoint` |
| Kinematics | `updateForwardKinematics`, `inverseKinematics`, `moveArmTo` |
| Sensors | `sampleSensors`, `readEncoders`, `readImu` |
| Twin | `worldToEditor`, params file helpers, `resolveGroundContact` |
| Robot | `createArm3`, `setArmTargets`, `simulateArm`, `buildTwinLesson` |
| Soft | `sim/soft` — `createCloth2x2`, `stepSoftBody`, `addSoftBody` / `stepWorldSoft` |
| ABA | `abaApplyTorques`, `createArticulatedBody`, `params.solver = "aba"` |
| Teleop | `bindArmEditor`, `syncWorldToEditor`, `selectLink`, `teleopSetJoint`, `teleopSetArm`, `teleopPlaceEe`, `teleopDragLink`, `teleopStep`, `setLearnParam`, `setLearnJoint`, `enterLearnMode` / `enterIkMode` / `enterJointMode` |

Solver: `params.solver` = `"euler"` (default) or `"rk4"`. Control: PD (`kp`/`kd`) + damping on joint inertia.

Teleop modes: **joint** (sliders → `qTarget`), **ik** (place EE → planar IK), **learn** (live `kp`/`kd` + hot-reload stamp).

## Files

- `lib/sim.kab` — core
- `lib/sim/robot.kab` — 3-DOF arm
- `lib/sim/teleop.kab` — GP7 live teleop
- `lib/sim/soft.kab` — particle–spring soft body
- `examples/sim_robot_arm.kab`
- `tests/sim_robot.rs`

Roadmap: [ROADMAP.md](ROADMAP.md) **Våg SIM**.
