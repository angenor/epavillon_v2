# Feature Specification: Direct + Tableaux de bord (B9)

**Feature Branch**: `007-direct-tableaux-de-bord`

**Created**: 2026-08-27

**Status**: Draft

**Input**: Prompt B9 — modules DIRECT (`live`) et TABLEAUX DE BORD (`analytics`) de l'API ePavillon v2. Fermeture de la dernière dette de données simulées : les huit routes que `make check-api-contract` compte encore « en attente d'API », toutes portées par les deux écrans A6 (tableau de bord du back-office) et A13 (messages d'incident).

---

## Ce que la session a constaté avant d'écrire

**Rien de ce qui suit n'est à créer une seconde fois.**

| Ce qui existe déjà | Où | Conséquence |
|---|---|---|
| Le schéma `live` en entier, incidents compris | `docs/database/080_live.sql` § 6 | La table, les deux fonctions de publication et les trois fonctions de lecture sont là |
| Le schéma `analytics` en entier | `docs/database/130_analytics.sql` | Huit projections, `refresh_all`, `enqueue_refresh`, `refresh_log`, `v_platform_overview`, `v_operational_health` |
| Les deux modules déclarés en base | `010_platform.sql` § 7 | `('live','live',…,'{programme}')` et `('analytics','analytics',…,'{}')` — **rien à semer** |
| La permission `live.incident.publish` | `030_identity.sql` | Portée par les rôles `admin` et `programmer` — **rien à ajouter** |
| La taxonomie `incident_kind`, neuf termes dont `overrun` | `080_live.sql` § 7 | **Rien à semer** |
| Les quatre contrats du front | `types/admin-dashboard.ts`, `types/analytics.ts`, `types/live.ts`, `types/admin-incidents.ts` | Ils **sont** le contrat de cette API ; ils ne se renégocient pas |
| Les quatre jeux d'exemple | `mocks/admin-dashboard.ts`, `mocks/analytics.ts`, `mocks/admin-incidents.ts`, `mocks/incidents.ts` | Spécification exécutable, composition de la zone d'alertes comprise |
| Les deux écrans, livrés et vérifiés au navigateur | A6 le 17-18/08, A13 le 18/08 | **Aucun écran n'est à réécrire** — seuls les appels basculent |
| Le worker écoute déjà les files `live` et `analytics` | `backend/crates/worker/src/registry.rs` + `jobs.rs` | Il ne manque qu'un gestionnaire pour la tâche `analytics.refresh_all` et sa chaîne récurrente |
| `GET /api/health` sert déjà `v_operational_health` | Livré en B1 | La zone 3 du tableau de bord a déjà sa route de rafraîchissement isolé — **elle n'est pas à refaire** |
| Le patron de module, de `repo/cross/`, de travail récurrent | `backend/crates/modules/programme/`, `worker/src/main.rs` | À suivre, pas à réinventer |

**Ce qui n'existe pas** : les deux crates `backend/crates/modules/live` et `backend/crates/modules/analytics`, et les huit routes.

**État mesuré du contrat au 27/08** : `139 appels sur 165 chemins, 130 formes annoncées — toutes définies. 8 route(s) en attente d'API.` Les huit sont exactement celles de ce jalon. La vitrine (B8) est déjà basculée : les en-têtes qui annoncent « trois écrans en données simulées » sont donc **déjà faux d'un tiers**, et le seront de trois tiers à la fin de ce jalon.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Le tableau de bord du back-office dit la vérité de la plateforme (Priority: P1)

Une chargée de programmation de l'IFDD ouvre le back-office le matin. Elle veut savoir, en un écran et pour l'édition dont elle a la charge : ce qui demande une action aujourd'hui, où en sont les dépôts et les inscriptions, et si un mécanisme silencieux est en panne. Aujourd'hui l'écran existe et affiche des exemples, avec un bandeau qui l'avoue. À la fin de cette histoire, il affiche la plateforme réelle et le bandeau disparaît.

**Why this priority** : c'est l'écran d'entrée du back-office, consulté plusieurs fois par jour, et le seul qui réunit les cinq familles d'alerte. Tant qu'il ment, aucune décision de pilotage ne peut s'appuyer dessus.

**Independent Test** : configurer `NUXT_PUBLIC_API_BASE`, se connecter en administratrice globale, ouvrir `/admin`, changer d'édition dans le sélecteur, et retrouver à l'écran les chiffres que la base porte réellement — sans bandeau de données d'exemple.

**Acceptance Scenarios**

1. **Given** une administratrice globale et une édition qui porte un appel ouvert, **When** elle ouvre le tableau de bord de cette édition, **Then** l'écran rend en **une seule réponse** l'édition, son fuseau, son appel, les lignes d'action, les chiffres, la santé opérationnelle et les incidents actifs.
2. **Given** cinq familles d'alerte réunies sur une même édition, **When** la réponse est composée, **Then** elle porte **une ligne par famille** — jamais une par élément — avec son décompte, trois exemples nommés au plus et le lien vers l'écran déjà filtré.
3. **Given** une famille dont aucun élément ne remonte, **When** la réponse est composée, **Then** **aucune ligne n'est émise pour elle** (une ligne à zéro n'existe pas), et une édition où tout va bien rend une liste d'actions vide.
4. **Given** un administrateur détaché sur une seule édition, **When** il demande le tableau de bord d'une autre édition en forgeant l'adresse, **Then** l'API **refuse l'accès** et ne rend jamais un tableau de bord vide.
5. **Given** une personne sans aucun droit d'administration, **When** elle demande un tableau de bord, **Then** l'API refuse — refus distinct d'une liste vide.
6. **Given** des projections matérialisées rafraîchies à 06:00, **When** l'écran est ouvert à 11:00, **Then** la réponse porte l'instant du **dernier `analytics.refresh_all` réussi**, et l'écran l'affiche.
7. **Given** une édition sans appel ni dépôt, **When** le tableau de bord est demandé, **Then** l'entonnoir est **nul** et non un entonnoir de zéros, et les autres blocs rendent leur état vide.
8. **Given** deux organisations présumées identiques non arbitrées, **When** un administrateur détaché sur une seule COP ouvre son tableau de bord, **Then** la famille « doublons » remonte — elle n'appartient à aucune édition et ne révèle l'existence d'aucune autre.
9. **Given** un message d'incident de portée globale publié et actif, **When** n'importe quelle édition est demandée, **Then** il figure dans les incidents actifs de cette édition.
10. **Given** l'écran branché sur l'API, **When** il se charge, **Then** `usesMockData` reste éteint et le bandeau « données d'exemple » n'apparaît pas.

---

### User Story 2 — L'équipe voit ce qui se joue, et ce qui est déjà dit (Priority: P1)

Pendant une COP, l'équipe de régie ouvre l'écran des messages d'incident. Elle n'y cherche pas un formulaire : elle veut voir les activités du jour, leur état d'antenne, et ce qui a déjà été annoncé sur chacune. L'écran existe et fonctionne sur des exemples. À la fin de cette histoire, il montre la vraie journée.

**Why this priority** : c'est la moitié « lecture » de l'écran, et elle conditionne les écritures — on ne rédige un message qu'après avoir vu ce qui se passe et ce qui est déjà en ligne.

**Independent Test** : ouvrir `/admin/incidents` sur une édition réelle et retrouver les activités du jour (ou le repli annoncé), les messages dans leur ordre d'action, les compteurs d'état, les natures d'incident et les cibles offertes.

**Acceptance Scenarios**

1. **Given** une édition, **When** l'écran des messages est demandé, **Then** l'API rend en **une seule réponse** l'édition, son fuseau, sa ville, les lignes de messages, le poste de direct, les compteurs par état, les natures d'incident et les cibles.
2. **Given** des messages de portée `session`, `event_day` et `organization` rattachés indirectement à l'édition, **When** la liste est composée, **Then** **tous remontent** — le rattachement est calculé par la fonction du modèle, jamais par une comparaison d'identifiant d'édition.
3. **Given** cinq messages dans cinq états différents, **When** la liste est rendue, **Then** chacun porte l'**état calculé par le modèle** (`active`, `scheduled`, `draft`, `expired`, `unpublished`), sa **cible résolue par son nom** et l'ordre d'action — actifs, programmés, brouillons, historique.
4. **Given** une journée d'édition **sans titre**, **When** elle est la cible d'un message, **Then** sa **date** en tient lieu de nom.
5. **Given** une édition qui tient trois activités aujourd'hui, **When** le poste de direct est composé, **Then** il rend ces trois activités dans le **fuseau de l'édition**, avec leur état temporel, leur diffusion et le **nombre de messages déjà actifs sur chacune**, et `is_fallback` est faux.
6. **Given** une édition qui ne tient rien aujourd'hui, **When** le poste est composé, **Then** il rend les **prochaines** activités et **le dit** (`is_fallback` vrai) — le jour rendu reste celui d'aujourd'hui dans le fuseau de l'édition.
7. **Given** un administrateur détaché sur une autre édition, **When** il demande cette liste, **Then** l'API refuse l'accès.
8. **Given** un membre du comité **sans** `live.incident.publish`, **When** il ouvre la liste, **Then** il la voit — **la lecture n'est pas un privilège**, un bandeau est déjà public.
9. **Given** un identifiant de message, **When** il est demandé pour relecture, **Then** l'API rend la ligne complète ; un identifiant hors périmètre est refusé **de la même façon** qu'un identifiant inexistant.
10. **Given** une activité du planificateur, **When** le raccourci « Signaler un débordement » demande son gabarit, **Then** l'API rend l'activité, son titre, son créneau et son édition — de quoi pré-remplir portée, cible, nature et fin d'affichage sans une saisie.
11. **Given** les cibles offertes au formulaire, **When** elles sont composées, **Then** elles ne portent **que** les journées et activités de l'édition et **que** les organisations qui y animent une activité.

---

### User Story 3 — Un message se rédige, se publie, se corrige et se retire (Priority: P1)

La régie constate qu'une activité déborde sur le créneau suivant. En deux gestes, elle publie un bandeau qui le dit au public, dans les deux langues, avec une fin d'affichage. Plus tard, elle le retire en donnant un motif — et la trace reste.

**Why this priority** : sans les écritures, l'écran est une vitrine. C'est aussi la moitié qui porte l'autorisation et les invariants de la base.

**Independent Test** : depuis le poste de direct, publier un message sur une activité, le voir passer « En ligne » et remonter en tête de liste, le corriger, le retirer avec motif, et le retrouver à l'historique avec son auteur et son motif.

**Acceptance Scenarios**

1. **Given** une régie titulaire de `live.incident.publish` sur l'édition, **When** elle enregistre un message sans demander la publication, **Then** il est créé à l'état `draft` et **ne s'affiche nulle part**.
2. **Given** la même personne, **When** elle enregistre en demandant la publication, **Then** le message est créé **et publié dans le même geste**, horodaté et attribué par la fonction du modèle.
3. **Given** un brouillon, **When** il est publié, **Then** il passe « en ligne », remonte en tête de liste et un événement de domaine est écrit dans l'outbox — **dans la même transaction**.
4. **Given** un message publié, **When** il est dépublié avec un motif, **Then** la ligne **reste** avec son instant de retrait, son auteur et son motif : **ce n'est pas une suppression**.
5. **Given** un message dépublié, **When** il est publié à nouveau, **Then** le retrait est effacé — instant, auteur et motif — exactement comme le fait la fonction du modèle.
6. **Given** un message jamais publié, **When** on tente de le dépublier, **Then** l'API refuse en le disant, sans inventer un état.
7. **Given** un message dont le texte n'est renseigné que dans une langue, **When** il est enregistré, **Then** l'API **refuse et désigne le champ** — les deux langues sont une règle d'API, **la base n'est pas durcie**.
8. **Given** une portée changée sans que la cible précédente soit vidée, **When** l'écriture atteint la base, **Then** le refus de la contrainte est **traduit en message français exploitable** et non réimplémenté.
9. **Given** une fenêtre d'affichage dont la fin précède le début, **When** l'écriture est tentée, **Then** elle est refusée en désignant le champ.
10. **Given** un client qui déclare lui-même ses droits, **When** il écrit, **Then** cela ne change rien : **l'API lit sa propre session** — le paramètre `granted` disparaît du site.
11. **Given** un administrateur détaché sur la COP31, **When** il écrit un message visant une autre édition, **Then** l'API refuse — la permission se vérifie **sur la portée visée par l'écriture**, pas sur celle de l'appelant.
12. **Given** un administrateur détaché sur une seule édition, **When** il tente de retirer un message de portée **globale**, **Then** l'API refuse : un bandeau qui couvre toute la plateforme se retire avec la permission globale (voir Décision D3).
13. **Given** toute écriture, **When** elle est exécutée, **Then** l'acteur et l'identifiant de requête sont posés en début de transaction, et l'audit en porte la trace.

---

### User Story 4 — Les chiffres ne vieillissent pas en silence (Priority: P2)

L'équipe se fie aux courbes du tableau de bord. Rien ne les recalcule aujourd'hui hors d'un appel manuel : sans travail périodique, l'écran affiche des chiffres du jour de l'installation et l'annonce d'un âge qui grandit.

**Why this priority** : sans elle, l'histoire 1 est juste le premier jour puis fausse tous les autres. Elle est P2 parce que l'écran **dit** son âge : il vieillit visiblement, il ne ment pas.

**Independent Test** : lancer le worker, attendre une période, et constater dans le journal de rafraîchissement une exécution complète réussie — puis voir l'instant affiché par l'écran avancer.

**Acceptance Scenarios**

1. **Given** un worker qui démarre, **When** aucune chaîne de rafraîchissement n'est posée, **Then** il en arme une, et **dix redémarrages n'en arment pas dix**.
2. **Given** la période écoulée, **When** le travail s'exécute, **Then** les huit projections sont rafraîchies **de façon concurrente**, chacune journalisée avec sa durée et son nombre de lignes.
3. **Given** une projection qui échoue, **When** le travail s'exécute, **Then** **les autres sont rafraîchies quand même**, l'échec est journalisé, et l'indicateur de santé de fraîcheur le reflète.
4. **Given** cent événements de domaine dans la même minute, **When** un rafraîchissement est demandé pour chacun, **Then** l'anti-rebond du modèle n'en produit **qu'un seul** travail.
5. **Given** une exécution réussie, **When** le tableau de bord est demandé ensuite, **Then** l'instant qu'il affiche est celui de cette exécution.
6. **Given** aucun rafraîchissement jamais réussi, **When** le tableau de bord est demandé, **Then** l'instant est **nul** et l'écran le dit, plutôt que d'inventer une fraîcheur.

---

### User Story 5 — Un bandeau publié se voit du public (Priority: P3)

Une régie publie « la diffusion de l'atelier est interrompue ». Un visiteur ouvre le programme de l'édition : il lit le message, qui **nomme l'activité concernée**. Aujourd'hui, le composant de bandeau n'est monté que dans l'aperçu du formulaire et le guide de style — **le message ne s'affiche nulle part**.

> **Amendé le 27/08, en écrivant le plan** ([research.md](research.md) R26). Cette histoire visait « la page publique de l'activité ». **Cette page n'existe pas** : `frontend/app/pages/` n'en porte aucune, et `useApi.ts` le dit lui-même à propos du détail public d'une séance — « aucun écran ne l'appelle encore : la page publique d'une séance n'est pas au jalon ». L'exposition se fait donc sur la **page des programmations**, à l'échelle de l'**édition**, par la fonction descendante. Le cas d'usage du commanditaire — une activité en direct dont la diffusion tombe — est servi : le message s'affiche et porte le nom de son activité, `target_label` étant déjà résolu par le modèle.

**Why this priority** : publier un message que personne ne voit est un demi-livrable, et c'est le seul endroit du jalon où la valeur atteint le public. P3 parce que les pièces existent toutes (fonction en base, contrat, composant vérifié) : c'est un raccordement, pas une construction. Elle se retire sans casser les autres.

**Independent Test** : publier un message de portée `session` depuis le back-office, ouvrir la page des programmations sur cette édition dans une session de navigateur neuve, et voir le bandeau nommer l'activité — puis le retirer et voir le bandeau disparaître.

**Acceptance Scenarios**

1. **Given** un message actif de portée `session`, **When** la page des programmations est ouverte sur son édition, **Then** le bandeau s'affiche et **nomme l'activité concernée**.
2. **Given** des messages actifs de portée `event_day`, `event`, `organization` qui anime, ou `global`, **When** la même page est ouverte, **Then** ils s'affichent aussi — le balayage de portée **descend** depuis l'édition et n'est **pas** recomposé côté site.
3. **Given** plus de trois messages actifs, **When** la page est ouverte, **Then** les trois plus graves s'affichent et le reste se replie en « +N ».
4. **Given** un message dont la fenêtre est close ou qui a été dépublié, **When** la page est ouverte, **Then** rien ne s'affiche.
5. **Given** un message marqué refermable, **When** le visiteur le referme, **Then** il ne réapparaît pas pendant sa visite ; un message non refermable reste.
6. **Given** un visiteur non connecté, **When** il ouvre la page, **Then** la lecture aboutit : un bandeau d'incident est **public par nature**.

---

### User Story 6 — Le dépôt cesse d'annoncer une dette qui n'existe plus (Priority: P3)

Trois fichiers du dépôt affirment que « trois écrans lisent des données simulées ». Après ce jalon, aucun ne le fait. Une affirmation fausse dans un fichier lu à chaque session coûte plus qu'elle ne coûte à corriger.

**Why this priority** : c'est la porte de sortie du jalon, mécaniquement vérifiable.

**Independent Test** : `make check-api-contract` compte **zéro route en attente**, et aucun des trois fichiers d'en-tête n'annonce d'écran en données simulées.

**Acceptance Scenarios**

1. **Given** les huit routes servies, **When** la vérification du contrat tourne, **Then** elle rend **0 route en attente** et **0 route en attente qui existe pourtant au contrat**.
2. **Given** la bascule faite, **When** on lit `CLAUDE.md` § Périmètre actuel, l'en-tête de la primitive d'attente et celui du composable de signalement, **Then** aucun n'annonce d'écran en données simulées.
3. **Given** `NUXT_PUBLIC_API_BASE` vide, **When** on ouvre les deux écrans, **Then** ils fonctionnent toujours sur les jeux d'exemple — **les mocks restent, pour les tests et le travail hors ligne**.

---

### Edge Cases

- **Une édition sans aucune activité, ni aujourd'hui ni demain** : le poste de direct rend une liste vide, `is_fallback` vrai, et l'écran dit qu'il n'y a rien plutôt que d'afficher un bloc muet.
- **Une édition dont le fuseau diffère de celui du serveur** : le jour du poste de direct est celui de l'édition. À Belém il est 06:00 quand il est 11:00 à Paris ; une équipe qui pilote depuis Québec ne doit pas voir la journée de la veille.
- **Un message sans fin d'affichage** : c'est légitime (une panne ouverte n'a pas de fin connue) et c'est le seul vrai danger de la table — la v1 en a laissé des mois en ligne. Rien n'est interdit ; l'interface le signale déjà deux fois.
- **Un message de portée `organization` dont l'organisation n'anime plus rien dans l'édition** : il cesse de remonter pour cette édition. C'est le critère du modèle, il n'est pas contourné.
- **Une organisation fusionnée** qui était la cible d'un message : le registre de références du modèle réaffecte déjà la cible. Rien à écrire.
- **Un appel prolongé** : l'échéance qui fait foi est la prolongation, jamais la clôture initiale.
- **Une échéance déjà passée** : le décompte de jours devient négatif et n'est pas une alerte — un fait acquis n'a pas à rester en rouge six mois.
- **Aucun dossier tranché** : le taux d'acceptation est **nul**, jamais zéro. « 0 % » ferait passer un comité qui n'a pas commencé pour un comité qui a tout refusé.
- **Une série trop courte** : la variation sur sept jours est nulle plutôt que calculée sur une semaine tronquée.
- **Une projection matérialisée jamais peuplée** : les chiffres qui en dépendent sont nuls, l'instant de fraîcheur aussi, et la santé opérationnelle porte l'alerte.
- **Deux régies qui publient le même message en même temps** : rien ne l'empêche, et rien ne doit l'empêcher ; le poste de direct affiche le nombre de messages déjà actifs sur l'activité précisément pour que cela se voie avant.
- **Un message visant une activité annulée** : rien ne l'interdit — une annulation est justement ce qu'on annonce.

---

## Requirements *(mandatory)*

### Ce que l'API sert

- **FR-001** : L'API DOIT servir `GET /admin/dashboard` et rendre, pour l'édition demandée, **une seule réponse** portant l'édition, son fuseau, son appel (zéro ou un), les lignes d'action, les chiffres, la santé opérationnelle et les incidents actifs.
- **FR-002** : L'API DOIT servir `GET /admin/incidents` (paramétrée par l'édition) et rendre **une seule réponse** portant l'édition, son fuseau, sa ville, les lignes de messages, le poste de direct, les compteurs par état, les natures d'incident et les cibles offertes.
- **FR-003** : L'API DOIT servir `GET /admin/incidents/{id}` et rendre la ligne de gestion d'un message.
- **FR-004** : L'API DOIT servir `GET /admin/incidents/overrun-template` (paramétrée par l'activité) et rendre l'activité, son titre, son créneau et son édition.
- **FR-005** : L'API DOIT servir `POST /admin/incidents` — rédiger, et publier dans le même geste si c'est demandé.
- **FR-006** : L'API DOIT servir `PUT /admin/incidents/{id}` — corriger.
- **FR-007** : L'API DOIT servir `POST /admin/incidents/{id}/publish` — publier un brouillon, ou rétablir un message retiré.
- **FR-008** : L'API DOIT servir `DELETE /admin/incidents/{id}/publish` — dépublier avec motif. **Ce n'est pas une suppression** : la ligne demeure.
- **FR-009** : Les corps de ces huit routes DOIVENT être exactement les formes déjà déclarées par le site. Aucun champ n'est ajouté, retiré ni renommé.
- **FR-010** : Chaque route DOIT être annotée pour que le contrat engendré la porte, avec ses paramètres et ses codes d'erreur stables.

### Le périmètre d'administration

- **FR-011** : Toute route de ce jalon DOIT borner sa réponse au périmètre d'administration de l'appelant, lu par la fonction du modèle prévue pour cela, **URL forgée comprise**.
- **FR-012** : Les trois cas du périmètre DOIVENT rester distincts : accès complet, éditions listées, **aucun droit → refus explicite**, jamais une liste vide.
- **FR-013** : Le rattachement d'un message à une édition DOIT passer par la fonction du modèle qui le calcule. Un filtre écrit à la main sur l'identifiant d'édition est **interdit** : il laisserait fuir les portées `session`, `event_day` et `organization`, qui n'ont aucune colonne d'édition.
- **FR-014** : Un identifiant hors périmètre DOIT se refuser **de la même façon** qu'un identifiant inexistant — la forme de la réponse ne révèle pas l'existence d'une donnée hors périmètre.

### L'autorisation

- **FR-015** : La **lecture** des messages d'incident NE DOIT exiger **aucune permission** au-delà d'administrer l'édition. Aucune permission de lecture n'est à ajouter au modèle : un bandeau est affiché au public, le cacher à l'équipe ne protégerait rien.
- **FR-016** : Les **quatre écritures** DOIVENT exiger `live.incident.publish`, vérifiée **sur la portée visée par l'écriture** : l'édition pour une portée d'édition, de journée, d'activité ou d'organisation ; la **portée globale** pour un message global (Décision D3).
- **FR-017** : L'API NE DOIT PAS lire les droits déclarés par le client. Le paramètre par lequel le site les transmettait disparaît.
- **FR-018** *(amendé le 27/08 — [research.md](research.md) R10)* : Le tableau de bord DOIT exiger `analytics.dashboard.read` **sur l'édition demandée**, **et** être refusé à qui n'administre aucune édition. Les deux conditions se cumulent : la permission ouvre l'écran, le périmètre borne ce qu'on y voit. Le modèle porte cette permission pour cet écran et `GET /health` la teste depuis B1 ; elle n'est pas attribuée au rôle `programmer`, à qui une **ligne de semis** l'accorde — sans quoi le tableau de bord serait refusé au rôle qui pilote une édition, et au compte qui a servi à le vérifier au navigateur le 17/08.

### Les invariants et les refus

- **FR-019** : Publier et dépublier DOIVENT passer par les fonctions du modèle, qui horodatent, attribuent et gardent le motif. **Jamais une écriture directe des colonnes de publication** : l'historique est le sujet, pas un effet de bord.
- **FR-020** : Publier à nouveau un message retiré DOIT effacer l'instant, l'auteur et le motif du retrait — le comportement de la fonction du modèle, non recomposé.
- **FR-021** : Dépublier un message jamais publié DOIT être refusé, en traduisant l'exception de la fonction.
- **FR-022** : Le texte du message DOIT être exigé **dans les deux langues** par l'API. La base n'est **pas** durcie : elle accepte un français seul, pour les données reprises de la v1.
- **FR-023** : Le refus de la contrainte de cohérence portée/cible DOIT être **traduit** en message français exploitable désignant le champ, jamais réimplémenté en Rust.
- **FR-024** : Le refus de la contrainte de fenêtre d'affichage DOIT l'être de même.
- **FR-025** : Chaque refus DOIT porter un **code stable** et un message français, et désigner le champ fautif quand il y en a un. Les issues d'écriture déjà nommées par le site font foi.
- **FR-026** : Toute transaction en écriture DOIT poser l'acteur et l'identifiant de requête avant la première écriture.
- **FR-027** : Les événements de domaine émis par les fonctions de publication NE DOIVENT PAS être réémis par le service : la base les écrit déjà.

### La composition du tableau de bord

- **FR-028** : La zone d'action DOIT rendre **une ligne par famille**, jamais une par élément : famille, gravité d'affichage, décompte, échéance s'il y en a une, **trois exemples nommés au plus**, et le lien vers l'écran **déjà filtré** sur le problème.
- **FR-029** : Les cinq familles DOIVENT être exactement : dossiers déposés sans évaluation, revues en retard, doublons d'organisation, chevauchements de créneaux, messages d'incident actifs.
- **FR-030** : Une famille sans élément NE DOIT PAS produire de ligne.
- **FR-031** : Un dossier entre dans la famille « sans évaluation » soit parce que son **échéance applicable** approche — celle de son affectation de revue la plus proche, à défaut la clôture qui fait foi —, soit parce qu'**aucun révisionniste ne lui est affecté**, déports exclus.
- **FR-032** : Le seuil de proximité d'échéance DOIT venir de la base et non du code (Décision D2).
- **FR-033** : Les chevauchements DOIVENT être **signalés, jamais bloqués** ; ils viennent de la fonction de détection du modèle, non recomposée.
- **FR-034** : Les doublons d'organisation et les incidents de portée globale NE DOIVENT PAS être filtrés par édition, et ne DOIVENT révéler l'existence d'aucune autre édition.
- **FR-035** : Les lignes DOIVENT être rangées par gravité, puis par échéance la plus proche, puis par décompte.
- **FR-036** : Les six indicateurs de tête DOIVENT chacun se tracer à une colonne du modèle ou à une fonction du modèle. **Aucun n'est calculé à l'écran.**
- **FR-037** : Un indicateur dont la donnée n'existe pas DOIT être **nul**, jamais zéro.
- **FR-038** : Les deux courbes DOIVENT être rendues **continues**, jours vides compris, telles que les projections les portent. Aucun trou n'est rebouché.
- **FR-039** : Les répartitions DOIVENT porter la clé stable, le libellé **multilingue venu de la base**, la couleur venue de la taxonomie, le décompte et la part.
- **FR-040** : L'échéance à marquer sur la courbe DOIT être celle qui fait foi (la prolongation prime), obtenue par la fonction du modèle.
- **FR-041** : La réponse DOIT porter l'instant du **dernier rafraîchissement complet réussi**, nul s'il n'y en a jamais eu.
- **FR-042** : De la vue d'ensemble de la plateforme, la réponse NE DOIT servir que ce qui n'appartient à aucune édition — personnes, organisations, doublons. Tout le reste vient des projections par événement (écart n° 44).
- **FR-043** : Les indicateurs de santé DOIVENT être rendus par leur **code stable**, le libellé français de la vue restant un repli (écart n° 45). Les seuils ne sont **pas** recalculés.
- **FR-044** : Les incidents actifs de l'édition DOIVENT venir de la fonction descendante du modèle, jamais d'un balayage de portée recomposé.

### Le poste de direct

- **FR-045** : Le poste DOIT porter les activités du **jour de l'édition**, dans le fuseau de l'édition.
- **FR-046** : Quand l'édition n'a rien aujourd'hui, le poste DOIT rendre les **prochaines** activités et **le dire**.
- **FR-047** : Chaque activité DOIT porter son créneau, sa salle, son état d'antenne, son état temporel — **calculé comme le fait la programmation publique**, dans le même ordre de décision — et le **nombre de messages déjà actifs** sur elle.
- **FR-048** : Les cibles offertes au formulaire NE DOIVENT porter que les journées et activités de l'édition, et que les **organisations qui y animent** une activité.
- **FR-049** : Une journée sans titre DOIT être désignée par sa **date**.
- **FR-050** : Une cible d'activité DOIT porter son début comme **instant**, à part de toute précision textuelle : le formatage appartient à l'interface.

### Le rafraîchissement des projections

- **FR-051** : Le worker DOIT porter un gestionnaire pour la tâche de rafraîchissement complet, sur la file du module analytique.
- **FR-052** : Le worker DOIT **armer une chaîne récurrente** au démarrage, qui se replanifie elle-même, sans qu'un redémarrage n'en produise une seconde.
- **FR-053** : La période DOIT être configurable, avec une valeur par défaut, comme les autres chaînes récurrentes du dépôt.
- **FR-054** : La mise en file DOIT passer par la fonction d'anti-rebond du modèle : une rafale d'événements ne produit **qu'un** travail.
- **FR-055** : L'échec d'une projection NE DOIT PAS empêcher le rafraîchissement des autres.
- **FR-056** : Chaque exécution DOIT être journalisée par le mécanisme du modèle — durée, lignes, succès ou erreur. Aucun journal parallèle.

### L'exposition publique

- **FR-057** *(amendé le 27/08 — [research.md](research.md) R26)* : L'API DOIT servir une lecture publique des messages actifs d'une **édition**, en utilisant la fonction **descendante** du modèle — l'édition, ses journées, ses activités, les organisations qui y animent, plus les messages globaux. Chaque ligne DOIT porter la **cible résolue**, pour qu'un message d'activité reste lisible sur une page qui parle de trente activités.
- **FR-058** : Cette lecture DOIT être ouverte sans authentification.
- **FR-059** *(amendé le 27/08 — [research.md](research.md) R26)* : Le site DOIT afficher le bandeau sur la **page des programmations**, pour l'édition ouverte, le plus grave en tête et **trois au plus**, le reste replié en « +N » — la règle des pastilles de la charte, appliquée à un cas qu'elle décrit. Aucun filtre de portée n'est recomposé côté site.
- **FR-060** : Un message refermable NE DOIT PAS réapparaître pendant la visite après avoir été refermé ; un message non refermable reste.

### La bascule du site

- **FR-061** : Les huit appels DOIVENT passer de la primitive d'attente aux primitives réelles, **en rétablissant** les paramètres, les corps des quatre écritures et leurs verbes — correction en `PUT`, dépublication en `DELETE`.
- **FR-062** : Le paramètre par lequel le site déclarait les droits de l'acteur DOIT disparaître des quatre écritures.
- **FR-063** : `make check-api-contract` DOIT compter **zéro route en attente**.
- **FR-064** : Les trois fichiers qui annoncent « trois écrans en données simulées » DOIVENT être corrigés.
- **FR-065** : Les jeux d'exemple DOIVENT rester en place et fonctionner quand l'API n'est pas configurée.
- **FR-066** : Aucun écran de ces deux prompts NE DOIT être réécrit : seuls leurs appels changent.

### Les frontières

- **FR-067** : Deux crates DOIVENT être créés, jamais un : l'un pour le schéma `live`, l'autre pour le schéma `analytics`.
- **FR-068** : Aucun des deux NE DOIT dépendre de l'autre ni d'un autre crate de module.
- **FR-069** : Leurs routes DOIVENT être montées par le binaire HTTP derrière le registre des modules, et leurs travaux différés déclarés au worker.
- **FR-070** : Les lectures hors schéma DOIVENT vivre sous `repo/cross/`, comme dans le module de programmation, et rester des **lectures** : aucun des deux crates n'écrit hors de son schéma, et aucun n'appelle un autre module.
- **FR-071** : Aucun fichier de code applicatif NE DOIT dépasser mille lignes.

---

### Key Entities

- **Message d'incident** — le texte affiché en bandeau : sa portée et son unique cible, sa nature (vocabulaire ouvert), sa gravité, son texte multilingue, sa fenêtre d'affichage, sa publication et son retrait tracés. Il ne se supprime pas.
- **État d'un message** — calculé, jamais stocké : publié et dans sa fenêtre, programmé, rédigé, expiré, retiré. Quatre conditions cumulées, que la v1 oubliait une par une.
- **Poste de direct** — la journée de l'édition telle qu'elle se joue : ses activités, leur état d'antenne, ce qui est déjà annoncé sur chacune, et le repli assumé quand il n'y a rien.
- **Ligne d'action** — une famille de choses à régler aujourd'hui, son décompte, trois exemples et l'écran qui la règle. Un jugement composé, pas une donnée.
- **Indicateur de tête** — un chiffre tracé à une colonne du modèle, sa comparaison, sa série courte et son état de couleur.
- **Projection matérialisée** — un chiffre calculé à l'avance, qui porte son âge. Un chiffre matérialisé présenté comme instantané est un chiffre faux.
- **Indicateur de santé** — un mécanisme silencieux, sa valeur et ses deux seuils, portés côte à côte par le modèle.
- **Périmètre d'administration** — global, la liste des éditions confiées, ou aucun droit. Les trois cas ne se confondent pas.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : `make check-api-contract` compte **0 route en attente d'API** — contre 8 aujourd'hui.
- **SC-002** : Les deux écrans, ouverts avec l'API configurée, n'affichent **aucun bandeau de données d'exemple**.
- **SC-003** : Le tableau de bord d'une édition s'obtient en **un seul aller-retour**, et tous ses chiffres portent le même instant de mesure.
- **SC-004** : Un compte détaché sur une seule édition ne peut atteindre **aucune** donnée d'une autre édition par ces routes, **y compris en forgeant l'adresse** — vérifié route par route.
- **SC-005** : Les cinq portées d'un message d'incident remontent toutes dans la liste d'une édition, la portée `organization` comprise, éprouvé sur une base réelle.
- **SC-006** : Publier puis retirer un message laisse une ligne **complète** — auteur, instants, motif — et **zéro ligne supprimée**.
- **SC-007** : Un message enregistré dans une seule langue est refusé par l'API, tandis que la base **accepte toujours** un message d'une seule langue (les données reprises de la v1 restent lisibles).
- **SC-008** : Après une période, l'instant de fraîcheur affiché par l'écran a avancé, sans intervention manuelle.
- **SC-009** : Une projection mise en échec ne bloque pas les sept autres — vérifié.
- **SC-010** : Un message publié depuis le back-office est **visible par un visiteur non connecté** sur la page de l'activité concernée, en moins d'un rechargement.
- **SC-011** : `make check` passe en entier — base rechargée de zéro, rapport de frontières vide, mise en forme, analyse statique sans un avertissement, tests au vert.
- **SC-012** : Le graphe de dépendances des crates ne porte **aucune arête** entre les deux nouveaux modules, ni entre eux et un autre module.
- **SC-013** : Aucun fichier de `backend/` ni de `frontend/` ne dépasse **1000 lignes**.
- **SC-014** : Chacune des huit routes est couverte par au moins un test de chemin nominal et un test de refus par périmètre, sur base réelle et jetable.

---

## Assumptions

- **Le modèle n'est pas modifié**, à une exception nommée et justifiée : une **ligne de réglage** ajoutée au semis pour le seuil de l'écart n° 43 (Décision D2). C'est une donnée de configuration dans une table existante, pas un changement de schéma.
- **Les contrats du site font foi.** Un désaccord entre ce qu'un contrat déclare et ce que le modèle porte se résout **en faveur du contrat** pour la forme, et **en faveur du modèle** pour la donnée — et se consigne comme écart.
- **Les deux écrans ne sont pas retouchés** au-delà de leurs appels. Ils ont été livrés et vérifiés au navigateur les 17 et 18/08 ; les rouvrir serait rouvrir leurs vérifications.
- **La route de santé opérationnelle existe déjà** (livrée en B1) et n'est pas refaite. Le tableau de bord porte la même vue dans sa composition ; les deux servent la même vue non matérialisée, donc le même instant.
- **La vue d'ensemble de la plateforme reste non matérialisée** : ses compteurs doivent être exacts à la seconde. Seule la part qui n'appartient à aucune édition en est servie.
- **La période de rafraîchissement par défaut** est prise du même ordre que les autres chaînes récurrentes du dépôt, et l'alerte de fraîcheur du modèle (attention à deux heures, critique à vingt-quatre) borne le choix : la période doit rester bien en deçà de deux heures.
- **Les jeux d'exemple ne sont pas supprimés.** Ils servent les tests et le travail hors ligne, et c'est déjà la règle du dépôt.
- **Le module `live` ne livre dans ce jalon que sa part « messages d'incident ».** Réunions de visioconférence, diffusions et journal de webhooks restent hors périmètre : ni écran, ni contrat, ni prompt ne les demande.

---

## Décisions tranchées dans cette spécification

### D1 — La composition du tableau de bord vit dans le crate `analytics`, sous `repo/cross/`

**Pourquoi cela ne viole pas la frontière.** La règle de B2 dit : un module lit hors de son schéma **quand la question porte sur ses propres entités**, il n'y écrit jamais, et il n'appelle jamais un autre module.

L'entité propre du module analytique **est la mesure de la plateforme entière** — ce n'est pas un contournement, c'est la définition que le modèle lui donne. Ses huit projections lisent déjà, en base, les schémas des propositions, des organisations, des événements, de l'identité, de l'engagement et du direct. Sa vue de santé lit sept schémas dans une seule requête. Sa vue d'ensemble en lit six. Le SQL a **déjà** placé cette lecture large dans `analytics` ; le crate ne fait que porter le code au même endroit que le modèle porte ses vues.

Trois autres emplacements ont été écartés :

- **Dans le binaire HTTP.** Ce n'est pas un module : il ne s'extrait pas, il n'a pas de schéma, et la logique métier y devient inextricable. Le principe de frontière y perd son sens.
- **Dans le module de programmation.** Trois des cinq familles d'alerte ne lui appartiennent pas — les doublons d'organisation, les incidents, la santé opérationnelle. Il porterait alors un écran qui n'est pas le sien.
- **Découpée en cinq lectures, une par module propriétaire.** C'est exactement ce que le contrat du site interdit : neuf allers-retours et **neuf instants de mesure différents dans un même écran**, où l'entonnoir et la liste des dossiers finissent par ne plus dire la même chose.

**Ce que la décision engage.** Les lectures hors schéma du crate analytique restent **strictement des lectures**, isolées sous `repo/cross/`, un fichier par schéma lu. Elles interrogent des **tables, vues et fonctions SQL** — jamais le code Rust d'un autre module, ce que la constitution interdit. Le graphe de dépendances des crates reste sans arête. Et le jour où le module partirait en service autonome, la liste des liens à couper se produit mécaniquement par la fonction de découplage du modèle, exactement comme pour ses vues matérialisées.

**Ce qui reste chez `live`.** Le crate `live` porte les huit routes des messages d'incident et la lecture publique. Le crate `analytics` **lit** la fonction descendante du schéma `live` pour composer les incidents actifs du tableau de bord — un appel de fonction SQL, pas un appel de module. Sans cela, la composition redeviendrait deux réponses.

### D2 — Le seuil « urgent » vit dans les réglages de la plateforme

L'écart n° 43 est ouvert depuis le 17/08 : vingt et un jours sont écrits dans le code du site. Le principe I l'interdit — un champ qui existe côté application sans exister en base est une dette immédiate.

**Décision : une clé de réglage globale**, semée à vingt et un jours, lue par la composition du tableau de bord.

**Pourquoi pas une colonne de l'appel à propositions.** Ce seuil n'est pas une propriété de l'appel : c'est le réglage d'affichage d'un écran de pilotage. Le porter par l'appel obligerait à le renseigner sur chaque appel de chaque édition — douze valeurs à tenir d'accord pour un réglage que personne ne veut faire varier — et le laisserait vide sur les appels déjà créés. Les réglages de la plateforme portent déjà un seuil de même nature, celui qui gouverne l'arbitrage des doublons d'organisation.

**Ce que cela coûte** : une ligne au fichier de semis, aucun changement de schéma. **Ce que cela rend** : l'IFDD change le seuil sans redéploiement, et le site cesse de porter un chiffre métier.

**Ce qui reste ouvert, sans bloquer** : si le commanditaire veut un jour un seuil différent par édition, la clé globale devient le défaut et une colonne s'ajoute. Le contraire — retirer une colonne devenue inutile sur douze appels — coûterait plus cher.

### D3 — Un message de portée globale ne se retire pas depuis une édition

Un administrateur détaché sur une seule COP voit les messages globaux dans sa liste : c'est voulu, une équipe qui pilote un pavillon doit savoir qu'un bandeau de maintenance le couvre. Aujourd'hui, rien ne l'empêche de le **retirer**, et ce retrait s'applique à toutes les COP.

**Décision : la permission d'écriture se vérifie sur la portée réellement visée.** Pour un message d'édition, de journée, d'activité ou d'organisation, c'est l'édition. Pour un message **global**, c'est la **portée globale** — qu'un compte détaché n'a pas. Il voit donc le message et son état ; il ne peut ni le corriger, ni le publier, ni le retirer, et le refus le dit.

**Pourquoi ce sens plutôt que l'autre.** C'est la lecture stricte du principe V, déjà tenue partout ailleurs dans le dépôt : attribuer un rôle global demande la permission globale. C'est aussi le sens **réversible** : ouvrir plus tard ne casse rien, alors que fermer plus tard casserait une habitude prise.

**Question au commanditaire, non bloquante** — inscrite aux points bloqués : *quand un message d'entretien s'affiche sur tout le site, l'équipe d'une seule COP doit-elle pouvoir l'enlever elle-même, ou faut-il passer par l'équipe centrale ?* Si la réponse est « elle-même », la vérification revient à l'édition depuis laquelle le geste est fait, sans autre changement — le geste reste tracé et réversible dans les deux cas.

### D4 — L'exposition publique entre dans ce jalon

**Décision : oui, et en dernière priorité.**

Publier un message que personne ne voit est un demi-livrable, et toutes les pièces existent déjà : la fonction montante en base, le contrat du site, et le composant de bandeau — écrit, dessiné et vérifié. Il manque **une lecture publique et un montage**. Ce n'est pas une construction, c'est un raccordement.

> **Amendée le 27/08 par le plan** ([research.md](research.md) R26). La page publique d'une activité **n'existe pas** — constat fait en écrivant le plan, et confirmé par `useApi.ts` lui-même. La question que cette décision reportait — « quel message s'affiche sur une page qui parle de trente activités » — devient donc la question à trancher, et elle se tranche **par une règle qui existe déjà** : trois au plus, le plus grave en tête, le reste replié en « +N », comme les pastilles thématiques de la charte. Ce qui rend la réponse acceptable est que le modèle résout **déjà** la cible : un bandeau qui dit « Atelier de négociation — diffusion interrompue » informe, là où un bandeau anonyme serait du bruit.

**Ce que le jalon prend** : la lecture publique des messages actifs d'une **édition**, et le bandeau sur la **page des programmations**, trois au plus.

**Ce que le jalon ne prend pas**, et qui appartient au suivant : le bandeau sur la page d'accueil, et la **page publique d'une activité** — qui viendra avec son écran, et avec la fonction montante du modèle, aujourd'hui appelée par personne.

**Elle est en dernière priorité pour rester retirable** : si le jalon déborde, l'histoire se reporte sans casser les cinq autres.

---

## Hors périmètre

- Les **réunions de visioconférence**, les **diffusions en direct** et le **journal des webhooks fournisseur** du schéma `live`. Aucun écran ne les demande.
- La **mesure d'audience** du schéma analytique — la table des vues de page, l'empreinte de visiteur, la popularité des contenus. Aucun écran ne les affiche.
- Les **projections servies à part**, hors de la composition du tableau de bord : fiche de performance des organisations, participation par activité, charge du comité. Elles alimentent la composition ; aucune route ne les sert seule.
- Le **rappel automatique** sur un bandeau sans fin d'affichage (« ce bandeau est en ligne depuis 3 jours »), pressenti au point n° 6 de l'écran A13. L'interface le signale déjà deux fois ; le rappel demande une règle de notification qui n'est pas écrite.
- L'**écriture** dans les schémas lus par la composition. Les deux crates ne font qu'y lire.
- Le bandeau d'incident sur la **programmation complète** et sur l'**accueil** (voir D4).
