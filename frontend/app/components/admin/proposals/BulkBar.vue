<script setup lang="ts">
/**
 * LA BARRE D'ACTIONS GROUPÉES — elle n'existe que lorsqu'une sélection existe.
 *
 * ELLE DIT CE QUE LA SÉLECTION COUVRE VRAIMENT. « Tout sélectionner » ne touche
 * que les lignes AFFICHÉES — c'est le comportement de `UiTable`, et la barre le
 * rappelle dès que la liste est filtrée ou paginée. Une action de masse qui
 * porterait silencieusement sur cent trente-sept dossiers dont douze sont sous
 * les yeux est un piège, et c'est ainsi qu'on envoie une notification à des
 * organisations qu'on n'avait pas l'intention de prévenir.
 *
 * DEUX ACTIONS, PAS SIX. Affecter et changer de statut sont les deux gestes que
 * l'équipe répète ; l'export, lui, ne modifie rien et reste dans la barre
 * d'outils du tableau, avec la recherche. Mélanger ce qui écrit et ce qui lit
 * dans la même barre efface la seule distinction qui compte ici.
 *
 * LES ACTIONS ABSENTES NE SONT PAS GRISÉES, ELLES NE SONT PAS RENDUES : une
 * personne sans droit de décision n'a pas à voir un bouton qu'elle ne peut pas
 * actionner. Le refus définitif appartient à l'API — le masquage n'est qu'un
 * confort de lecture.
 */

interface Props {
  count: number
  /** La sélection ne couvre-t-elle qu'une partie des résultats ? */
  partial?: boolean
  canAssign?: boolean
  canDecide?: boolean
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ assign: []; changeStatus: []; export: []; clear: [] }>()

const { t } = useI18n()
</script>

<template>
  <div
    v-if="props.count > 0"
    class="flex flex-wrap items-center gap-x-4 gap-y-3 rounded-lg border border-accent-solid bg-accent-surface px-4 py-3"
    role="region"
    :aria-label="t('admin.proposals.selection.count', props.count)"
  >
    <p class="text-sm font-semibold text-text">
      {{ t('admin.proposals.selection.count', props.count) }}
      <span v-if="props.partial" class="block text-xs font-normal text-text-secondary">
        {{ t('admin.proposals.selection.onlyVisible') }}
      </span>
    </p>

    <div class="ml-auto flex flex-wrap items-center gap-2">
      <UiButton
        v-if="props.canAssign"
        variant="secondary"
        size="sm"
        icon="users"
        :disabled="props.busy"
        @click="emit('assign')"
      >
        {{ t('admin.proposals.selection.assign') }}
      </UiButton>

      <UiButton
        v-if="props.canDecide"
        variant="secondary"
        size="sm"
        icon="check"
        :disabled="props.busy"
        @click="emit('changeStatus')"
      >
        {{ t('admin.proposals.selection.changeStatus') }}
      </UiButton>

      <UiButton variant="secondary" size="sm" icon="download" :disabled="props.busy" @click="emit('export')">
        {{ t('admin.proposals.export.selection') }}
      </UiButton>

      <UiButton variant="ghost" size="sm" :disabled="props.busy" @click="emit('clear')">
        {{ t('admin.proposals.selection.clear') }}
      </UiButton>
    </div>
  </div>
</template>
