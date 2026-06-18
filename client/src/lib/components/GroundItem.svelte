<script lang="ts">
  import { T } from '@threlte/core'
  import { onDestroy } from 'svelte'
  import * as THREE from 'three'
  import { getItemDef } from '../data/itemDefs'
  import { getWeaponModelPath } from '../utils/modelPaths'
  import { loadGLB } from '../utils/gltfCache'
  import { localPlayerRightHand } from '../stores/playerHandRegistry'
  import { createGroundItemGlowMaterial } from '../shaders/ground-item-glow-material'
  import type { TerrainHeightManager } from '../managers/terrainHeightManager'
  import {
    evaluateSpawnAnimation,
    type GroundItemData,
  } from '../managers/groundItemManager'

  interface Props {
    data: GroundItemData
    rotation?: number
    animationTimeMs?: number
    heightManager?: TerrainHeightManager
  }

  let {
    data,
    rotation = 0,
    animationTimeMs = 0,
    heightManager,
  }: Props = $props()

  const def = $derived(getItemDef(data.itemDefId))
  const label = $derived(def?.name ?? data.itemDefId)
  const UP = new THREE.Vector3(0, 1, 0)
  const TERRAIN_NORMAL_SAMPLE_DISTANCE = 0.75
  const MAX_TERRAIN_Y_DELTA_FOR_TILT = 0.75

  const { material: outlineGlowMaterial, uniforms: outlineGlowUniforms } =
    createGroundItemGlowMaterial()

  let worldModelScene: THREE.Object3D | undefined = $state()
  let outlineGlowScene: THREE.Object3D | undefined = $state()
  let groundParentRef: THREE.Group | undefined = $state()
  let terrainAlignedRef: THREE.Group | undefined = $state()

  // Self-animating loot (the dungeon coin pile): the GLB ships a spill/settle
  // clip that plays once on spawn, so the pile pours out of the chest and lands
  // instead of using the generic loot arc. The mixer is advanced by real frame
  // deltas (LoopOnce + clampWhenFinished holds the settled pose) — seeking with
  // setTime each frame would snap a finished, paused action back to frame 0.
  // `selfAnimated` flips on once the clip is set up; `poured` flips on when it
  // finishes (the glow waits for that so it outlines the settled pile, not the
  // mid-air coins).
  let selfMixer: THREE.AnimationMixer | undefined
  let selfClipLastMs = 0
  let selfAnimated = $state(false)
  let poured = $state(false)

  function cloneScene(
    scene: THREE.Object3D,
    onMesh: (mesh: THREE.Mesh) => void
  ): THREE.Object3D {
    const clone = scene.clone(true)
    clone.traverse((child) => {
      if (child instanceof THREE.Mesh) onMesh(child)
    })
    return clone
  }

  function cloneGroundItemScene(scene: THREE.Object3D): THREE.Object3D {
    return cloneScene(scene, (mesh) => {
      mesh.castShadow = true
      mesh.receiveShadow = true
    })
  }

  function cloneOutlineGlowScene(scene: THREE.Object3D): THREE.Object3D {
    return cloneScene(scene, (mesh) => {
      mesh.material = outlineGlowMaterial
      mesh.castShadow = false
      mesh.receiveShadow = false
      mesh.renderOrder = 1
    })
  }

  /** Bind a clip to a freshly cloned scene as a one-shot (LoopOnce + clamp) and
   *  return the mixer. The clip is node-based TRS (no skeleton), so the clone's
   *  named nodes bind directly. Pass `holdAtEnd` to jump straight to the settled
   *  pose (used for the glow outline, which we don't animate frame-by-frame). */
  function bindClipOnce(
    scene: THREE.Object3D,
    clip: THREE.AnimationClip,
    holdAtEnd: boolean
  ): THREE.AnimationMixer {
    const mixer = new THREE.AnimationMixer(scene)
    const action = mixer.clipAction(clip)
    action.loop = THREE.LoopOnce
    action.clampWhenFinished = true
    action.play()
    if (holdAtEnd) mixer.setTime(clip.duration)
    return mixer
  }

  function getTerrainAlignmentQuaternion(
    worldX: number,
    worldY: number,
    worldZ: number,
    shouldTilt: boolean
  ): THREE.Quaternion {
    if (!shouldTilt || !heightManager?.hasHeightData(worldX, worldZ)) {
      return new THREE.Quaternion()
    }

    const d = TERRAIN_NORMAL_SAMPLE_DISTANCE
    if (
      !heightManager.hasHeightData(worldX - d, worldZ) ||
      !heightManager.hasHeightData(worldX + d, worldZ) ||
      !heightManager.hasHeightData(worldX, worldZ - d) ||
      !heightManager.hasHeightData(worldX, worldZ + d)
    ) {
      return new THREE.Quaternion()
    }

    const terrainY = heightManager.getHeightAtWorldPosition(worldX, worldZ)
    if (Math.abs(worldY - terrainY) > MAX_TERRAIN_Y_DELTA_FOR_TILT) {
      return new THREE.Quaternion()
    }

    const hL = heightManager.getHeightAtWorldPosition(worldX - d, worldZ)
    const hR = heightManager.getHeightAtWorldPosition(worldX + d, worldZ)
    const hB = heightManager.getHeightAtWorldPosition(worldX, worldZ - d)
    const hF = heightManager.getHeightAtWorldPosition(worldX, worldZ + d)
    const normal = new THREE.Vector3(hL - hR, 2 * d, hB - hF).normalize()
    return new THREE.Quaternion().setFromUnitVectors(UP, normal)
  }

  $effect(() => {
    const worldModel = def?.worldModel
    if (!worldModel) {
      worldModelScene = undefined
      outlineGlowScene = undefined
      return
    }
    let cancelled = false
    let loadedScene: THREE.Object3D | undefined
    let loadedGlowScene: THREE.Object3D | undefined
    selfMixer = undefined
    selfClipLastMs = 0
    selfAnimated = false
    poured = false
    const path = getWeaponModelPath(worldModel)
    loadGLB(path).then((gltf) => {
      if (cancelled) return
      const scene = cloneGroundItemScene(gltf.scene)
      const glowScene = cloneOutlineGlowScene(gltf.scene)
      // An animated loot model (e.g. the coin pile) plays its spill/settle clip
      // once on spawn: the model is ticked frame-by-frame (see the mixer effect)
      // while the glow outline jumps to the settled pose and only shows once the
      // pour finishes (`poured`, set by the mixer's own `finished` event).
      // Static models have no clip and are unaffected.
      const clip = gltf.animations[0]
      if (clip) {
        selfMixer = bindClipOnce(scene, clip, false)
        selfMixer.addEventListener('finished', () => {
          poured = true
        })
        selfAnimated = true
        bindClipOnce(glowScene, clip, true)
      }
      loadedScene = scene
      loadedGlowScene = glowScene
      worldModelScene = scene
      outlineGlowScene = glowScene
    })
    return () => {
      cancelled = true
      selfMixer?.stopAllAction()
      selfMixer = undefined
      if (loadedScene?.parent) loadedScene.parent.remove(loadedScene)
      if (loadedGlowScene?.parent) loadedGlowScene.parent.remove(loadedGlowScene)
    }
  })

  // Tick the coin pile's spill clip only while it's still pouring. Gating on the
  // reactive `selfAnimated`/`poured` before reading the frame clock means Svelte
  // never schedules this for static loot, and stops re-running it once a pile
  // settles (the `finished` event flips `poured`, which clamps the final pose).
  $effect(() => {
    if (!selfAnimated || poured) return
    const now = animationTimeMs
    if (!selfMixer) return
    if (selfClipLastMs === 0) selfClipLastMs = now
    const dt = (now - selfClipLastMs) / 1000
    selfClipLastMs = now
    if (dt > 0) selfMixer.update(dt)
  })

  $effect(() => {
    const scene = worldModelScene
    const ground = groundParentRef
    if (!scene || !ground) return
    // A spread-out coin pile held up to the face reads as an awkward flat slab,
    // so self-animating loot is never parented to the hand. Instead it just
    // vanishes the instant the pickup "grabs" it (data.inHand) — the same moment
    // a normal item snaps to the hand — rather than lingering on the ground
    // until the gesture finishes.
    if (selfAnimated) {
      if (scene.parent !== ground) {
        scene.position.set(0, 0, 0)
        scene.rotation.set(0, 0, 0)
        ground.add(scene)
      }
      scene.visible = !data.inHand
      return
    }
    const hand = data.inHand ? $localPlayerRightHand : null
    const targetParent = hand ?? ground
    if (scene.parent === targetParent) return
    scene.position.set(0, hand ? 0.08 : 0, 0)
    scene.rotation.set(0, 0, 0)
    targetParent.add(scene)
  })

  $effect(() => {
    const scene = outlineGlowScene
    const ground = groundParentRef
    if (!scene || !ground) return
    if (!showGlow) {
      if (scene.parent) scene.parent.remove(scene)
      return
    }
    if (scene.parent !== ground) ground.add(scene)
    scene.position.set(0, 0, 0)
    scene.rotation.set(0, 0, 0)
  })

  function makeNameTexture(text: string): THREE.CanvasTexture {
    const c = document.createElement('canvas')
    c.width = 256
    c.height = 64
    const ctx = c.getContext('2d')!
    ctx.fillStyle = 'rgba(0,0,0,0.6)'
    ctx.fillRect(0, 0, 256, 64)
    ctx.font = 'bold 28px Courier New'
    ctx.fillStyle = '#f0c040'
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(text, 128, 32)
    return new THREE.CanvasTexture(c)
  }

  const nameTexture = $derived(
    def?.worldModel || worldModelScene ? null : makeNameTexture(label)
  )

  onDestroy(() => {
    nameTexture?.dispose()
    outlineGlowMaterial.dispose()
  })

  // Self-animating loot pours in via its own clip, so it skips the generic loot
  // arc and rests near floor level (the clip bakes its own rise/fall) instead of
  // the usual +0.3 hover. The settled coin pile's lowest coins sink ~0.036m
  // below the model origin, so a small lift keeps them from clipping under the
  // floor. Outdoor terrain (non-negative floor) gets a touch more lift than a
  // dungeon's flat floor (negative floor level) to keep the wide pile from
  // clipping into small ground rises.
  const SELF_ANIM_REST_HOVER_TERRAIN = 0.03
  const SELF_ANIM_REST_HOVER_DUNGEON = 0.01
  const spawnTransform = $derived(
    data.spawnAnimation && !data.inHand && !selfAnimated
      ? evaluateSpawnAnimation(data.spawnAnimation, animationTimeMs)
      : null
  )
  const restHover = $derived(
    selfAnimated
      ? data.floorLevel < 0
        ? SELF_ANIM_REST_HOVER_DUNGEON
        : SELF_ANIM_REST_HOVER_TERRAIN
      : 0.3
  )
  const displayX = $derived(data.position.x + (spawnTransform?.offsetX ?? 0))
  const displayY = $derived(
    data.position.y + restHover + (spawnTransform?.offsetY ?? 0)
  )
  const displayZ = $derived(data.position.z + (spawnTransform?.offsetZ ?? 0))
  const shouldTiltToTerrain = $derived(!data.inHand && !spawnTransform)
  // Depends only on the (post-animation, constant) display position and tilt
  // flag — so a resting item computes its terrain alignment once and stops,
  // rather than re-running terrain height lookups every frame.
  const terrainAlignmentQuaternion = $derived(
    getTerrainAlignmentQuaternion(
      displayX,
      data.position.y,
      displayZ,
      shouldTiltToTerrain
    )
  )
  const glowPulse = $derived(
    0.5 + Math.sin(animationTimeMs * 0.004 + data.instanceId) * 0.5
  )
  const outlineGlowOpacity = $derived(0.22 + glowPulse * 0.12)
  const outlineGlowScale = $derived(1.03 + glowPulse * 0.008)
  const outlineGlowShellOffset = $derived(0.044 + glowPulse * 0.012)
  // Hold the glow until a self-animating pile has settled, so the outline wraps
  // the resting coins rather than the ones still pouring through the air.
  const showGlow = $derived(!data.inHand && (!selfAnimated || poured))

  $effect(() => {
    terrainAlignedRef?.quaternion.copy(terrainAlignmentQuaternion)
  })

  $effect(() => {
    if (!showGlow) return
    outlineGlowUniforms.uTime.value = animationTimeMs / 1000
    outlineGlowUniforms.uOpacity.value = outlineGlowOpacity
    outlineGlowUniforms.uShellOffset.value = outlineGlowShellOffset
    outlineGlowScene?.scale.setScalar(outlineGlowScale)
  })
</script>

<T.Group
  position.x={displayX}
  position.y={displayY}
  position.z={displayZ}
  userData={{ groundItemId: data.instanceId }}
>
  <T.Group bind:ref={terrainAlignedRef}>
    <T.Group
      rotation.y={data.restingRotationY + (worldModelScene || data.spawnAnimation ? 0 : rotation)}
      rotation.z={spawnTransform?.spinZ ?? 0}
    >
      <T.Group bind:ref={groundParentRef} />

      {#if !worldModelScene}
        {#if showGlow}
          <T.Mesh scale={[outlineGlowScale, outlineGlowScale, outlineGlowScale]} renderOrder={1}>
            <T.BoxGeometry args={[0.3, 0.3, 0.3]} />
            <T is={outlineGlowMaterial} />
          </T.Mesh>
        {/if}

        <T.Mesh>
          <T.BoxGeometry args={[0.3, 0.3, 0.3]} />
          <T.MeshStandardMaterial color="#f0c040" />
        </T.Mesh>

        {#if nameTexture}
          <T.Sprite position.y={0.5} scale={[label.length * 0.08, 0.2, 1]}>
            <T.SpriteMaterial map={nameTexture} transparent={true} />
          </T.Sprite>
        {/if}
      {/if}
    </T.Group>
  </T.Group>
</T.Group>
