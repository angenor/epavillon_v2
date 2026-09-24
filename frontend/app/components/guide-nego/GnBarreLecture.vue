<script setup lang="ts">
import { partLue } from '~/utils/guide-nego/forme-lisible'
import type { ModeDeLecture } from '~/utils/guide-nego/appareil-lecture'
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'

type Action = 'sommaire' | 'rechercher' | 'mode' | 'reglages'

/**
 * La barre du lecteur, en bas, à portée de pouce. Le toucher au centre de la page est
 * capté par l'écran, qui bascule `depliee` ; la barre ne fait que se replier au défilement.
 * Pas de « Marquer » (écart 42) : un bouton sans effet n'apparaît pas. Son emplacement
 * porte le choix « Pages · Texte » (écart 43), et « Réglages » ne ressemble pas au « Aa »
 * de l'en-tête (écart 44).
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
/** Le mode lu, si l'action `mode` est offerte. */
const mode = defineModel<ModeDeLecture>('mode', { default: 'pages' })
const emit = defineEmits<{ action: [Exclude<Action, 'mode'>] }>()

const { t } = useI18n()

const PICTOS: Record<Exclude<Action, 'mode'>, NomDePicto> = { sommaire: 'toc', rechercher: 'search', reglages: 'sliders' }

const lignePage = computed(() =>
  t('gn-barre-lecture.page', { page: props.etiquette ?? String(props.page), total: props.total }),
)
const part = computed(() => (props.total > 0 ? partLue(props.page, props.total) : null))
const actionsVisibles = computed(() => depliee.value && props.actions.length > 0)

const barre = useTemplateRef<HTMLElement>('barre')

// Un défilement replie la barre — sauf quand le focus y est : c'est le clavier qui la parcourt.
function replier() {
  if (depliee.value && !barre.value?.contains(document.activeElement)) depliee.value = false
}

/** Au clavier, la ligne de page est le bouton qui déplie : le toucher au centre n'y existe pas. */
let focaliser = false
function basculer() {
  focaliser = !depliee.value
  depliee.value = !depliee.value
}
// Les actions ne sont dessinées qu'une fois le parent passé : le focus les attend.
watch(
  actionsVisibles,
  (visibles) => {
    if (visibles && focaliser) barre.value?.querySelector<HTMLElement>('.gn-barre-lecture__action, .gn-choix-mode__segment')?.focus()
    focaliser = false
  },
  { flush: 'post' },
)

onMounted(() => window.addEventListener('scroll', replier, { passive: true }))
onBeforeUnmount(() => window.removeEventListener('scroll', replier))
</script>

<template>
  <div ref="barre" class="gn-barre-lecture">
    <div v-if="actionsVisibles" class="gn-barre-lecture__actions" role="group" :aria-label="t('gn-barre-lecture.actions')">
      <template v-for="action in actions" :key="action">
        <GnChoixMode v-if="action === 'mode'" v-model="mode" variante="barre" />
        <button v-else type="button" class="gn-barre-lecture__action" @click="emit('action', action)">
          <GnPicto :nom="PICTOS[action]" :taille="26" />
          <span>{{ t(`gn-barre-lecture.${action}`) }}</span>
        </button>
      </template>
    </div>
    <button
      v-if="actions.length"
      type="button"
      class="gn-barre-lecture__ligne gn-barre-lecture__ligne--bouton"
      :aria-expanded="depliee"
      :aria-label="t('gn-barre-lecture.outils', { page: lignePage })"
      @click="basculer"
    >
      <span class="gn-barre-lecture__page">{{ lignePage }}</span>
      <span v-if="section" class="gn-barre-lecture__section">{{ section }}</span>
    </button>
    <p v-else class="gn-barre-lecture__ligne">
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

[data-app="guide-nego"] .gn-barre-lecture__ligne--bouton {
  inline-size: 100%;
  border: none;
  background: none;
  color: inherit;
  font-family: inherit;
  text-align: start;
  cursor: pointer;
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
