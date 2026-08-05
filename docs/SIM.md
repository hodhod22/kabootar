# Simulation / robotics (`import "sim"`)

Kab-first **digital twin** module: articulated bodies, hinge/slider joints, joint-space ODE, forward kinematics, sensor stubs, and GP7 editor mapping — one runtime with game + STEM.

## Quick start

```kab
import "sim"
import "sim/robot"

let arm = createArm3(defaultArmParams())
arm = setArmTargets(arm, 0.6, 0.8, -0.4)
arm = simulateArm(arm, 1.0 / 60.0, 120)
let ee = endEffector(arm)
let enc = readEncoders(arm)
let imu = readImu(arm)
let root = worldToEditor(arm)   // digital twin in editor scene graph
```

## API (MVP)

| Area | Functions |
|------|-----------|
| World | `createWorld`, `addBody`, `addJoint`, `step`, `stepN`, `applyLiveParams` |
| Bodies | `createBody`, `createFixedBase`, `findBody` |
| Joints | `createHinge`, `createSlider`, `setJointTarget`, `findJoint` |
| Kinematics | `updateForwardKinematics` |
| Sensors | `sampleSensors`, `readEncoders`, `readImu` |
| Twin | `worldToEditor`, params file helpers |
| Robot | `createArm3`, `setArmTargets`, `simulateArm`, `buildTwinLesson` |

Solver: `params.solver` = `"euler"` (default) or `"rk4"`. Control: PD (`kp`/`kd`) + damping on joint inertia.

## Files

- `lib/sim.kab` — core
- `lib/sim/robot.kab` — 3-DOF arm
- `examples/sim_robot_arm.kab`
- `tests/sim_robot.rs`

Roadmap: [ROADMAP.md](ROADMAP.md) **Våg SIM**.
