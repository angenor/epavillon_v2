<script setup lang="ts">
import type { Block, CorrectionNote, ReadingPage, Span } from '~/types/negotiation-documents'
import { ancrerLesNotes, blocsDeLaPage } from '~/utils/guide-nego/forme-lisible'
import { surligner, type Champ, type Occurrence, type SegmentSurligne } from '~/utils/guide-nego/lecteur'
import GnNoteCorrection from './GnNoteCorrection.vue'

/**
 * Une page de « Texte agrandi » — maquette 04 · 01 : le texte recomposé à la largeur de
 * l'écran. Les notes de correction bordent le bloc qui porte leur extrait, ou la tête
 * de la page : le texte, lui, se rend tel qu'il a été téléchargé.
 */
const props = withDefaults(
  defineProps<{
    page: ReadingPage
    /** Les occurrences cherchées sur cette page. */
    surlignages?: Occurrence[]
    /** L'occurrence courante, si elle est sur cette page : seule cette page se recalcule au suivant. */
    courant?: Occurrence | null
    /** Les notes vivantes posées sur cette page. */
    notes?: CorrectionNote[]
    /** Un tableau ou une figure renvoie à sa page du PDF, si ce téléphone l'affiche. */
    renvois?: boolean
  }>(),
  { surlignages: () => [], courant: null, notes: () => [], renvois: false },
)

const emit = defineEmits<{ terme: [texte: string]; renvoi: [page: number, texte: string] }>()

/** Ce qui repère le tableau sur sa page : sa légende, sinon le début de son texte. */
const texteDuBloc = (bloc: Extract<Block, { kind: 'origin' }>): string =>
  (bloc.caption?.length ? bloc.caption : (bloc.text ?? [])).map((s) => s.text).join('')

const { t } = useI18n()

const blocs = computed<Block[]>(() => blocsDeLaPage(props.page))
const ancrees = computed(() => ancrerLesNotes(blocs.value, props.notes))
// Un bloc sans note garde une enveloppe neutre (`display: contents`) : rien ne bouge.
const enveloppes = computed(() =>
  blocs.value.map((_, rang) => {
    const notes = ancrees.value.parBloc.get(rang)
    return notes
      ? { composant: GnNoteCorrection, attributs: { notes } }
      : { composant: 'div', attributs: { class: 'gn-page-lue__bloc' } }
  }),
)

const estCourant = (o: Occurrence) =>
  !!props.courant && o.bloc === props.courant.bloc && o.champ === props.courant.champ && o.debut === props.courant.debut

function segments(rang: number, champ: Champ, spans: Span[] | undefined): SegmentSurligne[] {
  const ici = props.surlignages.filter((s) => s.bloc === rang && s.champ === champ)
  return surligner(spans ?? [], ici.map((o) => ({ debut: o.debut, fin: o.fin, courant: estCourant(o) })))
}

// Le texte replié d'un tableau s'ouvre quand l'occurrence courante s'y trouve.
const texteOuvert = (rang: number) => props.courant?.bloc === rang && props.courant.champ === 'text'

const balisesDeTitre = { 1: 'h2', 2: 'h3', 3: 'h4' } as const
</script>

<template>
  <div class="gn-page-lue">
    <GnNoteCorrection v-if="ancrees.enTete.length" :notes="ancrees.enTete" />

    <component
      :is="enveloppes[rang]?.composant"
      v-for="(bloc, rang) in blocs"
      :key="rang"
      v-bind="enveloppes[rang]?.attributs"
    >
      <component :is="balisesDeTitre[bloc.level]" v-if="bloc.kind === 'heading'" :class="`gn-page-lue__titre-${bloc.level}`">
        <GnSegmentsLus :segments="segments(rang, 'spans', bloc.spans)" @terme="emit('terme', $event)" />
      </component>

      <p v-else-if="bloc.kind === 'paragraph'" class="gn-page-lue__paragraphe">
        <GnSegmentsLus :segments="segments(rang, 'spans', bloc.spans)" @terme="emit('terme', $event)" />
      </p>

      <p
        v-else-if="bloc.kind === 'list_item'"
        class="gn-page-lue__element"
        :style="{ '--gn-profondeur': bloc.depth }"
      >
        <span class="gn-page-lue__puce" aria-hidden="true">{{ bloc.marker }}</span>
        <span>
          <GnSegmentsLus :segments="segments(rang, 'spans', bloc.spans)" @terme="emit('terme', $event)" />
        </span>
      </p>

      <p v-else-if="bloc.kind === 'note'" class="gn-page-lue__note">
        <sup class="gn-page-lue__appel">{{ bloc.mark }}</sup>
        <GnSegmentsLus :segments="segments(rang, 'spans', bloc.spans)" @terme="emit('terme', $event)" />
      </p>

      <div v-else-if="bloc.kind === 'origin'" class="gn-page-lue__origine">
        <button
          v-if="props.renvois"
          type="button"
          class="gn-page-lue__renvoi"
          @click.stop="emit('renvoi', page.index, texteDuBloc(bloc))"
        >
          <GnPicto nom="doc" :taille="20" />
          {{ t(`gn-page-lue.renvoi.${bloc.reason}`, { page: page.label }) }}
        </button>
        <p v-if="bloc.caption?.length" class="gn-page-lue__legende">
          <GnSegmentsLus :segments="segments(rang, 'caption', bloc.caption)" @terme="emit('terme', $event)" />
        </p>
        <details v-if="bloc.text?.length" class="gn-page-lue__texte-origine" :open="texteOuvert(rang) || undefined" @click.stop>
          <summary>{{ t(`gn-page-lue.texte.${bloc.reason}`) }}</summary>
          <p>
            <GnSegmentsLus :segments="segments(rang, 'text', bloc.text)" @terme="emit('terme', $event)" />
          </p>
        </details>
      </div>
    </component>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-page-lue {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  font-size: var(--gn-taille-lecture, var(--gn-taille-17));
  line-height: 1.5;
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-page-lue__bloc {
  display: contents;
}

/* Les titres ne suivent pas la taille du texte (R14). */
[data-app="guide-nego"] .gn-page-lue__titre-1,
[data-app="guide-nego"] .gn-page-lue__titre-2,
[data-app="guide-nego"] .gn-page-lue__titre-3 {
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-page-lue__titre-1 {
  font-size: var(--gn-taille-28);
  line-height: var(--gn-interligne-28);
}

[data-app="guide-nego"] .gn-page-lue__titre-2 {
  font-size: var(--gn-taille-24);
  line-height: var(--gn-interligne-24);
}

[data-app="guide-nego"] .gn-page-lue__titre-3 {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
}

[data-app="guide-nego"] .gn-page-lue__paragraphe,
[data-app="guide-nego"] .gn-page-lue__element {
  overflow-wrap: break-word;
}

[data-app="guide-nego"] .gn-page-lue__element {
  display: flex;
  gap: var(--gn-espace-8);
  padding-inline-start: calc(var(--gn-profondeur, 0) * var(--gn-espace-24));
}

[data-app="guide-nego"] .gn-page-lue__puce {
  flex: none;
  min-width: 1.2em;
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-page-lue__note {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
  padding-top: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-page-lue__note + .gn-page-lue__note {
  border-top: none;
  padding-top: 0;
}

[data-app="guide-nego"] .gn-page-lue__appel {
  margin-inline-end: var(--gn-espace-4);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-page-lue__origine {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding: var(--gn-espace-12);
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-4);
}

[data-app="guide-nego"] .gn-page-lue__renvoi {
  align-self: flex-start;
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  padding: 0;
  border: none;
  background: none;
  color: var(--gn-accent);
  font: inherit;
  font-size: var(--gn-taille-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: var(--gn-espace-4);
  cursor: pointer;
}

[data-app="guide-nego"] .gn-page-lue__legende {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-page-lue__texte-origine summary {
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  font-size: var(--gn-taille-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte-2);
  cursor: pointer;
}
</style>
