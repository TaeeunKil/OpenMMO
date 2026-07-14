<script lang="ts">
  import { T } from '@threlte/core'
  import * as THREE from 'three'
  import type { TerrainTile } from './terrain-utils'
  import { parseTileId } from './terrain-utils'
  import type { WaterFieldManager } from '../../managers/waterFieldManager'
  import type { TerrainHeightManager } from '../../managers/terrainHeightManager'
  import {
    ShoreSpraySystem,
    computeShoreCells,
    computeCrestPhases,
    SHORE_SPRAY_EMIT_RADIUS_M,
    SHORE_SPRAY_MAX_ACTIVE,
    type ShoreCell,
  } from '../../effects/shore-spray'
  import { enqueueTileWork } from '../../utils/tileWorkQueue'

  /**
   * EXPERIMENTAL: whitewater spray on the breaking-wave crest. Extracts the
   * shore cells across the crest's depth travel per sea tile (shore-spray.ts)
   * and pulses billboard droplets along the moving crest while a wave breaks,
   * in phase with the water shader's swell cycle. Pure client-side dressing —
   * no server state.
   */

  interface Props {
    terrainTiles: TerrainTile[]
    waterFieldManager?: WaterFieldManager | null
    heightManager?: TerrainHeightManager | null
    foamMap?: THREE.Texture | null
    sunDirection?: THREE.Vector3 | null
    playerPosition?: { x: number; y: number; z: number } | null
  }

  let {
    terrainTiles,
    waterFieldManager = null,
    heightManager = null,
    foamMap = null,
    sunDirection = null,
    playerPosition = null,
  }: Props = $props()

  const group = new THREE.Group()
  group.name = 'shoreSpray'

  export function getGroup(): THREE.Group {
    return group
  }

  let system: ShoreSpraySystem | null = null

  function ensureSystem(): boolean {
    if (!foamMap) return false
    if (!system) {
      system = new ShoreSpraySystem(foamMap)
      group.add(system.mesh)
    }
    return true
  }

  // ── Per-tile shore-cell state ──
  /* eslint-disable-next-line svelte/prefer-svelte-reactivity */
  const tileCells = new Map<string, ShoreCell[]>()
  /* eslint-disable-next-line svelte/prefer-svelte-reactivity */
  const emptyTiles = new Set<string>()
  /* eslint-disable-next-line svelte/prefer-svelte-reactivity */
  const inflightTiles = new Set<string>()

  async function loadTile(id: string, tileX: number, tileZ: number) {
    if (
      inflightTiles.has(id) ||
      tileCells.has(id) ||
      emptyTiles.has(id) ||
      !waterFieldManager ||
      !heightManager
    )
      return
    inflightTiles.add(id)
    try {
      const hm = heightManager
      const [field, heightsOk] = await Promise.all([
        waterFieldManager.loadWaterField(tileX, tileZ),
        hm
          .loadHeightmap(tileX, tileZ)
          .then(() => true)
          .catch(() => false),
      ])
      // `field` may be null (404 = no river influence) — that's flat open
      // sea, the common breaking-wave case, so we still extract cells from
      // the heightmap. Bed heights ARE required, though: without them the
      // depth filter runs against bed=0. Retry on the next pass.
      if (!heightsOk) return
      const cells = computeShoreCells(field, tileX, tileZ, (x, z) =>
        hm.getHeightAtWorldPosition(x, z)
      )
      if (cells.length === 0) {
        emptyTiles.add(id)
        return
      }
      enqueueTileWork(() => {
        // Re-check: the tile may have scrolled out during the queue wait.
        if (!emptyTiles.has(id)) tileCells.set(id, cells)
      })
    } finally {
      inflightTiles.delete(id)
    }
  }

  $effect(() => {
    if (!waterFieldManager || !heightManager || !foamMap) return
    ensureSystem()
    const currentIds = new Set(terrainTiles.map((t) => t.id))
    for (const id of [...tileCells.keys()]) {
      if (!currentIds.has(id)) tileCells.delete(id)
    }
    for (const id of [...emptyTiles]) {
      if (!currentIds.has(id)) emptyTiles.delete(id)
    }
    for (const tile of terrainTiles) {
      const coords = parseTileId(tile.id)
      if (!coords) continue
      void loadTile(tile.id, coords.tileX, coords.tileZ)
    }
  })

  // ── Per-frame ──
  const activeCells: ShoreCell[] = []

  /** Called from GameScene's game loop each frame (deltaTime in ms).
   *  `waterTime` MUST be the same time driving the water shader's uTime,
   *  or the spray pulses desync from the visible whitecaps. */
  export function update(
    deltaTime: number,
    camera: THREE.Camera | undefined,
    waterTime: number
  ) {
    const dt = Math.min(deltaTime / 1000, 0.1)
    if (!camera || !system) return

    // Same day/night response as the water foam it decorates.
    const sunY = sunDirection?.y ?? 1
    const s = Math.min(Math.max((sunY + 0.05) / 0.15, 0), 1)
    system.setDayDim(0.1 + 0.9 * (s * s * (3 - 2 * s)))

    const phases = computeCrestPhases(waterTime)
    const anyBreaking = phases.some((p) => p.activity > 0.001)
    activeCells.length = 0
    if (playerPosition && anyBreaking) {
      const r2 = SHORE_SPRAY_EMIT_RADIUS_M * SHORE_SPRAY_EMIT_RADIUS_M
      outer: for (const cells of tileCells.values()) {
        for (const c of cells) {
          const dx = c.x - playerPosition.x
          const dz = c.z - playerPosition.z
          if (dx * dx + dz * dz < r2) {
            activeCells.push(c)
            if (activeCells.length >= SHORE_SPRAY_MAX_ACTIVE) break outer
          }
        }
      }
    }
    // Always call update so live droplets age out even when no wave is
    // breaking (activeCells empty → no new spawns).
    system.update(dt, camera, activeCells, phases)
  }
</script>

<T is={group} />
