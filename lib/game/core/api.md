# Game Engine API Documentation

## Copy-on-Write (CoW) Contract

Most game engine functions follow a Copy-on-Write pattern for immutability and functional programming style:

### World Operations
- `spawn(world)` → Returns `{ world, entity }` - Always returns new world state
- `add(world, id, name, data)` → Returns `world` - Returns modified world
- `remove(world, id, name)` → Returns `world` - Returns modified world
- `destroy(world, id)` → Returns `world` - Returns modified world

### Transform Operations
- `setLocalPosition(transform, x, y, z)` → Returns `transform` - Returns modified transform
- `setParent(transform, parent)` → Returns `transform` - Returns modified transform
- `updateWorldMatrix(transform)` → Returns `transform` - Returns modified transform with cached world matrix

### Physics Operations
- `addCollider(physicsWorld, entityId, collider)` → Returns `physicsWorld`
- `removeCollider(physicsWorld, entityId)` → Returns `physicsWorld`
- `physicsStep(physicsWorld, dt)` → Returns `physicsWorld`

### Render Pipeline Operations
- `addRenderable(pipeline, renderable)` → Returns `pipeline`
- `removeRenderable(pipeline, entityId)` → Returns `pipeline`
- `executeRenderPipeline(pipeline)` → Returns `{ pipeline, drawCalls }`

### Input Operations
- `setKeyDown(input, keyCode)` → Returns `input`
- `setMousePosition(input, x, y)` → Returns `input`
- `updateInputSystem(input)` → Returns `input`

### Game Context Operations
- `updateGameContext(context, dt)` → Returns `context`
- `fixedUpdateGameContext(context)` → Returns `context`
- `renderGameContext(context)` → Returns `{ context, drawCalls }`

**Important**: Always use the returned value from these functions. The original object is not modified.

## Error Handling

### Component Access
- `get(world, id, name)` → Returns `null` if component not found
- `has(world, id, name)` → Returns `false` if component not found
- Always check for null before accessing component data

### Entity Operations
- `isAlive(world, entityId)` → Returns `false` if entity doesn't exist or is destroyed
- Use this before operating on entities

### Physics Operations
- `raycast(physicsWorld, origin, direction, maxDistance)` → Returns empty array if no hits
- `getTriggerEvents(physicsWorld)` → Returns empty array if no events

### Input Operations
- `getActionValue(input, actionMapName, actionName)` → Returns `0.0` if action doesn't exist
- `isActionPressed(input, actionMapName, actionName)` → Returns `false` if action doesn't exist

## Best Practices

### 1. Always Chain Operations
```kab
// Good
world = add(world, id, "Transform", transform)
world = add(world, id, "Rigidbody", rigidbody)

// Bad (world not updated)
add(world, id, "Transform", transform)
add(world, id, "Rigidbody", rigidbody)
```

### 2. Check for Null
```kab
let transform = get(world, id, "Transform")
if transform != null {
    transform = setLocalPosition(transform, 10.0, 0.0, 0.0)
    world = add(world, id, "Transform", transform)
}
```

### 3. Use ECS 2.0 for Performance
```kab
import "game/core/ecs2"

let world = createWorld()
let spawned = spawn(world)
world = spawned["world"]
let entityId = spawned["entity"]["id"]

// Use sparse-set queries for better performance
let results = query2(world, "Transform", "Rigidbody")
```

### 4. Update Hierarchies Before Rendering
```kab
import "game/core/sceneGraph"

world = updateHierarchy(world)
```

### 5. Use Game Context for Complete Game Loop
```kab
import "game/core/gameContext"

let context = createGameContext(1920, 1080)

// Update loop
context = updateGameContext(context, dt)
context = fixedUpdateGameContext(context)
let renderResult = renderGameContext(context)
```

## Performance Guidelines

### 1. Minimize Allocations in Hot Paths
- Use ECS 2.0 sparse-set queries instead of dictionary lookups
- Reuse transform matrices when possible
- Pool frequently created objects

### 2. Batch Similar Operations
- Use `queryWith` for multi-component queries
- Process collisions in batches during physics step
- Sort renderables by material to minimize state changes

### 3. Use Dirty Flags
- Transform dirty flags prevent unnecessary matrix recalculations
- Only update world matrices when needed
- Mark children as dirty when parent changes

### 4. Spatial Partitioning
- Physics uses spatial grid for broadphase collision
- Render pipeline uses frustum culling
- Only process visible objects

## Component Naming Conventions

### Core Components
- `Transform` - Position, rotation, scale
- `Parent` - Parent-child relationship
- `Rigidbody` - Physics simulation
- `Collider` - Collision shape
- `MeshRenderer` - Visual rendering
- `Camera` - View and projection
- `Light` - Lighting

### Game Components
- `GameObject` - Entity metadata
- `Health` - Health points
- `Movement` - Movement data
- `CharacterController` - Character physics

## Integration with Bazi

### Behaviour Hooks
```kab
// Pre-update hook
context["events"] = emitEvent(context["events"], "preUpdate", { "dt": dt })

// Post-physics hook
context["events"] = emitEvent(context["events"], "postPhysics", { "collisions": collisions })

// Pre-render hook
context["events"] = emitEvent(context["events"], "preRender", { "camera": camera })
```

### Prefab Support
```kab
import "game/core/sceneGraph"

let spawned = instantiateHierarchy(world, sourceEntityId, position, rotation)
world = spawned["world"]
```

## Migration from ECS 1.0 to ECS 2.0

### Old API (ecs.kab)
```kab
import "game/core/ecs"

let world = createWorld()
let id = spawn(world)
world = add(world, id, "Transform", data)
let results = query(world, "Transform")
```

### New API (ecs2.kab)
```kab
import "game/core/ecs2"

let world = createWorld()
let spawned = spawn(world)
world = spawned["world"]
let entityId = spawned["entity"]["id"]
world = add(world, entityId, "Transform", data)
let results = query(world, "Transform")
```

### Key Differences
1. `spawn` now returns `{ world, entity }` with entity containing id and generation
2. Entity IDs are recycled with generation tracking for safe despawning
3. `destroy` function available for entity removal
4. Sparse-set provides O(1) component access
5. `query2`, `query3`, `queryWith` for multi-component queries

## Version Information

- ECS 1.0: `game/core/ecs.kab` - Original AoS implementation
- ECS 2.0: `game/core/ecs2.kab` - Sparse-set with entity recycling
- Transform 2.0: `game/core/transform2.kab` - Hierarchical with dirty flags
- Physics 2.0: `game/core/physics2.kab` - Spatial broadphase + integrated step
- Render Pipeline: `game/core/renderPipeline.kab` - Full pipeline with culling
- Game Context: `game/core/gameContext.kab` - Unified context with input/time/events
- Scene Graph: `game/core/sceneGraph.kab` - ECS-based hierarchy with Parent component
