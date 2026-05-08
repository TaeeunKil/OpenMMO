import { getTerrainApiUrl } from '../utils/networkUtils'
import {
  decodeRiverFieldData,
  type RiverFieldTileData,
} from '../utils/river-field-data'
import { tileKey } from './terrain-height-types'

/** Per-tile RFD1 fetcher + decoder. Mirrors the pattern of the other
 *  per-tile binary loaders (grass / trees / splat) — fetch once, cache,
 *  return decoded channels. Texture construction lives in
 *  `river-quad-geometry.ts` next to the geometry helper. */
export class RiverFieldManager {
  private cache = new Map<string, RiverFieldTileData | null>()
  private inflight = new Map<string, Promise<RiverFieldTileData | null>>()
  private terrainApiUrl = getTerrainApiUrl()

  async loadRiverField(
    tileX: number,
    tileZ: number
  ): Promise<RiverFieldTileData | null> {
    const key = tileKey(tileX, tileZ)

    if (this.cache.has(key)) return this.cache.get(key) ?? null
    const existing = this.inflight.get(key)
    if (existing) return existing

    const promise = (async () => {
      try {
        const url = `${this.terrainApiUrl}/api/terrain/river-field/${tileX}/${tileZ}`
        const response = await fetch(url)
        if (response.status === 404) {
          this.cache.set(key, null)
          return null
        }
        if (!response.ok) {
          console.error(
            `Failed to load river field (${tileX}, ${tileZ}): ${response.status}`
          )
          return null
        }
        const buffer = await response.arrayBuffer()
        const data = decodeRiverFieldData(buffer)
        this.cache.set(key, data)
        return data
      } catch (e) {
        console.error(`River field fetch error (${tileX}, ${tileZ}):`, e)
        return null
      } finally {
        this.inflight.delete(key)
      }
    })()
    this.inflight.set(key, promise)
    return promise
  }
}
