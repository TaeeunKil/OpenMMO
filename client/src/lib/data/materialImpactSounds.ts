import materialImpactSoundsJson from '../../../../data/material-impact-sounds.json'

export interface MaterialHitSoundRule {
  attackerMaterial: string
  targetMaterial: string
  sound: string
}

export interface MaterialMissSoundRule {
  attackerMaterial: string
  sound: string
}

export interface MaterialImpactSoundDefinitions {
  defaultHitSound: string
  defaultMissSound: string
  hitSounds: MaterialHitSoundRule[]
  missSounds: MaterialMissSoundRule[]
}

const materialImpactSounds =
  materialImpactSoundsJson as MaterialImpactSoundDefinitions

export const DEFAULT_MATERIAL_HIT_SOUND_URL =
  materialImpactSounds.defaultHitSound
export const DEFAULT_MATERIAL_MISS_SOUND_URL =
  materialImpactSounds.defaultMissSound

export function getMaterialHitSoundUrl(
  attackerMaterial?: string,
  targetMaterial?: string
): string {
  const rule = materialImpactSounds.hitSounds.find(
    (candidate) =>
      candidate.attackerMaterial === attackerMaterial &&
      candidate.targetMaterial === targetMaterial
  )

  return rule?.sound ?? DEFAULT_MATERIAL_HIT_SOUND_URL
}

export function getMaterialMissSoundUrl(attackerMaterial?: string): string {
  const rule = materialImpactSounds.missSounds.find(
    (candidate) => candidate.attackerMaterial === attackerMaterial
  )

  return rule?.sound ?? DEFAULT_MATERIAL_MISS_SOUND_URL
}

export function getAllMaterialHitSoundUrls(): string[] {
  return Array.from(
    new Set([
      DEFAULT_MATERIAL_HIT_SOUND_URL,
      ...materialImpactSounds.hitSounds.map((rule) => rule.sound),
    ])
  )
}

export function getAllMaterialMissSoundUrls(): string[] {
  return Array.from(
    new Set([
      DEFAULT_MATERIAL_MISS_SOUND_URL,
      ...materialImpactSounds.missSounds.map((rule) => rule.sound),
    ])
  )
}
