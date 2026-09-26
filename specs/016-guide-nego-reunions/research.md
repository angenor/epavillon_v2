# Research — Guide Négo, Réunions de la Francophonie (étape 4)

## R1 — Réutiliser `negotiation.meetings` et `meeting_registrations`

**Décision** : une réunion de la Francophonie est une ligne de `negotiation.meetings` de nature
`preparatory_workshop` ou `francophone_consultation`, **saisie** (`source_key` nul) ; ses inscriptions
vivent dans `negotiation.meeting_registrations`, dont le compteur est déjà tenu par la base
(`tg_sync_registered_count`). ADR-008 et `02-domaine.md` le prévoient ; aucune table n'est doublée.

**Écarté** : une table de réunions propre à la Francophonie — deux tables pour une réunion, la faute D2
de la v1.

## R2 — La nature fine est un vocabulaire

**Décision** : `reference.taxonomy_terms`, taxonomie `francophone_meeting_type` (termes
`preparatory_workshop`, `negotiators_consultation`, `ministerial_consultation`, libellés FR/EN), colonne
`meetings.francophone_type_term_id` gardée par `tg_check_term_taxonomy`. Le `kind` fermé reste ce qu'il
est (il engage un parcours applicatif) ; la concertation ministérielle n'est pas un parcours, c'est une
nature.

**Écarté** : `ALTER TYPE meeting_kind ADD VALUE` (non rejouable en transaction, et une valeur de plus
dans un ENUM métier) ; les termes de `negotiation_meeting_type` (vocabulaire des sessions officielles,
lu par l'import de 3a).

## R3 — Accès limité : une information (tranché le 26/09)

`is_open_access = false` et `access_audience` (`platform.i18n_text`, « ministres et chefs de
délégation ») ; le commentaire de `is_open_access` s'élargit (« selon la source » pour une session
importée, « selon l'IFDD » pour une réunion saisie). Aucune règle de lecture ni d'inscription.

## R4 — L'inscription : en base, sous verrou, sans piège

**Décision** (inspirée de `075_programme_sessions.sql`, corrigée de ce que ce patron ne couvre pas) :

- `meetings.requires_registration boolean NOT NULL DEFAULT true`, `meetings.waitlist_enabled boolean
  NOT NULL DEFAULT true` ;
- `meeting_registrations.waitlist_position integer`, `client_ref uuid`, `UNIQUE (person_id, client_ref)`,
  `ck_meeting_registrations_waitlist` ;
- `negotiation.tg_validate_meeting_registration()` BEFORE INSERT OR UPDATE OF status. **Les contrôles
  ne portent que sur une entrée** : `INSERT`, ou `cancelled → registered|waitlisted` (réinscription).
  Là : réunion publiée, non annulée, non commencée (`now() < start_at`), inscription requise, fenêtre
  ouverte — **une borne nulle veut dire sans limite** ; `SELECT … FOR UPDATE` sur la réunion, puis places
  prises = `count(*)` des `registered` (**jamais `registered_count`**, que le trigger AFTER ROW ne tient à
  jour qu'en fin d'instruction) ; plein **ou liste d'attente non vide** → `waitlisted` en fin de liste si
  `waitlist_enabled`, sinon refus. **Se désinscrire** (`→ cancelled`) n'est jamais contrôlé avant
  `start_at` ; **`waitlisted → registered`** (promotion) ne passe par aucun contrôle ;
- `negotiation.promote_meeting_waitlist(p_meeting_id)` : sous le même verrou, places libres =
  `capacity − count(registered)` (toutes si la capacité est nulle), promeut ce nombre de personnes dans
  l'ordre, recompacte les positions, **rend les personnes promues**. Appelée à la désinscription et
  quand l'administration relève ou retire la capacité ;
- `UNIQUE (meeting_id, person_id)` reste total : se réinscrire est un `UPDATE` (`cancelled_at = NULL`,
  position recalculée).

Les refus de la base se traduisent en codes stables (principe VIII). L'écriture d'une inscription met à
jour `meetings.registered_count` : l'édition au back-office ne se garde donc pas sur `updated_at`.

## R5 — Hors connexion, une référence par geste

L'intention `inscription-reunion:<meeting_id>` porte l'état voulu et **un `client_ref` neuf par geste**
(s'inscrire, se réinscrire). Le serveur : même `client_ref` que la ligne → l'état courant, rien d'écrit ;
ligne `cancelled` et `client_ref` différent → réinscription. Au retour, un refus (complet sans attente,
clos, annulé) abandonne l'intention et **se dit** : « Complet — votre inscription n'a pas pu être
prise. » ; une mise en attente se dit « Liste d'attente — position N ».

## R6 — Le lien de visioconférence n'est servi qu'aux inscrites (tranché le 26/09)

La liste publique **ne porte jamais** le lien. `GET /negotiation/me/meeting-registrations` le rend pour
chaque inscription `registered`, et pour une réunion sans inscription à toute personne admise ; réponse
`Cache-Control: private, no-store`, empreinte propre à la personne. Le lien est `external_url` ; **créer
ou choisir une salle `live` n'est pas de cette étape** (aucun sélecteur au back-office), la branche
`live.meetings_public` est donc écartée. Sur le téléphone : le lien quitte la garde **dès l'intention de
désinscription**, à la déconnexion (liste de `useGnSession`) et au retrait d'accès.

## R7 — Le lien au Pavillon : une clé, rien d'autre

Un trigger garde la **même édition** (`programme.sessions.event_id = meetings.event_id`) — principe
VIII. L'état de l'activité (annulée) est servi par l'étape 5, pas ici.

`meetings.pavilion_session_id uuid CONSTRAINT xmod_fk_negotiation_meetings_pavilion_session REFERENCES
programme.sessions(id) ON DELETE SET NULL` (tranché le 26/09) ; l'en-tête de `100_negotiations.sql`
ajoute `programme` à ses dépendances (075 est chargé avant 100). L'API publique ne sert que
l'identifiant ; l'étiquette ouvre la section Pavillon de l'onglet (étape 5 : l'activité elle-même). Le
back-office choisit l'activité parmi celles de l'édition, lues en SQL (patron de
`live/src/repo/cross/programme.rs`) : une lecture, jamais une écriture dans `programme`.

## R8 — Prévenir : la mécanique de 3b, avec une composition propre

`negotiation` calcule les destinataires (`meeting_audience()`) et compose l'avis, émet un événement
portant la charge `notification`, pose les courriels selon l'accord. Trois cas : annulation (motif),
changement d'heure ou de lieu (ancienne et nouvelle valeur, **comparées par le service au moment de
l'écriture**, jamais lues dans `meeting_changes`, qui appartient à l'import), place obtenue. Types semés :
`negotiation.francophone_meeting.changed`, `negotiation.meeting_registration.promoted` ; constantes dans
`contracts/src/negotiation.rs`. Courriels : une **cible nouvelle `FrancophoneMeeting`** du travail
`change_email` (lien `/guide-nego/francophonie/reunions/<id>`, titre `i18n`, heure, lieu, motif — la
composition de 3b lit `meeting_changes` et `title_original`, qui n'existent pas ici), et un **travail
distinct** pour la place obtenue (clé `promotion:<meeting>:<personne>`), pour qu'elle ne se fonde pas
dans un changement. `tg_meeting_status_event` émet déjà `negotiation.meeting.published/.cancelled` pour
une réunion saisie : aucun type n'est semé pour eux, `engagement` les ignore ; ils ne portent pas la
charge `notification`.

## R9 — La garde des permissions

Back-office : résoudre réunion → espace, puis `require_permission(…, MEETING_MANAGE,
Scope::NegotiationSpace(space_id))` — un administrateur global passe (la portée globale couvre),
un `space_lead` de l'espace aussi ; **ni `Requires<…>` (global seul, exclut le `space_lead`), ni
`RequiresAnyScope` (laisserait entrer un administrateur d'une seule édition, piège décrit dans
`routes/admin_codes.rs`)**. Adresse forgée ou réunion absente : même `404`. Liste : `WHERE
identity.has_permission($moi, 'negotiation.meeting.manage', 'negotiation_space', m.space_id)`. C'est la
règle 8 pour les espaces, sans fonction nouvelle. Inscription : `negotiation.space.access` sur
`Scope::NegotiationSpace(space_id)` (le global couvre).

## R9 bis — Ce que pose le serveur à la saisie

`space_id` = l'espace `climat` résolu côté serveur (précédent de l'import) ; `slug` engendré
(`<édition>-<nature>-<suffixe>`) ; `event_id` requis (`ck_meetings_francophone_event` : non nul pour ces
deux natures saisies) ; `timezone` recopié de `event.events` ; `format` dans le corps ; `kind` dérivé du
terme (`preparatory_workshop` → `preparatory_workshop`, les deux concertations →
`francophone_consultation`, `ck_meetings_francophone_kind`) ; `is_open_access` non nul pour une réunion
saisie (défaut vrai), `access_audience` requis quand il est faux (`ck_meetings_access_audience`) ;
organisateur : « IFDD » si `is_ifdd_organized`, sinon le nom de `organizer_org_id` lu en SQL dans `org`.
Les contraintes qui ne mordent qu'à la publication (`ck_meetings_online_access`,
`ck_meetings_onsite_venue`, nature requise) se traduisent aussi à `publish`. Les chevauchements entre
réunions ne sont jamais bloqués ; la seule exception de la base (`ex_meetings_live_room_overlap`) ne vise
que les salles `live`, que cette étape n'emploie pas.

## R10 — L'onglet partagé et « Ma journée »

La page `pages/guide-nego/francophonie/index.vue` porte le sélecteur (`GnSegmente`, deux segments, état
dans l'adresse `?section=reunions|pavillon`) et deux composants : `francophonie/GnSectionReunions.vue` et
`francophonie/GnSectionPavillon.vue` (état vide, pour l'étape 5). Le bloc « trois agendas » de « Ma
journée » porte `journee/GnJourneeLigneSessions.vue`, `GnJourneeLigneReunions.vue` et
`GnJourneeLignePavillon.vue` (vide, pour l'étape 5).

## R11 — La recherche globale

Elle vient de l'étape 2 (branche 013). Quand `main` la porte : fusionner `main`, ajouter
`reunionsTrouvees` à `utils/guide-nego/recherche-globale.ts` sur le patron de `sessionsTrouvees`, le groupe
à la page, et réécrire la clé `pas-encore` pour ne nommer que le Pavillon. Sinon, la tâche passe à la
fusion.
