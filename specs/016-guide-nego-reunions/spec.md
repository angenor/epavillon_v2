# Feature Specification: Guide Négo — Réunions de la Francophonie (étape 4)

**Feature Branch**: `016-guide-nego-reunions`

**Created**: 2026-09-26

**Status**: Draft — quatre questions tranchées le 26/09 (voir *Clarifications*)

**Input**: Étape 4 de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md) : les réunions de la Francophonie — atelier préparatoire, concertation des négociatrices et négociateurs, concertation ministérielle — dans l'onglet « Francophonie », sélecteur « Réunions · Pavillon » ; liste et détail (lieu, heure avec fuseau, visioconférence, accès limité, inscription : Inscrite, Liste d'attente, Complet, Terminée) ; lien facultatif « Se tient aussi au Pavillon », jamais une fusion ; lisible hors connexion ; réunions du jour dans « Ma journée » ; back-office : saisir, publier, annuler, lier. Critère : une réunion saisie par l'IFDD apparaît, avec son lien éventuel vers le Pavillon.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Les écrans | [10-francophonie.html](../../docs/AppNego/design/ecrans/10-francophonie.html) — en-tête et sélecteur (1a, 1c), liste (1a), détail (1b) ; la ligne « Réunions de la Francophonie » du bloc « Aujourd'hui, vos trois agendas » de [02-socle.html](../../docs/AppNego/design/ecrans/02-socle.html) (07, 08) |
| Le système de design | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) « 4 decies » — étiquette « Se tient aussi au Pavillon », états d'inscription, bouton « M'inscrire » → « Inscrite », « Complet » → « Rejoindre la liste d'attente » ; [design/passation/](../../docs/AppNego/design/passation/) |
| Les mots | [design/lexique.md](../../docs/AppNego/design/lexique.md) — « Réunions de la Francophonie », jamais « Sessions » ni « Programme » ; « Négociatrices et négociateurs » ; « Accès limité » ; toute heure porte son fuseau |
| Les décisions | [ADR-008](../../docs/AppNego/adr/008-trois-agendas-jamais-confondus.md) trois agendas, un lien entre deux objets, jamais une fusion · [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) — une inscription sans réseau part au retour |
| Le partage de l'onglet | Protocole de la session : l'étape 4 construit la page, le sélecteur et la section Réunions ; la section Pavillon est **un seul composant**, laissé vide, que l'étape 5 remplit |
| Ce qui existe | `negotiation.meetings` (natures `preparatory_workshop`, `francophone_consultation`), `negotiation.meeting_registrations` (inscrite, liste d'attente, annulée ; compteur tenu par la base), permission `negotiation.meeting.manage` ; les composants et la file d'écritures de 0c à 3b ; les marques d'état « Inscrite », « Liste d'attente », « Complet », « Terminée » |

## Clarifications

### Session 2026-09-26 — tranché le 26/09 par l'orchestrateur, pour le commanditaire

- Q : Que veut dire « Accès limité » ? → R : **Une information** : tout le monde voit la réunion, toute personne admise peut s'inscrire, l'IFDD contrôle à l'entrée.
- Q : Qui voit le lien de visioconférence ? → R : **Les seules personnes inscrites**, hors connexion compris ; une réunion sans inscription le montre à toute personne admise ; les autres voient « Réservé aux personnes inscrites ».
- Q : La liste d'attente monte-t-elle d'elle-même ? → R : **Oui** : la première prend la place libérée, et en est prévenue dans l'application, et par courriel si son accord est allumé, par la mécanique de 3b.
- Q : « Ma journée » ? → R : Le bloc « Aujourd'hui, vos trois agendas » porte les lignes **Sessions de négociation** et **Réunions de la Francophonie** ; **la ligne Pavillon est un composant à part, `GnJourneeLignePavillon.vue`, laissé vide**, que l'étape 5 remplit seule.
- Décisions prises seul, acceptées : inscription réservée à l'accès négociateur, lecture publique ; « Terminée » ; la nature est une donnée ; lien au Pavillon par une clé `xmod_fk_` facultative `ON DELETE SET NULL` ; la table des inscriptions de `negotiation` avec le patron de liste d'attente du Pavillon ; le groupe de la recherche globale après fusion de `main` quand l'étape 2 y sera.

**Hors périmètre** : la section Pavillon et ses activités (étape 5) ; la création de salles de visioconférence depuis le back-office (le lien se saisit) ; la présence et la rediffusion ; les notifications de changement d'une réunion (au-delà de ce que FR-020 dit) ; l'export vers le calendrier du téléphone.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Lire les réunions de la Francophonie (Priority: P1)

Aïssatou ouvre l'onglet « Francophonie ». Le sélecteur « Réunions · Pavillon » est sur « Réunions » ; le titre dit « Réunions de la Francophonie ». Trois lignes : chacune porte son origine, son jour, ses heures avec le fuseau, son titre, son lieu, une seule marque d'état (Prévue, Inscrite, Liste d'attente, Complet, Terminée, Annulée), et l'étiquette « Se tient aussi au Pavillon » quand elle existe. La fiche donne l'heure avec le jour et le fuseau, le lieu, la visioconférence, l'organisateur, l'accès limité et son public, la description.

**Why this priority**: c'est le critère de l'étape.

**Independent Test**: saisir et publier une réunion au back-office ; la voir dans l'application, sans compte, puis hors connexion.

**Acceptance Scenarios**:

1. **Given** une réunion publiée, **When** on ouvre l'onglet, **Then** elle paraît, triée par début, avec son fuseau.
2. **Given** une réunion en brouillon, **Then** elle ne paraît nulle part dans l'application.
3. **Given** une réunion liée à une activité du Pavillon, **Then** l'étiquette « Se tient aussi au Pavillon » paraît sur la ligne et la fiche ; touchée, elle ouvre l'activité côté Pavillon.
4. **Given** une réunion à accès limité, **Then** la ligne porte « Accès limité — <public> » (pictogramme, mot, couleur).
5. **Given** hors connexion, **Then** la liste et les fiches se relisent avec « lu à ».
6. **Given** le segment « Pavillon », **Then** le titre dit « Pavillon de la Francophonie » et la section Pavillon s'affiche (vide à cette étape).

### User Story 2 — S'inscrire, même sans réseau (Priority: P1)

Sur la fiche, Aïssatou touche « M'inscrire » : le bouton devient « Inscrite ». Pleine, la réunion propose « Rejoindre la liste d'attente ». Sans réseau, l'inscription se voit aussitôt et part au retour ; si la réunion s'est remplie entre-temps, l'écran le dit clairement.

**Acceptance Scenarios**:

1. **Given** une personne admise, **When** « M'inscrire », **Then** « Inscrite ».
2. **Given** la réunion pleine, **Then** « Complet » et « Rejoindre la liste d'attente » ; rejoindre → « Liste d'attente ».
3. **Given** sans réseau, **When** elle s'inscrit, **Then** l'état se voit aussitôt, part au retour du réseau, n'arrive qu'une fois ; si la réunion est devenue pleine, l'écran dit « Complet » et ce qu'il est advenu (liste d'attente ou non).
4. **Given** une inscrite qui se désinscrit, **Then** la première de la liste d'attente prend la place d'elle-même, et en est prévenue dans l'application et, si son accord est allumé, par courriel.
5. **Given** une réunion terminée ou annulée, **Then** aucun geste d'inscription.
6. **Given** sans compte ou sans accès négociateur, **Then** « M'inscrire » mène à la connexion ou à l'accès, sans perdre la fiche.

### User Story 3 — L'IFDD saisit, publie, annule, lie (Priority: P1)

Au back-office de l'ePavillon, l'administration saisit une réunion (nature, titre, description, début, fin, fuseau de l'édition, lieu, visioconférence, capacité, fenêtre d'inscription, accès limité et son public), la publie, la modifie, l'annule avec un motif, la lie à une activité du Pavillon de la même édition ou retire ce lien, et voit ses inscriptions.

**Acceptance Scenarios**:

1. **Given** une réunion saisie et publiée, **Then** elle paraît dans l'application (critère).
2. **Given** « Annuler » avec un motif, **Then** la réunion paraît « Annulée » avec son motif, et les inscrites en sont prévenues.
3. **Given** un lien vers une activité du Pavillon, **Then** l'activité n'est ni copiée ni modifiée ; la supprimer côté Pavillon retire le lien.
4. **Given** une personne sans la permission, **Then** l'écran lui est refusé, y compris par une adresse forgée.

### User Story 4 — Les réunions du jour dans « Ma journée » (Priority: P2)

Le bloc « Aujourd'hui, vos trois agendas » porte la ligne « Sessions de négociation » et la ligne « Réunions de la Francophonie » : les réunions du jour avec leur heure et leur origine, ou « Rien aujourd'hui. Prochaine : <titre> — <jour>, <heure> ».

### Edge Cases

- Réunion sans capacité : pas de « Complet », inscription toujours ouverte dans sa fenêtre.
- Réunion sans fenêtre d'inscription, ou fenêtre close : l'écran dit que les inscriptions sont closes.
- Réunion déplacée ou modifiée après inscription : la fiche montre la nouvelle heure ; les inscrites en sont prévenues (FR-020).
- Capacité abaissée sous le nombre d'inscrites : aucune inscription n'est retirée ; « Complet ».
- Activité du Pavillon supprimée ou annulée : le lien disparaît (supprimée) ou reste avec l'état de l'activité (annulée), sans toucher la réunion.
- Deux inscriptions rejouées depuis deux appareils : une seule inscription.
- Réunion en ligne seulement : pas de lieu, « En ligne ».

## Requirements *(mandatory)*

### Lire

- **FR-001** : L'onglet « Francophonie » DOIT offrir un sélecteur à deux sections « Réunions · Pavillon », le segment actif plein ; le titre d'écran dit le nom complet de la section ; jamais une liste mêlée.
- **FR-002** : La section Réunions DOIT lister les réunions publiées de l'édition (atelier préparatoire, concertation des négociatrices et négociateurs, concertation ministérielle), triées par début ; chaque ligne porte l'origine, le jour, les heures, le titre, le lieu, une seule marque d'état, l'étiquette Pavillon éventuelle, l'accès limité éventuel.
- **FR-003** : La fiche DOIT montrer l'heure avec le jour et le fuseau de la COP, le lieu (ou « En ligne »), la visioconférence selon FR-011, l'organisateur, la nature, l'accès limité et son public, la description, l'étiquette Pavillon, l'inscription.
- **FR-004** : Le pied de liste DOIT renvoyer aux Sessions de négociation et au Pavillon, sous leurs noms.
- **FR-005** : La section Pavillon DOIT être un composant à part, affiché vide à cette étape, que l'étape 5 remplit sans toucher la page.
- **FR-006** : La lecture DOIT être publique ; liste et fiches se relisent hors connexion avec « lu à ».
- **FR-007** : La nature d'une réunion (atelier préparatoire, concertation des négociatrices et négociateurs, concertation ministérielle) DOIT être une donnée, jamais un libellé du code.

### S'inscrire

- **FR-008** : S'inscrire DOIT demander l'accès négociateur ; se désinscrire est toujours possible avant le début.
- **FR-009** : La capacité, la fenêtre d'inscription et la liste d'attente DOIVENT être tenues par la base ; « Complet » se déduit de la capacité et des inscrites.
- **FR-010** : Une inscription faite sans réseau DOIT se voir aussitôt, partir au retour, n'arriver qu'une fois grâce à une référence posée par le téléphone ; au retour, un refus (complet sans liste d'attente, close, annulée) se dit clairement.
- **FR-011** : Le lien de visioconférence DOIT n'être servi qu'aux personnes inscrites — gardé sur leur téléphone hors connexion ; pour une réunion sans inscription, à toute personne admise ; les autres lisent « Réservé aux personnes inscrites ». L'API ne le sert jamais à qui n'y a pas droit.
- **FR-012** : L'accès limité DOIT être une information, avec son public (« ministres et chefs de délégation ») : il ne restreint ni la lecture ni l'inscription ; l'IFDD contrôle à l'entrée.
- **FR-013** : Les états affichés DOIVENT être Prévue, Inscrite, Liste d'attente, Complet, Terminée, Annulée — un pictogramme, un mot, une couleur ; une seule marque par ligne (priorité : Annulée, Terminée, Inscrite, Liste d'attente, Complet, Prévue).

### Le lien au Pavillon

- **FR-014** : Une réunion PEUT porter une référence vers une activité du Pavillon de la même édition ; jamais une copie, jamais une fusion ; l'activité supprimée, la référence tombe.
- **FR-015** : Touchée, l'étiquette DOIT ouvrir l'activité côté Pavillon (section Pavillon de l'onglet) ; tant que l'étape 5 n'existe pas, elle ouvre la section Pavillon.

### Back-office

- **FR-016** : L'administration DOIT pouvoir saisir, modifier, publier, annuler (avec motif) une réunion, et la lier à une activité du Pavillon de la même édition ou retirer ce lien, avec les composants du back-office de l'ePavillon.
- **FR-017** : Les écrans DOIVENT être réservés à la permission de gérer les réunions, sur la portée de l'espace de négociation, filtrés par périmètre (règle 8) ; une adresse forgée est refusée.
- **FR-018** : L'administration DOIT voir, par réunion, les inscrites et la liste d'attente.
- **FR-019** : Les chevauchements entre réunions ne sont jamais bloqués.

### Prévenir

- **FR-020** : L'annulation d'une réunion, un changement d'heure ou de lieu, et une place obtenue depuis la liste d'attente DOIVENT prévenir les personnes concernées dans l'application (le centre de notifications de 3b) et par courriel selon leur accord.

### « Ma journée » et recherche

- **FR-021** : Le bloc « Aujourd'hui, vos trois agendas » DOIT porter la ligne « Sessions de négociation » (les sessions du jour de « Mon agenda », sinon de mes thématiques) et la ligne « Réunions de la Francophonie » — les réunions du jour, ou « Rien aujourd'hui. Prochaine : … » ; la ligne Pavillon est le composant `GnJourneeLignePavillon.vue`, laissé vide ; les autres blocs ne changent pas.
- **FR-022** : La recherche globale DOIT gagner le groupe « Réunions de la Francophonie », et sa ligne « Pas encore ici » ne nommer plus que le Pavillon — dès que la recherche globale (étape 2) est dans `main`.

### Key Entities

- **Réunion de la Francophonie** : nature, titre, description, début, fin, fuseau, lieu, visioconférence, capacité, fenêtre d'inscription, accès limité et son public, état, motif d'annulation, activité du Pavillon liée.
- **Inscription** : personne, réunion, état (inscrite, liste d'attente, annulée), position d'attente, référence client.

## Success Criteria *(mandatory)*

- **SC-001** : Une réunion saisie et publiée au back-office paraît dans l'application, avec son lien éventuel vers le Pavillon.
- **SC-002** : Une inscription faite sans réseau arrive une seule fois au retour.
- **SC-003** : La capacité n'est jamais dépassée, même sous inscriptions simultanées.
- **SC-004** : Aucune activité du Pavillon n'est modifiée par le lien.
- **SC-005** : Liste et fiches se relisent hors connexion avec leur heure de lecture.
- **SC-006** : Une adresse forgée du back-office est refusée à qui n'a pas la permission sur l'espace.

## Assumptions

- Les réunions se rattachent à l'espace « climat » et à l'édition servie par l'application ; le fuseau est celui de l'édition.
- La visioconférence est un lien saisi (lien tiers ou salle existante) ; créer une salle n'est pas de cette étape.
- L'inscription reprend le patron éprouvé du Pavillon (capacité, liste d'attente, verrou) plutôt qu'une règle nouvelle.
- « Terminée » (maquette) plutôt que « Terminé » (prompt).
