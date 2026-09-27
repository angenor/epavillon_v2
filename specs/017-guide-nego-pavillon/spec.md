# Feature Specification: Guide Négo — Pavillon de la Francophonie (étape 5)

**Feature Branch**: `017-guide-nego-pavillon` (partie de `016-guide-nego-reunions`)

**Created**: 2026-09-26

**Status**: Draft — constat et trois questions tranchés le 26/09 (voir *Clarifications*)

**Input**: Étape 5 de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md) : les activités du Pavillon de la Francophonie — le stand OIF/IFDD, déjà géré par l'ePavillon — dans Guide Négo. Section « Pavillon » de l'onglet Francophonie (maquette 10, écrans 1c et 1d). Aucune donnée nouvelle : l'application sert l'API publique existante du programme et des inscriptions ; un manque se corrige dans le module existant, sans copie. Bloc de lieu en tête ; activités du jour et à venir ; activités passées avec leur rediffusion et sa durée ; détail (organisateurs, intervenants) ; inscription : Inscrite, Liste d'attente, Complet, Rediffusion. Lisible hors connexion ; les activités du jour entrent dans « Ma journée » avec leur origine. Critère : l'API existante est servie telle quelle ; **le scénario qui clôt le MVP se joue de bout en bout**.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Les écrans | [10-francophonie.html](../../docs/AppNego/design/ecrans/10-francophonie.html) 1c (liste) et 1d (détail) ; la ligne Pavillon du bloc « trois agendas » de [02-socle.html](../../docs/AppNego/design/ecrans/02-socle.html) 07 |
| Le système | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) « 4 decies » — bloc de lieu, états d'inscription, « Rediffusion · N min », bouton |
| Les mots | [lexique.md](../../docs/AppNego/design/lexique.md) — « Pavillon de la Francophonie, et ses activités » ; jamais « Programme », « Sessions », « Side events » |
| Les décisions | ADR-008 (trois agendas, un lien jamais une fusion) ; ADR-003 (hors connexion) ; **décision du 22/09 : le Pavillon n'est jamais filtré par « mes thématiques »** (spec 010, FR-011) |
| Le partage de l'onglet | L'étape 4 a posé la page et le sélecteur ; cette étape réécrit `GnSectionPavillon.vue` et `GnJourneeLignePavillon.vue`, et donne sa destination à `GnEtiquettePavillon` |
| Ce qui existe | `GET /schedule?event_id=` (vue `programme.v_public_schedule`), `GET /events/{id}/sessions/{slug}`, `GET /sessions/{id}/registration-form`, `POST /sessions/{id}/registrations` (six issues en 200), `POST /registrations/{id}/cancel`, `GET /registrations/mine`, `GET /events/{id}/venues` ; la rediffusion vit dans `live.streams` sans être servie ; les noms des intervenants et des organisations sont en base sans être servis |

## Clarifications

### Session 2026-09-26 — tranché le 26/09 par l'orchestrateur, pour le commanditaire

- Constat : l'API publique du programme ne sert ni la rediffusion, ni les noms des intervenantes et des organisations, ni la langue, ni la liste d'attente, ni d'empreinte. → R : **on l'expose dans le module `programme`, à partir de ce que la base a déjà**, à trois conditions : **des ajouts seulement** (aucun champ retiré ni renommé : le site lit ces routes) ; **le site inchangé**, prouvé par ses tests (`test:site`, `check-api-contract`) ; **rien de ce que le public ne voit pas aujourd'hui ne devient public**, sauf ce qui l'est déjà ailleurs — chaque donnée tirée du dossier de proposition se vérifie une à une : la langue oui, les coordonnées jamais.
- Q : Une activité dont le formulaire demande plus qu'un geste ? → R : **le formulaire se rend dans l'application**, avec les seuls types de champ que le modèle définit, le pays prérempli depuis le profil, une donnée sensible avec son consentement.
- Q : « Ajouter à mon agenda » sur une activité ? → R : **Non**, même règle que les réunions ; écart à inscrire.
- Q : La rediffusion ? → R : **publique**, comme le direct sur le site.
- Décisions prises seul, acceptées : inscription avec compte ; « Les jours suivants » jour par jour ; l'étiquette ouvre le détail de l'activité, l'édition lue en une fois ; une annulation hors connexion annule une inscription pas encore partie ; un rejeu d'annulation qui trouve l'inscription déjà annulée est un succès.

**Hors périmètre** : l'inscription sans compte (elle reste au site) ; la saisie ou la modification d'une activité (le back-office de l'ePavillon le fait déjà) ; la présence ; les notifications d'une activité.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Lire les activités du Pavillon (Priority: P1)

Sur le segment « Pavillon », Aïssatou voit le bloc de lieu (« Le stand OIF · IFDD », « Zone bleue, hall 4, stand B12 », « Voir sur le plan du site »), puis les activités d'aujourd'hui avec leur heure, leur état et, le cas échéant, son inscription ou la rediffusion ; puis les rediffusions de la veille ; puis « Les jours suivants ». Une activité s'ouvre sur son détail : heure avec jour et fuseau, lieu, organisateurs, langue, intervenantes et intervenants, rediffusion.

**Acceptance Scenarios**:

1. **Given** le programme publié de l'édition, **Then** toutes les activités paraissent — **jamais filtrées par mes thématiques**.
2. **Given** une activité passée avec rediffusion, **Then** « Rediffusion · 52 min » ; touchée, la rediffusion s'ouvre.
3. **Given** « Les jours suivants », **Then** les activités des jours à venir, jour par jour.
4. **Given** une réunion de la Francophonie liée à une activité, **When** on touche « Se tient aussi au Pavillon », **Then** le détail de cette activité s'ouvre.
5. **Given** hors connexion, **Then** liste et détails lus se relisent avec « lu à ».

### User Story 2 — S'inscrire à une activité (Priority: P1)

Sur le détail, « M'inscrire » → « Inscrite » ; complet avec liste d'attente → « Rejoindre la liste d'attente » ; complet sans → « Complet » ; passée → rediffusion. L'inscription passe par le mécanisme d'inscription existant du site.

**Acceptance Scenarios**:

1. **Given** une personne connectée et une activité dont le formulaire se remplit d'un geste, **When** « M'inscrire », **Then** « Inscrite » (ou « Liste d'attente — position N »).
2. **Given** sans réseau, **Then** l'inscription part au retour, une seule fois ; un refus (complet, clos, pas encore ouvert) se dit.
3. **Given** une activité dont le formulaire demande davantage, **Then** le formulaire s'affiche dans l'application (types de champ du modèle, pays prérempli, consentement pour une donnée sensible), puis l'inscription part.
4. **Given** sans compte, **Then** « M'inscrire » mène à la connexion.

### User Story 3 — Le Pavillon dans « Ma journée » (Priority: P2)

La ligne « Pavillon de la Francophonie » du bloc « trois agendas » montre les activités du jour (heure, titre, lieu, origine), ou « Rien aujourd'hui. Prochaine : … ».

### User Story 4 — Le scénario qui clôt le MVP (Priority: P1)

Une négociatrice installe Guide Négo avec le code reçu sur WhatsApp, télécharge le guide, le lit en salle sans réseau, trouve *contact group* dans le lexique, voit les sessions de négociation du jour de ses thématiques avec leur heure de dernière lecture, et signale une annulation que l'administrateur valide depuis son téléphone. Il se joue de bout en bout sur la version construite, une fois les étapes 2 et 4 fusionnées dans `main` et `main` fusionnée dans cette branche ; la personne qui suit la session est prévenue.

### Edge Cases

- Programme non publié : la section le dit, sans erreur.
- Activité annulée ou reportée : état affiché, aucune inscription.
- Activité sans rediffusion : pas de marque ; passée sans rediffusion : « Terminée ».
- Une activité supprimée alors qu'une réunion y renvoie : l'étiquette ne paraît plus (le lien est tombé, étape 4).
- Inscription annulée hors connexion avant que l'inscription ait été reçue : les deux intentions s'annulent, rien ne part.

## Requirements *(mandatory)*

- **FR-001** : La section Pavillon DOIT commencer par le bloc de lieu (nom, adresse, lien vers le plan si connu), lu dans les lieux de l'édition.
- **FR-002** : Elle DOIT montrer une bande des jours de toute l'édition (passés compris), les activités publiées du jour choisi, les rediffusions de la veille, et « Les jours suivants » ; chaque ligne porte l'origine, l'heure (fuseau dit une fois, et dans le nom accessible), le titre, l'état de l'activité et une marque d'inscription ou de rediffusion.
- **FR-003** : Le Pavillon NE DOIT JAMAIS être filtré par « mes thématiques ».
- **FR-004** : Le détail DOIT montrer l'heure avec jour et fuseau, le lieu, les organisateurs (noms), la langue quand elle est connue, les intervenantes et intervenants (nom, rôle), la rediffusion et sa durée.
- **FR-005** : Les données viennent de l'API existante du programme ; ce qui manque (noms des intervenants et des organisations, rediffusion et durée, langue, liste d'attente, empreinte) se sert **dans le module existant**, à partir de données déjà en base — aucune table, aucune route parallèle, aucune copie ; **par ajouts seulement**, le site inchangé et prouvé par ses tests ; aucune donnée non publique aujourd'hui (coordonnées notamment) ne le devient.
- **FR-006a** : Quand le formulaire d'inscription demande plus que le pays, il DOIT se rendre dans l'application avec les seuls types de champ que le modèle définit ; le pays est prérempli depuis le profil ; une donnée sensible demande son consentement.
- **FR-012** : Aucune activité ne porte « Ajouter à mon agenda » (écart à inscrire).
- **FR-006** : L'inscription DOIT passer par le mécanisme existant (`POST /sessions/{id}/registrations`, annulation, « mes inscriptions ») ; états Inscrite, Liste d'attente (position), Complet, Rediffusion, Sans inscription (activité qui n'en prend pas) ; aucun geste sur une activité annulée ou reportée ; hors connexion par la file d'écritures, une seule fois au retour.
- **FR-007** : Les issues de refus (complet, clos, pas encore ouvert) DOIVENT se dire en mots clairs.
- **FR-008** : L'étiquette « Se tient aussi au Pavillon » DOIT ouvrir le détail de l'activité liée.
- **FR-009** : La ligne Pavillon de « Ma journée » DOIT montrer les activités du jour avec leur origine, ou la prochaine.
- **FR-010** : Liste, détails lus et inscriptions DOIVENT se relire hors connexion avec « lu à ».
- **FR-011** : La recette DOIT jouer le scénario qui clôt le MVP de bout en bout, sur la version construite.

### Key Entities

Aucune entité nouvelle : activité (`programme.sessions`), lieu (`event.venues`), inscription (`programme.registrations`), rediffusion (`live.streams`), intervenants (`programme.session_speakers` et `identity.people`), organisations (`programme.session_organizations` et `org`).

## Success Criteria *(mandatory)*

- **SC-001** : Toutes les activités publiées de l'édition paraissent, sans filtre de thématique.
- **SC-002** : Une inscription faite sans réseau arrive une seule fois.
- **SC-003** : Aucune table nouvelle, aucune route parallèle à celles du programme.
- **SC-004** : Le scénario qui clôt le MVP se joue sans accroc sur la version construite.
- **SC-005** : Liste et détails se relisent hors connexion avec leur heure de lecture.

## Assumptions

- La rediffusion d'une activité est publique, comme sa diffusion.
- L'inscription demande un compte dans l'application ; l'anonyme reste au site.
- Le pays de la personne vient de son profil de Guide Négo (0c) quand le formulaire le demande.
