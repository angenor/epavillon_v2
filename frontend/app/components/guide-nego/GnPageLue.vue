<script setup lang="ts">
import type { ImageDePage } from '~/composables/guide-nego/useGnLecteur'
import type { Block, ReadingMode, ReadingPage, Span } from '~/types/negotiation-documents'
import { blocsDeLaPage } from '~/utils/guide-nego/forme-lisible'
import { surligner, type Champ, type Occurrence, type SegmentSurligne } from '~/utils/guide-nego/lecteur'

/**
 * Une page du lecteur — maquette 04 · 01. Le texte recomposé à la largeur de l'écran,
 * ou, « tel quel », l'image de la page. **Jamais une page blanche** : une image qui
 * manque laisse le texte, et une ligne dit pourquoi — le réseau absent, ou un échec
 * qui se réessaie.
 */
const props = withDefaults(
  defineProps<{
    page: ReadingPage
    mode: ReadingMode
    imageDe: (page: ReadingPage) => Promise<ImageDePage>
    /** Les occurrences cherchées sur cette page. */
    surlignages?: Occurrence[]
    /** L'occurrence courante, si elle est sur cette page : seule cette page se recalcule au suivant. */
    courant?: Occurrence | null
  }>(),
  { surlignages: () => [], courant: null },
)

defineEmits<{ terme: [texte: string] }>()

const { t } = useI18n()

const blocs = computed<Block[]>(() => blocsDeLaPage(props.page))
const telQuel = computed(() => props.mode === 'as_is')

type EtatDeLImage = 'aucune' | 'attente' | 'chargee' | 'hors-connexion' | 'echec'
const etatDeLImage = ref<EtatDeLImage>('aucune')
const adresse = ref<string | null>(null)
/** Les rangs des blocs d'origine ouverts : deux tableaux sur une page s'ouvrent chacun. */
const ouverts = ref(new Set<number>())

async function chargerLImage(): Promise<void> {
  if (etatDeLImage.value === 'attente' || etatDeLImage.value === 'chargee') return
  etatDeLImage.value = 'attente'
  const image = await props.imageDe(props.page)
  if (typeof image === 'string') {
    etatDeLImage.value = image
    return
  }
  adresse.value = image.adresse
  etatDeLImage.value = 'chargee'
}

// « Tel quel », l'image se demande quand la page approche : cent pages ne partent pas d'un coup.
const racine = ref<HTMLElement | null>(null)
let approche: IntersectionObserver | null = null
onMounted(() => {
  if (!telQuel.value || !racine.value) return
  if (typeof IntersectionObserver === 'undefined') return void chargerLImage()
  approche = new IntersectionObserver(
    (entrees) => {
      if (!entrees.some((e) => e.isIntersecting)) return
      approche?.disconnect()
      void chargerLImage()
    },
    { rootMargin: '100% 0px' },
  )
  approche.observe(racine.value)
})
onBeforeUnmount(() => approche?.disconnect())

function basculerLOrigine(rang: number): void {
  const suivants = new Set(ouverts.value)
  if (suivants.has(rang)) suivants.delete(rang)
  else suivants.add(rang)
  ouverts.value = suivants
  if (suivants.has(rang)) void chargerLImage()
}

function reessayer(): void {
  etatDeLImage.value = 'aucune'
  void chargerLImage()
}

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
  <div ref="racine" class="gn-page-lue">
    <GnImageDePage
      v-if="telQuel"
      :etat="etatDeLImage"
      :adresse="adresse"
      :libelle="t('gn-page-lue.image', { page: page.label })"
      :attente="t('gn-page-lue.page-avec-le-reseau', { page: page.label })"
      pleine
      @echec="etatDeLImage = 'echec'"
      @reessayer="reessayer"
    />

    <template v-for="(bloc, rang) in blocs" v-else :key="rang">
      <component :is="balisesDeTitre[bloc.level]" v-if="bloc.kind === 'heading'" :class="`gn-page-lue__titre-${bloc.level}`">
        <GnSegmentsLus :segments="segments(rang, 'spans', bloc.spans)" @terme="$emit('terme', $event)" />
      </component>

      <p v-else-if="bloc.kind === 'paragraph'" class="gn-page-lue__paragraphe">
        <GnSegmentsLus :segments="segments(rang, 'spans', bloc.spans)" @terme="$emit('terme', $event)" />
      </p>

      <p
        v-else-if="bloc.kind === 'list_item'"
        class="gn-page-lue__element"
        :style="{ '--gn-profondeur': bloc.depth }"
      >
        <span class="gn-page-lue__puce" aria-hidden="true">{{ bloc.marker }}</span>
        <span>
          <GnSegmentsLus :segments="segments(rang, 'spans', bloc.spans)" @terme="$emit('terme', $event)" />
        </span>
      </p>

      <p v-else-if="bloc.kind === 'note'" class="gn-page-lue__note">
        <sup class="gn-page-lue__appel">{{ bloc.mark }}</sup>
        <GnSegmentsLus :segments="segments(rang, 'spans', bloc.spans)" @terme="$emit('terme', $event)" />
      </p>

      <div v-else-if="bloc.kind === 'origin'" class="gn-page-lue__origine">
        <p v-if="bloc.caption?.length" class="gn-page-lue__legende">
          <GnSegmentsLus :segments="segments(rang, 'caption', bloc.caption)" @terme="$emit('terme', $event)" />
        </p>
        <button
          type="button"
          class="gn-page-lue__voir"
          :aria-expanded="ouverts.has(rang)"
          @click.stop="basculerLOrigine(rang)"
        >
          <GnPicto nom="doc" :taille="20" />
          {{ t('gn-page-lue.voir') }}
        </button>
        <GnImageDePage
          v-if="ouverts.has(rang)"
          :etat="etatDeLImage"
          :adresse="adresse"
          :libelle="t('gn-page-lue.image', { page: page.label })"
          :attente="t('gn-page-lue.origine-avec-le-reseau')"
          @echec="etatDeLImage = 'echec'"
          @reessayer="reessayer"
        />
        <details v-if="bloc.text?.length" class="gn-page-lue__texte-origine" :open="texteOuvert(rang) || undefined" @click.stop>
          <summary>{{ t(`gn-page-lue.texte.${bloc.reason}`) }}</summary>
          <p>
            <GnSegmentsLus :segments="segments(rang, 'text', bloc.text)" @terme="$emit('terme', $event)" />
          </p>
        </details>
      </div>
    </template>
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

[data-app="guide-nego"] .gn-page-lue__legende {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-page-lue__voir {
  display: inline-flex;
  align-items: center;
  gap: var(--gn-espace-8);
  align-self: flex-start;
  min-height: var(--gn-cible);
  padding: 0;
  border: none;
  background: none;
  color: var(--gn-accent);
  font: inherit;
  font-size: var(--gn-taille-17);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: var(--gn-espace-4);
  cursor: pointer;
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
