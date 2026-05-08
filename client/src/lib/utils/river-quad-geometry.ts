import * as THREE from 'three'
import {
  createTerrainGeometry,
  TERRAIN_TILE_SIZE,
} from '../components/game-scene/terrain-utils'
import { RIVER_FIELD_GRID, type RiverFieldTileData } from './river-field-data'

/** 65×65 flat-quad geometry covering one tile, ready for vertex Y to be
 *  driven by a baked `surfaceY` field via {@link applyRiverFieldToGeometry}.
 *  Layout matches the terrain geometry so heightmap UVs line up. */
export function createRiverQuadGeometry(): THREE.BufferGeometry {
  return createTerrainGeometry(TERRAIN_TILE_SIZE, RIVER_FIELD_GRID - 1)
}

/** Copy `surfaceY` row-major into vertex Y, then refresh the bounding
 *  sphere so isometric raycasts don't early-reject elevated meshes
 *  (same fix `applyHeightToGeometry` applies to terrain). */
export function applyRiverFieldToGeometry(
  geometry: THREE.BufferGeometry,
  field: RiverFieldTileData
): void {
  const G = RIVER_FIELD_GRID
  const posAttr = geometry.getAttribute('position') as THREE.BufferAttribute
  const positions = posAttr.array as Float32Array
  for (let vz = 0; vz < G; vz++) {
    const row = vz * G
    for (let vx = 0; vx < G; vx++) {
      positions[(row + vx) * 3 + 1] = field.surfaceY[row + vx]
    }
  }
  posAttr.needsUpdate = true
  geometry.computeBoundingSphere()
  geometry.computeBoundingBox()
}

/** Build a 65×65 RGBA32F DataTexture from a decoded river field. R =
 *  surfaceY (m), GB = unit downstream flow vector, A reserved. flipY
 *  matches the heightmap so `toHeightmapUV(uv())` works for both. */
export function buildRiverFieldTexture(
  data: RiverFieldTileData
): THREE.DataTexture {
  const W = RIVER_FIELD_GRID
  const buf = new Float32Array(W * W * 4)
  for (let i = 0; i < W * W; i++) {
    buf[i * 4 + 0] = data.surfaceY[i]
    buf[i * 4 + 1] = data.flowX[i]
    buf[i * 4 + 2] = data.flowZ[i]
    buf[i * 4 + 3] = 1
  }
  const tex = new THREE.DataTexture(
    buf,
    W,
    W,
    THREE.RGBAFormat,
    THREE.FloatType
  )
  tex.flipY = true
  tex.minFilter = THREE.LinearFilter
  tex.magFilter = THREE.LinearFilter
  tex.needsUpdate = true
  return tex
}
