/**
 * Schéma `content` — les CONTENUS MIS EN AVANT de la vitrine publique.
 * Dérivé de `docs/database/115_content.sql`.
 *
 * Une seule table, et c'est la décision structurante du module (C1) : la v1
 * avait deux tables de témoignages divergentes, une table de bonnes pratiques et
 * cinq composants Vue dans lesquels les annonces étaient écrites en dur. La
 * NATURE du contenu — témoignage, innovation, bonne pratique, annonce, chiffre
 * clé — n'est donc pas un ENUM : c'est un vocabulaire ouvert de
 * `reference.taxonomy_terms` (taxonomie `highlight_nature`), dont le libellé et
 * la couleur viennent de la base. Ne jamais les recopier dans un fichier i18n.
 *
 * Ce que ce fichier NE porte PAS, et où le trouver :
 *   · la vitrine prête à afficher — `content.v_showcase` → `ShowcaseRow`,
 *     dans `types/views.ts` : ses colonnes ont d'autres noms que la table
 *     (`nature_label`, `organization_name`) et portent des valeurs résolues ;
 *   · les contrats des écrans — `types/home.ts` (accueil public) et
 *     `types/admin-showcase.ts` (back-office de la vitrine).
 *
 * Les trois médias d'une diapositive ne sont PAS des colonnes (ADR-08) : ils
 * passent par `media.attachments`, sous les rôles `banner`, `video` et `cover`
 * déclarés pour `('content','highlights')` en § 5 du fichier SQL.
 */

import type {
  ColorHex,
  CountryId,
  EventId,
  I18nText,
  Int8,
  IsoDateTime,
  OrganizationId,
  PersonId,
  SessionId,
  TaxonomyTermCode,
  Url,
  Uuid,
} from './shared'

// ---------------------------------------------------------------------------
// Alias d'identifiants
//
// Déclarés ici et non dans `shared.ts` : le module `content` est délibérément
// petit et extractible, et rien d'autre que lui n'a besoin de ces noms.
// Documentaires, comme tous les alias du projet — TypeScript les tient pour
// interchangeables avec `Uuid`.
// ---------------------------------------------------------------------------

/** `content.highlights.id`. */
export type HighlightId = Uuid

/**
 * Code d'un terme de la taxonomie `highlight_nature`.
 * Semés par `115_content.sql` § 6 : `testimonial`, `negotiator_voice`,
 * `innovation`, `best_practice`, `announcement`, `key_figure`. La liste est
 * OUVERTE — un administrateur en ajoute depuis le back-office, ce qui interdit
 * d'en faire une union de littéraux.
 */
export type HighlightNatureCode = TaxonomyTermCode

// ---------------------------------------------------------------------------
// Types du module
// ---------------------------------------------------------------------------

/**
 * ENUM `content.highlight_placement` — `115_content.sql` § 1.
 *
 * Où la diapositive s'affiche. UN SEUL emplacement : `home_hero`, le bandeau
 * d'ouverture de l'accueil, qui défile.
 *
 * `home_aside` — le panneau latéral « À venir » — a été retiré du modèle le
 * 24/08 : cette colonne ne se compose plus, elle affiche les événements à venir
 * puis la frise des activités retenues. Ajouter un emplacement demande un
 * `ALTER TYPE` **et** un composant : c'est voulu, un emplacement sans rendu
 * n'existe pas — et c'est cette règle qui a fait retirer celui-là.
 */
export type HighlightPlacement = 'home_hero'

/**
 * ENUM `content.highlight_status` — `115_content.sql` § 1.
 *
 * `archived` n'est PAS une suppression : une diapositive retirée de la vitrine
 * reste consultable au back-office et réutilisable l'année suivante — un
 * témoignage de la COP30 se remet en avant à la COP31.
 *
 * Attention : le statut ne dit pas à lui seul si le contenu est à l'écran. La
 * FENÊTRE DE DIFFUSION (`starts_at` / `ends_at`) s'y ajoute, et c'est
 * `content.v_showcase` qui applique les deux — voir `ShowcaseBroadcastState`
 * dans `types/admin-showcase.ts`, qui les recompose pour le back-office.
 */
export type HighlightStatus = 'draft' | 'published' | 'archived'

// ---------------------------------------------------------------------------
// La table
// ---------------------------------------------------------------------------

/**
 * Table `content.highlights` — `115_content.sql` § 2.
 *
 * Quatre contraintes de la base qu'un formulaire doit reprendre, parce qu'elles
 * produisent des erreurs PostgreSQL et non des messages exploitables :
 *   · `ck_highlights_window` — `ends_at > starts_at` quand les deux existent ;
 *   · `ck_highlights_organization_shape` — une organisation se DÉSIGNE
 *     (`organization_id`) ou se NOMME (`organization_label`), jamais les deux.
 *     C'est la règle métier n° 1 : retaper « IFDD » à côté d'une fiche
 *     existante recrée le doublon que la v2 corrige ;
 *   · `ck_highlights_published_dated` — `status = 'published'` impose
 *     `published_at`, que le trigger pose au premier passage ;
 *   · `ck_highlights_link_shape` — un libellé de lien sans lien ne mène nulle
 *     part.
 */
export interface Highlight {
  id: HighlightId
  placement: HighlightPlacement
  status: HighlightStatus
  /** Code d'un terme de `highlight_nature`. Sans clé étrangère, comme
   *  `org.organizations.organization_type_code` : le module reste extractible. */
  nature_code: HighlightNatureCode
  /** L'ordre CHOISI par l'IFDD, croissant. `smallint` en base. La v1 n'en avait
   *  aucun : le carrousel suivait `created_at DESC`. */
  sort_order: number

  /** Repère du back-office autant que titre de la diapositive. Toujours présent. */
  title: I18nText
  /** Le texte porté en grand sur le fond. Pour un témoignage, l'EXTRAIT choisi
   *  par l'éditeur : couper appartient à la rédaction, pas à la mise en page. */
  quote: I18nText | null
  /** Texte long, pour la page de détail. */
  body: I18nText | null

  /** `identity.people` quand la personne est inscrite ; la vue préfère alors son
   *  `display_name` à `author_name`, pour qu'une correction de patronyme se
   *  répercute sans réédition. */
  person_id: PersonId | null
  /** Nom libre : beaucoup de témoignages viennent de personnes sans compte. */
  author_name: string | null
  /** « Négociatrice, délégation du Bénin » — traduisible, donc `i18n_text`. */
  author_title: I18nText | null

  organization_id: OrganizationId | null
  /** EXCLUSIF de `organization_id` (`ck_highlights_organization_shape`). */
  organization_label: string | null

  country_id: CountryId | null

  /** Édition de rattachement. Deux rôles indissociables : elle SITUE la
   *  diapositive et elle PORTE le périmètre d'administration (ADR-14, règle
   *  métier n° 8). `null` = contenu de plateforme, portée globale uniquement. */
  event_id: EventId | null
  /** Séance mise en avant. Le trigger `tg_highlights_normalize` en dérive
   *  `event_id` s'il manque et REFUSE la contradiction s'il diffère. */
  session_id: SessionId | null

  link_url: Url | null
  /** Nul si `link_url` est nul. Traduisible : « Voir le témoignage complet »
   *  change d'une diapositive à l'autre, ce n'est pas une chaîne d'interface. */
  link_label: I18nText | null

  /** Fond de repli quand aucun média n'est rattaché ou que les variantes ne sont
   *  pas prêtes. Couleur DE DONNÉE, saisie au back-office : elle n'a rien à
   *  faire dans la feuille de style. `#RRGGBB`. */
  background_color_hex: ColorHex | null

  /** Fenêtre de diffusion. `null` des deux côtés = sans limite de temps. C'est
   *  elle qui éteint toute seule l'annonce d'une journée spéciale, là où la v1
   *  comparait des dates en JavaScript dans le composant. */
  starts_at: IsoDateTime | null
  ends_at: IsoDateTime | null

  /** Posé par trigger au PREMIER passage en `published` et jamais rejoué :
   *  republier après archivage ne réécrit pas l'histoire. */
  published_at: IsoDateTime | null

  created_by: PersonId | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Rôles de média du module
// ---------------------------------------------------------------------------

/**
 * Les trois rôles déclarés pour `('content','highlights')` dans
 * `media.attachable_roles` — `115_content.sql` § 5.
 *
 * `video` a été AJOUTÉ à l'énumération `media.attachment_role` pour ce module
 * (`050_media.sql` § 1). Ce type le nomme ici parce que `AttachmentRole` de
 * `types/media.ts` ne l'a pas encore repris — écart consigné dans le rapport
 * A15 ; le jour où il l'intègre, ce type se réduit à une restriction du sien.
 */
export type HighlightMediaRole = 'banner' | 'video' | 'cover'

/**
 * Une contrainte de téléversement, lue de `media.attachable_roles` et affichée
 * telle quelle par le formulaire : « image, 15 Mio au plus ». Le téléversement
 * réel arrive en phase B ; la contrainte, elle, s'annonce dès maintenant.
 */
export interface HighlightMediaRule {
  role: HighlightMediaRole
  /** `media.attachable_roles.label` — une DONNÉE multilingue, pas une clé i18n. */
  label: I18nText
  /** Préfixes MIME acceptés, `*` autorisé : `['image/*']`, `['video/*']`. */
  allowed_mime_prefixes: string[]
  /** 15 Mio pour `banner`, 200 Mio pour `video`, 5 Mio pour `cover`. */
  max_byte_size: Int8
}
