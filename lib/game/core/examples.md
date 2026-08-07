# Game Engine Examples

## Motor Only Example

This example shows how to use the game engine without Bazi, using the core ECS 2.0 system directly.

```kab
import "game/core/ecs2"
import "game/core/transform2"
import "game/core/physics2"
import "game/core/renderPipeline"
import "game/core/gameContext"
import "game/core/sceneGraph"

// Create game context
let context = createGameContext(1920, 1080)

// Spawn a player entity
let spawned = spawn(context["world"])
context["world"] = spawned["world"]
let playerId = spawned["entity"]["id"]

// Add transform component
let transform = createTransform(0.0, 1.0, 0.0)
context["world"] = add(context["world"], playerId, "Transform", transform)

// Add rigidbody component
let rigidbody = {
    "kind": "Rigidbody",
    "mass": 1.0,
    "velocity": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "useGravity": true,
    "isKinematic": false
}
context["world"] = add(context["world"], playerId, "Rigidbody", rigidbody)

// Add collider to physics world
let collider = {
    "kind": "BoxCollider",
    "center": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "size": { "x": 1.0, "y": 2.0, "z": 1.0 }
}
context["physicsWorld"] = addCollider(context["physicsWorld"], playerId, collider)

// Add renderable to pipeline
let bounds = {
    "min": { "x": -0.5, "y": -1.0, "z": -0.5 },
    "max": { "x": 0.5, "y": 1.0, "z": 0.5 }
}
let renderable = createRenderable(playerId, "player.mesh", "player.mat", transform, bounds)
context["renderPipeline"] = addRenderable(context["renderPipeline"], renderable)

// Game loop
while true {
    let dt = 0.016
    
    // Update input
    context["input"] = setKeyDown(context["input"], "W")
    
    // Update game context
    context = updateGameContext(context, dt)
    
    // Apply input to rigidbody
    let rb = get(context["world"], playerId, "Rigidbody")
    if rb != null && getKey(context["input"], "W") {
        rb["velocity"]["z"] = rb["velocity"]["z"] + 10.0 * dt
        context["world"] = add(context["world"], playerId, "Rigidbody", rb)
    }
    
    // Fixed update physics
    context = fixedUpdateGameContext(context)
    
    // Update hierarchy
    context["world"] = updateHierarchy(context["world"])
    
    // Render
    let renderResult = renderGameContext(context)
    let drawCalls = renderResult["drawCalls"]
}
```

## Motor + Bazi Integration Example

This example shows how to integrate the game engine with Bazi's behaviour system.

```kab
import "game/core/ecs2"
import "game/core/transform2"
import "game/core/baziIntegration"
import "game/core/gameContext"

// Create game context
let context = createGameContext(1920, 1080)

// Create behaviour system
let behaviourSystem = createBehaviourSystem()
behaviourSystem = setSystemContext(behaviourSystem, context)

// Spawn a player with behaviour
let spawned = spawn(context["world"])
context["world"] = spawned["world"]
let playerId = spawned["entity"]["id"]

// Add transform
let transform = createTransform(0.0, 1.0, 0.0)
context["world"] = add(context["world"], playerId, "Transform", transform)

// Add behaviour component
let behaviour = createBehaviourComponent("PlayerController")
behaviour = setBehaviourHook(behaviour, "onUpdate", fn(world, entityId, dt, ctx) {
    let input = ctx["input"]
    let transform = get(world, entityId, "Transform")
    
    if transform != null {
        if getKey(input, "W") {
            transform = translate(transform, 0.0, 0.0, 5.0 * dt)
        }
        if getKey(input, "S") {
            transform = translate(transform, 0.0, 0.0, -5.0 * dt)
        }
        if getKey(input, "A") {
            transform = translate(transform, -5.0 * dt, 0.0, 0.0)
        }
        if getKey(input, "D") {
            transform = translate(transform, 5.0 * dt, 0.0, 0.0)
        }
        
        world = add(world, entityId, "Transform", transform)
    }
    
    return world
})

behaviour = setBehaviourHook(behaviour, "onSpawn", fn(world, entityId, ctx) {
    let name = createNameComponent("Player")
    world = add(world, entityId, "Name", name)
    return world
})

context["world"] = add(context["world"], playerId, "Behaviour", behaviour)
behaviourSystem = addBehaviourToSystem(behaviourSystem, playerId, behaviour)

// Call spawn hook
context["world"] = onBehaviourSpawn(behaviourSystem, context["world"], playerId)

// Game loop
while true {
    let dt = 0.016
    
    // Update input
    context["input"] = setKeyDown(context["input"], "W")
    
    // Update game context
    context = updateGameContext(context, dt)
    
    // Execute behaviours in update phase
    behaviourSystem = setSystemPhase(behaviourSystem, "update")
    behaviourSystem = executeBehaviourSystem(behaviourSystem, context["world"], dt)
    
    // Update hierarchy
    context["world"] = updateHierarchy(context["world"])
    
    // Render
    let renderResult = renderGameContext(context)
}
```

## Prefab Example with Bazi

```kab
import "game/core/ecs2"
import "game/core/baziIntegration"
import "game/core/gameContext"

// Create game context and prefab manager
let context = createGameContext(1920, 1080)
let prefabManager = createPrefabManager()

// Register enemy prefab
let enemyPrefabData = {
    "components": [
        {
            "kind": "Transform",
            "localPosition": { "x": 0.0, "y": 0.0, "z": 0.0 },
            "localRotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
            "localScale": { "x": 1.0, "y": 1.0, "z": 1.0 }
        },
        {
            "kind": "Name",
            "name": "Enemy"
        },
        {
            "kind": "Tag",
            "tag": "Enemy"
        }
    ]
}
prefabManager = registerPrefab(prefabManager, "Enemy", enemyPrefabData)

// Instantiate enemy at position
let position = { "x": 10.0, "y": 0.0, "z": 5.0 }
let rotation = { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 }
let result = instantiatePrefabFromManager(prefabManager, context["world"], "Enemy", position, rotation)
context["world"] = result["world"]
prefabManager = result["manager"]
let enemyId = result["entityId"]

// Add behaviour to enemy
let enemyBehaviour = createBehaviourComponent("EnemyAI")
enemyBehaviour = setBehaviourHook(enemyBehaviour, "onUpdate", fn(world, entityId, dt, ctx) {
    let transform = get(world, entityId, "Transform")
    if transform != null {
        transform = translate(transform, 0.0, 0.0, 2.0 * dt)
        world = add(world, entityId, "Transform", transform)
    }
    return world
})

context["world"] = add(context["world"], enemyId, "Behaviour", enemyBehaviour)

// Find all enemies by tag
let enemies = findEntitiesByTag(context["world"], "Enemy")
```

## Hierarchical Scene Example

```kab
import "game/core/ecs2"
import "game/core/transform2"
import "game/core/sceneGraph"

let world = createWorld()

// Spawn root entity
let rootSpawned = spawn(world)
world = rootSpawned["world"]
let rootId = rootSpawned["entity"]["id"]

let rootTransform = createTransform(0.0, 0.0, 0.0)
world = add(world, rootId, "Transform", rootTransform)

// Spawn child entity
let childSpawned = spawn(world)
world = childSpawned["world"]
let childId = childSpawned["entity"]["id"]

let childTransform = createTransform(2.0, 0.0, 0.0)
world = add(world, childId, "Transform", childTransform)

// Set parent relationship
world = setParent(world, childId, rootId)

// Update hierarchy
world = updateHierarchy(world)

// Get world position of child (includes parent transform)
let childComp = get(world, childId, "Transform")
if childComp != null {
    let worldPos = getWorldPosition(childComp)
    // worldPos will be { x: 2.0, y: 0.0, z: 0.0 }
}

// Move root
let rootComp = get(world, rootId, "Transform")
if rootComp != null {
    rootComp = translate(rootComp, 5.0, 0.0, 0.0)
    world = add(world, rootId, "Transform", rootComp)
}

// Update hierarchy again
world = updateHierarchy(world)

// Child world position is now { x: 7.0, y: 0.0, z: 0.0 }
```

## Physics Integration Example

```kab
import "game/core/ecs2"
import "game/core/physics2"
import "game/core/gameContext"

let context = createGameContext(1920, 1080)

// Spawn ground
let groundSpawned = spawn(context["world"])
context["world"] = groundSpawned["world"]
let groundId = groundSpawned["entity"]["id"]

let groundCollider = {
    "kind": "BoxCollider",
    "center": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "size": { "x": 10.0, "y": 1.0, "z": 10.0 }
}
context["physicsWorld"] = addCollider(context["physicsWorld"], groundId, groundCollider)

// Spawn player
let playerSpawned = spawn(context["world"])
context["world"] = playerSpawned["world"]
let playerId = playerSpawned["entity"]["id"]

let playerRigidbody = {
    "kind": "Rigidbody",
    "mass": 1.0,
    "velocity": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "useGravity": true,
    "isKinematic": false
}
context["world"] = add(context["world"], playerId, "Rigidbody", playerRigidbody)

let playerCollider = {
    "kind": "BoxCollider",
    "center": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "size": { "x": 1.0, "y": 2.0, "z": 1.0 }
}
context["physicsWorld"] = addCollider(context["physicsWorld"], playerId, playerCollider)

// Game loop with physics
while true {
    let dt = 0.016
    
    context = updateGameContext(context, dt)
    context = fixedUpdateGameContext(context)
    
    // Check collisions
    let collisions = getCollisionPairs(context["physicsWorld"])
    let i = 0
    while i < len(collisions) {
        let collision = collisions[i]
        // Handle collision
        i = i + 1
    }
    
    // Raycast
    let hits = raycast(context["physicsWorld"], { "x": 0.0, "y": 10.0, "z": 0.0 }, { "x": 0.0, "y": -1.0, "z": 0.0 }, 20.0)
}
```

## Render Pipeline Example

```kab
import "game/core/renderPipeline"
import "game/core/gameContext"

let context = createGameContext(1920, 1080)

// Add camera
let camera = {
    "kind": "Camera",
    "position": { "x": 0.0, "y": 5.0, "z": -10.0 },
    "rotation": { "pitch": 0.2, "yaw": 0.0, "roll": 0.0 },
    "fieldOfView": 60.0,
    "nearClipPlane": 0.3,
    "farClipPlane": 1000.0,
    "aspectRatio": 1920.0 / 1080.0
}
context["renderPipeline"] = addCamera(context["renderPipeline"], camera)

// Add material
let material = createMaterial("Standard", "pbr.wgsl")
material = setMaterialProperty(material, "metallic", 0.5)
material = setMaterialProperty(material, "roughness", 0.3)
context["renderPipeline"] = addMaterial(context["renderPipeline"], "Standard", material)

// Add renderable
let bounds = {
    "min": { "x": -1.0, "y": -1.0, "z": -1.0 },
    "max": { "x": 1.0, "y": 1.0, "z": 1.0 }
}
let renderable = createRenderable(1, "cube.mesh", "Standard", null, bounds)
context["renderPipeline"] = addRenderable(context["renderPipeline"], renderable)

// Render
let renderResult = renderGameContext(context)
let drawCalls = renderResult["drawCalls"]

// Get stats
let stats = getRenderQueueStats(context["renderPipeline"])
```

## Performance Optimization Example

```kab
import "game/core/performance"
import "game/core/ecs2"

// Create object pool for vectors
let vectorPool = createVector3Pool(100)

// Use pooled vector in hot path
let v = poolGet(vectorPool)
v["x"] = 10.0
v["y"] = 20.0
v["z"] = 30.0

// Use v for calculations...

// Release back to pool
vectorPool = poolRelease(vectorPool, v)

// Create temp allocator for frame
let tempAlloc = createTempAllocator()
tempAlloc = tempMark(tempAlloc)

// Allocate temporary objects
let tempVec = tempAlloc(tempAlloc, { "x": 1.0, "y": 2.0, "z": 3.0 })

// Reset at end of frame
tempAlloc = tempReset(tempAlloc)

// Use ECS 2.0 for efficient queries
let world = createWorld()
let results = query2(world, "Transform", "Rigidbody")
// Returns only entities with both components, O(n) instead of O(n²)
```

## Complete Game Loop Example

```kab
import "game/core/gameContext"
import "game/core/baziIntegration"
import "game/core/sceneGraph"

// Initialize
let context = createGameContext(1920, 1080)
let behaviourSystem = createBehaviourSystem()
behaviourSystem = setSystemContext(behaviourSystem, context)

// Setup input actions
context["input"] = createActionMap(context["input"], "Player")
context["input"] = createAction(context["input"], "Player", "Move", { "type": "axis", "source": "gamepad", "axis": "leftStick" })
context["input"] = createAction(context["input"], "Player", "Jump", { "type": "key", "key": "Space" })

// Main loop
while true {
    let dt = 0.016
    
    // Update input
    context["input"] = updateInputSystem(context["input"])
    
    // Update game context
    context = updateGameContext(context, dt)
    
    // Pre-update behaviours
    behaviourSystem = setSystemPhase(behaviourSystem, "preUpdate")
    behaviourSystem = executeBehaviourSystem(behaviourSystem, context["world"], dt)
    
    // Fixed update physics
    context = fixedUpdateGameContext(context)
    
    // Post-physics behaviours
    behaviourSystem = setSystemPhase(behaviourSystem, "postPhysics")
    behaviourSystem = executeBehaviourSystem(behaviourSystem, context["world"], dt)
    
    // Update hierarchy
    context["world"] = updateHierarchy(context["world"])
    
    // Pre-render behaviours
    behaviourSystem = setSystemPhase(behaviourSystem, "preRender")
    behaviourSystem = executeBehaviourSystem(behaviourSystem, context["world"], dt)
    
    // Render
    let renderResult = renderGameContext(context)
    
    // Post-render behaviours
    behaviourSystem = setSystemPhase(behaviourSystem, "postRender")
    behaviourSystem = executeBehaviourSystem(behaviourSystem, context["world"], dt)
}
```

## Key Differences: Motor Only vs Motor + Bazi

### Motor Only
- Direct ECS 2.0 usage
- Manual component management
- Custom game loop
- No behaviour system
- More control, more boilerplate

### Motor + Bazi
- Behaviour system integration
- Prefab management
- Tag/layer/name components
- Automated lifecycle hooks
- Less boilerplate, more structure

Choose based on your needs:
- **Motor Only**: For maximum control and custom game architectures
- **Motor + Bazi**: For Unity-like workflow with behaviours and prefabs
