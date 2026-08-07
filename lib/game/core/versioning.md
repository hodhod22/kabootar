# Game Engine Versioning Guide

## ECS Versions

### ECS 1.0 (Legacy)
- **File**: `game/core/ecs.kab`
- **Status**: Legacy, maintained for backward compatibility
- **Features**: Simple AoS (Array of Structures) implementation
- **Use case**: Existing projects that haven't migrated yet
- **Performance**: O(n) queries, no entity recycling, manual archetype management

```kab
import "game/core/ecs"

let world = createWorld()
let id = spawn(world)
world = add(world, id, "Transform", data)
let results = query(world, "Transform")
```

### ECS 2.0
- **File**: `game/core/ecs2.kab`
- **Status**: Stable production release
- **Features**: Sparse-set, entity recycling, manual archetypes
- **Use case**: Projects needing entity destruction and recycling
- **Performance**: O(1) component access, entity generation tracking

```kab
import "game/core/ecs2"

let world = createWorld()
let spawned = spawn(world)
world = spawned["world"]
let entityId = spawned["entity"]["id"]
world = add(world, entityId, "Transform", data)
let results = query2(world, "Transform", "Rigidbody")
```

### ECS 3.0 (Current)
- **File**: `game/core/ecs3.kab`
- **Status**: Current recommended version
- **Features**: Automatic archetypes, fast isAlive, reduced allocations
- **Use case**: New projects and production use
- **Performance**: O(1) isAlive, automatic archetype management, minimal GC

```kab
import "game/core/ecs3"

let world = createWorld()
let spawned = spawn(world)
world = spawned["world"]
let entityId = spawned["entityId"]
world = add(world, entityId, "Transform", data)
let results = queryArchetype(world, ["Transform", "Rigidbody"])
```

## Physics Versions

### Physics 1.0 (Legacy)
- **File**: `game/core/physics.kab`
- **Status**: Legacy
- **Features**: Basic 2D/3D physics, manual collision detection
- **Use case**: Legacy projects

### Physics 2.0
- **File**: `game/core/physics2.kab`
- **Status**: Stable
- **Features**: Spatial broadphase, integrated physics step, trigger events
- **Use case**: Projects needing spatial acceleration

### Physics 3.0 (Current)
- **File**: `game/core/physics3.kab`
- **Status**: Current recommended version
- **Features**: Improved narrowphase (sphere/capsule), automatic trigger events, character controller integration
- **Use case**: Production use with full collision support

```kab
import "game/core/physics3"

let physicsWorld = createPhysicsWorld()
physicsWorld = setEventCallback(physicsWorld, fn(eventType, eventData) {
    // Handle trigger events automatically
})
physicsWorld = physicsStep(physicsWorld, dt)
```

## Render Pipeline Versions

### Render Pipeline 1.0
- **File**: `game/core/renderPipeline.kab`
- **Status**: Legacy
- **Features**: Basic camera, culling, draw calls

### Render Pipeline 2.0 (Current)
- **File**: `game/core/renderPipeline2.kab`
- **Status**: Current recommended version
- **Features**: Material system, opaque/transparent sorting, render queues
- **Use case**: Production use with proper material management

```kab
import "game/core/renderPipeline2"

let pipeline = createRenderPipeline()
let material = createMaterial("Standard", "pbr.wgsl")
material = setMaterialTransparent(material, true)
pipeline = addMaterial(pipeline, "Standard", material)
```

## Game Context Versions

### Game Context 1.0
- **File**: `game/core/gameContext.kab` (v1.0.0)
- **Status**: Legacy
- **Features**: Basic ECS 2.0, physics, rendering integration

### Game Context 2.0 (Current)
- **File**: `game/core/gameContext.kab` (v2.0.0)
- **Status**: Current recommended entry point
- **Features**: ECS 3.0, Physics 3.0, Render Pipeline 2.0, instancing, character controller
- **Use case**: New projects, recommended entry point

```kab
import "game/core/gameContext"

let context = createGameContext(1920, 1080)
context = updateGameContext(context, dt)
context = fixedUpdateGameContext(context)
let renderResult = renderGameContext(context)
```

## Migration Guide

### From ECS 1.0 to ECS 3.0

**Old API**:
```kab
import "game/core/ecs"
let world = createWorld()
let id = spawn(world)
world = add(world, id, "Transform", data)
```

**New API**:
```kab
import "game/core/ecs3"
let world = createWorld()
let spawned = spawn(world)
world = spawned["world"]
let entityId = spawned["entityId"]
world = add(world, entityId, "Transform", data)
```

**Key Changes**:
- `spawn()` now returns `{ world, entityId }` instead of just entity ID
- Entity IDs are recycled with generation tracking
- `isAlive()` is now O(1) instead of O(n)
- Archetypes are automatic - no manual management needed
- `destroy()` function available for entity removal

### From Physics 2.0 to Physics 3.0

**New Features**:
- Sphere and capsule collision detection
- Automatic trigger event callbacks
- Character controller integration
- Sleeping bodies support

**API Changes**:
```kab
// Set up automatic trigger event handling
physicsWorld = setEventCallback(physicsWorld, fn(eventType, eventData) {
    // eventType: "triggerEnter" or "triggerExit"
    // eventData: { trigger, collider, type }
})
```

### From Render Pipeline 1.0 to 2.0

**New Features**:
- Material system with properties and textures
- Opaque and transparent render queues
- Automatic sorting by material and depth
- Blend modes and depth control

**API Changes**:
```kab
// Create material
let material = createMaterial("Standard", "pbr.wgsl")
material = setMaterialProperty(material, "metallic", 0.5)
material = setMaterialTransparent(material, true)
pipeline = addMaterial(pipeline, "Standard", material)
```

## Recommended Stack (Current)

For new projects, use the following stack:

```kab
import "game/core/gameContext"

let context = createGameContext(1920, 1080)

// ECS 3.0 is used automatically via context["world"]
let spawned = spawn(context["world"])
context["world"] = spawned["world"]
let entityId = spawned["entityId"]

// Physics 3.0 is used automatically via context["physicsWorld"]
context["physicsWorld"] = setEventCallback(context["physicsWorld"], fn(eventType, eventData) {
    // Handle events
})

// Render Pipeline 2.0 is used automatically via context["renderPipeline"]
let material = createMaterial("Standard", "pbr.wgsl")
context["renderPipeline"] = addMaterial(context["renderPipeline"], "Standard", material)

// Game loop
while true {
    let dt = 0.016
    context = updateGameContext(context, dt)
    context = fixedUpdateGameContext(context)
    let renderResult = renderGameContext(context)
}
```

## Version Compatibility Matrix

| Component | 1.0 | 2.0 | 3.0 | Notes |
|-----------|-----|-----|-----|-------|
| ECS | Legacy | Stable | Current | 3.0 recommended |
| Physics | Legacy | Stable | Current | 3.0 recommended |
| Render Pipeline | Legacy | - | Current | 2.0 is current |
| Game Context | Legacy | Current | - | 2.0 uses ECS 3.0 |
| Transform | - | Current | - | transform2.kab |
| Scene Graph | - | Current | - | sceneGraph.kab |
| Instancing | - | - | Current | instancing.kab |
| Character Controller | - | - | Current | characterController.kab |

## Deprecation Timeline

- **ECS 1.0**: Deprecated, will be removed in version 3.0
- **ECS 2.0**: Stable, will be maintained until 3.0 release
- **ECS 3.0**: Current, recommended for all new projects
- **Physics 1.0**: Deprecated, will be removed in version 3.0
- **Physics 2.0**: Stable, will be maintained until 3.0 release
- **Physics 3.0**: Current, recommended for all new projects
- **Render Pipeline 1.0**: Deprecated, will be removed in version 3.0
- **Render Pipeline 2.0**: Current, recommended for all new projects

## API Stability Guarantees

### Stable APIs (No breaking changes in minor versions)
- ECS 3.0 core API
- Physics 3.0 core API
- Render Pipeline 2.0 core API
- Game Context 2.0 API
- Transform 2.0 API

### Experimental APIs (May change)
- Instancing system
- Character controller
- Advanced physics features (CCD, sleeping bodies)
- Post-processing integration

## Checking Engine Version

```kab
import "game/core/gameContext"

let context = createGameContext(1920, 1080)
let version = context["version"]
// version will be "2.0.0"
```

## Support Policy

- **Current versions (3.0/2.0)**: Full support, bug fixes, new features
- **Stable versions (2.0)**: Bug fixes only, no new features
- **Legacy versions (1.0)**: Security fixes only, no new features

## Upgrade Path

1. **Start with Game Context 2.0** - This automatically uses the latest stable versions
2. **Migrate ECS calls** - Update spawn/add/remove calls to use new API
3. **Update physics integration** - Add event callbacks for triggers
4. **Adopt material system** - Replace direct rendering with material-based approach
5. **Enable instancing** - Add instancing system for performance
