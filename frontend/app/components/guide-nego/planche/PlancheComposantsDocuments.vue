<script setup lang="ts">
import type { ImageDePage } from '~/composables/guide-nego/useGnLecteur'
import type { LibraryDocument, ReadingPage } from '~/types/negotiation-documents'
/**
 * Section 5, cinquième lot : la bibliothèque de documents — ligne de document, feuille
 * de filtre, bandeau « Remplacé par… », progression, barre de lecture, page lue. Les spécimens sont les cinq documents de la
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
  </div>
</template>

<style>
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
