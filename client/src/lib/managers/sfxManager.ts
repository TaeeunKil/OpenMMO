import {
  DEFAULT_MATERIAL_HIT_SOUND_URL,
  DEFAULT_MATERIAL_MISS_SOUND_URL,
  getAllMaterialHitSoundUrls,
  getAllMaterialMissSoundUrls,
} from '../data/materialImpactSounds'

const SWORD_HIT_VOLUME = 0.55
const SWORD_MISS_VOLUME = 0.5
const SWORD_HIT_POOL_SIZE = 4
const SWORD_MISS_POOL_SIZE = 4
export const SWORD_MISS_DELAY_MS = 450

interface AudioPool {
  audios: HTMLAudioElement[]
  index: number
}

const swordHitPools = new Map<string, AudioPool>()
const swordMissPools = new Map<string, AudioPool>()

function canUseAudio(): boolean {
  return typeof Audio !== 'undefined'
}

function createAudio(url: string, volume: number): HTMLAudioElement {
  const audio = new Audio(url)
  audio.preload = 'auto'
  audio.volume = volume
  return audio
}

function preloadAudioPool(
  pools: Map<string, AudioPool>,
  url: string,
  volume: number,
  poolSize: number
) {
  if (!canUseAudio() || pools.has(url)) return

  const pool = {
    audios: Array.from({ length: poolSize }, () => createAudio(url, volume)),
    index: 0,
  }

  for (const audio of pool.audios) {
    audio.load()
  }

  pools.set(url, pool)
}

function playAudioFromPool(
  pools: Map<string, AudioPool>,
  url: string,
  volume: number,
  poolSize: number
) {
  preloadAudioPool(pools, url, volume, poolSize)

  const pool = pools.get(url)
  if (!pool) return

  const audio = pool.audios[pool.index]
  pool.index = (pool.index + 1) % pool.audios.length

  try {
    audio.currentTime = 0
    audio.volume = volume
    audio.play().catch(() => {})
  } catch {
    // Browser audio policies can reject playback until the first user gesture.
  }
}

export function preloadSwordHitSound() {
  for (const url of getAllMaterialHitSoundUrls()) {
    preloadAudioPool(swordHitPools, url, SWORD_HIT_VOLUME, SWORD_HIT_POOL_SIZE)
  }
}

export function preloadSwordMissSound() {
  for (const url of getAllMaterialMissSoundUrls()) {
    preloadAudioPool(
      swordMissPools,
      url,
      SWORD_MISS_VOLUME,
      SWORD_MISS_POOL_SIZE
    )
  }
}

export function playSwordHitSound(url = DEFAULT_MATERIAL_HIT_SOUND_URL) {
  if (!canUseAudio()) return
  playAudioFromPool(swordHitPools, url, SWORD_HIT_VOLUME, SWORD_HIT_POOL_SIZE)
}

export function playSwordMissSound(
  url = DEFAULT_MATERIAL_MISS_SOUND_URL,
  delayMs = 0
) {
  if (!canUseAudio()) return
  if (delayMs > 0) {
    window.setTimeout(() => playSwordMissSound(url), delayMs)
    return
  }
  playAudioFromPool(
    swordMissPools,
    url,
    SWORD_MISS_VOLUME,
    SWORD_MISS_POOL_SIZE
  )
}
