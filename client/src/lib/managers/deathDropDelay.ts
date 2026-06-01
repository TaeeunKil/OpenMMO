const STALE_PENDING_MS = 5000

type PendingDeathDrop = {
  monsterId: string
  createdAtMs: number
  ready: boolean
  spawnInstanceId?: number
  spawn?: () => void
}

function nowMs(): number {
  return typeof performance !== 'undefined' ? performance.now() : Date.now()
}

export class DeathDropDelayQueue {
  private pending: PendingDeathDrop[] = []

  expectDrop(monsterId: string, now = nowMs()) {
    this.prune(now)
    this.pending.push({ monsterId, createdAtMs: now, ready: false })
  }

  // Spawns the ground item now, or — when it was dropped by a monster whose
  // death-impact animation hasn't played yet — defers it until
  // releaseForMonster fires. The queue always takes ownership of the spawn so
  // callers don't need a fallback branch.
  handleSpawn(
    sourceMonsterId: string | undefined,
    instanceId: number,
    spawn: () => void,
    now = nowMs()
  ) {
    this.prune(now)

    const pendingIndex =
      sourceMonsterId == null
        ? -1
        : this.pending.findIndex((p) => p.monsterId === sourceMonsterId)
    if (pendingIndex === -1) {
      spawn()
      return
    }

    const pending = this.pending[pendingIndex]
    if (pending.ready) {
      this.pending.splice(pendingIndex, 1)
      spawn()
      return
    }

    pending.spawnInstanceId = instanceId
    pending.spawn = spawn
  }

  cancelSpawn(instanceId: number) {
    for (let i = this.pending.length - 1; i >= 0; i--) {
      const pending = this.pending[i]
      if (pending.spawnInstanceId !== instanceId) continue

      this.pending.splice(i, 1)
      return true
    }
    return false
  }

  releaseForMonster(monsterId: string) {
    for (let i = this.pending.length - 1; i >= 0; i--) {
      const pending = this.pending[i]
      if (pending.monsterId !== monsterId) continue

      pending.ready = true
      if (pending.spawn) {
        this.pending.splice(i, 1)
        pending.spawn()
      }
    }
  }

  reset() {
    this.pending = []
  }

  get size() {
    return this.pending.length
  }

  private prune(now: number) {
    this.pending = this.pending.filter(
      (pending) => now - pending.createdAtMs <= STALE_PENDING_MS
    )
  }
}

export const deathDropDelayQueue = new DeathDropDelayQueue()
