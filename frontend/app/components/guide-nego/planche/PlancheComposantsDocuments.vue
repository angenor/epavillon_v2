<script setup lang="ts">
import type { ImageDePage } from '~/composables/guide-nego/useGnLecteur'
import type { Passage } from '~/utils/guide-nego/lecteur'
import type { LibraryDocument, OutlineEntry, ReadingPage } from '~/types/negotiation-documents'
/**
 * Section 5, cinquième lot : la bibliothèque de documents — ligne de document, feuille
 * de filtre, bandeau « Remplacé par… », progression, barre de lecture, page lue, sommaire,
 * occurrence. Les spécimens sont les cinq documents de la
 * maquette 03 ; leurs libellés de type viennent d'ici, comme ils viendraient de l'API.
 */
const { t } = useI18n()

const k = (cle: string) => t(`gn-planche-composants-documents.${cle}`)

const VERS = '/guide-nego/ressources/documents'

// Chaque sorte de bloc, et une page d'origine qui n'est pas sur le téléphone.
const pageLue = computed<ReadingPage>(() => ({
  index: 59,
  label: '59',
  blocks: [
    { kind: 'heading', level: 2, spans: [{ text: k('page-titre') }] },
    {
      kind: 'paragraph',
      spans: [{ text: k('page-avant-terme') }, { text: 'global goal on adaptation', italic: true, term: true }, { text: k('page-apres-terme') }],
    },
    { kind: 'list_item', depth: 0, marker: '•', spans: [{ text: k('page-element') }] },
    { kind: 'origin', reason: 'table', text: [{ text: k('page-tableau') }] },
    { kind: 'note', mark: '1', spans: [{ text: k('page-note') }] },
  ],
}))
// La recherche, le sommaire en panneau et les réglages : ce que la barre de lecture ouvre.
const rechercheDeSpecimen = ref(k('occurrence-expression'))
const passagesDeSpecimen = computed<Passage[]>(() =>
  [18, 59].map((page, rang) => ({
    page,
    bloc: rang,
    champ: 'spans',
    debut: 0,
    fin: 0,
    etiquette: String(page),
    section: k(rang ? 'panneau-section-ici' : 'panneau-section'),
    extrait: { avant: k('panneau-avant'), trouve: rechercheDeSpecimen.value, apres: k('panneau-apres') },
  })),
)
const pagesDeSpecimen = computed<ReadingPage[]>(() =>
  [47, 57, 59, 60, 61, 62].map((index) => ({ index, label: String(index), blocks: [] })),
)
const panneauOuvert = ref(false)
const reglagesOuverts = ref(false)

const sansImage = async (): Promise<ImageDePage> => 'hors-connexion'

function specimen(id: string, champs: Partial<LibraryDocument>): LibraryDocument {
  return {
    id,
    slug: id,
    version: '1',
    title: '',
    summary: null,
    type: 'guide',
    themes: [],
    themes_hidden: false,
    cop: null,
    issued_on: null,
    published_at: '2026-11-02T09:00:00Z',
    publisher: null,
    locale: 'fr',
    source: 'file',
    external_url: null,
    link_host: null,
    restricted: false,
    accessible: true,
    page_count: null,
    reading_bytes: null,
    mode: 'reflow',
    superseded_by: null,
    reading_etag: null,
    ...champs,
  }
}

const guide = computed(() =>
  specimen('guide', {
    title: k('titre-guide'),
    issued_on: '2026-11-02',
    publisher: 'IFDD',
    page_count: 92,
    reading_bytes: 7_000_000,
  }),
)
const resume = computed(() =>
  specimen('resume', {
    title: k('titre-resume'),
    type: 'summary',
    issued_on: '2026-10-20',
    publisher: 'IFDD',
    page_count: 12,
    reading_bytes: 1_800_000,
    restricted: true,
    accessible: false,
  }),
)
const bulletin = computed(() =>
  specimen('bulletin', {
    title: k('titre-bulletin'),
    type: 'bulletin',
    source: 'link',
    publisher: 'IISD',
    link_host: 'enb.iisd.org',
    mode: null,
  }),
)
const delegation = computed(() =>
  specimen('delegation', {
    title: k('titre-delegation'),
    publisher: 'IISD',
    page_count: 148,
    reading_bytes: 9_400_000,
  }),
)
const note = computed(() =>
  specimen('note', {
    title: k('titre-note'),
    type: 'technical_note',
    issued_on: '2025-12-15',
    publisher: 'IFDD',
    page_count: 24,
    reading_bytes: 3_100_000,
    superseded_by: { id: 'guide', title: k('titre-guide'), published_at: '2026-11-02T09:00:00Z', page_count: 92 },
  }),
)

const feuille = ref(false)
const choixDeType = ref<string[]>(['bulletin'])

const optionsDeType = computed(() => [
  { valeur: 'guide', libelle: k('type-guide'), compte: 2 },
  { valeur: 'summary', libelle: k('type-resume'), compte: 1 },
  { valeur: 'technical_note', libelle: k('type-note'), compte: 1 },
  { valeur: 'bulletin', libelle: k('type-bulletin'), compte: 1 },
])

function compterPour(brouillon: string[]): number {
  const retenues = optionsDeType.value.filter((o) => brouillon.length === 0 || brouillon.includes(o.valeur))
  return retenues.reduce((somme, o) => somme + o.compte, 0)
}

const LECTEUR_ACTIONS: Array<'sommaire' | 'rechercher' | 'reglages'> = ['sommaire', 'rechercher', 'reglages']
const lectureDepliee = ref(true)

function entree(cle: string, level: 1 | 2 | 3, page: number, children: OutlineEntry[] = []): OutlineEntry {
  return { title: k(`sommaire-${cle}`), level, page_index: page, children }
}

const sommaire = computed<OutlineEntry[]>(() => {
  const enCours = entree('3-6-1', 3, 59)
  return [
    entree('2', 1, 26, [entree('2-1', 2, 26), entree('2-2', 2, 31)]),
    entree('3', 1, 47, [
      entree('3-5', 2, 57),
      entree('3-6', 2, 59, [enCours, entree('3-6-2', 3, 60), entree('3-6-3', 3, 61)]),
      entree('3-7', 2, 62),
    ]),
    entree('annexes', 1, 65, [entree('annexe-1', 2, 65), entree('annexe-2', 2, 70)]),
  ]
})
const sommaireEnCours = computed(() => sommaire.value[1]?.children[1]?.children[0] ?? null)
// Par rang : le sommaire se recompose au changement de langue, ses entrées avec lui.
const chapitresOuverts = ref(new Set<number>([1]))
const sommaireDepliees = computed<ReadonlySet<OutlineEntry>>(
  () => new Set(sommaire.value.filter((_, rang) => chapitresOuverts.value.has(rang))),
)
const sommairePage = ref<number | null>(null)
const etiquetteDePage = (index: number) => String(index)

function basculerSommaire(chapitre: OutlineEntry) {
  const rang = sommaire.value.indexOf(chapitre)
  const ouverts = new Set(chapitresOuverts.value)
  if (!ouverts.delete(rang)) ouverts.add(rang)
  chapitresOuverts.value = ouverts
}

const OCCURRENCES = 5
const occurrence = ref(3)
const occurrenceOuverte = ref(true)
const occurrenceDecalee = (pas: number) => {
  occurrence.value = ((occurrence.value - 1 + pas + OCCURRENCES) % OCCURRENCES) + 1
}

const choixLisible = computed(() =>
  choixDeType.value.length === 0
    ? k('filtre-tous')
    : optionsDeType.value.filter((o) => choixDeType.value.includes(o.valeur)).map((o) => o.libelle).join(', '),
)
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection
      :titre="k('ligne')"
      :propos="k('ligne-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <span class="gn-planche-composants__legende">{{ k('ligne-en-ligne') }}</span>
        <div class="gn-planche-liste">
          <GnLigneDocument :document="guide" :type="k('type-guide')" :marques="['telecharge', 'a-jour']" :vers="VERS" />
          <GnLigneDocument :document="resume" :type="k('type-resume')" :marques="['nouveau', 'reserve']" :vers="VERS" />
          <GnLigneDocument :document="bulletin" :type="k('type-bulletin')" :marques="['nouveau', 'lien-externe']" :vers="VERS" />
          <GnLigneDocument :document="note" :type="k('type-note')" :marques="['remplace']" :vers="VERS" />
        </div>

        <span class="gn-planche-composants__legende">{{ k('ligne-hors-connexion') }}</span>
        <div class="gn-planche-liste">
          <GnLigneDocument :document="resume" :type="k('type-resume')" :marques="['reserve', 'non-telecharge']" :vers="VERS" />
          <GnLigneDocument :document="bulletin" :type="k('type-bulletin')" :marques="['lien-reseau']" :vers="VERS" />
        </div>

        <span class="gn-planche-composants__legende">{{ k('ligne-telechargement') }}</span>
        <div class="gn-planche-liste">
          <GnLigneDocument
            :document="delegation"
            :type="k('type-guide')"
            :marques="['a-jour']"
            :vers="VERS"
            :progression="{ recus: 3_300_000, total: 9_400_000 }"
          />
          <GnLigneDocument
            :document="delegation"
            :type="k('type-guide')"
            :marques="['a-jour']"
            :vers="VERS"
            :progression="{ recus: 3_300_000, total: null }"
          />
        </div>

        <span class="gn-planche-composants__legende">{{ k('ligne-recherche') }}</span>
        <div class="gn-planche-liste">
          <GnLigneDocument
            :document="guide"
            :type="k('type-guide')"
            :marques="['telecharge', 'a-jour']"
            :vers="VERS"
            :trouve="{ page: '59', extrait: k('extrait') }"
          />
        </div>
      </div>
      <p class="gn-planche-note">{{ k('ligne-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="k('filtre')"
      :propos="k('filtre-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnBouton variante="secondaire" @clic="feuille = true">{{ k('filtre-ouvrir') }}</GnBouton>
        <p class="gn-planche-valeur" role="status">{{ t('gn-planche-composants-documents.filtre-applique', { choix: choixLisible }) }}</p>
      </div>
      <p class="gn-planche-note">{{ k('filtre-note') }}</p>

      <GnFeuilleFiltre
        :ouverte="feuille"
        :titre="k('filtre-titre')"
        :tous="k('filtre-tous')"
        :options="optionsDeType"
        :choix="choixDeType"
        :compter-pour="compterPour"
        @appliquer="choixDeType = $event"
        @fermer="feuille = false"
      />
    </GnPlancheSection>

    <GnPlancheSection
      :titre="k('bandeau')"
      :propos="k('bandeau-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnBandeauRemplace
          :titre="k('titre-guide')"
          publie-le="2026-11-02T09:00:00Z"
          :pages="92"
          telecharge
          :vers="VERS"
        />
        <GnBandeauRemplace
          :titre="k('titre-guide')"
          publie-le="2026-11-02T09:00:00Z"
          :pages="null"
          :telecharge="false"
          reserve-sans-acces
          :vers="VERS"
        />
      </div>
      <p class="gn-planche-note">{{ k('bandeau-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="k('progression')"
      :propos="k('progression-propos')"
    >
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <span class="gn-planche-composants__legende">{{ k('progression-connue') }}</span>
        <GnProgression :part="0.35" :libelle="k('progression-libelle')" />
        <span class="gn-planche-composants__legende">{{ k('progression-inconnue') }}</span>
        <GnProgression :part="null" :libelle="k('progression-libelle')" />
      </div>
      <p class="gn-planche-note">{{ k('progression-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection
      :titre="k('lecture')"
      :propos="k('lecture-propos')"
    >
      <span class="gn-planche-composants__legende">{{ k('lecture-repliee') }}</span>
      <div class="gn-planche-composants__cadre gn-planche-cadre-lecture">
        <div class="gn-planche-cadre-lecture__page">
          <p class="gn-planche-cadre-lecture__titre">{{ k('lecture-section') }}</p>
          <p>{{ k('lecture-texte') }}</p>
        </div>
        <GnBarreLecture :page="59" :total="92" :section="k('lecture-section')" :actions="LECTEUR_ACTIONS" />
      </div>

      <span class="gn-planche-composants__legende">{{ k('lecture-depliee') }}</span>
      <div class="gn-planche-composants__cadre gn-planche-cadre-lecture">
        <div class="gn-planche-cadre-lecture__page" @click="lectureDepliee = !lectureDepliee">
          <p class="gn-planche-cadre-lecture__titre">{{ k('lecture-section') }}</p>
          <p>{{ k('lecture-texte') }}</p>
        </div>
        <GnBarreLecture
          v-model:depliee="lectureDepliee"
          :page="59"
          :total="92"
          :section="k('lecture-section')"
          :actions="LECTEUR_ACTIONS"
        />
      </div>
      <div class="gn-planche-composants__cadre">
        <GnBouton variante="secondaire" @clic="lectureDepliee = !lectureDepliee">{{ k('lecture-basculer') }}</GnBouton>
      </div>
      <p class="gn-planche-note">{{ k('lecture-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('page')" :propos="k('page-propos')">
      <div class="gn-planche-composants__cadre">
        <GnPageLue :page="pageLue" mode="reflow" :image-de="sansImage" />
      </div>
      <span class="gn-planche-composants__legende">{{ k('image-attente') }}</span>
      <div class="gn-planche-composants__cadre">
        <GnImageDePage etat="attente" :adresse="null" :libelle="k('image-libelle')" :attente="k('image-hors')" />
      </div>
      <span class="gn-planche-composants__legende">{{ k('image-echec') }}</span>
      <div class="gn-planche-composants__cadre">
        <GnImageDePage etat="echec" :adresse="null" :libelle="k('image-libelle')" :attente="k('image-hors')" />
      </div>
      <p class="gn-planche-note">{{ k('page-note-planche') }}</p>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('sommaire')" :propos="k('sommaire-propos')">
      <div class="gn-planche-composants__cadre gn-planche-cadre-sommaire">
        <ul class="gn-sommaire-liste" role="list">
          <GnLigneSommaire
            v-for="(chapitre, rang) in sommaire"
            :key="rang"
            :entree="chapitre"
            :etiquette-de="etiquetteDePage"
            :en-cours="sommaireEnCours"
            :depliees="sommaireDepliees"
            @aller="sommairePage = $event"
            @basculer="basculerSommaire"
          />
        </ul>
      </div>
      <p class="gn-planche-valeur" role="status">
        {{ sommairePage === null ? k('sommaire-aucun') : t('gn-planche-composants-documents.sommaire-aller', { page: sommairePage }) }}
      </p>
      <p class="gn-planche-note">{{ k('sommaire-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('occurrence')" :propos="k('occurrence-propos')">
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnOccurrence
          v-if="occurrenceOuverte"
          :rang="occurrence"
          :total="OCCURRENCES"
          :expression="k('occurrence-expression')"
          @precedente="occurrenceDecalee(-1)"
          @suivante="occurrenceDecalee(1)"
          @fermer="occurrenceOuverte = false"
        />
        <GnBouton v-else variante="secondaire" @clic="occurrenceOuverte = true">{{ k('occurrence-rouvrir') }}</GnBouton>
        <span class="gn-planche-composants__legende">{{ k('occurrence-seule') }}</span>
        <GnOccurrence :rang="1" :total="1" :expression="k('occurrence-expression')" />
      </div>
      <p class="gn-planche-note">{{ k('occurrence-note') }}</p>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('panneau')" :propos="k('panneau-propos')">
      <div class="gn-planche-composants__cadre">
        <GnLecteurRecherche
          v-model:expression="rechercheDeSpecimen"
          :passages="rechercheDeSpecimen ? passagesDeSpecimen : []"
          :pages="92"
          :ici="1"
        />
      </div>
      <div class="gn-planche-composants__cadre gn-planche-composants__vitrine">
        <GnBouton variante="secondaire" @clic="panneauOuvert = true">{{ k('panneau-sommaire') }}</GnBouton>
        <GnBouton variante="secondaire" @clic="reglagesOuverts = true">{{ k('panneau-reglages') }}</GnBouton>
      </div>
      <GnPanneauLecteur v-model="panneauOuvert" :titre="k('panneau-titre')" :sous-titre="k('panneau-sous-titre')">
        <GnLecteurSommaire :sommaire="sommaire" :pages="pagesDeSpecimen" :page-en-cours="59" @aller="panneauOuvert = false" />
      </GnPanneauLecteur>
      <GnReglagesLecture v-model="reglagesOuverts" />
      <p class="gn-planche-note">{{ k('panneau-note') }}</p>
    </GnPlancheSection>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-planche-cadre-sommaire {
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-bottom: 0;
}

/* La barre est en position fixe : le cadre transformé devient son bloc conteneur, comme
   pour la barre d'onglets, et elle s'y pose sur le texte comme sur l'écran réel. */
[data-app="guide-nego"] .gn-planche-cadre-lecture {
  position: relative;
  transform: translateZ(0);
  block-size: 220px;
  overflow: hidden;
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-cadre-lecture__page {
  padding: var(--gn-espace-12) var(--gn-marge-ecran);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-planche-cadre-lecture__page p {
  margin: 0;
}

[data-app="guide-nego"] .gn-planche-cadre-lecture__titre {
  color: var(--gn-titre);
  font-size: var(--gn-taille-24);
  line-height: var(--gn-interligne-24);
  font-weight: var(--gn-graisse-gras);
}
</style>
