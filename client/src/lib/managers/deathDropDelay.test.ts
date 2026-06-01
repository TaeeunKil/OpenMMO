import { describe, expect, it, vi } from 'vitest'
import { DeathDropDelayQueue } from './deathDropDelay'

describe('DeathDropDelayQueue', () => {
  it('holds a matching death drop until the monster impact is released', () => {
    const queue = new DeathDropDelayQueue()
    const spawn = vi.fn()

    queue.expectDrop('m1', 100)
    queue.handleSpawn('m1', 1, spawn, 150)

    expect(spawn).not.toHaveBeenCalled()

    queue.releaseForMonster('m1')

    expect(spawn).toHaveBeenCalledTimes(1)
    expect(queue.size).toBe(0)
  })

  it('spawns immediately if the impact was already released', () => {
    const queue = new DeathDropDelayQueue()
    const spawn = vi.fn()

    queue.expectDrop('m1', 100)
    queue.releaseForMonster('m1')
    queue.handleSpawn('m1', 1, spawn, 150)

    expect(spawn).toHaveBeenCalledTimes(1)
    expect(queue.size).toBe(0)
  })

  it('spawns drops from other monsters immediately without holding the queue', () => {
    const queue = new DeathDropDelayQueue()
    const spawn = vi.fn()

    queue.expectDrop('m1', 100)
    queue.handleSpawn('m2', 1, spawn, 150)

    expect(spawn).toHaveBeenCalledTimes(1)
    expect(queue.size).toBe(1)
  })

  it('spawns non-monster (player/debug) drops immediately', () => {
    const queue = new DeathDropDelayQueue()
    const spawn = vi.fn()

    queue.expectDrop('m1', 100)
    queue.handleSpawn(undefined, 1, spawn, 150)

    expect(spawn).toHaveBeenCalledTimes(1)
    expect(queue.size).toBe(1)
  })

  it('prunes stale death drop expectations and spawns immediately', () => {
    const queue = new DeathDropDelayQueue()
    const spawn = vi.fn()

    queue.expectDrop('m1', 100)
    queue.handleSpawn('m1', 1, spawn, 5200)

    expect(spawn).toHaveBeenCalledTimes(1)
    expect(queue.size).toBe(0)
  })

  it('cancels a delayed spawn before the monster impact is released', () => {
    const queue = new DeathDropDelayQueue()
    const spawn = vi.fn()

    queue.expectDrop('m1', 100)
    queue.handleSpawn('m1', 1, spawn, 150)

    expect(queue.cancelSpawn(1)).toBe(true)
    queue.releaseForMonster('m1')

    expect(spawn).not.toHaveBeenCalled()
    expect(queue.size).toBe(0)
  })
})
