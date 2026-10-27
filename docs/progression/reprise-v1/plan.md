# Reprise des données de la v1 (Supabase) : plan

> Rédigé le 27/10/2026. Périmètre arrêté par le commanditaire : **les données Supabase** (activités 2025, et ce qui s'y trouve de 2026 : COP31, PACO). Les années 2021 à 2024 (serveur Laravel) sont une autre reprise, hors périmètre. Les écarts tranchés vivent dans [`ecarts.md`](ecarts.md), un par ligne.

## 1. Ce qu'on sait, et ce qu'on ne sait pas encore

**La structure réelle de Supabase est inconnue.** Le SQL du dépôt v1 (`bank/shema_et_requettes/database_complete.sql`) n'a jamais été rejoué tel quel : il est syntaxiquement invalide, porte du code supprimé depuis, et des migrations ont été jouées directement dans Supabase. Exemples d'écarts probables : `activity_status` à 3 valeurs dans le fichier, 9 en production ; anciennes valeurs d'`activity_theme` (`climate_resilience`, `nature_oceans`…) encore portées par des activités 2025 ; tables utilisées par le code et absentes de tout SQL (`organization_requests`, `event_registrations`, `activity_panelists`). **Rien ne s'écrit avant d'avoir extrait le schéma réel.**

**Volumes indicatifs** (README v1, septembre 2025) : 2 556 utilisateurs, 450 organisations. À remesurer.

**Supabase contient aussi la COP31 (2026)** : l'appel a tourné sur la v1. La production v2 porte de son côté 1 édition, 13 propositions, 12 séances, 4 organisations, 8 personnes : données réelles ou de recette, à trancher (D2).

**`docs/database/910_migration_v1.sql` est à moitié écrit et ne tourne pas** : `migrate_activities` est un bouchon ; `propose_organization_clusters` échoue à la première paire floue (clé étrangère vers `org.organizations` posée avec des identifiants v1) ; `migrate_organizations` fusionne en silence par slug avec les fiches de production (dont `ifdd`) et force le type `ngo_association` ; `v_reconciliation` compare `id_map` à lui-même. On garde son principe (quatre temps, `id_map`), on réécrit l'outillage.

**Inventaires complets** : rendus des deux relevés du 27/10 (v1 côté Supabase, modèle cible), résumés aux §§ 4 et 5.

## 2. Principes

1. **Répéter avant de jouer.** Tout se joue d'abord sur une copie locale de la production, autant de fois qu'il faut. La production ne reçoit que la dernière répétition réussie, à l'identique.
2. **Rejouable de bout en bout.** Une commande repart de l'extraction et refait tout. Corriger une règle de correspondance, c'est relancer, jamais retoucher des lignes à la main.
3. **Chaque ligne v1 rend des comptes.** Dans `legacy.id_map` : reprise, fusionnée ou écartée, **avec la raison**. Un écart non expliqué bloque la bascule.
4. **Aucun effet de bord.** Aucun courriel, aucune notification, aucune réunion Zoom créée par la reprise (§ 5).
5. **On n'invente rien.** Une valeur v1 sans équivalent ouvre un écart à trancher, elle n'est pas devinée.
6. **Données personnelles** : extractions dans `sauvegardes/reprise-v1/` (ignoré par Git), jamais dans le dépôt, un journal ou un message.

## 3. Les étapes

| # | Étape | Qui | Résultat |
|---|---|---|---|
| 0 | **Accès** | commanditaire | chaîne de connexion Postgres de Supabase (connexion directe, port 5432) et clé `service_role`, dans `.env` seulement ; accès à l'ancien cPanel pour ses fichiers |
| 1 | **Extraction** | Claude | `pg_dump` du schéma réel (`public`, `auth`, `storage`) et des données ; tous les fichiers des buckets (`epavillonp`, `epavillonv`, `activity-media`, `video-thumbnails`, `documents`) avec leur empreinte ; les fichiers de l'ancien serveur que la base désigne (`images_uploades/`, `documents_uploades/`…), lus par son adresse IP |
| 2 | **Carte réelle** | Claude | comptage par table ; écarts entre le schéma réel et l'inventaire du dépôt ; valeurs réellement présentes de chaque ENUM ; URL de fichiers par forme (Supabase, ancien serveur, relative, autre) |
| 3 | **Écarts** | ensemble | une décision par écart dans `ecarts.md` (§ 6), une question à la fois |
| 4 | **Outillage** | Claude | `ops/reprise-v1/` : scripts numérotés, rejouables, un par domaine ; `910` corrigé pour ne garder que la zone de transit et `id_map` |
| 5 | **Résolution** | ensemble | doublons d'organisations proposés par groupes (domaine de courriel, nom normalisé, sigle), validés par vous ; personnes rapprochées par courriel, y compris avec celles déjà en production |
| 6 | **Transformation** | Claude | écriture dans les schémas v2, dans l'ordre du § 4 |
| 7 | **Réconciliation** | Claude, puis vous | chaque table v1 : lignes reprises + fusionnées + écartées = lignes extraites ; contrôles visuels (pages d'activités, photos, logos) en administrateur |
| 8 | **Bascule** | Claude, avec votre feu vert | extraction finale, sauvegarde de la production, jeu en production, vérification, réouverture (§ 7) |
| 9 | **Après** | ensemble | redirections des anciennes adresses, courriel aux utilisateurs, suppression de `legacy` hors `id_map`, révocation des clés de la v1 |

## 4. Ordre de reprise et correspondances pressenties

Chaque ligne sera confirmée sur la carte réelle (étape 2).

| Ordre | v1 | v2 | Points durs |
|---|---|---|---|
| 1 | `countries` | `reference.countries` | rapprochement par code ISO |
| 1 | valeurs d'ENUM | `reference.taxonomy_terms` | termes manquants : `ace`, `mecanismes_de_cooperation`, `bilan_mondial`, anciennes thématiques (D6) |
| 2 | `auth.users` + `users` | `identity.people` + `accounts` | **mots de passe bcrypt illisibles par la v2** (D1) ; reprendre `email_confirmed_at`, sinon la connexion est refusée ; comptes Google et Microsoft ; `created_at` faussé par l'import Laravel |
| 2 | `user_roles` | `identity.role_assignments` | `paco` → `standard` ; `negotiator` en portée globale ouvre tous les espaces |
| 2 | `negotiators` | `negotiator_profiles`, `negotiator_designations` | thèmes en texte libre |
| 3 | `organizations` (+ alias, validations) | `org.organizations`, `organization_names`, `organization_domains` | doublons (règle métier n° 1) ; nom unique par pays ; type obligatoire |
| 3 | `users.organization_id` | `org.memberships` | une adhésion active exige une fonction (`job_title`) que la v1 n'avait pas |
| 4 | `events` | `event.events` (séries déjà semées) | COP31 existe déjà en production (D2) ; PACO (D9) |
| 5 | `activities` | `programme.proposals` (+ `sessions` si retenue) | deux statuts mêlés ; un seul porteur ; dates COP31 décalées de 3 h par l'ancien fuseau (D8) ; insérer **directement à l'état final** |
| 5 | `activity_speakers` | `identity.people` + `proposal_speakers` / `session_speakers` | une ligne par activité, rapprochement par courriel |
| 5 | `activity_documents` | `media.assets` + `proposal_documents` | |
| 5 | `activity_ratings`, `revision_comments` | `programme.reviews`, `proposal_comments` | une revue par évaluateur ; recalcul des moyennes (`refresh_proposal_score`) |
| 6 | `activities.youtube_link` | `live.streams` (rediffusion) | identifiant ou URL de formes variées ; une vidéo ne sert qu'une séance |
| 7 | `activity_registrations` (+ invités, `session_edition`) | `identity.people` + `programme.registrations` | **déclenche Zoom** (§ 5) ; une seule inscription par séance et personne (invité puis compte = doublon) |
| 8 | fichiers | `media.assets` + `media.attachments` | texte alternatif obligatoire pour une image ; formats imposés (32:9, 16:9, 1:1) ; un même contenu une seule fois |
| 9 | `negotiation_documents` | `negotiation.documents` | `type` et `catégorie` inversés pour ceux venus de Laravel ; fichiers sur l'ancien serveur |
| 9 | `negotiation_sessions`, `francophonie_meetings` | `negotiation.meetings` | place dans les agendas (D10) |
| 10 | `user_testimonials`, `video_testimonials`, `innovations_practices` | `content.highlights` | |
| 10 | `media_gallery` | aucune table dédiée | D7 |

**Proposé à l'écart** (D3) : chatbot et vecteurs (à recalculer avec un autre modèle), notifications, messagerie, mises en relation, rendez-vous, sondages, lettres d'information, historique des courriels, statistiques de consultation, journaux, réunions Zoom (expirées, mots de passe en clair), données démographiques PACO (à purger d'après la v1 elle-même), formations (module fermé).

## 5. Garde-fous techniques

Relevés dans le modèle le 27/10, à câbler dans l'outillage :

- **Plateforme en maintenance et worker arrêté** pendant toute écriture.
- Écrire sous une personne technique « Reprise v1 » (`app.actor_id`) et une étiquette de lot (`app.request_id`) ; avant de relancer le worker, marquer publiés les événements d'outbox du lot et annuler les travaux `live.*` et de rappel, **en gardant** `media.process_asset`.
- **Une inscription reprise sur une séance diffusée ferait créer une réunion Zoom et écrire aux inscrits** : neutralisation vérifiée par un test avant tout jeu.
- Propositions et séances insérées **à leur état final, dates fournies** : un `UPDATE` d'état réécrit `decided_at` et émet des notifications.
- Séance insérée sans jauge ni formulaire ni clôture, posés après les inscriptions ; sinon le contrôle d'inscription refuse l'historique.
- Fichiers : déposés dans Garage puis déclarés `uploaded`, pour que le worker produise les déclinaisons ; un texte alternatif pour chaque image.
- Aucune fusion silencieuse : un rapprochement avec une ligne de production est une décision de l'étape 5, jamais un `ON CONFLICT`.

## 6. Écarts à trancher

Posés un par un au commanditaire, réponses consignées dans `ecarts.md`.

| | Question | Proposition |
|---|---|---|
| D1 | Mots de passe : chacun le réinitialise, ou l'ancien reste valable | garder l'ancien (vérification bcrypt une fois, puis conversion) : 2 500 comptes à quelques jours de la COP31 |
| D2 | COP31 : la v1 fait-elle foi, et les 13 propositions déjà en v2 sont-elles réelles | à demander |
| D3 | Ce qui n'est pas repris (liste du § 4) | confirmer la liste |
| D4 | Doublons d'organisations : qui valide | Claude propose par groupes, le commanditaire valide |
| D5 | Fonction exigée pour une adhésion active | « Non renseignée » ou adhésion en attente |
| D6 | Thématiques sans équivalent | terme créé ou rapprochement, une par une |
| D7 | Médiathèque d'activité | galerie de l'édition, ou écartée |
| D8 | Dates COP31 saisies avec l'ancien fuseau | correction de 3 h vérifiée sur les journées spéciales connues |
| D9 | PACO (sessions décrites dans le code, pas en base) | une édition de la série « Webinaires IFDD », une séance par numéro |
| D10 | Sessions de négociation v1 | agenda des sessions, ou réunions de la Francophonie |
| D11 | Anciennes adresses (`/activities/<uuid>`…) | redirection permanente par `id_map` |
| D12 | Annonce aux utilisateurs | un courriel après la bascule, selon D1 |
| D13 | Date limite | à demander |

## 7. La bascule, geste par geste

1. Maintenance allumée (déjà le cas), worker arrêté.
2. Extraction finale de Supabase et des fichiers.
3. `./deploy.sh backup` de la production.
4. Jeu de l'outillage, lot par lot, chacun dans sa transaction.
5. Réconciliation : zéro écart non expliqué, sinon restauration de la sauvegarde.
6. Neutralisation de l'outbox et des travaux du lot, puis relance du worker (traitement des fichiers).
7. Vérification en administrateur, puis réouverture.
8. Redirections et courriel aux utilisateurs.
