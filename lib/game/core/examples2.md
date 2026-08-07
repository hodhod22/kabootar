# Game Engine Examples - GameContext + Bazi Integration

## Complete Game Loop with GameContext 2.0

This example shows a complete game loop using GameContext 2.0 with all systems integrated.

```kab
import "game/core/gameContext"
import "game/core/characterController"
import "game/core/sceneGraph"

// Initialize game context
let context = createGameContext(1920, 1080)

// Set up physics event callback
context["physicsWorld"] = setEventCallback(context["physicsWorld"], fn(eventType, eventData) {
    if eventType == "triggerEnter" {
        // Handle trigger enter
    } else if eventType == "triggerExit" {
        // Handle trigger exit
    }
})

// Spawn player
let playerSpawned = spawn(context["world"])
context["world"] = playerSpawned["world"]
let playerId = playerSpawned["entityId"]

// Add transform
let transform = createTransform(0.0, 1.0, 0.0)
context["world"] = add(context["world"], playerId, "Transform", transform)

// Add character controller
let charController = createCharacterController(playerId, 2.0, 0.5)
charController = setMoveSpeed(charController, 5.0)
charController = setJumpSpeed(charController, 8.0)

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

// Game loop
while true {
    let dt = 0.016
    
    // Update input
    context["input"] = setKeyDown(context["input"], "W")
    context["input"] = updateInputSystem(context["input"])
    
    // Update game context
    context = updateGameContext(context, dt)
    
    // Handle player movement
    let moveDir = { "x": 0.0, "y": 0.0, "z": 0.0 }
    if getKey(context["input"], "W") { moveDir["z"] = 1.0 }
    if getKey(context["input"], "S") { moveDir["z"] = -1.0 }
    if getKey(context["input"], "A") { moveDir["x"] = -1.0 }
    if getKey(context["input"], "D") { moveDir["x"] = 1.0 }
    if getKeyDown(context["input"], "Space") { charController = jump(charController) }
    
    charController = move(charController, context["physicsWorld"], moveDir, dt)
    
    // Sync transform with character controller
    let charPos = getPosition(charController)
    let playerTransform = get(context["world"], playerId, "Transform")
    if playerTransform != null {
        playerTransform = setLocalPosition(playerTransform, charPos["x"], charPos["y"], charPos["z"])
        context["world"] = add(context["world"], playerId, "Transform", playerTransform)
    }
    
    // Fixed update physics
    context = fixedUpdateGameContext(context)
    
    // Update hierarchy
    context["world"] = updateHierarchy(context["world"])
    
    // Render
    let renderResult = renderGameContext(context)
}
```

## GameContext + Bazi Behaviour System

This example shows how to integrate GameContext with Bazi's behaviour system.

```kab
import "game/core/gameContext"
import "game/core/baziIntegration"

// Initialize game context
let context = createGameContext(1920, 1080)

// Create behaviour system
let behaviourSystem = createBehaviourSystem()
behaviourSystem = setSystemContext(behaviourSystem, context)

// Spawn player with behaviour
let playerSpawned = spawn(context["world"])
context["world"] = playerSpawned["world"]
let playerId = playerSpawned["entityId"]

// Add transform
let transform = createTransform(0.0, 1.0, 0.0)
context["world"] = add(context["world"], playerId, "Transform", transform)

// Add behaviour component
let behaviour = createBehaviourComponent("PlayerController")
behaviour = setBehaviourHook(behaviour, "onUpdate", fn(world, entityId, dt, ctx) {
    let input = ctx["input"]
 let transform = get(world, entityId, "Transform")
    
    if transform != null {
        let moveDir = { "x": 0.0, "y": 0.0, "z": 0.0 }
        if getKey(input, "W") { moveDir["z"] = 1.0 }
        if getKey(input, "S") { moveDir["z"] = -1.0 }
        if getKey(input, "A") { moveDir["x"] = -1.0 }
        if getKey(input, "D") { moveDir["x"] = 1.0 }
        
        let speed = 5.0 * dt
        transform = translate(transform, moveDir["x"] * speed, 0.0, moveDir["z"] * speed)
        world = add(world, entityId, "Transform", transform)
    }
    
    return world
})

behaviour = setBehaviourHook(behaviour, "onSpawn", fn(world, entityId, ctx) {
    let name = createNameComponent("Player")
    let tag = createTagComponent("Player")
    world = add(world, entityId, "Name", name)
    world = add(world, entityId, "Tag", tag)
    return world
})

behaviour = setBehaviourHook(behaviour, "onCollisionEnter", fn(world, entityId, collision, ctx) {
    // Handle collision
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
    context["input"] = updateInputSystem(context["input"])
    
    // Update game context
    context = updateGameContext(context, dt)
    
    // Execute behaviours in update phase
    behaviourSystem = setSystemPhase(behaviourSystem, "update")
    behaviourSystem = executeBehaviourSystem(behaviourSystem, context["world"], dt)
    
    // Fixed update physics
    context = fixedUpdateGameContext(context)
    
    // Update hierarchy
    context["world"] = updateHierarchy(context["world"])
    
    // Render
    let renderResult = renderGameContext(context)
}
```

## Prefab System with GameContext

```kab
import "game/core/gameContext"
import "game/core/baziIntegration"

// Initialize game context and prefab manager
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

// Game loop
while true {
    let dt = 0.016
    context = updateGameContext(context, dt)
    context = fixedUpdateGameContext(context)
    context["world"] = updateHierarchy(context["world"])
    let renderResult = renderGameContext(context)
}
```

## Action Maps with GameContext

```kab
import "game/core/gameContext"

let context = createGameContext(1920, 1080)

// Set up action maps
context["input"] = createActionMap(context["input"], "Player")
context["input"] = createAction(context["input"], "Player", "Move", { "type": "axis", "source": "gamepad", "axis": "leftStick" })
context["input"] = createAction(context["input"], "Player", "Jump", { "type": "key", "key": "Space" })
context["input"] = createAction(context["input"], "Player", "Fire", { "type": "mouseButton", "button": "left" })

// Game loop
while true {
    let dt = 0.016
    
    // Update input
    context["input"] = updateInputSystem(context["input"])
    
    // Use action values
    let moveAxis = getActionValue(context["input"], "Player", "Move")
    let jumpPressed = isActionPressed(context["input"], "Player", "Jump")
    let firePressed = isActionPressed(context["input"], "Player", "Fire")
    
    context = updateGameContext(context, dt)
    context = fixedUpdateGameContext(context)
    let renderResult = renderGameContext(context)
}
```

## Instancing with GameContext

```kab
import "game/core/gameContext"
import "game/core/instancing"

let context = createGameContext(1920, 1080)

// Create instancing system
context["instancingSystem"] = createInstancingSystem()

// Add multiple renderables with same mesh/material
let i = 0
while i < 100 {
    let spawned = spawn(context["world"])
    context["world"] = spawned["world"]
    let entityId = spawned["entityId"]
    
    let transform = createTransform(i * 2.0, 0.0, 0.0)
    context["world"] = add(context["world"], entityId, "Transform", transform)
    
    let renderable = createRenderable(entityId, "tree.mesh", "tree.mat", transform, null)
    context["renderPipeline"] = addRenderable(context["renderPipeline"], renderable)
    
    // Add to instancing system
    context["instancingSystem"] = addRenderableForInstancing(context["instancingSystem"], renderable)
    
    i = i + 1
}

// Game loop
while true {
    let dt = 0.016
    context = updateGameContext(context, dt)
    context = fixedUpdateGameContext(context)
    
    // Instancing is automatically merged during render
    let renderResult = renderGameContext(context)
    
    let stats = getInstancingStats(context["instancingSystem"])
}
```

## Trigger Events with GameContext

```kab
import "game/core/gameContext"

let context = createGameContext(1920, 1080)

// Set up physics event callback
context["physicsWorld"] = setEventCallback(context["physicsWorld"], fn(eventType, eventData) {
    if eventType == "triggerEnter" {
        let trigger = eventData["trigger"]
        let collider = eventData["collider"]
        // Handle trigger enter
    } else if eventType == "triggerExit" {
        // Handle trigger exit
    }
})

// Spawn trigger zone
let triggerSpawned = spawn(context["world"])
context["world"] = triggerSpawned["world"]
let triggerId = triggerSpawned["entityId"]

let trigger = {
    "kind": "BoxCollider",
    "center": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "size": { "x": 5.0, "y": 2.0, "z": 5.0 }
}
context["physicsWorld"] = addTrigger(context["physicsWorld"], triggerId, trigger)

// Spawn player
let playerSpawned = spawn(context["world"])
context["world"] = playerSpawned["world"]
let playerId = playerSpawned["entityId"]

let playerCollider = {
    "kind": "BoxCollider",
    "center": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "size": { "x": 1.0, "y": 2.0, "z": 1.0 }
}
context["physicsWorld"] = addCollider(context["physicsWorld"], playerId, playerCollider)

// Game loop
while true {
    let dt = 0.016
    context = updateGameContext(context, dt)
    context = fixedUpdateGameContext(context)
    // Trigger events are automatically emitted to event system
    let renderResult = renderGameContext(context)
}
```

## Material System with GameContext

```kab
import "game/core/gameContext"

let context = createGameContext(1920, 1080)

// Create materials
let standardMaterial = createMaterial("Standard", "pbr.wgsl")
standardMaterial = setMaterialProperty(standardMaterial, "metallic", 0.5)
standardMaterial = setMaterialProperty(standardMaterial, "roughness", 0.3)
standardMaterial = setMaterialTexture(standardMaterial, "albedo", "albedo.png")
context["renderPipeline"] = addMaterial(context["renderPipeline"], "Standard", standardMaterial)

let transparentMaterial = createMaterial("Glass", "pbr.wgsl")
transparentMaterial = setMaterialTransparent(transparentMaterial, true)
transparentMaterial = set materialProperty(transparentMaterial, "roughness", 0.1)
transparentMaterial = setMaterialBlendMode(transparentMaterial, "alpha")
context["renderPipeline"] = addMaterial(context["renderPipeline"], "Glass", transparentMaterial)

// Add renderables with different materials
let opaqueRenderable = createRenderable(1, "cube.mesh", "Standard", null, null)
context["renderPipeline"] = addRenderable(context["renderPipeline"], opaqueRenderable)

let transparentRenderable = createRenderable(2, "window.mesh", "Glass", null, null)
context["renderPipeline"] = addRenderable(context["renderPipeline"], transparentRenderable)

// Game loop
while true {
    let dt = 0.016
    context = updateGameContext(context, dt)
    context = fixedUpdateGameContext(context)
    let renderResult = renderGameContext(context)
    // Opaque and transparent are automatically sorted correctly
}
```

## Hierarchical Scene with GameContext

```kab
import "game/core/gameContext"
import "game/core/sceneGraph"

let context = createGameContext(1920, 1080)

// Spawn root entity
let rootSpawned = spawn(context["world"])
context["world"] = rootSpawned["world"]
let rootId = rootSpawned["entityId"]

let rootTransform = createTransform(0.0, 0.0, 0.0)
context["world"] = add(context["world"], rootId, "Transform", rootTransform)

// Spawn child entity
let childSpawned = spawn(context["world"])
context["world"] = childSpawned["world"]
let childId = childSpawned["entityId"]

let childTransform = createTransform(2.0, 0.0, 0.0)
context["world"] = add(context["world"], childId, "Transform", childTransform)

// Set parent relationship
context["world"] = setParent(context["world"], childId, rootId)

// Game loop
while true {
    let dt = 0.016
    context = updateGameContext(context, dt)
    
    // Update hierarchy
    context["world"] = updateHierarchy(context["world"])
    
    // Move root
    let rootComp = get(context["world"], rootId, "Transform")
    if rootComp != null {
        rootComp = translate(rootComp, 0.1, 0.0, 0.0)
        context["world"] = add(context["world"], rootId, "Transform", rootComp)
    }
    
    context = fixedUpdateGameContext(context)
    let renderResult = renderGameContext(context)
}
```

## Complete Platformer Example

```kab
import "game/core/gameContext"
import "game/core/characterController"
import "game/core/sceneGraph"
import "game/core/baziIntegration"

let context = createGameContext(1920, 1080)

// Spawn player
let playerSpawned = spawn(context["world"])
context["world"] = playerSpawned["world"]
let playerId = playerSpawned["entityId"]

let transform = createTransform(0.0, 2.0, 0.0)
context["world"] = add(context["world"], playerId, "Transform", transform)

let charController = createCharacterController(playerId, 2.0, 0.5)
charController = setMoveSpeed(charController, 6.0)
charController = setJumpSpeed(charController, 10.0)

// Add player behaviour
let behaviour = createBehaviourComponent("PlayerBehaviour")
behaviour = setBehaviourHook(behaviour, "onUpdate", fn(world, entityId, dt, ctx) {
    let input = ctx["input"]
    let transform = get(world, entityId, "Transform")
    
    if transform != null {
        let moveDir = { "x": 0.0, "y": 0.0, "z": 0.0 }
        if getKey(input, "A") { moveDir["x"] = -1.0 }
        if getKey(input, "D") { moveDir["x"] = 1.0 }
        
        return world
    }
    return world
})

context["world"] = add(context["world"], playerId, "Behaviour", behaviour)

// Spawn ground
let groundSpawned = spawn(context["world"])
context["world"] = groundSpawned["world"]
let groundId = groundSpawned["entityId"]

let groundTransform = createTransform(0.0, -1.0, 0.0)
context["world"] = add(context["world"], groundId, "Transform", groundTransform)

let groundCollider = {
    "kind": "BoxCollider",
    "center": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "size": { "x": 20.0, "y": 1.0, "z": 20.0 }
}
context["physicsWorld"] = addCollider(context["physicsWorld"], groundId, groundCollider)

// Add camera
let camera = {
    "kind": "Camera",
    "position": { "x": 0.0, "y": 5.0, "z": -15.0 },
    "rotation": { "pitch": 0.2, "yaw": 0.0, "roll": 0.0 },
    "fieldOfView": 60.0,
    "nearClipPlane": 0.3,
    "farClipPlane": 1000.0,
    "aspectRatio": 1920.0 / 1080.0
}
context["renderPipeline"] = addCamera(context["renderPipeline"], camera)

// Game loop
while true {
    let dt = 0.016
    
    // Update input
    context["input"] = setKeyDown(context["input"], "D")
    context["input"] = updateInputSystem(context["input"])
    
    // Update game context
    context = updateGameContext(context, dt)
    
    // Handle player movement
    let moveDir = { "x": 0.0, "y": 0.0, "z": 0.0 }
    if getKey(context["input"], "A") { moveDir["x"] = -1.0 }
    if getKey(context["input"], "D") { moveDir["x"] = 1.0 }
    if getKeyDown(context["input"], "Space") { charController = jump(charController) }
    
    charController = move(charController, context["physicsWorld"], moveDir, dt)
    
    // Sync transform
    let charPos = getPosition(charController)
    let playerTransform = get(context["world"], playerId, "Transform")
    if playerTransform != null {
        playerTransform = setLocalPosition(playerTransform, charPos["x"], charPos["y"], charPos["z"])
        context["world"] = add(context["world"], playerId, "Transform", playerTransform)
    }
    
    // Fixed update physics
    context = fixedUpdateGameContext(context)
    
    // Update hierarchy
    context["world"] = updateHierarchy(context["world"])
    
    // Render
    let renderResult = renderGameContext(context)
}
```

## Performance Monitoring

```kab
import "game/core/gameContext"

let context = createGameContext(1920, 1080)

// Game loop with stats
while true {
    let dt = 0.016
    let startTime = context["time"]["realtimeSinceStartup"]
    
    context = updateGameContext(context, dt)
    context = fixedUpdateGameContext(context)
    context["world"] = updateHierarchy(context["world"])
    let renderResult = renderGameContext(context)
    
    let endTime = context["time"]["realtimeSinceStartup"]
    let frameTime = endTime - startTime
    
    // Get stats
    let worldStats = getWorldStats(context["world"])
    let renderStats = getRenderQueueStats(context["renderPipeline"])
    let instancingStats = getInstancingStats(context["instancingSystem"])
}
```
