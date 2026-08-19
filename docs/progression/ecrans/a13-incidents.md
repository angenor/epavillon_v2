# A13 — Messages d'incident

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 18/08. **Le modèle a été complété d'abord, sur deux points** : la taxonomie `incident_kind` ne portait pas le **débordement sur le créneau suivant** — le cas que le commanditaire nomme en premier —, et le back-office n'avait aucune fonction pour lire les messages AUTRES qu'actifs. `live.event_incidents(event, at)` rend les cinq états avec la cible résolue, et `active_incidents_for_event()` est réécrite AU-DESSUS d'elle : un seul balayage de portée pour le tableau de bord et pour cet écran. 3 pages sous `app/pages/admin/incidents/`, 6 composants sous `app/components/admin/incidents/`, 1 utilitaire pur (`utils/incident-list.ts`), 1 fichier de contrats (`types/admin-incidents.ts`), 1 fichier de mocks d'écran (`mocks/admin-incidents.ts`, où `incidents.ts` ne garde que la donnée), 1 fabrique d'API, 4 fichiers de traduction. Raccourci « Signaler un débordement » posé dans le panneau de séance du planificateur, qui pré-remplit le formulaire par l'URL. `IncidentBanner`, `types/live.ts` et `mocks/incidents.ts` existaient depuis A0 : complétés, pas réécrits. **Recentré sur le direct le même jour** (retour du commanditaire) : l'écran ouvre sur le POSTE DE DIRECT — activités du jour, état d'antenne, quatre gestes par activité —, le formulaire complet restant dessous

---

## Écarts relevés en écrivant les messages d'incident (A13, 18/08)

**Deux compléments du MODÈLE, faits avant d'écrire les écrans** (voir le tableau plus haut).
Suivent sept points qui ne se tranchent pas depuis un écran.

0. **LE PROMPT DÉCRIT UN ÉCRAN PLUS LARGE QUE L'USAGE — arbitré par le commanditaire le 18/08.** Le prompt A13 énumère
   cinq portées et six natures, et le modèle les porte toutes ; l'écran qui en découle directement est un gestionnaire
   de bandeaux générique, où l'on entre par un formulaire. Or l'usage réel est celui que nomme le commanditaire :
   « signaler un dysfonctionnement lié à la **diffusion d'une activité en direct** ». Les deux ne se contredisent pas —
   la portée `session` est celle qui sert, les autres existent pour les cas rares (la visionneuse en panne, la régie du
   pavillon coupée). **Décision : l'écran s'ouvre sur le poste de direct** — les activités du jour, leur état d'antenne,
   quatre gestes par activité — et le formulaire complet reste dessous. Rien n'est retiré du modèle ni de l'interface ;
   c'est l'ORDRE qui change, et il change tout : on ne choisit plus une portée, on désigne ce qui se passe. **À
   reproduire côté API** : la composition de l'écran porte les activités du jour, pas seulement les messages.

1. **LIRE LES MESSAGES D'INCIDENT N'EST PAS UN PRIVILÈGE, et le modèle n'a qu'une permission.**
   `live.incident.publish` couvre la publication ; il n'existe pas de `live.incident.read`. Décision prise : la LISTE
   s'ouvre à quiconque administre l'édition — un bandeau est affiché au public, le cacher à l'équipe n'a aucun sens —,
   et seules les ACTIONS exigent la permission, vérifiée **sur l'édition visée**. Un membre du comité voit donc ce qui
   est en ligne, sans bouton. **Ne pas ajouter de permission** : elle ne protégerait rien qui ne soit déjà public.

2. **`live.incidents` N'A AUCUNE COLONNE D'ÉDITION pour les portées `session`, `event_day` et `organization`.**
   Le rattachement à une COP est un CALCUL — la journée appartient à l'édition, la séance aussi, l'organisation y anime
   ou non. La règle métier n° 8 ne peut donc pas filtrer sur `event_id`, et c'est exactement ce que
   `live.event_incidents()` fait à sa place. **À reproduire tel quel côté API** : le filtre de périmètre passe par la
   fonction, jamais par un `WHERE event_id = …` qui laisserait fuir les portées indirectes.

3. **L'INCIDENT GLOBAL APPARAÎT DANS TOUTES LES ÉDITIONS, et il ne s'y modifie pas comme les autres.** La fonction le
   renvoie pour chaque COP — c'est voulu, une équipe qui pilote un pavillon doit savoir qu'un bandeau de maintenance le
   couvre. Conséquence non écrite dans le modèle : un administrateur détaché sur une seule édition peut donc DÉPUBLIER
   un message qui s'affiche partout. L'écran ne l'empêche pas aujourd'hui, l'API devra trancher — **une portée globale
   se retire-t-elle depuis une édition ?** Question au commanditaire, non bloquante ; le geste reste tracé et
   réversible.

4. **LES DEUX LANGUES SONT UNE RÈGLE D'INTERFACE, PAS DE MODÈLE.** `message` est un `platform.i18n_text` non nul : la
   base accepte un français seul. Le prompt exige les deux, et l'écran refuse d'enregistrer sans anglais. À reproduire
   côté API ; **ne pas durcir la base** — la contrainte serait fausse pour les données reprises de la v1, qui n'ont
   qu'une langue.

5. **`ck_incidents_scope_target` NE PARDONNE PAS UNE CIBLE ORPHELINE**, et c'est un piège d'interface autant que de
   données : changer de portée sans vider la cible précédente produit deux colonnes renseignées et un refus de la base
   que rien à l'écran n'explique. Le formulaire vide la cible **dans le gestionnaire de choix, jamais dans un
   observateur** — un observateur effacerait aussi ce qu'un pré-remplissage vient de poser (défaut trouvé en éprouvant
   le raccourci).

6. **`display_until` NULL EST LE SEUL VRAI DANGER DE CETTE TABLE.** C'est le défaut de la v1 — des bandeaux restés en
   ligne des mois — et le modèle ne l'interdit pas, à juste titre : une panne ouverte n'a pas de fin connue.
   L'interface le SIGNALE deux fois, à la saisie et dans la liste, plutôt que de l'empêcher. **Rien à corriger dans le
   modèle** ; à reprendre dans le rappel automatique que l'API pourra poser (« ce bandeau est en ligne depuis 3 jours »).

7. **Le jeu de données simulées date les incidents de l'INCIDENT, pas de la saisie.** Les messages de portée `session`
   et `event_day` portent donc des fenêtres de novembre 2027 et se lisent « Programmé » aujourd'hui. Ce n'est pas une
   lacune : c'est ce que produira la réalité, l'édition n'ayant pas commencé. La démonstration d'un bandeau ACTIF passe
   par les deux messages de portée plateforme et édition, ou par une publication faite à l'écran.

---

## Ce qui a été vérifié le 18/08 sur les messages d'incident, et comment

**Sur la base, schéma rechargé de zéro** (`make check-db`, qui commence par `down -v`) :

- **Chaîne complète chargée sans erreur**, `platform.cross_module_fk_report` vide, projections rafraîchies —
  « Base : conforme ».
- `reference.taxonomy_terms` : la taxonomie `incident_kind` compte **neuf termes**, `overrun` rangé entre `delay` et
  `schedule_change` par son `sort_order` 35.
- Les **cinq fonctions** du schéma `live` sont créées, `event_incidents` comprise.
- **Les cinq états et les cinq portées, éprouvés par une transaction jetable** plutôt que par lecture : une édition,
  une journée, une séance et six incidents insérés, puis `ROLLBACK`. `live.event_incidents()` rend exactement
  `active`, `active`, `scheduled`, `draft`, `expired`, `unpublished` — dans cet ordre —, la cible résolue à chaque
  fois (« Atelier de négociation », « 18/08/2026 » pour une journée SANS titre, le nom légal de l'organisation), et
  `active_incidents_for_event()` réécrite au-dessus n'en garde bien que les deux premiers.
  `live.active_incidents(session)` remonte de son côté la même paire : les deux sens de la hiérarchie s'accordent.

**Sur l'interface, dans un navigateur, en agissant réellement** — c'est là qu'a été trouvé le seul défaut :

- **La liste** rend les sept messages du jeu dans l'ordre d'action (deux en ligne, deux programmés, un rédigé, puis
  l'historique), chaque cible résolue par son nom — « Journée finance », « Réseau ouest-africain pour l'adaptation
  côtière » —, et les dates dans le fuseau de l'ÉDITION.
- **Publier un brouillon depuis la ligne** le fait passer en « En ligne » et remonter en tête : le journal d'écritures
  de session est bien relu par `event_incidents()`.
- **Dépublier** ouvre un dialogue à un seul champ, facultatif ; le message part à l'historique avec son motif affiché
  sous le texte, et la ligne ne disparaît pas.
- **Le formulaire** : l'aperçu suit la saisie et rend le VRAI bandeau (`UiIncidentBanner`), avec sa fin d'affichage
  annoncée en heure de Belém ; publier redirige vers la liste, où le message apparaît en ligne.
- **Le raccourci « Signaler un débordement »**, depuis le panneau d'une séance du planificateur, pointe bien
  `/admin/incidents/nouveau?portee=session&cible=<id>&nature=overrun` et ouvre un formulaire dont la portée, l'activité,
  la nature et la fin d'affichage — la fin du créneau débordé — sont déjà posées.
- **Un compte SANS `live.incident.publish`** (un membre du comité) voit la liste et **aucune action** ; le formulaire,
  lui, refuse l'accès. La distinction est délibérée (écart n° 1).
- **LE DÉFAUT** : le `watch` sur la portée, qui vide la cible pour ne pas violer `ck_incidents_scope_target`, effaçait
  aussi la cible qu'un pré-remplissage venait de poser. Le raccourci arrivait donc — par intermittence, selon l'ordre
  d'arrivée des données — sur un formulaire dont l'activité s'était effacée toute seule. La remise à zéro appartient
  au **gestionnaire du choix**, pas à un observateur : un observateur ne distingue pas un geste d'un chargement.
- Deux corrections de lecture au passage : la cellule de portée répétait « Toute la plateforme » deux fois pour un
  message global, et la liste des séances affichait un `2027-11-13T09:30:00-03:00` brut — une donnée d'instant passée
  en libellé. Les cibles portent désormais `starts_at` à part de `hint`, et le formatage revient à l'interface.

**Après recentrage sur le direct** (même jour, retour du commanditaire) :

- **Le poste de direct** rend les activités du jour dans le fuseau de l'ÉDITION. L'édition se tenant en novembre 2027,
  le repli s'affiche — « Aucune activité aujourd'hui — voici les prochaines » — et la date apparaît alors dans chaque
  créneau : « 14:00 — 15:30 » pour une activité d'octobre 2027 se lirait sinon comme un créneau de cet après-midi.
- **« Diffusion interrompue » n'apparaît que sur les activités diffusées** (`sessions.is_streamed`) : trois gestes sur
  le webinaire en ligne, quatre sur l'ouverture du pavillon.
- **Chaque geste ouvre le formulaire rempli** : « Retard » sur l'ouverture du pavillon donne portée `session`, cible
  « Ouverture du pavillon de la Francophonie », nature « Retard », gravité « Avertissement », fin d'affichage 11:00 —
  la fin du créneau. « Diffusion interrompue » donne « Problème de connexion » et la gravité « Incident ».

**Outils** : `npm run typecheck` et `npm run build` au vert.
