import { describe, it, expect } from 'vitest'
import { houseFloorHeightAt } from './housing-queries'
import type { HouseData, RoomData } from '../types/housing'

const WALL_HEIGHT = 3
const FLOOR_THICKNESS = 0.1

const room = (over: Partial<RoomData>): RoomData =>
  ({
    roomType: 'normal',
    floorLevel: 0,
    localX: 0,
    localZ: 0,
    sizeX: 4,
    sizeZ: 6,
    wallHeight: WALL_HEIGHT,
    floorTexture: 0,
    wallNorth: [],
    wallSouth: [],
    wallEast: [],
    wallWest: [],
    ...over,
  }) as RoomData

/** r-23_+73_1's shape: two stacked rooms with a 1x4 stairwell inside them. */
function house(): ReadonlyMap<string, HouseData> {
  const h = {
    id: 'h',
    origin: { x: 0, y: 0, z: 0 },
    rooms: [
      room({ floorLevel: 0 }),
      room({ floorLevel: 1 }),
      room({
        roomType: 'stairwell',
        floorLevel: 0,
        localX: 3,
        localZ: 0,
        sizeX: 1,
        sizeZ: 4,
      }),
    ],
  } as unknown as HouseData
  return new Map([['h', h]])
}

const at = (floor: number, x: number, z: number) =>
  houseFloorHeightAt(house(), floor, x, z)

describe('houseFloorHeightAt', () => {
  it('rises along the ramp as a floor-0 climber advances', () => {
    // The climber reports floor 0 until the sender's hysteresis flips, so every
    // sample here shares a floor with the flat ground-floor room underneath.
    const bottom = at(0, 3.5, 0.25)!
    const mid = at(0, 3.5, 2)!
    const top = at(0, 3.5, 3.75)!

    expect(bottom).toBeCloseTo(FLOOR_THICKNESS / 2, 5)
    expect(mid).toBeGreaterThan(bottom)
    expect(top).toBeGreaterThan(mid)
  })

  it('does not latch to the flat floor once the ramp lifts off', () => {
    // The regression: a continuity tiebreak against the entity's previous Y
    // always preferred the flat floor, because the flat floor is exactly where
    // the climber just was. The remote then walked *under* the stairs.
    const flat = at(0, 1, 2)!
    expect(at(0, 3.5, 2)).toBeGreaterThan(flat + 0.5)
  })

  it('meets the upper floor exactly at the top landing', () => {
    const landing = at(1, 3.5, 3.9)!
    const upperRoom = at(1, 1, 2)!
    expect(landing).toBeCloseTo(upperRoom, 5)
    expect(upperRoom).toBeCloseTo(
      WALL_HEIGHT + FLOOR_THICKNESS + FLOOR_THICKNESS / 2,
      5
    )
  })

  it('returns the flat floor away from the stairwell, and null off-house', () => {
    expect(at(0, 1, 2)).toBeCloseTo(FLOOR_THICKNESS / 2, 5)
    expect(at(0, 99, 99)).toBeNull()
  })
})
