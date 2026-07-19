import { writable } from 'svelte/store'

/** Y offset to raise entities above the house floor surface */
export const playerFloorOffset = writable(0)

/** Current floor level the player is on (-1 = outdoors, 0 = 1F, 1 = 2F) */
export const playerFloorLevel = writable(-1)

/**
 * Floor the player *visually* occupies, from the stairwell hysteresis alone
 * (GameSceneHousingLayer is the only writer). Unlike `playerFloorLevel`, which
 * movement also pre-sets to the next A* waypoint's floor before the climb
 * starts, this only flips once the player has actually risen. Use it for
 * anything about who can see whom — the wire `floor_level` and the remote
 * visibility gate — so a player doesn't vanish from the floor they're still
 * standing on.
 */
export const playerVisualFloorLevel = writable(-1)

/** ID of the house the player is currently inside, or null if outdoors */
export const playerInsideHouseId = writable<string | null>(null)
