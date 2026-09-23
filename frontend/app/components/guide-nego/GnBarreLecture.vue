<script setup lang="ts">
import { partLue } from '~/utils/guide-nego/forme-lisible'
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'

type Action = 'sommaire' | 'rechercher' | 'reglages'

/**
 * La barre du lecteur, en bas, à portée de pouce. Le toucher au centre de la page est
 * capté par l'écran, qui bascule `depliee` ; la barre ne fait que se replier au défilement.
 * Pas de « Marquer » (écart 42) : un bouton sans effet n'apparaît pas.
 */
const props = defineProps<{
  page: number
  total: number
  /** L'étiquette imprimée de la page, si elle diffère de l'index. */
  etiquette?: string
  section: string | null
  actions: Action[]
}>()

const depliee = defineModel<boolean>('depliee', { default: false })
const emit = defineEmits<{ action: [Action] }>()

const { t } = useI18n()

const PICTOS: Record<Action, NomDePicto> = { sommaire: 'toc', rechercher: 'search', reglages: 'text-size' }

const lignePage = computed(() =>
  t('gn-barre-lecture.page', { page: props.etiquette ?? String(props.page), total: props.total }),
)
const part = computed(() => (props.total > 0 ? partLue(props.page, props.total) : null))
const actionsVisibles = computed(() => depliee.value && props.actions.length > 0)

function replier() {
  if (depliee.value) depliee.value = false
}

onMounted(() => window.addEventListener('scroll', replier, { passive: true }))
onBeforeUnmount(() => window.removeEventListener('scroll', replier))
</script>

<template>
  <div class="gn-barre-lecture">
    <div v-if="actionsVisibles" class="gn-barre-lecture__actions" role="group" :aria-label="t('gn-barre-lecture.actions')">
      <button
        v-for="action in actions"
        :key="action"
        type="button"
        class="gn-barre-lecture__action"
        @click="emit('action', action)"
      >
        <GnPicto :nom="PICTOS[action]" :taille="26" />
        <span>{{ t(`gn-barre-lecture.${action}`) }}</span>
      </button>
    </div>
    <p class="gn-barre-lecture__ligne">
      <span class="gn-barre-lecture__page">{{ lignePage }}</span>
      <span v-if="section" class="gn-barre-lecture__section">{{ section }}</span>
    </p>
    <GnProgression :part="part" decoratif />
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-barre-lecture {
  position: fixed;
  bottom: 0;
  /* Sur un écran large, la barre reste dans la colonne de l'application. */
  left: 50%;
  transform: translateX(-50%);
  width: min(100%, var(--gn-colonne-largeur));
  z-index: 5;
  padding-bottom: env(safe-area-inset-bottom);
  background: var(--gn-fond);
}

[data-app="guide-nego"] .gn-barre-lecture__actions {
  display: flex;
  height: var(--gn-barre-onglets);
  border-top: var(--gn-filet-2) solid var(--gn-filet-fort);
}

[data-app="guide-nego"] .gn-barre-lecture__action {
  flex: 1 1 0;
  min-width: var(--gn-onglet-min);
  padding-inline: var(--gn-onglet-air);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  border: 0;
  background: none;
  color: var(--gn-texte-2);
  font: inherit;
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-demi-gras);
  white-space: nowrap;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-barre-lecture__action:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-barre-lecture__action:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-anneau));
}

[data-app="guide-nego"] .gn-barre-lecture__ligne {
  margin: 0;
  min-height: var(--gn-barre-lecture-repliee);
  padding-inline: var(--gn-marge-ecran);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gn-espace-12);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-barre-lecture__page {
  flex: none;
  color: var(--gn-titre);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-barre-lecture__section {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--gn-texte-2);
}
</style>
