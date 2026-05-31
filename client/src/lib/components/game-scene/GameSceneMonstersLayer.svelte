<script lang="ts">
  import Monster from '../Monster.svelte'
  import { monsterManager } from '../../managers/monsterManager'
  import type { MonsterData } from '../../types/Monster'

  interface Props {
    monsters: Map<string, MonsterData>
    monsterModels?: (Monster | undefined)[]
  }

  let { monsters, monsterModels = $bindable<(Monster | undefined)[]>([]) }: Props =
    $props()
</script>

{#each [...monsters.values()] as monster, index (monster.id)}
  <Monster
    bind:this={monsterModels[index]}
    id={monster.id}
    type={monster.type}
    position={monster.position}
    rotation={monster.rotation}
    monsterState={monster.state}
    attackCounter={monster.attackCounter}
    lastDamageInfo={monster.lastDamageInfo}
    droppedWeaponItemDefId={monster.droppedWeaponItemDefId}
    onHitFinished={() => monsterManager.handleMonsterHitFinished(monster.id)}
  />
{/each}
