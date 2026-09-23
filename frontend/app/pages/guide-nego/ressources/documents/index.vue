<script setup lang="ts">
import type { LibraryDocument } from '~/types/negotiation-documents'
import {
  AXES,
  aucunFiltre,
  compterAvec,
  correspondALaRecherche,
  filtrer,
  filtresActifs,
  libelleDuTerme,
  marquesDeLigne,
  optionsDuFiltre,
  type Axe,
  type Critere,
  type Filtres,
} from '~/utils/guide-nego/documents'

/**
 * Écran 03 · 01 à 04 — la bibliothèque. Publique : un réservé paraît avec sa marque,
 * sa fiche dit ce qu'il faut pour l'ouvrir. L'état « accès refusé » est donc sans objet.
 *
 * Filtres et recherche vivent dans l'adresse : revenir d'une fiche les retrouve.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const connexion = useGnConnexion()
const { momentLisible } = useGnMomentLecture()
const { progressions } = useGnCopies()
const { etat, rafraichir, bibliotheque, documents, telecharges, nouveaux, dansLeTexte, chercherDansLeTexte } =
  useGnDocuments()

const PARAMETRES: Record<Axe, string> = { types: 'type', themes: 'theme', cops: 'cop' }
const DELAI_TEXTE = 300

const enLigne = computed(() => connexion.etat.value.enLigne)

function lirePremier(valeur: unknown): string {
  const premier = Array.isArray(valeur) ? valeur[0] : valeur
  return typeof premier === 'string' ? premier : ''
}

const filtres = computed<Filtres>(() => {
  const lus = aucunFiltre()
  for (const axe of AXES) {
    lus[axe] = lirePremier(route.query[PARAMETRES[axe]]).split(',').filter(Boolean)
  }
  return lus
})

const recherche = ref(lirePremier(route.query.q))

const retour = useGnRetourALaBibliotheque()
watch(() => route.fullPath, (adresse) => (retour.value = adresse), { immediate: true })

function poserDansLAdresse(suivants: Filtres, q: string): void {
  const query: Record<string, string> = {}
  for (const axe of AXES) if (suivants[axe].length) query[PARAMETRES[axe]] = suivants[axe].join(',')
  if (q.trim()) query.q = q
  void router.replace({ query })
}

watch(recherche, (q) => {
  if (q !== lirePremier(route.query.q)) poserDansLAdresse(filtres.value, q)
})
watch(
  () => lirePremier(route.query.q),
  (q) => {
    if (q.trim() !== recherche.value.trim()) recherche.value = q
  },
)

// Le texte ne compte que pour la recherche qui l'a demandé : une réponse d'une autre
// saisie ferait paraître des documents étrangers à ce qui est tapé.
const texteCherche = ref<string | null>(null)
const texteEnCours = ref(false)
let minuterie: ReturnType<typeof setTimeout> | undefined
let rang = 0

async function chercherLeTexte(q: string): Promise<void> {
  const demande = ++rang
  await chercherDansLeTexte(q)
  if (demande !== rang) return
  texteCherche.value = q.trim()
  texteEnCours.value = false
}

watch(recherche, (q) => {
  clearTimeout(minuterie)
  texteEnCours.value = enLigne.value && q.trim().length > 1
  minuterie = setTimeout(() => void chercherLeTexte(q), DELAI_TEXTE)
})

onMounted(() => {
  void rafraichir()
  if (recherche.value.trim()) void chercherLeTexte(recherche.value)
})
onBeforeUnmount(() => clearTimeout(minuterie))

const trouvesDansLeTexte = computed<ReadonlySet<string> | null>(() =>
  dansLeTexte.value && texteCherche.value === recherche.value.trim() ? new Set(dansLeTexte.value.keys()) : null,
)

const critere = computed<Critere>(() => ({
  filtres: filtres.value,
  recherche: recherche.value,
  dansLeTexte: trouvesDansLeTexte.value,
}))

const retenus = computed(() => filtrer(documents.value, critere.value))
const cherche = computed(() => filtresActifs(filtres.value) || !!recherche.value.trim())

const vocabulaire = computed(() => bibliotheque.value?.vocabulary ?? { types: [], themes: [], cops: [] })

function libelleDe(axe: Axe, valeur: string): string {
  if (axe === 'cops') return vocabulaire.value.cops.find((c) => c.id === valeur)?.label ?? valeur
  return libelleDuTerme(axe === 'types' ? vocabulaire.value.types : vocabulaire.value.themes, valeur) ?? valeur
}

function libelleDePilule(axe: Axe): string {
  const nom = t(`guide-nego.documents.filtres.${axe}.pilule`)
  const choix = filtres.value[axe]
  if (!choix.length) return nom
  const valeur = choix.length === 1 ? libelleDe(axe, choix[0] ?? '') : String(choix.length)
  return t('guide-nego.documents.filtres.choisi', { nom, valeur })
}

const feuilleOuverte = ref(false)
const axeDeLaFeuille = ref<Axe>('types')

function ouvrirLaFeuille(axe: Axe): void {
  axeDeLaFeuille.value = axe
  feuilleOuverte.value = true
}

const optionsDeLaFeuille = computed(() =>
  optionsDuFiltre(documents.value, vocabulaire.value, critere.value, axeDeLaFeuille.value),
)

function compterPour(brouillon: string[]): number {
  return compterAvec(documents.value, critere.value, axeDeLaFeuille.value, brouillon)
}

function appliquer(valeurs: string[]): void {
  poserDansLAdresse({ ...filtres.value, [axeDeLaFeuille.value]: valeurs }, recherche.value)
}

const nombreTelecharges = computed(() => documents.value.filter((d) => telecharges.value.has(d.id)).length)
const nombreNouveaux = computed(() => documents.value.filter((d) => nouveaux.value.has(d.id)).length)
const lu = computed(() => momentLisible(etat.value.luA))

const sousTitre = computed(() => {
  if (!bibliotheque.value) return undefined
  const total = documents.value.length
  const ouverts = nombreTelecharges.value
  return t('guide-nego.documents.sous-titre.ligne', {
    documents: t('guide-nego.documents.sous-titre.documents', { count: total }, total),
    telecharges: enLigne.value
      ? t('guide-nego.documents.sous-titre.telecharges', { count: ouverts }, ouverts)
      : t('guide-nego.documents.sous-titre.lisibles', { count: ouverts }, ouverts),
  })
})

const groupe = computed(() => {
  if (cherche.value) {
    return {
      titre: t('guide-nego.documents.groupe.resultats'),
      note: t('guide-nego.documents.groupe.sur', { n: retenus.value.length, total: documents.value.length }),
    }
  }
  let note: string | undefined
  if (!enLigne.value) note = lu.value ? t('guide-nego.documents.groupe.connus', { moment: lu.value }) : undefined
  else if (nombreNouveaux.value) {
    note = t('guide-nego.documents.groupe.nouveaux', { count: nombreNouveaux.value }, nombreNouveaux.value)
  }
  return { titre: t('guide-nego.documents.groupe.tous'), note }
})

const titreDuVide = computed(() => {
  const criteres = AXES.flatMap((axe) => filtres.value[axe].map((v) => libelleDe(axe, v)))
  const q = recherche.value.trim()
  if (q) criteres.unshift(t('guide-nego.documents.vide-filtre.recherche', { recherche: q }))
  return t('guide-nego.documents.vide-filtre.titre', { criteres: criteres.join(' · ') })
})

function trouveDe(document: LibraryDocument): { page: string; extrait: string } | null {
  if (correspondALaRecherche(document, recherche.value, null)) return null
  const page = trouvesDansLeTexte.value && dansLeTexte.value?.get(document.id)
  return page ? { page: page.label, extrait: page.excerpt } : null
}

const lignes = computed(() =>
  retenus.value.map((document) => ({
    document,
    type: libelleDuTerme(vocabulaire.value.types, document.type) ?? '',
    marques: marquesDeLigne(document, {
      telecharge: telecharges.value.has(document.id),
      nouveau: nouveaux.value.has(document.id),
      enLigne: enLigne.value,
    }),
    progression: progressions.value[document.id] ?? null,
    trouve: trouveDe(document),
  })),
)

useHead({ title: t('guide-nego.documents.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.documents.titre')"
    :sous-titre="sousTitre"
    retour="/guide-nego/ressources"
    :ce-qui-se-lit="t('guide-nego.documents.hors-connexion')"
  >
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="etat.luA" />
    </template>

    <GnChargement
      v-if="!etat.pret"
      forme="squelette"
      :lignes="6"
      :libelle="t('guide-nego.documents.chargement')"
      class="gn-documents__attente"
    />

    <GnEtatErreur
      v-else-if="!bibliotheque"
      :titre="t('guide-nego.documents.erreur.titre')"
      :texte="t(enLigne ? 'guide-nego.documents.erreur.texte' : 'guide-nego.documents.erreur.texte-hors-connexion')"
      :sortie="t('guide-nego.documents.erreur.reessayer')"
      @sortie="rafraichir()"
    />

    <GnEtatVide
      v-else-if="!documents.length"
      picto="doc"
      :titre="t('guide-nego.documents.vide.titre')"
      :texte="t('guide-nego.documents.vide.texte')"
    />

    <template v-else>
      <div class="gn-documents__outils">
        <GnChampRecherche
          v-model="recherche"
          :libelle="t('guide-nego.documents.recherche')"
          :indication="t('guide-nego.documents.recherche')"
          :chargement="texteEnCours"
        />
        <div class="gn-documents__filtres">
          <GnPilule
            v-for="axe in AXES"
            :key="axe"
            variante="chevron"
            :choisie="filtres[axe].length > 0"
            :desactive="!vocabulaire[axe].length && !filtres[axe].length"
            :ouverte="feuilleOuverte && axeDeLaFeuille === axe"
            @clic="ouvrirLaFeuille(axe)"
          >
            <span class="gn-documents__pilule">{{ libelleDePilule(axe) }}</span>
          </GnPilule>
        </div>
      </div>

      <GnEnteteGroupe :titre="groupe.titre" :note="groupe.note" />

      <GnEtatVide
        v-if="!lignes.length"
        picto="doc"
        :titre="titreDuVide"
        :texte="t('guide-nego.documents.vide-filtre.texte', { count: documents.length }, documents.length)"
        :sortie="t('guide-nego.documents.vide-filtre.retirer')"
        sortie-vers="/guide-nego/ressources/documents"
      />

      <ul v-else class="gn-documents__liste">
        <li v-for="ligne in lignes" :key="ligne.document.id">
          <GnLigneDocument
            :document="ligne.document"
            :type="ligne.type"
            :marques="ligne.marques"
            :vers="`/guide-nego/ressources/documents/${ligne.document.id}`"
            :progression="ligne.progression"
            :trouve="ligne.trouve"
          />
        </li>
      </ul>

      <GnFeuilleFiltre
        :ouverte="feuilleOuverte"
        :titre="t(`guide-nego.documents.filtres.${axeDeLaFeuille}.titre`)"
        :tous="t(`guide-nego.documents.filtres.${axeDeLaFeuille}.tous`)"
        :options="optionsDeLaFeuille"
        :choix="filtres[axeDeLaFeuille]"
        :compter-pour="compterPour"
        @appliquer="appliquer"
        @fermer="feuilleOuverte = false"
      />
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-documents__attente {
  margin-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-documents__outils {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-documents__filtres {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gn-espace-8);
}

/* Une pilule choisie peut porter un long libellé : elle se coupe plutôt que d'élargir l'écran. */
[data-app="guide-nego"] .gn-documents__filtres > .gn-pilule {
  max-inline-size: 100%;
}

[data-app="guide-nego"] .gn-documents__pilule {
  min-inline-size: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
