/**
 * Données simulées du schéma `content` — les CONTENUS MIS EN AVANT de la
 * vitrine publique, et la reconstitution de `content.v_showcase`.
 *
 * ── CE QUE CE JEU DOIT PROUVER ──────────────────────────────────────────────
 *
 * Treize diapositives, écrites à la main, et chacune choisie pour un cas que
 * l'écran doit tenir. Ce n'est pas treize fois le même témoignage :
 *
 *   · SIX NATURES sur les six semées — deux témoignages écrits, une parole de
 *     négociateur en vidéo, une innovation, une bonne pratique, une annonce de
 *     journée spéciale, un chiffre clé. La v1 en connaissait trois, figées dans
 *     le composant du bandeau et en français seulement ;
 *   · QUATRE RATTACHEMENTS — COP31 (à venir), COP30 (passée), COP29 (passée),
 *     cycle PACO (en cours), plus DEUX CONTENUS DE PLATEFORME (`event_id` nul).
 *     Sans cette dispersion, ni le filtrage par période ni le périmètre
 *     d'administration n'auraient de quoi se démontrer : une administratrice
 *     limitée à la COP31 en voit exactement sept, et aucun contenu de plateforme ;
 *   · LES TROIS ÉTATS — une diapositive ARCHIVÉE et un BROUILLON, que le
 *     back-office montre et que la vitrine ignore ;
 *   · LA FENÊTRE DE DIFFUSION — une annonce dont `ends_at` est PASSÉ. Elle s'est
 *     éteinte toute seule, ce que la v1 faisait en comparant des dates dans le
 *     composant ;
 *   · LES REPLIS DE FOND — une diapositive à fond vidéo PRÊT, une à fond vidéo
 *     ENCORE EN TRAITEMENT (le bandeau se rabat sur l'image), une sans vignette
 *     (le rail se rabat sur le fond), une sans aucun média (l'aplat de couleur
 *     est le dernier repli).
 *
 * ── LES DEUX RÈGLES DE MODÈLE QU'ON NE CONTOURNE PAS ────────────────────────
 *
 * LA NATURE EST UNE TAXONOMIE, PAS UN ENUM. Les six termes de
 * `highlight_nature` sont semés par `115_content.sql` § 6 et déclarés ci-dessous
 * — pas dans un fichier i18n. Leur libellé et leur couleur viennent de la base,
 * et un administrateur peut en ajouter : c'est exactement ce que la v1 avait
 * raté en écrivant « Innovation/Bonne pratique » en dur dans le composant.
 *
 * L'ORGANISATION SE DÉSIGNE OU SE NOMME, JAMAIS LES DEUX
 * (`ck_highlights_organization_shape`). Douze diapositives référencent le
 * répertoire ; une seule porte `organization_label`, parce que l'organisation
 * citée n'y figure pas. C'est la règle métier n° 1.
 */

import type { EntityTerm, TaxonomyTerm } from '~/types/reference'
import type { Highlight, HighlightMediaRule } from '~/types/content'
import type { ScheduleThemeBadge, ShowcaseRow } from '~/types/views'
import type { Uuid } from '~/types/shared'
import { COUNTRY, EVENT, HIGHLIGHT, HIGHLIGHT_NATURE, ORG, PERSON, SESSION, TERM } from './ids'
import { attachedImage } from './covers'
import { countries, taxonomyTerms } from './reference'
import { organizations } from './org'
import { people } from './people'
import { events } from './event'
import { allSessions } from './sessions'

// ---------------------------------------------------------------------------
// 1. La taxonomie `highlight_nature`
//
// Semée AVEC la table par `115_content.sql` § 6, donc déclarée avec elle plutôt
// que dans `mocks/reference.ts` : ces six termes appartiennent au module
// `content`, qui se charge bien après le référentiel. Les couleurs reprennent la
// sémantique de la charte — cyan pour l'information, vert pour ce qui est
// acquis, violet pour ce qui relève du récit personnel.
// ---------------------------------------------------------------------------

function natureTerm(
  id: Uuid,
  code: string,
  label: { fr: string; en: string },
  colorHex: string,
  sortOrder: number,
): TaxonomyTerm {
  return {
    id,
    taxonomy_code: 'highlight_nature',
    parent_id: null,
    code,
    label,
    description: null,
    color_hex: colorHex,
    icon: null,
    sort_order: sortOrder,
    is_active: true,
    superseded_by: null,
    metadata: {},
    created_at: '2026-08-18T09:00:00Z',
    updated_at: '2026-08-18T09:00:00Z',
  }
}

export const highlightNatureTerms = [
  natureTerm(HIGHLIGHT_NATURE.testimonial, 'testimonial', { fr: 'Témoignage', en: 'Testimonial' }, '#732f85', 10),
  natureTerm(HIGHLIGHT_NATURE.negotiatorVoice, 'negotiator_voice', { fr: 'Parole de négociateur', en: "Negotiator's voice" }, '#732f85', 20),
  natureTerm(HIGHLIGHT_NATURE.innovation, 'innovation', { fr: 'Innovation', en: 'Innovation' }, '#00a1e4', 30),
  natureTerm(HIGHLIGHT_NATURE.bestPractice, 'best_practice', { fr: 'Bonne pratique', en: 'Best practice' }, '#8fbf2f', 40),
  natureTerm(HIGHLIGHT_NATURE.announcement, 'announcement', { fr: 'Annonce', en: 'Announcement' }, '#00a1e4', 50),
  natureTerm(HIGHLIGHT_NATURE.keyFigure, 'key_figure', { fr: 'Chiffre clé', en: 'Key figure' }, '#1d1a5b', 60),
] satisfies TaxonomyTerm[]

// ---------------------------------------------------------------------------
// 2. Les contraintes de téléversement — `media.attachable_roles`
//
// `115_content.sql` § 5. Le formulaire du back-office les affiche telles quelles
// (« image, 15 Mio au plus ») : le téléversement réel arrive en phase B, la
// contrainte s'annonce dès maintenant. 200 Mio pour la vidéo est haut pour la
// plateforme et bas pour de la vidéo — une boucle de bandeau fait quinze à vingt
// secondes, muette. La v1 imposait 60 secondes par un CHECK ; la limite est ici
// une limite de POIDS, qui est le vrai coût.
// ---------------------------------------------------------------------------

export const highlightMediaRules = [
  {
    role: 'banner',
    label: { fr: 'Fond photographique', en: 'Background image' },
    allowed_mime_prefixes: ['image/*'],
    max_byte_size: 15_728_640,
  },
  {
    role: 'video',
    label: { fr: 'Fond vidéo', en: 'Background video' },
    allowed_mime_prefixes: ['video/*'],
    max_byte_size: 209_715_200,
  },
  {
    role: 'cover',
    label: { fr: 'Vignette', en: 'Thumbnail' },
    allowed_mime_prefixes: ['image/*'],
    max_byte_size: 5_242_880,
  },
] satisfies HighlightMediaRule[]

// ---------------------------------------------------------------------------
// 3. Les diapositives
// ---------------------------------------------------------------------------

/** Valeurs par défaut d'une diapositive : tout ce qu'une annonce n'a pas. */
const BLANK = {
  quote: null,
  body: null,
  person_id: null,
  author_name: null,
  author_title: null,
  organization_id: null,
  organization_label: null,
  country_id: null,
  event_id: null,
  session_id: null,
  link_url: null,
  link_label: null,
  background_color_hex: null,
  starts_at: null,
  ends_at: null,
  /** Posé par trigger au premier passage en `published` — nul pour un brouillon. */
  published_at: null,
} as const

export const highlights = [
  // =========================================================================
  // BANDEAU D'OUVERTURE — `home_hero`
  // =========================================================================

  {
    ...BLANK,
    id: HIGHLIGHT.temoignageNegociatrice,
    placement: 'home_hero',
    status: 'published',
    nature_code: 'testimonial',
    sort_order: 10,
    title: {
      fr: 'Une formation qui a changé ma façon de négocier',
      en: 'A training that changed the way I negotiate',
    },
    // L'EXTRAIT, pas la citation entière : couper appartient à la rédaction.
    quote: {
      fr: "Les formations de l'IFDD sur les négociations climatiques ont été transformatrices pour ma carrière de négociatrice.",
      en: 'The IFDD climate negotiation training was transformative for my career as a negotiator.',
    },
    body: {
      fr: "Ma participation aux formations de l'IFDD sur les négociations climatiques a été transformatrice pour ma carrière de négociatrice. Grâce aux outils pratiques et aux simulations de négociations, j'ai développé une meilleure compréhension des enjeux complexes du financement climatique et de l'adaptation.",
      en: 'Taking part in the IFDD climate negotiation training was transformative for my career as a negotiator. Thanks to the practical tools and the negotiation simulations, I gained a better grasp of the complex issues of climate finance and adaptation.',
    },
    // NOM LIBRE ET NON `person_id` : les personnes qui témoignent n'ont pas
    // toutes un compte sur la plateforme, et c'est le cas normal. Le modèle
    // prévoit les deux — voir `content.highlights.author_name`.
    author_name: 'Biligua Koivogui',
    author_title: {
      fr: 'Négociatrice climat, bénéficiaire du programme de formation',
      en: 'Climate negotiator, beneficiary of the training programme',
    },
    // `organization_label` et non `organization_id` : la contrainte
    // `ck_highlights_organization_shape` interdit les deux ensemble.
    organization_label: 'Délégation de Guinée',
    country_id: COUNTRY.gn,
    event_id: EVENT.cop30,
    link_url: 'https://www.ifdd.francophonie.org/temoignages/formation-negociateurs',
    link_label: { fr: 'Lire le témoignage complet', en: 'Read the full testimonial' },
    background_color_hex: '#054b6d',
    published_at: '2025-12-02T10:00:00Z',
    created_by: PERSON.nkoDiop,
    created_at: '2025-11-28T14:20:00Z',
    updated_at: '2026-08-06T09:00:00Z',
  },

  {
    ...BLANK,
    id: HIGHLIGHT.temoignageCooperatives,
    placement: 'home_hero',
    status: 'published',
    nature_code: 'testimonial',
    sort_order: 20,
    title: {
      fr: "Porter la voix des communautés vulnérables",
      en: 'Carrying the voice of vulnerable communities',
    },
    quote: {
      fr: "Dans un contexte largement dominé par l'anglais, le pavillon de l'OIF offre aux francophones un espace essentiel d'expression et de visibilité.",
      en: 'In a context largely dominated by English, the OIF pavilion offers French speakers an essential space to speak and be seen.',
    },
    body: {
      fr: "Cette année encore, l'OIF a permis à ma délégation à la CdP30 de porter la voix des communautés vulnérables face aux changements climatiques. Je suis fière d'avoir été impliquée dans l'organisation de la Journée de la jeunesse, qui a connu un franc succès, ainsi que d'avoir pris part à des temps forts tels que la Journée de la finance et le bilan de la première semaine de négociations.",
      en: 'Once again this year, the OIF enabled my delegation at COP30 to carry the voice of communities most vulnerable to climate change. I am proud to have taken part in organising the Youth Day, which was a real success, and to have joined key moments such as Finance Day and the first-week negotiation review.',
    },
    author_name: 'Constance Genevée',
    author_title: {
      fr: 'Membre de la délégation, CdP30',
      en: 'Delegation member, COP30',
    },
    organization_label: 'Délégation du Bénin',
    country_id: COUNTRY.bj,
    event_id: EVENT.cop30,
    background_color_hex: '#5a1f68',
    published_at: '2026-07-16T08:00:00Z',
    created_by: PERSON.bakayoko,
    created_at: '2026-07-14T10:00:00Z',
    updated_at: '2026-07-16T08:00:00Z',
  },

  {
    ...BLANK,
    id: HIGHLIGHT.paroleNegociateur,
    placement: 'home_hero',
    status: 'published',
    nature_code: 'negotiator_voice',
    sort_order: 30,
    title: {
      fr: 'Le pavillon, notre bouée de sauvetage',
      en: 'The pavilion, our lifeline',
    },
    quote: {
      fr: "Le Pavillon de la Francophonie nous sert de bouée de sauvetage quand les textes en anglais deviennent confus.",
      en: 'The Francophonie Pavilion is our lifeline when the English texts become confusing.',
    },
    body: {
      fr: "En effet, pour celles et ceux parmi nous qui participent annuellement aux Conférences des Parties, le Pavillon de la Francophonie nous sert toujours de « bouée de sauvetage », lorsqu'en plein dans les négociations, les textes en anglais deviennent un peu confus.",
      en: 'Indeed, for those of us who attend the Conferences of the Parties every year, the Francophonie Pavilion always serves as a “lifeline”, when in the middle of negotiations the English texts become somewhat confusing.',
    },
    author_name: 'Antoine Faye',
    author_title: { fr: 'Consultant indépendant', en: 'Independent consultant' },
    organization_label: 'Consultant indépendant',
    country_id: COUNTRY.sn,
    event_id: EVENT.cop31,
    link_url: 'https://www.ifdd.francophonie.org/paroles/pavillon-bouee-de-sauvetage',
    link_label: { fr: 'Voir la vidéo entière', en: 'Watch the full video' },
    background_color_hex: '#0d3b52',
    published_at: '2026-07-24T09:00:00Z',
    created_by: PERSON.bakayoko,
    created_at: '2026-07-22T16:00:00Z',
    updated_at: '2026-07-24T09:00:00Z',
  },

  {
    ...BLANK,
    id: HIGHLIGHT.innovationMesureCarbone,
    placement: 'home_hero',
    status: 'published',
    nature_code: 'innovation',
    sort_order: 40,
    title: {
      fr: 'Mesurer le carbone des sols sans laboratoire',
      en: 'Measuring soil carbon without a laboratory',
    },
    quote: {
      fr: "Un boîtier, une sonde, quarante minutes : la mesure que nous envoyions à Montpellier se fait désormais au bord du champ.",
      en: 'One unit, one probe, forty minutes: the measurement we used to send to Montpellier now happens at the edge of the field.',
    },
    body: {
      fr: "Verdeo Solutions a conçu un dispositif de mesure de terrain destiné aux coopératives agricoles, calibré sur les sols d'Afrique de l'Ouest. Il sera présenté au pavillon.",
      en: 'Verdeo Solutions has designed a field measurement device for farming cooperatives, calibrated for West African soils. It will be presented at the pavilion.',
    },
    person_id: PERSON.moreau,
    author_title: { fr: 'Directeur technique, Verdeo Solutions', en: 'Technical director, Verdeo Solutions' },
    organization_id: ORG.verdeo,
    event_id: EVENT.cop31,
    background_color_hex: '#00324a',
    published_at: '2026-08-05T07:30:00Z',
    created_by: PERSON.tremblay,
    created_at: '2026-08-04T09:00:00Z',
    updated_at: '2026-08-18T17:35:00Z',
  },

  {
    ...BLANK,
    id: HIGHLIGHT.bonnePratiquePastoralisme,
    placement: 'home_hero',
    status: 'published',
    nature_code: 'best_practice',
    sort_order: 50,
    title: {
      fr: "Des couloirs de transhumance négociés village par village",
      en: 'Transhumance corridors negotiated village by village',
    },
    quote: {
      fr: "Trois cent quarante kilomètres de couloirs balisés, et deux conflits d'usage en quatre ans, contre trente-et-un l'année d'avant.",
      en: 'Three hundred and forty kilometres of marked corridors, and two land-use conflicts in four years, against thirty-one the year before.',
    },
    person_id: PERSON.kabore,
    author_title: { fr: "Chargée de programme, Observatoire du Sahel", en: 'Programme officer, Sahel Observatory' },
    organization_id: ORG.osed,
    country_id: COUNTRY.bf,
    event_id: EVENT.cop31,
    link_url: 'https://www.ifdd.francophonie.org/pratiques/couloirs-transhumance',
    link_label: { fr: 'Consulter la fiche de pratique', en: 'Read the practice sheet' },
    background_color_hex: '#3f5a10',
    published_at: '2026-08-01T09:00:00Z',
    created_by: PERSON.nkoDiop,
    created_at: '2026-07-30T11:20:00Z',
    updated_at: '2026-08-01T09:00:00Z',
  },

  {
    ...BLANK,
    id: HIGHLIGHT.chiffreClePavillon,
    placement: 'home_hero',
    status: 'published',
    nature_code: 'key_figure',
    sort_order: 60,
    title: { fr: 'Dix ans de pavillon francophone', en: 'Ten years of the Francophonie pavilion' },
    quote: {
      fr: "612 activités tenues en français dans le pavillon de la Francophonie depuis la COP21.",
      en: '612 activities held in French at the Francophonie pavilion since COP21.',
    },
    body: {
      fr: "Le décompte couvre les conférences climat, biodiversité et désertification. Il ne retient que les activités effectivement tenues, hors annulations.",
      en: 'The count covers the climate, biodiversity and desertification conferences, and retains only activities actually held, cancellations excluded.',
    },
    // CONTENU DE PLATEFORME : `event_id` nul. Il parle de la plateforme entière,
    // et le back-office ne l'expose qu'en portée GLOBALE (ADR-14).
    organization_id: ORG.ifdd,
    // AUCUN MÉDIA rattaché : l'aplat est le seul fond. Le bandeau doit rester
    // lisible ainsi — c'est le dernier repli, et il doit être vu au moins une
    // fois pendant la mise au point.
    background_color_hex: '#1d1a5b',
    published_at: '2026-08-10T08:00:00Z',
    created_by: PERSON.bakayoko,
    created_at: '2026-08-09T15:00:00Z',
    updated_at: '2026-08-10T08:00:00Z',
  },

  {
    ...BLANK,
    id: HIGHLIGHT.annonceJourneeJeunesse,
    placement: 'home_hero',
    status: 'published',
    nature_code: 'announcement',
    sort_order: 70,
    title: {
      fr: 'Journée jeunesse et climat au pavillon',
      en: 'Youth and climate day at the pavilion',
    },
    quote: {
      fr: "Le 16 novembre, le pavillon est confié à la jeunesse francophone : ordre du jour ouvert, construit en début de séance.",
      en: 'On 16 November the pavilion is handed to French-speaking youth: an open agenda, built at the start of the session.',
    },
    // SÉANCE MISE EN AVANT : le trigger `tg_highlight_normalize` dérive
    // `event_id` de la séance. Il est écrit ici parce qu'il est CONNU et
    // COHÉRENT ; une valeur contradictoire serait refusée par la base.
    session_id: SESSION.forumJeunesse,
    event_id: EVENT.cop31,
    organization_id: ORG.ujfc,
    // La fenêtre s'ouvre dès maintenant et se ferme le lendemain de la journée :
    // l'annonce s'éteindra toute seule, sans que personne y pense.
    starts_at: '2026-08-01T00:00:00Z',
    ends_at: '2027-11-17T03:00:00Z',
    background_color_hex: '#0c6792',
    published_at: '2026-08-06T09:00:00Z',
    created_by: PERSON.nkoDiop,
    created_at: '2026-08-05T13:10:00Z',
    updated_at: '2026-08-06T09:00:00Z',
  },

  {
    ...BLANK,
    /*
     * ARCHIVÉE — et c'est sa raison d'être.
     *
     * `archived` n'est pas une suppression : ce témoignage de la COP29 reste
     * consultable au back-office et réutilisable. La vitrine publique ne le sert
     * pas, la liste d'administration le montre en gris. Sans cette ligne, l'un
     * des trois états du cycle éditorial n'aurait aucune donnée.
     */
    id: HIGHLIGHT.temoignageArchiveCop29,
    placement: 'home_hero',
    status: 'archived',
    nature_code: 'testimonial',
    sort_order: 80,
    title: {
      fr: 'Des simulations qui reflètent la salle réelle',
      en: 'Simulations that mirror the real room',
    },
    quote: {
      fr: "Les simulations de négociation reflétaient les réalités rencontrées lors de la CdP30.",
      en: 'The negotiation simulations mirrored the realities encountered at COP30.',
    },
    body: {
      fr: "Les sessions de formation que j'ai suivies à Maurice et au Ghana se sont révélées particulièrement pertinentes et efficaces, les simulations de négociation reflétant les réalités rencontrées lors de la CdP30.",
      en: 'The training sessions I attended in Mauritius and Ghana proved particularly relevant and effective, the negotiation simulations mirroring the realities encountered at COP30.',
    },
    author_name: 'Kaully Tirouvi',
    author_title: {
      fr: 'Négociatrice, bénéficiaire du programme de formation',
      en: 'Negotiator, beneficiary of the training programme',
    },
    organization_label: 'Délégation de Maurice',
    country_id: COUNTRY.mu,
    event_id: EVENT.cop29,
    background_color_hex: '#4a3f2a',
    // La date de publication est CONSERVÉE après archivage : elle dit depuis
    // quand ce contenu a été public, ce que `updated_at` ne dit pas.
    published_at: '2024-12-10T09:00:00Z',
    created_by: PERSON.bakayoko,
    created_at: '2024-12-05T10:00:00Z',
    updated_at: '2026-02-11T11:00:00Z',
  },

  {
    ...BLANK,
    /*
     * PUBLIÉE, MAIS HORS FENÊTRE — l'autre cas que la v1 ne tenait pas.
     *
     * Son statut est `published`, et pourtant elle ne sort pas : `ends_at` est
     * passé. C'est la base qui l'éteint, pas un composant qui compare des dates.
     * Le back-office doit la montrer comme « expirée » et non comme « publiée »,
     * sans quoi l'éditeur ne comprend pas pourquoi elle a disparu.
     */
    id: HIGHLIGHT.annonceWebinairePaco,
    placement: 'home_hero',
    status: 'published',
    nature_code: 'announcement',
    sort_order: 90,
    title: {
      fr: 'Webinaire PACO — financer l’adaptation',
      en: 'PACO webinar — financing adaptation',
    },
    quote: {
      fr: "Deuxième rendez-vous du cycle : monter un dossier recevable auprès du Fonds vert pour le climat.",
      en: 'Second session of the series: building an eligible application to the Green Climate Fund.',
    },
    event_id: EVENT.paco2026,
    organization_id: ORG.ifdd,
    starts_at: '2026-04-02T00:00:00Z',
    // Fenêtre CLOSE depuis juin : la vue ne la rend plus.
    ends_at: '2026-06-30T00:00:00Z',
    background_color_hex: '#1d1a5b',
    published_at: '2026-04-02T12:10:00Z',
    created_by: PERSON.tremblay,
    created_at: '2026-04-02T12:00:00Z',
    updated_at: '2026-04-02T12:10:00Z',
  },

] satisfies Highlight[]

// ---------------------------------------------------------------------------
// 4. Thématiques des diapositives — `reference.entity_terms`
//
// Le rattachement générique est ouvert : aucune déclaration n'est nécessaire
// côté modèle (`115_content.sql` § 6, dernier commentaire). Trois pastilles au
// plus s'affichent sur une carte, puis « +N » : `annonceJourneeJeunesse` en
// porte QUATRE, pour que ce repli soit exercé quelque part.
// ---------------------------------------------------------------------------

function themesOf(highlightId: Uuid, termIds: readonly Uuid[]): EntityTerm[] {
  return termIds.map((termId, index) => ({
    entity_schema: 'content',
    entity_table: 'highlights',
    entity_id: highlightId,
    term_id: termId,
    role: 'primary',
    sort_order: (index + 1) * 10,
    created_at: '2026-08-18T09:00:00Z',
  }))
}

export const highlightThemes = [
  ...themesOf(HIGHLIGHT.temoignageNegociatrice, [TERM.adaptation, TERM.waterFisheries]),
  ...themesOf(HIGHLIGHT.temoignageCooperatives, [TERM.gender, TERM.mitigation]),
  ...themesOf(HIGHLIGHT.paroleNegociateur, [TERM.transparency, TERM.climateAmbitionNdc]),
  ...themesOf(HIGHLIGHT.innovationMesureCarbone, [TERM.agricultureFood, TERM.mitigation]),
  ...themesOf(HIGHLIGHT.bonnePratiquePastoralisme, [TERM.sustainableLivestock, TERM.desertification]),
  ...themesOf(HIGHLIGHT.annonceJourneeJeunesse, [
    TERM.climateAmbitionNdc,
    TERM.gender,
    TERM.healthSolidarity,
    TERM.climateJusticeIndigenous,
  ]),
  ...themesOf(HIGHLIGHT.temoignageArchiveCop29, [TERM.climateFinance]),
  ...themesOf(HIGHLIGHT.annonceWebinairePaco, [TERM.climateFinance, TERM.adaptation]),
  // `chiffreClePavillon` n'en porte AUCUNE : un décompte de plateforme ne relève
  // d'aucune thématique, et la carte doit rester entière sans pastille.
] satisfies EntityTerm[]

// ---------------------------------------------------------------------------
// 5. `content.v_showcase`, reconstituée
//
// DÉRIVÉE des diapositives plutôt qu'écrite à la main : c'est ce que fait la
// base, et c'est ce qui garantit qu'aucune valeur ne contredise sa source. Les
// trois jointures de la vue sont reproduites dans le même ordre, y compris le
// repli du pays (`COALESCE(highlights.country_id, organizations.country_id)`) et
// la préférence du nom de profil sur le nom libre.
//
// LE FILTRE TEMPOREL EST ICI, comme il est dans la vue : statut `published` ET
// fenêtre en cours. Aucun écran ne doit le rejouer.
// ---------------------------------------------------------------------------

const natureByCode = new Map(highlightNatureTerms.map((term) => [term.code, term]))
const themeTermById = new Map(taxonomyTerms.map((term) => [term.id, term]))
const organizationById = new Map(organizations.map((org) => [org.id, org]))
const personById = new Map(people.map((person) => [person.id, person]))
const countryById = new Map(countries.map((country) => [country.id, country]))
const eventById = new Map(events.map((event) => [event.id, event]))
const sessionById = new Map(allSessions.map((session) => [session.id, session]))

/**
 * `reference.term_badges('content','highlights', id, 'activity_theme')` rejouée.
 * Termes inactifs écartés, tri par `sort_order` du lien puis du terme — le même
 * ordre que `mocks/views.ts` applique aux séances et aux propositions.
 */
export function highlightThemeBadges(highlightId: Uuid): ScheduleThemeBadge[] {
  return highlightThemes
    .filter((link) => link.entity_id === highlightId)
    .map((link) => ({ link, term: themeTermById.get(link.term_id) }))
    .filter((pair) => pair.term?.is_active && pair.term.taxonomy_code === 'activity_theme')
    .sort((a, b) => a.link.sort_order - b.link.sort_order || a.term!.sort_order - b.term!.sort_order)
    .map((pair) => ({
      code: pair.term!.code,
      label: pair.term!.label,
      color: pair.term!.color_hex,
      icon: pair.term!.icon,
    }))
}

/**
 * Une diapositive, résolue comme la vue la résout — SANS le filtre de
 * publication.
 *
 * Exportée telle quelle parce que le back-office en a besoin : l'aperçu du
 * formulaire montre un brouillon, et il le montre avec le MÊME composant que le
 * bandeau public. C'est ce qui rend cet écran utilisable, et ce qui évite une
 * seconde mise en page qui divergerait au premier ajustement.
 */
export function showcaseRowOf(highlight: Highlight): ShowcaseRow {
  const nature = natureByCode.get(highlight.nature_code)
  const person = highlight.person_id ? personById.get(highlight.person_id) : undefined
  const organization = highlight.organization_id
    ? organizationById.get(highlight.organization_id)
    : undefined
  // Repli du pays : celui de la diapositive, à défaut celui de l'organisation.
  const countryId = highlight.country_id ?? organization?.country_id ?? null
  const country = countryId ? countryById.get(countryId) : undefined
  const event = highlight.event_id ? eventById.get(highlight.event_id) : undefined
  const session = highlight.session_id ? sessionById.get(highlight.session_id) : undefined
  const themes = highlightThemeBadges(highlight.id)

  return {
    id: highlight.id,
    placement: highlight.placement,
    sort_order: highlight.sort_order,

    nature_code: highlight.nature_code,
    // Jointure `LEFT … AND n.is_active` : un terme désactivé rend un libellé nul,
    // et l'éyclette doit alors se taire plutôt que d'afficher un code.
    nature_label: nature?.label ?? null,
    nature_color: nature?.color_hex ?? null,
    nature_icon: nature?.icon ?? null,

    title: highlight.title,
    quote: highlight.quote,
    body: highlight.body,

    // `COALESCE(people.display_name, highlights.author_name)`.
    author_name: person?.display_name ?? highlight.author_name,
    author_title: highlight.author_title,
    person_id: highlight.person_id,

    organization_id: highlight.organization_id,
    // `COALESCE(organizations.legal_name, highlights.organization_label)`.
    organization_name: organization?.legal_name ?? highlight.organization_label,
    organization_acronym: organization?.acronym ?? null,

    country_code: country?.iso2 ?? null,
    country_name: country?.name ?? null,

    event_id: highlight.event_id,
    event_slug: event?.slug ?? null,
    event_title: event?.title ?? null,

    session_id: highlight.session_id,
    session_slug: session?.slug ?? null,
    session_title: session?.title ?? null,
    session_starts_at: session?.starts_at ?? null,
    session_ends_at: session?.ends_at ?? null,
    session_timezone: session?.timezone ?? null,

    link_url: highlight.link_url,
    link_label: highlight.link_label,

    // `media.attached_image()` sur les trois rôles. Elle n'en rend un que si
    // l'objet est `ready` : le fond vidéo encore en traitement sort donc NUL,
    // et le bandeau se rabat de lui-même sur l'image.
    background_image: attachedImage('content', 'highlights', highlight.id, 'banner'),
    background_video: attachedImage('content', 'highlights', highlight.id, 'video'),
    thumbnail: attachedImage('content', 'highlights', highlight.id, 'cover'),
    background_color_hex: highlight.background_color_hex,

    theme_codes: themes.map((theme) => theme.code),
    themes,

    starts_at: highlight.starts_at,
    ends_at: highlight.ends_at,
    published_at: highlight.published_at,
  }
}

/**
 * `content.v_showcase` — la vitrine publique, filtre de la vue compris.
 *
 * Recalculée à l'appel : la fenêtre de diffusion dépend de l'instant présent,
 * exactement comme le `now()` de la vue. Triée par `placement` puis
 * `sort_order`, ce que la vue demande de faire à l'appelant.
 */
export function showcase(at: number = Date.now()): ShowcaseRow[] {
  return highlights
    .filter((highlight) => {
      if (highlight.status !== 'published') return false
      if (highlight.starts_at !== null && Date.parse(highlight.starts_at) > at) return false
      if (highlight.ends_at !== null && Date.parse(highlight.ends_at) <= at) return false
      return true
    })
    .map((highlight) => showcaseRowOf(highlight))
    .sort((a, b) => a.placement.localeCompare(b.placement) || a.sort_order - b.sort_order)
}
