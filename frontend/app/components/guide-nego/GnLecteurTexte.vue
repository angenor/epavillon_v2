<script setup lang="ts">
import type { CorrectionNote, DocumentReading } from '~/types/negotiation-documents'
import type { Occurrence } from '~/utils/guide-nego/lecteur'

/**
 * « Texte agrandi » — le lecteur recomposé de l'étape 1, sorti de la page : chaque page
 * en texte, à la taille choisie, avec ses notes et ses passages cherchés. La page du
 * lecteur tient la barre, la reprise et les feuilles ; celui-ci ne rend que le texte.
 */
const props = defineProps<{
  lecture: DocumentReading
  taille: number
  /** Les occurrences cherchées, par page ; `null` hors recherche. */
  surlignagesParPage: Map<number, Occurrence[]> | null
  passageCourant: Occurrence | null
  notesParPage: Map<number, CorrectionNote[]>
  suivreLaPage: (element: Element | null, index: number) => void
  /** Les tableaux et figures renvoient à leur page du PDF. */
  renvois: boolean
}>()

const emit = defineEmits<{
  terme: [texte: string]
  basculer: [evenement: MouseEvent]
  redimension: []
  renvoi: [page: number, texte: string]
}>()

const { t } = useI18n()

// Une rotation de l'écran recompose le texte : la page la garde en se recalant.
const article = ref<HTMLElement | null>(null)
let largeur = 0
let redimension: ResizeObserver | null = null
onMounted(() => {
  const element = article.value
  if (!element || typeof ResizeObserver === 'undefined') return
  largeur = element.clientWidth
  redimension = new ResizeObserver(() => {
    if (element.clientWidth === largeur) return
    largeur = element.clientWidth
    emit('redimension')
  })
  redimension.observe(element)
})
onBeforeUnmount(() => redimension?.disconnect())
</script>

<template>
  <article
    ref="article"
    class="gn-lecteur"
    :style="{ '--gn-taille-lecture': `${props.taille}px` }"
    @click="emit('basculer', $event)"
  >
    <section
      v-for="page in props.lecture.pages"
      :id="`page-${page.index}`"
      :key="page.index"
      :ref="(element) => props.suivreLaPage(element as Element | null, page.index)"
      class="gn-lecteur__page"
      tabindex="-1"
    >
      <p class="gn-lecteur__repere">{{ t('guide-nego.lecteur.page', { page: page.label }) }}</p>
      <GnPageLue
        :page="page"
        :surlignages="props.surlignagesParPage?.get(page.index)"
        :courant="props.passageCourant?.page === page.index ? props.passageCourant : null"
        :notes="props.notesParPage.get(page.index)"
        :renvois="props.renvois"
        @terme="emit('terme', $event)"
        @renvoi="(index, texte) => emit('renvoi', index, texte)"
      />
    </section>
  </article>
</template>

<style>
[data-app="guide-nego"] .gn-lecteur {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-24);
  /* La barre et la ligne de reprise, fixées en bas, ne cachent jamais la fin du texte. */
  padding-block: var(--gn-espace-16)
    calc(var(--gn-barre-onglets) + var(--gn-espace-48) + env(safe-area-inset-bottom));
  overflow-wrap: break-word;
}

[data-app="guide-nego"] .gn-lecteur__page {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  scroll-margin-top: var(--gn-espace-16);
}

/* Le repère de page est invisible (R14) : l'observateur le suit, un lecteur d'écran le dit. */
[data-app="guide-nego"] .gn-lecteur__repere {
  position: absolute;
  inline-size: 1px;
  block-size: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>
