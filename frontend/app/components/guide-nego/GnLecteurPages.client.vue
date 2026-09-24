<script setup lang="ts">
import '~/assets/guide-nego/pdfjs-viewer.css'
import type { PDFDocumentLoadingTask } from 'pdfjs-dist/legacy/build/pdf.min.mjs'
import type { PDFViewer } from 'pdfjs-dist/legacy/web/pdf_viewer.mjs'
import { chargerPdfjs, ouvrirLeDocument, type SourceDuDocument } from '~/utils/guide-nego/pdf/charger'
import { creerLaSurveillance, type CauseDeBascule } from '~/utils/guide-nego/pdf/bascule'
import { bornerLEchelle, creerLeLecteurDeGestes, echelleApresDoubleToucher, type Minuterie } from '~/utils/guide-nego/pdf/gestes'
import { ErreurDeReseau, type SuiviDesPlages } from '~/utils/guide-nego/pdf/transport'

/**
 * Les pages d'origine, par le visionneur de pdf.js (ADR-022, R6) : il ne garde prêtes
 * que les pages proches, plafonne les canevas et redessine net la zone visible. Ici :
 * les gestes, la seconde sécurité, et ce que dit une page que le réseau n'a pas amenée.
 */
const props = withDefaults(
  defineProps<{
    source: SourceDuDocument
    pageInitiale: number
    /**
     * Pendant l'attente ou la lecture du texte : le visionneur continue de se mettre en
     * page, invisible. Retiré du rendu, il calculerait la largeur de page sur zéro.
     */
    cachee?: boolean
  }>(),
  { cachee: false },
)

const emit = defineEmits<{
  page: [index: number]
  premierePage: []
  plages: [suivi: SuiviDesPlages]
  /** `indisponible` : ce téléphone n'a pas ce que pdf.js exige, rien n'a été chargé. */
  bascule: [cause: CauseDeBascule | 'indisponible']
  basculerLaBarre: []
  reseauPerdu: []
  reseauRevenu: []
}>()

const { t } = useI18n()

const ancre = ref<HTMLElement | null>(null)
const conteneur = ref<HTMLElement | null>(null)
const haut = ref(0)
const reseauPerdu = ref(false)

let viewer: PDFViewer | null = null
let tache: PDFDocumentLoadingTask | null = null
/** L'échelle qui fait tenir la page dans la largeur : le plancher, et le point de retour. */
let largeur = 1
const arret = new AbortController()

const minuterie: Minuterie = {
  planifier: (fn, ms) => setTimeout(fn, ms),
  annuler: (poignee) => clearTimeout(poignee as ReturnType<typeof setTimeout>),
}

const surveillance = creerLaSurveillance({
  minuterie,
  maintenant: () => performance.now(),
  surBascule: (cause) => emit('bascule', cause),
})

function changerDEchelle(cible: number, origine: [number, number] | undefined, delai: number): void {
  if (!viewer) return
  const bornee = bornerLEchelle(cible, largeur)
  if (Math.abs(bornee - viewer.currentScale) < 0.001) return
  viewer.updateScale({ drawingDelay: delai, scaleFactor: bornee / viewer.currentScale, origin: origine })
}

const gestes = creerLeLecteurDeGestes({
  minuterie,
  surGeste: (geste) => {
    if (geste.type === 'simple') emit('basculerLaBarre')
    else if (viewer) changerDEchelle(echelleApresDoubleToucher(viewer.currentScale, largeur), [geste.x, geste.y], 0)
  },
})

/** Un toucher sur un lien fait ce qu'il dit ; une sélection en cours n'est pas un toucher. */
function auClic(evenement: MouseEvent): void {
  if ((evenement.target as Element | null)?.closest('a, button')) return
  if (window.getSelection()?.toString()) return
  gestes.toucher({ x: evenement.clientX, y: evenement.clientY, t: evenement.timeStamp })
}

function recalerLaLargeur(): void {
  if (!viewer?.pdfDocument) return
  const page = viewer.currentPageNumber
  viewer.currentScaleValue = 'page-width'
  largeur = viewer.currentScale
  viewer.currentPageNumber = page
}

// Le visionneur est fixe, sous l'en-tête : il part du haut de la zone de contenu, que
// seuls l'en-tête et le bandeau déplacent — pas l'attente, qui passe au-dessus de lui.
const zone = () => ancre.value?.parentElement ?? null
function mesurerLeHaut(): void {
  const contenu = zone()
  if (contenu) haut.value = contenu.getBoundingClientRect().top + window.scrollY
}

let taille: ResizeObserver | null = null

function perdreLeReseau(): void {
  // La seconde sécurité ne sanctionne jamais le réseau.
  surveillance.arreter()
  reseauPerdu.value = true
  emit('reseauPerdu')
}

function auRetourDuReseau(): void {
  if (reseauPerdu.value) emit('reseauRevenu')
}

onMounted(async () => {
  mesurerLeHaut()
  if (typeof ResizeObserver !== 'undefined') {
    taille = new ResizeObserver(() => {
      mesurerLeHaut()
      recalerLaLargeur()
    })
    // La zone change de taille dès que l'en-tête ou le bandeau change.
    const contenu = zone()
    if (contenu) taille.observe(contenu)
  }
  window.addEventListener('online', auRetourDuReseau, { signal: arret.signal })

  const chargement = await chargerPdfjs()
  if (chargement.etat !== 'pret' || !conteneur.value || arret.signal.aborted) {
    if (chargement.etat !== 'pret') emit('bascule', 'indisponible')
    return
  }
  const { pdfjs, visionneur } = chargement
  const bus = new visionneur.EventBus()
  const liens = new visionneur.PDFLinkService({ eventBus: bus, externalLinkTarget: visionneur.LinkTarget.BLANK })
  viewer = new visionneur.PDFViewer({ container: conteneur.value as HTMLDivElement, eventBus: bus, linkService: liens })
  liens.setViewer(viewer)

  let premiere = false
  bus.on('pagesinit', () => {
    if (!viewer) return
    viewer.currentScaleValue = 'page-width'
    largeur = viewer.currentScale
    viewer.currentPageNumber = Math.min(Math.max(1, props.pageInitiale), viewer.pagesCount)
  })
  bus.on('pagechanging', ({ pageNumber }: { pageNumber: number }) => emit('page', pageNumber))
  bus.on('pagerendered', ({ error }: { error: unknown }) => {
    if (error) return surveillance.erreur()
    if (premiere) return
    premiere = true
    surveillance.rendue()
    emit('premierePage')
  })

  new pdfjs.TouchManager({
    container: conteneur.value,
    signal: arret.signal,
    isPinchingDisabled: () => !viewer?.pdfDocument,
    onPinchStart: () => gestes.annulerEnCours(),
    onPinching: (origine: [number, number], avant: number, apres: number) => {
      if (viewer && avant > 0) changerDEchelle(viewer.currentScale * (apres / avant), origine, 400)
    },
  })

  surveillance.demarrer()
  try {
    const ouvert = await ouvrirLeDocument(pdfjs, props.source, {
      surChangement: (suivi) => {
        surveillance.reseau(suivi.enAttenteDuReseau)
        emit('plages', suivi)
      },
    })
    tache = ouvert.tache
    void ouvert.plages?.echec.then(perdreLeReseau)
    const pdf = await ouvert.document
    if (arret.signal.aborted) return
    viewer.setDocument(pdf)
    liens.setDocument(pdf)
  } catch (erreur) {
    if (erreur instanceof ErreurDeReseau) perdreLeReseau()
    else if (!arret.signal.aborted) surveillance.erreur()
  }
})

onBeforeUnmount(() => {
  arret.abort()
  taille?.disconnect()
  gestes.arreter()
  surveillance.arreter()
  // Les données du document quittent le travailleur : un réservé ne s'y attarde pas (FR-029).
  void tache?.destroy()
  viewer = null
})

defineExpose({
  allerALaPage(index: number): void {
    if (viewer?.pdfDocument) viewer.currentPageNumber = index
  },
})
</script>

<template>
  <div ref="ancre" class="gn-lecteur-pages__ancre" />
  <div
    class="gn-lecteur-pages"
    :class="{ 'gn-lecteur-pages--sans-reseau': reseauPerdu, 'gn-lecteur-pages--cachee': props.cachee }"
    :style="{ '--gn-lecteur-haut': `${haut}px`, '--gn-page-sans-reseau': JSON.stringify(t('gn-lecteur-pages.page-sans-reseau')) }"
  >
    <div ref="conteneur" class="gn-lecteur-pages__conteneur" @click="auClic">
      <div class="pdfViewer" />
    </div>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-lecteur-pages {
  position: fixed;
  inset-block: var(--gn-lecteur-haut, 0) calc(var(--gn-barre-lecture-repliee) + var(--gn-jauge) + env(safe-area-inset-bottom));
  /* Sur un grand écran, la page tient la colonne de lecture (R6). */
  inset-inline: max(0px, calc((100% - var(--gn-colonne-largeur)) / 2));
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-lecteur-pages--cachee {
  visibility: hidden;
}

/* PDFViewer exige un conteneur positionné en absolu. */
[data-app="guide-nego"] .gn-lecteur-pages__conteneur {
  position: absolute;
  inset: 0;
  overflow: auto;
  /* Le pincement et le double toucher sont ceux du lecteur, pas ceux du navigateur. */
  touch-action: pan-x pan-y;
  -webkit-overflow-scrolling: touch;
}

/* Une page que le réseau n'a pas amenée le dit, au lieu de rester blanche. */
[data-app="guide-nego"] .gn-lecteur-pages--sans-reseau .page:not([data-loaded]) {
  display: grid;
  place-items: center;
}

[data-app="guide-nego"] .gn-lecteur-pages--sans-reseau .page:not([data-loaded])::after {
  content: var(--gn-page-sans-reseau);
  max-inline-size: 80%;
  padding: var(--gn-espace-16);
  /* Sur la page blanche du document, dans les deux thèmes : le message porte son fond. */
  background: var(--gn-fond);
  border: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  text-align: center;
}
</style>
