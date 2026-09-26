# Quickstart — recette de l'étape 3b

Dossier `/Users/mac/Documents/projets/IFDD/epavillon_v2-dev2`, base `epavillon_dev2`, API **8096**,
site **3005**, version **construite**. On n'arrête que ses propres PID. Courriels : Mailpit
`http://localhost:8025`.

## 0. Préalables

Base migrée ; API et worker lancés ; import allumé sur `cop30/lecture-1`, premier jour = aujourd'hui.
Trois comptes : **A** admis (accès négociateur), qui a une session S dans « Mon agenda » et suit
« Adaptation » ; **B** connecté sans accès ; **V** administrateur (permission de valider).

## 1. Signaler (US1)

- A, fiche de S : « Signaler un changement » → « La salle a changé » → « Salle 9 » → « Envoyer » :
  trois gestes, « Signalement envoyé. Vérification en cours. », « Votre signalement — envoyé à … ».
- A, réseau coupé : signaler une autre session ; « Mes signalements » la montre en attente d'envoi ;
  réseau rétabli → **une** ligne en base (SC-001).
- A : second signalement sur S → refusé, dit à l'écran.
- B : « Signaler » mène à l'accès. Sans compte : idem.
- A, bas de la liste : « Signaler une réunion non annoncée » — Quoi, Où, Quand, Adaptation.

## 2. Valider (US2)

- V : entrée « Signalements » dans Ressources ; la carte montre A, le motif, la source du moment.
- « Valider » puis « Annuler » avant six secondes → à traiter de nouveau ; **aucune notification,
  aucun courriel** (Mailpit, centre de A) — SC-004.
- « Valider » sans annuler → encart visible ; A prévenue.
- Un autre : « Ne pas retenir » → « Déjà pris en compte » → A voit « Non retenu à … » et le motif.
- Réseau coupé chez V : valider est refusé, dit à l'écran.

## 3. Par-dessus (US3)

- Fiche de S : encart violet sous l'état, « Signalé par le réseau — validé par l'IFDD à … », sans nom ;
  « Source officielle » intacte. En base, la ligne de S et ses changements identiques à avant (SC-003).
- Liste et « Mon agenda » : repère losange + « Signalé ».
- Réunion non annoncée validée : dans la liste, « Non annoncée — signalée par le réseau, validée à … ».
- Import sur une archive où S est en salle 9 → l'encart se retire à cette lecture (SC-007).
- Seuil atteint (`injoignable`) : sessions officielles coupées, la réunion non annoncée reste, avec sa
  phrase.
- V : « Retirer » la réunion non annoncée → disparaît.

## 4. Notifier (US4)

- `lecture-2` déplace S → cloche de A avec compteur jaune, centre : « Déplacée — … », un toucher ouvre
  la fiche et marque lu ; un courriel dans Mailpit.
- Deux changements sur S en moins de dix minutes (`lecture-2` puis une archive qui la re-déplace) →
  **un** courriel, qui dit l'état final (SC-006).
- « Tout marquer comme lu » → compteur à zéro.

## 5. Réglages (US5)

- « À propos » : « Notifications » éteint → nouveau changement : notification dans l'application,
  **aucun courriel** ; rallumé → courriels de nouveau. Une ligne par bascule dans
  `identity.consents` avec la version servie.
- Profil : « Adaptation » allumée → un changement d'une session d'Adaptation hors agenda prévient A ;
  éteinte → non. Les sessions de l'agenda préviennent toujours.

## 6. Hors connexion et écrans

Centre, « Mes signalements », encarts et réunions non annoncées relus sans réseau avec « lu à ».
360 px, clair et sombre, contre 07, 08, 09, 11 (écran 1) et 02 (10, 11, 12) ; repère jamais la couleur
seule ; cibles de 48 px. **À la fin : import éteint, signalements de recette laissés en base.**
