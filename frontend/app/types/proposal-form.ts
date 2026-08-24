/**
 * Contrats du FORMULAIRE DE SOUMISSION (A4) — ce qui circule entre l'écran et
 * l'API, et rien d'autre.
 *
 * Comme `types/auth.ts` et `types/organization-join.ts`, ces types ne décrivent
 * aucune table : ils nomment un brouillon en cours de saisie et les deux
 * écritures de l'écran. Les entités elles-mêmes vivent dans
 * `types/programme/proposal.ts`, dérivées de `070_programme_proposals.sql`.
 *
 * QUATRE DÉCISIONS DE CONTRAT, ET CHACUNE VIENT DU MODÈLE :
 *
 *  · LES TEXTES SE SAISISSENT EN FRANÇAIS. `platform.i18n_text` exige la clé
 *    `fr` non vide (`platform.is_i18n_text()`, `000_bootstrap.sql` § 5.1) : un
 *    dossier rédigé en anglais seul serait refusé par la base, pas par l'écran.
 *    Le brouillon porte donc des `string` et l'envoi les enveloppe en
 *    `{ fr: … }`. La traduction anglaise est un travail éditorial de l'IFDD, pas
 *    une seconde colonne de formulaire à remplir par le déposant.
 *  · LE CRÉNEAU EST UNE HEURE MURALE, pas un instant. On saisit « le 12 novembre
 *    à 14:30 à Belém » ; la conversion en `timestamptz` se fait au dernier
 *    moment, avec le fuseau de l'ÉDITION (`event.events.timezone`). Garder
 *    l'heure murale dans le brouillon évite qu'un changement de fuseau du
 *    navigateur déplace un créneau déjà saisi.
 *  · UN INTERVENANT EST UNE PERSONNE. `programme.proposal_speakers.person_id`
 *    est NOT NULL : l'API crée la personne à la volée si l'adresse est inconnue
 *    (`identity.people`, § 4 du fichier 070). Le brouillon ne porte donc pas un
 *    identifiant mais de quoi la retrouver ou la créer.
 *  · LE NUMÉRO DE DOSSIER EXISTE DÈS LE BROUILLON. `tg_proposals_reference_code`
 *    est un trigger `BEFORE INSERT` : la ligne naît avec son `reference_code`.
 *    L'écran peut donc l'annoncer dès le premier enregistrement automatique, et
 *    la confirmation d'envoi affiche le MÊME numéro — pas un second.
 */

import type {
  AssetId,
  CallId,
  CountryId,
  EventId,
  IsoDateTime,
  OrganizationId,
  PersonId,
  ProposalId,
  TaxonomyTermCode,
  Uuid,
} from './shared'
import type { ParticipationMode } from './event/edition'
import type { OrganizationRole, ProposalStatus, SpeakerRole } from './programme/proposal'

// ---------------------------------------------------------------------------
// Les sept étapes
// ---------------------------------------------------------------------------

/**
 * Les étapes, dans leur ordre de parcours. Ce sont des CLÉS, pas des libellés :
 * elles nomment les fichiers de traduction (`proposal.form.step-<clé>.json`) et
 * servent à rattacher chaque erreur à l'étape qui la corrige.
 */
export type ProposalFormStep =
  | 'organizations'
  | 'presentation'
  | 'classification'
  | 'speakers'
  | 'schedule'
  | 'documents'
  | 'review'

/**
 * LES ÉTAPES RÉELLEMENT PARCOURUES.
 *
 * **L'étape des documents est masquée depuis le 17/08, à la demande du
 * commanditaire** : le comité n'aura pas le temps de lire des pièces jointes
 * pour cette campagne, et proposer d'en déposer serait promettre une lecture qui
 * n'aura pas lieu. Le type, le composant, sa validation et ses traductions
 * restent en place — rien n'est supprimé, la ligne ci-dessous suffira à la
 * rouvrir. `programme.proposal_documents` existe de toute façon en base : c'est
 * une décision d'écran, pas un renoncement du modèle.
 */
export const PROPOSAL_FORM_STEPS: ProposalFormStep[] = [
  'organizations',
  'presentation',
  'classification',
  'speakers',
  'schedule',
  // 'documents',
  'review',
]

/**
 * LE SÉLECTEUR DE PHOTO D'UN INTERVENANT, ouvert ou fermé.
 *
 * Fermé pour la même raison que l'étape des documents, mais celle-ci tient au
 * chemin et non au calendrier : `DraftSpeaker.photo` part avec le brouillon et
 * l'API l'ignore en silence — la photo est un objet de `media.assets`, déposé
 * par un envoi multipart que le point d'entrée ne sait pas encore faire. Une
 * photo choisie, montrée, puis perdue est pire que pas de photo du tout. La
 * ligne se rebascule le jour où le téléversement existe.
 */
export const SPEAKER_PHOTO_ENABLED = false

// ---------------------------------------------------------------------------
// Limites de saisie
// ---------------------------------------------------------------------------

/**
 * Longueurs maximales des champs longs.
 *
 * ELLES NE VIENNENT PAS DU MODÈLE — `platform.i18n_text` est un `jsonb` sans
 * borne, et c'est justifié : la base n'a pas à trancher ce qu'est un résumé
 * lisible. Ce sont des règles d'ÉCRAN, posées à un seul endroit, et elles
 * répondent à un besoin d'aval : un résumé de mille signes ne tient pas sur une
 * carte de programmation, et personne ne le raccourcira après coup.
 *
 * La saisie n'est jamais coupée à la limite (voir `UiTextarea`) : le compteur
 * passe en rouge et l'envoi refuse, avec un message.
 */
export const TEXT_LIMITS = {
  title: 180,
  summary: 400,
  objectives: 1200,
  detailed_presentation: 4000,
  expected_outcomes: 1200,
  target_audience: 600,
  scheduling_constraints: 500,
  speaker_bio: 800,
} as const

/**
 * Contraintes de téléversement du rôle `('programme','proposals','document')` —
 * `media.attachable_roles`, semé par `050_media.sql` § 8. Recopiées ici parce
 * que l'écran doit refuser AVANT l'envoi ; elles restent vérifiées en base par
 * `media.tg_validate_attachment()`, qui fait foi.
 */
export const DOCUMENT_MAX_BYTES = 26_214_400
export const DOCUMENT_MIME_PREFIXES = ['application/pdf', 'application/vnd.']

// ---------------------------------------------------------------------------
// Le brouillon en cours de saisie
// ---------------------------------------------------------------------------

/** Une organisation associée au dossier, telle que l'étape 1 la retient. */
export interface DraftOrganization {
  organization_id: OrganizationId
  /** Jamais `lead` : le porteur principal est `ProposalDraft.organization_id`. */
  role: Exclude<OrganizationRole, 'lead'>
  /** Repris de la recherche pour afficher la ligne sans requête de plus. */
  legal_name: string
  acronym: string | null
  country_id: CountryId | null
}

/**
 * Un intervenant en cours de saisie.
 *
 * `key` n'est PAS un identifiant de base : c'est une clé de liste, stable le
 * temps de la saisie, qui permet de modifier et de réordonner avant que quoi que
 * ce soit n'existe côté serveur.
 */
export interface DraftSpeaker {
  key: string
  /**
   * PERSONNE EXISTANTE RETENUE, ou `null` pour quelqu'un que ce dossier fait
   * connaître à la plateforme.
   *
   * C'est la distinction qui commande tout le reste. `programme.proposal_speakers.person_id`
   * est NOT NULL : l'API crée la personne à la volée si l'adresse est inconnue.
   * Retenir l'identifiant quand elle EST connue évite la seconde fiche pour la
   * même personne — le défaut n° 1 de la v1, transposé de l'organisation à
   * l'intervenant, et bien moins visible.
   */
  person_id: PersonId | null
  /**
   * Le profil est-il celui d'un compte de la plateforme ?
   *
   * Vrai : l'identité appartient à son titulaire, le déposant ne la modifie pas.
   * Faux avec `person_id` renseigné : la personne existe (elle a été intervenante
   * ailleurs) mais n'a pas de compte — elle reste modifiable tant que l'activité
   * n'est pas validée. Voir `SPEAKER_IDENTITY_FIELDS`.
   */
  has_account: boolean
  /** `identity.people.civility` : `mme`, `m`, `dr`, `pr`, `other`. */
  civility: string | null
  first_name: string
  last_name: string
  /** Clé de rapprochement avec une personne existante — `people.primary_email`. */
  email: string
  /** `proposal_speakers.job_title_snapshot` : la fonction AU MOMENT de l'activité. */
  job_title: string
  /** `proposal_speakers.organization_snapshot`. */
  organization_name: string
  /** Renseigné quand l'organisation a été reconnue dans le référentiel. */
  organization_id: OrganizationId | null
  role: SpeakerRole
  /** `proposal_speakers.bio`, en français comme le reste du dossier. */
  bio: string
  /** Photo : rôle `avatar` sur `identity.people`, jamais sur la proposition. */
  photo: DraftUpload | null
}

/**
 * LES CHAMPS QUI APPARTIENNENT À LA PERSONNE, et non au dossier.
 *
 * Ils sont verrouillés dès qu'un profil EXISTANT est retenu : seul son titulaire
 * — ou un administrateur de la plateforme — modifie son identité. Un déposant qui
 * corrigerait « Awa Sow Fall » en « A. Sowfall » pour son propre confort
 * réécrirait la fiche de quelqu'un d'autre, visible de toutes ses autres
 * participations.
 *
 * `job_title` et `organization_name` n'en font PAS partie : ce sont les
 * instantanés de l'activité (`job_title_snapshot`, `organization_snapshot`), que
 * le modèle distingue explicitement de la fiche de la personne — « une personne
 * change d'employeur, l'archive de la COP28 ne doit pas être réécrite pour
 * autant ». Ils sont pré-remplis depuis le profil et restent modifiables.
 */
export const SPEAKER_IDENTITY_FIELDS = [
  'civility',
  'first_name',
  'last_name',
  'email',
  'bio',
  'photo',
] as const

/**
 * Un fichier retenu par l'écran mais pas encore déposé sur le stockage.
 *
 * Le téléversement réel (URL signée, `media.assets`) appartient au prompt B6 :
 * ici on retient ce que l'API devra recevoir, et de quoi refuser tout de suite
 * un fichier trop lourd ou d'un format non prévu.
 */
export interface DraftUpload {
  file_name: string
  mime_type: string
  byte_size: number
  /** Renseigné une fois le fichier réellement déposé (prompt B6). */
  asset_id: AssetId | null
  /**
   * APERÇU LOCAL, produit par `URL.createObjectURL()`. Il ne part JAMAIS à
   * l'API : c'est une adresse valable dans cet onglet et dans lui seul.
   *
   * Il existe parce qu'une photo choisie sans être montrée est une photo qu'on
   * ne peut pas vérifier — mauvais fichier, portrait de travers, image d'une
   * autre personne. Tant que le téléversement réel n'existe pas (prompt B6),
   * l'aperçu disparaît à la reprise d'un brouillon : l'API rendra alors l'URL de
   * l'objet stocké, et cette propriété cédera la place.
   */
  preview_url?: string
}

/** Une pièce jointe du dossier — `programme.proposal_documents`. */
export interface DraftDocument {
  key: string
  upload: DraftUpload
  /** `proposal_documents.title`, en français. */
  title: string
  /** Code de la taxonomie `document_type`. */
  document_type_code: TaxonomyTermCode | null
  /** Visible du public une fois l'activité publiée, ou pièce interne au dossier. */
  is_public: boolean
}

/**
 * L'état complet du formulaire. Chaque champ porte le nom de sa colonne quand
 * elle existe : c'est ce qui rend l'envoi lisible et le raccordement mécanique.
 */
export interface ProposalDraft {
  // — Étape 1 : organisations
  /** Porteur principal — `proposals.organization_id`, rôle `lead` par trigger. */
  organization_id: OrganizationId | null
  co_organizations: DraftOrganization[]

  // — Étape 2 : présentation (français, voir l'en-tête)
  title: string
  summary: string
  objectives: string
  /** HTML RESTREINT produit par `UiRichText` — structure seulement, aucune
   *  couleur ni police. Vide, c'est la chaîne vide et non `<p></p>`. */
  detailed_presentation: string
  expected_outcomes: string
  /** Publics visés, UN PAR ENTRÉE — `proposals.target_audiences`. Une chaîne
   *  unique « Ministères, ONG, journalistes » ne se réaffiche pas : elle
   *  s'imprime telle quelle et se découpe à la virgule par qui essaie. */
  target_audiences: string[]

  // — Étape 3 : classification
  /** Codes de `reference.taxonomy_terms` (taxonomie `activity_theme`). */
  theme_codes: TaxonomyTermCode[]
  /** Code de la taxonomie `activity_category` — `proposals.activity_type_code`. */
  activity_type_code: TaxonomyTermCode | null
  format: ParticipationMode | null
  /** Codes de `reference.locales`. `proposals.language_codes` est NOT NULL. */
  language_codes: string[]
  /** Pays sur lequel porte l'activité — `proposals.country_id`. */
  country_id: CountryId | null

  // — Étape 4 : intervenants
  speakers: DraftSpeaker[]

  // — Étape 5 : créneau souhaité
  /** Heure MURALE `AAAA-MM-JJTHH:MM` dans le fuseau de l'édition. */
  preferred_start_at: string | null
  duration_minutes: number | null
  requested_sessions: number
  scheduling_constraints: string

  /**
   * — Étape 6 : documents.
   *
   * ILS NE PARTENT PAS AVEC LE BROUILLON : l'enregistrement automatique ne les
   * transmet pas, et la recomposition d'un dossier ne les rend pas. Les pièces
   * ont leurs propres routes (`GET`/`POST /proposals/{id}/documents`) et un
   * objet déjà stocké ; ce champ ne tient que la saisie de l'écran.
   */
  documents: DraftDocument[]
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/**
 * Un défaut du dossier, rattaché à l'étape qui le corrige.
 *
 * DEUX GRAVITÉS, et elles ne se confondent pas : `error` empêche l'envoi parce
 * que la base ou l'appel le refuseraient ; `warning` signale ce qui affaiblit un
 * dossier sans l'invalider — un résumé absent, aucun public visé. Peindre les
 * seconds en rouge apprendrait à ignorer les premiers.
 */
export interface DraftIssue {
  step: ProposalFormStep
  /** Nom du champ, tel que l'écran l'identifie (`title`, `speakers`…). */
  field: string
  severity: 'error' | 'warning'
  /** Clé i18n du message, résolue par l'écran. */
  messageKey: string
  /** Paramètres du message (limites, bornes de l'appel). */
  params?: Record<string, string | number>
}

// ---------------------------------------------------------------------------
// Les deux écritures
// ---------------------------------------------------------------------------

export interface SaveDraftPayload {
  /** Nul au tout premier enregistrement : c'est lui qui crée la ligne. */
  proposal_id: ProposalId | null
  call_id: CallId
  event_id: EventId
  draft: ProposalDraft
}

/**
 * Réponse d'un enregistrement automatique.
 *
 * Elle rend le NUMÉRO DE DOSSIER dès le premier appel — le trigger l'attribue à
 * l'insertion — et l'instant d'écriture tel que le serveur l'a daté. L'écran
 * n'affiche jamais sa propre horloge comme heure d'enregistrement : les deux
 * divergent, et c'est celle du serveur qui fait foi.
 */
export interface SaveDraftResult {
  proposal_id: ProposalId
  reference_code: string
  saved_at: IsoDateTime
  status: ProposalStatus
}

export interface SubmitProposalPayload {
  proposal_id: ProposalId
  call_id: CallId
  event_id: EventId
  draft: ProposalDraft
}

/**
 * Issue d'un dépôt.
 *
 * LES TROIS REFUS SONT CEUX DE LA BASE, pas des inventions d'écran :
 * `tg_check_submission_eligibility()` (`070` § 3) refuse hors fenêtre de l'appel,
 * au-delà du plafond `max_proposals_per_organization`, et quand l'appel exige une
 * organisation vérifiée (`requires_verified_organization`). L'écran les prévient,
 * mais doit savoir les rendre : entre le chargement de la page et le clic, une
 * échéance peut tomber.
 *
 * L'ÉCRAN REND AUSSI CE QU'IL NE CONNAÎT PAS. Un quatrième refus ajouté à l'API
 * arriverait ici en discriminant inconnu ; le taire laisserait un bouton sans
 * effet, ce qui est pire qu'un message imparfait.
 */
export type SubmitProposalResult =
  | {
      status: 'submitted'
      proposal_id: ProposalId
      reference_code: string
      submitted_at: IsoDateTime
      /** Revues indépendantes attendues — `calls_for_proposals.required_reviews`. */
      required_reviews: number
      /** Annonce des résultats — `calls_for_proposals.results_expected_at`. */
      results_expected_at: string | null
    }
  | { status: 'call_closed'; deadline: IsoDateTime }
  | { status: 'quota_reached'; max: number }
  | { status: 'organization_not_verified' }

// ---------------------------------------------------------------------------
// Rouvrir un dossier existant
// ---------------------------------------------------------------------------

/**
 * UN INTERVENANT TEL QUE L'API LE REND — sans clé de liste ni photo.
 *
 * `key` est locale à l'écran par définition : la faire porter par l'API
 * obligerait à la persister pour qu'elle reste stable, alors qu'elle ne survit
 * pas à la saisie. C'est donc l'écran qui la pose à la réception, sans quoi tous
 * les intervenants rouverts partagent la même clé vide — en modifier un les
 * remplace tous.
 *
 * `photo` est absente pour une autre raison : elle appartient à la fiche de la
 * personne, que la recomposition ne rouvre pas.
 */
export type ReopenedSpeaker = Omit<DraftSpeaker, 'key' | 'photo'>

/**
 * LE BROUILLON RECOMPOSÉ PAR L'API.
 *
 * Il ne porte PAS les pièces jointes : elles ont leurs propres routes et leur
 * propre objet stocké. L'écran les charge à part et complète le brouillon —
 * voir `draftFromReopened()`.
 */
export interface ReopenedDraft extends Omit<ProposalDraft, 'speakers' | 'documents'> {
  speakers: ReopenedSpeaker[]
}

/**
 * CE QUE L'ÉCRAN REÇOIT POUR ROUVRIR UN DOSSIER — `GET /proposals/{id}/draft`.
 *
 * La recomposition n'est pas un `SELECT` : le formulaire travaille sur une
 * structure d'écran — français, heures murales, identités verrouillées — quand
 * la base range la même chose dans cinq tables. Elle appartient à l'API, et une
 * seule implémentation sert le dépôt et la correction.
 */
export interface EditableProposal {
  proposal_id: ProposalId
  reference_code: string
  /** Nul pour un dossier qui n'est rattaché à aucun appel. */
  call_id: CallId | null
  event_id: EventId
  status: ProposalStatus
  saved_at: IsoDateTime
  draft: ReopenedDraft
}

/**
 * Ce que l'écran charge avant d'afficher quoi que ce soit : l'édition qui reçoit
 * les dossiers, son appel, et le brouillon en cours s'il y en a un.
 */
export interface ProposalFormContext {
  /** Nul quand aucune édition n'ouvre de dépôt — l'écran l'annonce et s'arrête. */
  call_id: CallId | null
  event_id: EventId | null
  /** Dossiers déjà comptés dans le plafond de l'organisation, ce brouillon exclu. */
  counted_proposals: number
}

/**
 * CE QUE RÉPOND LA RECHERCHE D'UNE PERSONNE PAR SON ADRESSE.
 *
 * L'écran interroge l'annuaire à la saisie de l'adresse, plutôt que de charger
 * l'annuaire entier : il comptera des milliers de personnes, et une plateforme
 * ne diffuse pas sa liste de contacts pour remplir un formulaire.
 *
 * `null` quand personne ne porte cette adresse — il faudra alors créer la
 * personne, ce que l'API fait à l'enregistrement du dossier.
 */
export interface PersonLookup {
  person_id: PersonId
  civility: string | null
  first_name: string
  last_name: string
  email: string
  /** Fonction et organisation DU PROFIL : elles amorcent les instantanés de
   *  l'activité, qui restent modifiables (voir `SPEAKER_IDENTITY_FIELDS`). */
  job_title: string | null
  organization_name: string | null
  organization_id: OrganizationId | null
  bio: string | null
  /** La personne a-t-elle un COMPTE ? Si oui, son identité lui appartient. */
  has_account: boolean
}
