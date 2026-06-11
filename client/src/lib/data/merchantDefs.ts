import merchantsJson from '../../../../data/merchants.json'

export interface MerchantDefinition {
  id: string
  npcName: string
  sellRatePercent: number
  /** Semicolon-separated item def ids. */
  catalog: string
}

const merchantDefs = merchantsJson as Record<string, MerchantDefinition>

const byNpcName = new Map(
  Object.values(merchantDefs).map((def) => [def.npcName, def])
)

/** Merchant lookup by NPC character name (NPCs are agent-controlled players).
 *  Non-merchant traders live in traderDefs.ts; use getNpcCapabilities there
 *  to decide how an NPC can be interacted with. */
export function getMerchantByNpcName(
  npcName: string
): MerchantDefinition | undefined {
  return byNpcName.get(npcName)
}

export default merchantDefs
