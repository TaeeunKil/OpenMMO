import { writable } from 'svelte/store'

/** One entry in the NPC right-click context menu. Actions are closures
 *  built by PlayerControl (which owns movement/network), so the menu
 *  component stays purely presentational. */
export interface NpcMenuEntry {
  label: string
  action: () => void
}

/** An open NPC context menu, anchored at screen coordinates. Null = closed. */
export interface NpcContextMenuState {
  npcName: string
  screenX: number
  screenY: number
  entries: NpcMenuEntry[]
}

export const npcContextMenu = writable<NpcContextMenuState | null>(null)

export function closeNpcContextMenu() {
  npcContextMenu.set(null)
}

/** Incremented to ask the chat panel to focus its input (the "Talk"
 *  default action for non-merchant NPCs). */
export const chatFocusRequest = writable(0)

export function requestChatFocus() {
  chatFocusRequest.update((n) => n + 1)
}
