# A13 bis : Envoi de courriels du back-office

> Extrait de la [progression](../../PROGRESSION.md). Demande urgente du commanditaire du 28/10, sans prompt dans
> [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) : l'équivalent de l'envoi de courriels de la v1, « similaire
> mais mieux développé, pas aussi désordonné ».

**État** : livré le 28/10 (modèle, API, worker, écrans) ; éprouvé au navigateur sur données d'exemple, pas contre l'API ni vers le serveur réel.

---

## Ce que faisait la v1 (relevé du 28/10)

Code : `epavillonvue/src/views/admin/EmailManager.vue`, `components/email/EmailManagerModal.vue`, `composables/useEmailSender.js`,
`supabase/functions/send-email/`, relais Laravel `laravel/email_polivalent/`. `send-flexible-email/` est vide, rien ne l'appelle.

- **Destinataires** : saisie libre en À, Cc, Cci (autocomplétion sur les comptes, collage séparé par virgules) ; ajouts
  rapides par événement (« tous les participants » : soumissionnaires, inscrits, panélistes) ou par activité
  (soumissionnaire, panélistes, inscrits) ; trois filtres groupés (activités datées, approuvées, en évaluation), en Cci.
  Liste construite dans le navigateur, dédoublonnée par `Set`. Ni rôles, ni liste de suppression.
- **Copie** : Cc et Cci, **pas d'adresse de réponse**. Cc par défaut selon l'écran (deux adresses personnelles de l'équipe,
  ou `epavillonclimatique@francophonie.org`).
- **Sémantique de copie** : **un message par destinataire À, chacun portant tous les Cc et Cci**. Pour N destinataires,
  l'équipe en copie reçoit N copies (exception : plus de cinq À sans variable et sans Cci, le relais passe les À en Cci
  et n'envoie qu'un message).
- **Contenu** : objet, corps en texte brut (`textarea`, `nl2br`), une trentaine de variables `{nom}` (destinataire,
  événement, activité, dates), modèles codés en dur (une douzaine). Aperçu sans les vraies valeurs, **pas d'essai**,
  pas de pièce jointe. Signature et pied de page ajoutés d'office.
- **Envoi** : synchrone depuis le navigateur, un appel par destinataire en parallèle, délai dépassé compté comme succès,
  aucune cadence. **Aucun historique** (l'onglet était « bientôt disponible »).

## Ce qui a été livré

| Couche | Où | Quoi |
|---|---|---|
| Modèle | `docs/database/110_engagement.sql` § 12 | `engagement.mailings` (brouillon, critères, adresses libres, Cc, Cci, réponse, auteur, dates), `engagement.mailing_recipients` (destinataires figés, valeurs des variables, critères d'origine, état), `engagement.mailing_audience()` (la liste, bornée à l'édition, dédoublonnée sur l'adresse, liste de suppression signalée), réglage `engagement.mailing.team_cc` ([]), sans limite d'envoi, permission `engagement.mailing.send` (rôle `admin`) |
| Migration | `ops/migrations/2026-10-28-envoi-de-courriels.sql` | rejouable, additive ; jouée deux fois en local |
| Noyau | `kernel/src/mail.rs` | `Copies` (Cc, Cci, réponse) et `Mailer::send_with_copies`, servie par le SMTP ; un transport qui ne sait pas la porter la refuse. Aucun courriel livré ne change |
| Garde d'envoi | `engagement/src/mail.rs` | les copies traversent la garde ; trace et liste de suppression sur le destinataire principal |
| API | `engagement/src/routes/mailings.rs`, `service/mailings.rs`, `repo/mailings.rs`, `domain/mailing.rs` | dix routes sous `/admin/mailings` : historique, brouillon (créer, lire, modifier, supprimer), liste avant envoi, essai, envoi, arrêt, duplication. Deux codes : `ENGAGEMENT_MAILING_NOT_DRAFT` (409), `ENGAGEMENT_MAILING_EMPTY` (422) |
| Worker | `engagement/src/jobs/send_mailing.rs` | `engagement.send_mailing` (file « email ») : lots de dix messages enchaînés sans attente, sans limite d'envoi ; `engagement.send_mailing_test` |
| Écrans | `pages/admin/courriels/index.vue`, `[id].vue`, `components/admin/mailings/` (7), `utils/mailing.ts`, `composables/api/admin-mailings.ts`, `types/admin-mailings.ts`, `mocks/admin-mailings.ts` | historique filtrable ; brouillon en quatre étapes ; fiche d'un envoi parti |
| Traductions | `i18n/locales/{fr,en}/pages/admin.mailing.{common,list,compose,record}.json` | trois fichiers au plus par page |
| Tests | `engagement/tests/envois.rs` (4), tests unitaires du domaine et du travail, `frontend/tests/site/mailing.test.ts` (3) | liste dédoublonnée, adresse supprimée écartée, copie sur chaque message, variables, refus hors périmètre, variable inconnue refusée, envoi parti non modifiable, duplication |

## Le parcours

1. **Historique** (`/admin/courriels`) : les envois de l'édition choisie, filtrables par état (onglets) et par objet ou
   auteur ; « Écrire un courriel » crée tout de suite un brouillon enregistré.
2. **Destinataires** : quatre familles nommées qui s'additionnent (déposants et contacts des dossiers, intervenants,
   inscrits, équipe de l'édition), chacune avec ses filtres (état du dossier, état de l'inscription, rôle, activité),
   plus des adresses libres.
3. **Message** : objet, corps dans `UiRichText` (le même éditeur que le reste du back-office), variables `{{prenom}}`,
   `{{nom}}`, `{{organisation}}`, `{{email}}`, `{{edition}}` à copier, et le bloc **Réponses** (copie proposée : l'adresse
   de qui écrit et `team_cc`, copie cachée, adresse de réponse), avec la phrase qui en explique l'usage.
4. **Vérification** : décompte de la base, nombre de messages remis au serveur (copies comprises), mises en garde chiffrées (adresses écartées,
   destinataires sans prénom, copies par adresse en copie), aperçu rendu « vu par » un destinataire au choix, essai à soi,
   liste complète avec recherche.
5. **Envoi** : confirmation, destinataires figés, file du worker. La fiche montre ensuite chaque destinataire et son état
   (en file, en cours, parti, échec avec le message du serveur, écarté, non envoyé), avec « Dupliquer » et « Arrêter ».

## Décisions et écarts

1. **Copie : la sémantique v1 est reprise**, un message par destinataire, chacun avec la copie. C'est ce qui fait voir
   les réponses à l'équipe quand le destinataire répond à tous. **Coût dit à l'écran** : 300 destinataires, 300 copies
   par adresse en copie. **Proposé sans l'imposer** : mettre l'équipe en adresse de réponse plutôt qu'en copie ; les
   réponses lui arrivent, et l'historique remplace la copie comme trace de ce qui est parti.
2. **Sans limite d'envoi, décision du commanditaire le 28/10.** Une cadence horaire (200 destinataires par heure) avait
   d'abord été posée, faute d'en trouver une dans le code ; elle est retirée du modèle, de l'API et des écrans. Les lots
   de dix s'enchaînent sans attente ; le marquage « en cours » avant l'expédition empêche un double envoi à la reprise,
   et un serveur injoignable rend le destinataire à la file. La limite horaire de l'ancien hébergement existe, de valeur
   inconnue (`DEPLOIEMENT.md` § 10) : un refus du serveur se lit dans l'état de chaque destinataire.
3. **Un envoi appartient à une édition** (`event_id` obligatoire) : règle n° 8 tenue par construction. Les adresses
   libres restent permises à un administrateur d'édition : il les tape lui-même, elles ne révèlent personne.
4. **Modèles : pas de modèles réutilisables**, la duplication d'un envoi passé en tient lieu. `message_templates` est
   lié aux types de notification et multilingue, trop lourd pour ce jalon.
5. **Pas de pièce jointe** (la v1 n'en avait pas). Pas de signature ajoutée : l'écran demande de signer.
6. **Critère par activité** : la liste des activités vient de `/admin/showcase/sessions`, qui ne rend que les séances
   **publiées** ; une activité non publiée ne se cible pas encore seule (tous les critères restent possibles sur
   l'édition entière).
7. **Le texte brut** est dérivé du HTML (paragraphes et listes gardés) ; `{{edition}}` est rendu en français.

## Vérifications

- **Au navigateur, sur les données d'exemple** (thème sombre, 1280 et 375 px, sans défilement horizontal) : historique,
  création d'un brouillon, les quatre étapes, aperçu « vu par », essai, confirmation, fiche d'un envoi parti, duplication.
  Corrigé en chemin : la page ne se remontait pas en passant d'un envoi à sa copie (`key` de page), un en-tête d'aperçu se
  coupait au milieu d'un mot, une case vide dans les compteurs à 375 px.
- **Mailpit** relit un message parti avec copie, copie cachée et adresse de réponse (`kernel/tests/envoi_smtp.rs`).
- **Base** : `mailing_audience()` sur la base locale (édition CdP31 de la reprise), frontières de modules conformes.

## Ce qui n'a pas été vérifié

- Le parcours au navigateur contre l'API réelle (API et worker non lancés : la base locale est partagée avec une autre
  session, dont le worker aurait pu prendre les travaux).
- L'envoi par le serveur SMTP de l'ancien hébergement, et sa limite horaire réelle.
- Seul le thème sombre a été regardé ; le thème clair suit les jetons.
