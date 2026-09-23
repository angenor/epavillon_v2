<script setup lang="ts">
import type { ImageDePage } from '~/composables/guide-nego/useGnLecteur'
import type { Block, ReadingMode, ReadingPage, Span } from '~/types/negotiation-documents'
import { blocsDeLaPage } from '~/utils/guide-nego/forme-lisible'

/**
 * Une page du lecteur — maquette 04 · 01. Le texte recomposé à la largeur de l'écran,
 * ou, « tel quel », l'image de la page. **Jamais une page blanche** : une image qui
 * manque laisse le texte, et une ligne dit pourquoi — le réseau absent, ou un échec
 * qui se réessaie.
 */
const props = defineProps<{
  page: ReadingPage
  mode: ReadingMode
  imageDe: (page: ReadingPage) => Promise<ImageDePage>
}>()

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

const classesDuSegment = (s: Span) => ({
  'gn-page-lue__italique': s.italic,
  'gn-page-lue__gras': s.bold,
  'gn-page-lue__terme': s.term,
})

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
        <span v-for="(s, i) in bloc.spans" :key="i" :class="classesDuSegment(s)">{{ s.text }}</span>
      </component>

      <p v-else-if="bloc.kind === 'paragraph'" class="gn-page-lue__paragraphe">
        <span v-for="(s, i) in bloc.spans" :key="i" :class="classesDuSegment(s)">{{ s.text }}</span>
      </p>

      <p
        v-else-if="bloc.kind === 'list_item'"
        class="gn-page-lue__element"
        :style="{ '--gn-profondeur': bloc.depth }"
      >
        <span class="gn-page-lue__puce" aria-hidden="true">{{ bloc.marker }}</span>
        <span>
          <span v-for="(s, i) in bloc.spans" :key="i" :class="classesDuSegment(s)">{{ s.text }}</span>
        </span>
      </p>

      <p v-else-if="bloc.kind === 'note'" class="gn-page-lue__note">
        <sup class="gn-page-lue__appel">{{ bloc.mark }}</sup>
        <span v-for="(s, i) in bloc.spans" :key="i" :class="classesDuSegment(s)">{{ s.text }}</span>
      </p>

      <div v-else-if="bloc.kind === 'origin'" class="gn-page-lue__origine">
        <p v-if="bloc.caption?.length" class="gn-page-lue__legende">
          <span v-for="(s, i) in bloc.caption" :key="i" :class="classesDuSegment(s)">{{ s.text }}</span>
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
        <details v-if="bloc.text?.length" class="gn-page-lue__texte-origine" @click.stop>
          <summary>{{ t(`gn-page-lue.texte.${bloc.reason}`) }}</summary>
          <p>
            <span v-for="(s, i) in bloc.text" :key="i" :class="classesDuSegment(s)">{{ s.text }}</span>
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

[data-app="guide-nego"] .gn-page-lue__italique {
  font-style: italic;
}

[data-app="guide-nego"] .gn-page-lue__gras {
  font-weight: var(--gn-graisse-gras);
}

/* Le terme anglais : italique souligné, comme la maquette. Il deviendra touchable avec la feuille du lexique. */
[data-app="guide-nego"] .gn-page-lue__terme {
  text-decoration: underline;
  text-decoration-thickness: var(--gn-filet-2);
  text-decoration-color: var(--gn-accent);
  text-underline-offset: var(--gn-espace-4);
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
