<script setup lang="ts">
import type { CorrectionNote } from '~/types/negotiation-documents'
import GnNoteCorrection from './GnNoteCorrection.vue'

/**
 * La note de correction en marge de la page du PDF (R10) : un filet et le triangle, à la
 * hauteur du passage ou en tête. Dépliée, un panneau non modal en bas de l'écran : la page
 * reste visible, rien ne s'écrit sur elle (FR-033, FR-035).
 */
const props = defineProps<{
  note: Pick<CorrectionNote, 'id' | 'body' | 'author_name' | 'posted_at' | 'passage'>
  /** En pourcentage de la page. */
  haut: number
  /** En pourcentage de la page ; `null` : en tête, sur une cible. */
  hauteur: number | null
  /** Le rang parmi les notes en tête de la même page : elles s'empilent. */
  decalage: number
  /** Faux : le passage cité n'est pas sur la page, le panneau le montre. */
  retrouve: boolean
  ouverte: boolean
}>()

const emit = defineEmits<{ basculer: [] }>()

const { t } = useI18n()

const panneau = computed(() => `gn-marge-note-${props.note.id}`)
const position = computed(() => ({
  top: `calc(${props.haut}% + ${props.decalage} * var(--gn-cible))`,
  height: props.hauteur === null ? undefined : `${props.hauteur}%`,
}))
</script>

<template>
  <button
    type="button"
    class="gn-marge-note"
    :style="position"
    :data-note="note.id"
    :aria-expanded="ouverte"
    :aria-controls="panneau"
    :aria-label="t('gn-marge-note.signal')"
    @click="emit('basculer')"
  >
    <GnPicto nom="warn" :taille="20" class="gn-marge-note__triangle" />
  </button>
  <Teleport to="#gn-portail">
    <section v-if="ouverte" :id="panneau" class="gn-marge-note__panneau" :aria-label="t('gn-marge-note.panneau')">
      <GnNoteCorrection :notes="[note]" depliee @basculer="(_, ouverte) => !ouverte && emit('basculer')">
        <p v-if="!retrouve && note.passage" class="gn-marge-note__passage">
          {{ t('gn-marge-note.passage', { passage: note.passage }) }}
        </p>
      </GnNoteCorrection>
      <button type="button" class="gn-marge-note__fermer" :aria-label="t('gn-marge-note.fermer')" @click="emit('basculer')">
        <GnPicto nom="close" :taille="20" />
      </button>
    </section>
  </Teleport>
</template>

<style>
/* Sur le papier, dans les deux thèmes : le filet et le triangle gardent le rouge de charte. */
[data-app="guide-nego"] .gn-marge-note {
  position: absolute;
  left: 0;
  width: var(--gn-cible);
  min-height: var(--gn-cible);
  display: flex;
  align-items: flex-start;
  padding: var(--gn-espace-4) 0 0 calc(var(--gn-filet-3) + var(--gn-espace-4));
  border: none;
  border-inline-start: var(--gn-filet-3) solid var(--gn-page-note);
  background: none;
  color: var(--gn-page-note);
  pointer-events: auto;
  cursor: pointer;
}

/* Sous `.pdfViewer`, tout passe en `content-box` : la cible ne tiendrait plus ses 48 px. */
[data-app="guide-nego"] .pdfViewer .gn-marge-note {
  box-sizing: border-box;
}

[data-app="guide-nego"] .gn-marge-note[aria-expanded="true"] {
  border-inline-start-width: calc(2 * var(--gn-filet-3));
}

[data-app="guide-nego"] .gn-marge-note__panneau {
  position: fixed;
  bottom: calc(var(--gn-barre-lecture-repliee) + var(--gn-jauge) + env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
  z-index: 7;
  display: flex;
  align-items: flex-start;
  width: min(100%, var(--gn-colonne-largeur));
  max-height: 40dvh;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: var(--gn-espace-8) 0 var(--gn-espace-12) var(--gn-marge-ecran);
  background: var(--gn-fond);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-marge-note__panneau > .gn-note-correction {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-marge-note__passage {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
  overflow-wrap: break-word;
}

[data-app="guide-nego"] .gn-marge-note__fermer {
  position: sticky;
  top: 0;
  flex: none;
  display: inline-grid;
  place-items: center;
  min-width: var(--gn-cible);
  min-height: var(--gn-cible);
  border: none;
  background: none;
  color: var(--gn-texte);
  cursor: pointer;
}
</style>
