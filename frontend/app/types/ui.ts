/**
 * Vocabulaire des composants d'interface — `app/components/ui/`.
 *
 * Ces types ne décrivent AUCUNE table : ils nomment les variantes, les tailles
 * et les structures de données que les composants attendent en propriété. Ils
 * vivent à part des types du modèle (`types/reference.ts`, `types/programme/…`)
 * pour que la frontière reste nette — un composant d'interface ne connaît pas le
 * schéma, il reçoit ce qu'on lui donne.
 *
 * RÈGLE : aucun libellé ici. Une option de `Select`, une colonne de `Table`, une
 * étape de `Stepper` portent soit une clé i18n (`labelKey`) pour un libellé
 * d'interface, soit un libellé déjà résolu (`label`) quand il vient de la base.
 * Les deux ne se confondent pas — c'est le piège exact de la v1.
 */

import type { I18nText } from './shared'

// ---------------------------------------------------------------------------
// Vocabulaire commun
// ---------------------------------------------------------------------------

/**
 * Intention d'un composant coloré. La couleur DISTINGUE un état, elle ne décore
 * pas : `neutral` est le défaut, et les quatre autres ne s'emploient que quand
 * l'information qu'ils portent est réellement de cette nature.
 */
export type Intent = 'neutral' | 'info' | 'success' | 'warning' | 'danger'

/**
 * Variantes de bouton, dans l'ordre décroissant de poids visuel.
 *
 * `glass` est à part : elle ne s'emploie QUE sur un média voilé — un bandeau
 * photographique — et jamais sur une surface de page. C'est la même borne que
 * les jetons `--color-glass-*` ; elle existe ici pour qu'aucun écran n'écrive
 * son propre bouton translucide.
 */
export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'link' | 'glass'

/** Trois tailles suffisent : `sm` est la version compacte des barres d'outils. */
export type Size = 'sm' | 'md' | 'lg'

/** Sens d'un tri de colonne ; `null` = colonne non triée. */
export type SortDirection = 'asc' | 'desc' | null

// ---------------------------------------------------------------------------
// Champs de formulaire
// ---------------------------------------------------------------------------

/**
 * Propriétés communes à tout champ. `FormField` les consomme pour composer le
 * libellé, l'aide contextuelle et le message d'erreur autour du contrôle.
 *
 * `readonly` et `disabled` ne sont pas la même chose et ne se rendent pas de la
 * même façon : un champ désactivé est hors du parcours (sa valeur ne part pas),
 * un champ en lecture seule reste focalisable, copiable et soumis.
 */
export interface FieldProps {
  /** Identifiant HTML ; généré si absent. */
  id?: string
  /** Libellé d'interface, déjà traduit par l'appelant. */
  label?: string
  /** Aide contextuelle affichée sous le champ, avant toute erreur. */
  hint?: string
  /** Message d'erreur. Sa seule présence bascule le champ en état d'erreur. */
  error?: string
  required?: boolean
  disabled?: boolean
  readonly?: boolean
  /** Le champ occupe-t-il toute la largeur disponible ? */
  block?: boolean
  /**
   * Libellé lisible des seuls lecteurs d'écran — barre de recherche d'en-tête,
   * champ de cellule. Le libellé reste OBLIGATOIRE : le masquer est une
   * décision de mise en page, jamais une dispense.
   */
  hideLabel?: boolean
}

/** Option d'un `Select` ou d'un groupe de `Radio`. */
export interface SelectOption {
  value: string
  /** Libellé prêt à l'affichage — traduit par l'appelant, ou résolu depuis la base. */
  label: string
  /** Précision affichée en second rang (pays d'une organisation, code d'une salle). */
  description?: string
  disabled?: boolean
}

// ---------------------------------------------------------------------------
// Tableau
// ---------------------------------------------------------------------------

/**
 * Colonne de `Table`. La densité est assumée : un tableau de back-office affiche
 * douze lignes lisibles plutôt que six aérées.
 */
export interface TableColumn {
  /** Clé de la colonne ; sert aussi de nom de créneau (`cell-<key>`). */
  key: string
  /** En-tête, déjà traduit. */
  label: string
  /** Alignement du contenu. Les nombres s'alignent à droite, toujours. */
  align?: 'start' | 'end' | 'center'
  /** La colonne peut-elle être triée par l'utilisateur ? */
  sortable?: boolean
  /** Largeur CSS explicite (`12rem`, `10%`) ; sinon la colonne s'ajuste. */
  width?: string
  /**
   * Colonne masquée sous 640 px. La règle du projet interdit le défilement
   * horizontal du corps de page : sur mobile, on retire des colonnes plutôt que
   * de laisser filer le tableau.
   */
  hideOnMobile?: boolean
  /**
   * Même idée, à un autre seuil : la colonne n'apparaît qu'à partir de cette
   * largeur. `sm` équivaut à `hideOnMobile` ; `lg`, `xl` et `2xl` servent aux tableaux
   * DENSES, dont la liste des propositions du back-office est le cas type — onze
   * colonnes ne tiennent pas dans les 1 130 px que laisse la navigation
   * latérale, et ce qui sert à DÉCIDER (titre, note, rang) doit rester lisible
   * avant ce qui sert à situer (pays, format, thématiques).
   */
  hideBelow?: 'sm' | 'lg' | 'xl' | '2xl'
  /** Chiffres alignés en colonne — nombres, notes, montants. */
  numeric?: boolean
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

/** Onglet de `Tabs`. */
export interface TabItem {
  value: string
  label: string
  /** Compteur affiché à droite du libellé (« Propositions 40 »). */
  count?: number
  disabled?: boolean
}

/**
 * État d'une étape de `Stepper` ou de `StatusTimeline`.
 *
 * `error` et `refused` ne disent PAS la même chose et ne se remplacent pas :
 * une correction demandée attend une action du déposant et le dossier repartira ;
 * un refus clôt le parcours. Les confondre ferait annoncer « à corriger » à
 * quelqu'un dont le dossier a été écarté — c'est un lecteur d'écran qui
 * l'entendrait, et personne ne s'en apercevrait à la relecture.
 */
export type StepState = 'done' | 'current' | 'upcoming' | 'error' | 'skipped' | 'refused'

/** Étape de `Stepper` — parcours en plusieurs écrans (formulaire de soumission). */
export interface StepItem {
  value: string
  label: string
  /**
   * Sous-libellé d'avancement, sur une ligne (« 2 intervenants sur 5 »). Absent,
   * la barre affiche l'état de l'étape à sa place : une cellule ne reste jamais
   * muette sous son titre.
   */
  description?: string
  state?: StepState
  /** L'étape est-elle atteignable par clic ? Une étape à venir ne l'est pas. */
  disabled?: boolean
}

/** Entrée d'un `ContextMenu`. */
export interface MenuItem {
  value: string
  label: string
  /** Nom d'icône de `UiIcon`. */
  icon?: string
  /** Action destructrice : rendue en rouge — libellé ET icône —, séparée du reste. */
  destructive?: boolean
  disabled?: boolean
  /** Trait de séparation AVANT cette entrée. */
  separatorBefore?: boolean
  /**
   * Surtitre de groupe affiché AVANT cette entrée — « Activité », « Diffusion ».
   * Déjà traduit par l'appelant, comme `label`. Il coiffe les entrées qui
   * suivent jusqu'au surtitre suivant : c'est ce qui permet de ranger douze
   * actions sans ouvrir un second niveau de menu.
   */
  groupLabel?: string
  /**
   * Raccourci clavier, tel qu'il doit s'AFFICHER (« Ctrl D »). Le menu ne le
   * capte pas : un raccourci actif alors que le menu est fermé se branche sur
   * l'écran, pas sur le composant qui l'annonce.
   */
  shortcut?: string
}

// ---------------------------------------------------------------------------
// Composants métier
// ---------------------------------------------------------------------------

/**
 * Pastille thématique d'une session — un terme de `reference.taxonomy_terms`.
 *
 * La couleur vient de la BASE (`taxonomy_terms.color_hex`) : c'est une donnée,
 * pas un jeton de design. Elle sert de repère de teinte, jamais de fond de
 * texte — voir `UiBadge`, qui la rend en point coloré pour garantir le contraste
 * dans les deux thèmes.
 */
export interface ThemeBadge {
  code: string
  label: I18nText | string
  color?: string | null
}

/**
 * Étape de `StatusTimeline` — frise d'avancement d'un dossier.
 * Les dates sont des instants ISO ; la frise les affiche dans le fuseau qu'on
 * lui donne, comme toute date de la plateforme.
 */
export interface TimelineStep {
  /** Statut du modèle (`programme.proposal_status`) ou clé libre. */
  value: string
  label: string
  /** Instant de franchissement ; `null` tant que l'étape n'est pas atteinte. */
  at?: string | null
  /**
   * L'étape est datée au JOUR, pas à l'instant. C'est le cas des colonnes `date`
   * du modèle — `calls_for_proposals.results_expected_at` : personne n'a saisi
   * d'heure, et en afficher une (« à 09:00 », produite par la conversion de
   * fuseau) inventerait une précision que la donnée n'a pas.
   */
  dateOnly?: boolean
  /** Auteur de l'étape, quand il est connu et utile (« par Awa Diop »). */
  actor?: string | null
  /** Précision : motif de refus, nature de la demande de correction. */
  detail?: string | null
  state?: StepState
}
