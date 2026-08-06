# Framework pillars (MIT motor)

Kabootar (`nova-interpreter`) is MIT. Commercial kits live in separate repos and
sit on these motor modules:

| Motor (`import`) | Commercial kit | Role | Kit license |
|------------------|----------------|------|-------------|
| `doc` / `doc/*` | [nabz](https://github.com/hodhod22/nabz) | Editable documents (PDF-like) | **MIT (free)** |
| `web` / `web/*` | [peyvand](https://github.com/hodhod22/peyvand) | Next-like routes/pages | $10 / $100 / year |
| `cad/aero` | [rasmejaryan](https://github.com/hodhod22/rasmejaryan) | Aerodynamics CAD | $10 / $100 / year |
| `cad/arch` | [rasmesakht](https://github.com/hodhod22/rasmesakht) | Architecture CAD | $10 / $100 / year |
| `cad/circuit` | [rasmemadar](https://github.com/hodhod22/rasmemadar) | Electronics CAD | $10 / $100 / year |
| `cad/power` | [rasmebargh](https://github.com/hodhod22/rasmebargh) | Electrical installation CAD | $10 / $100 / year |
| `game/*` | [bazi](https://github.com/hodhod22/bazi) | Gameplay (ECS already in motor) | $10 / $100 / year |

Shared CAD helpers: `cad/geom`, `cad/graph`.

```bash
kabootar run examples/pillars_smoke.kab
```

Kits should prefer leaf imports of motor modules, then add branded helpers.
Do not put Health/AI/combat (or other kit gameplay) back into `lib/game/`.
