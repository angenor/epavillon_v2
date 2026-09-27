# Étape 0c — Thématiques, « Ma journée », profil

> Le détail de l'étape, déplacé tel quel de l'ancien suivi le 27/09. Son état en une phrase est au [point central](../../progress.md) ; ce qui s'est fait jour par jour, au [journal](../journal/).

**État** : 🟡 Close côté code, 22/09/2026 — mise en ligne et appareil réel au § 15 de DEPLOIEMENT.md

## Références

[spec](../../../../specs/010-guide-nego-accueil-profil/spec.md) · [plan](../../../../specs/010-guide-nego-accueil-profil/plan.md) · [recherche](../../../../specs/010-guide-nego-accueil-profil/research.md) · [modèle](../../../../specs/010-guide-nego-accueil-profil/data-model.md) · [trois contrats](../../../../specs/010-guide-nego-accueil-profil/contracts/) · [recette](../../../../specs/010-guide-nego-accueil-profil/quickstart.md)

## Cadre et décisions

- quatre récits, 44 exigences, douze critères, sur les écrans « 06 Thématiques », « 07 Ma journée », « 08 Ma journée hors connexion », « 11 Profil » et « 12 À propos ».
- **Deux questions tranchées par le commanditaire le 22/09.** (1) *Les thématiques de négociation sont un vocabulaire à elles* — `negotiation_theme` dans `reference.taxonomy_terms`, semé des dix thématiques de la maquette, comme `activity_theme` l'est pour les activités : une donnée, jamais une liste dans le code. Les sessions (3a) et les alertes (3b) s'y rattacheront ; **le Pavillon garde `activity_theme` et n'est jamais filtré par « mes thématiques »**. Pas d'écran d'administration des vocabulaires : le site n'en a pour aucun. (2) *Les textes qui engagent ont une source unique* — un fichier Markdown `fr`/`en` par texte, avec sa version en tête, embarqué dans l'API et servi par une route publique ; **la version servie remplace le réglage `PRIVACY_POLICY_VERSION`**, `identity.consents` enregistre celle que l'API a servie, et un test refuse un texte modifié sans nouvelle version. Les pages `/confidentialite` et `/conditions-utilisation` du site, aujourd'hui liées et absentes, se construiront dessus, hors de cette étape ; un éditeur pourra venir plus tard derrière la même route.
- **Trois divergences avec la maquette, inscrites** : les thématiques suivent le compte (son écran « À propos » dit l'inverse) ; **les trois interrupteurs de consentement ne sont pas livrés** — aucun n'a d'effet à cette étape, et un interrupteur sans effet trompe (constitution, XII), [écart 40](../../05-design.md) ; la cloche attend 3b.
- **Laissé au plan** : où vit le lien personne ↔ thématiques — `identity.negotiator_profiles.specializations`, qui existe déjà, ou une table propre. **Une vérité, pas deux**.

## Historique

- **Phase 1 livrée le 22/09** sur la branche `010-guide-nego-accueil-profil` : le bloc `auth` est sorti de `useApi.ts` (973 → 895 lignes) vers `composables/api/auth.ts`, sans qu'aucun des appelants ne bouge.
- **Phase 2 livrée le 22/09** : vocabulaire `negotiation_theme` et dix termes, table `negotiation.theme_subscriptions`, base locale **migrée** deux fois par le script rejouable et schémas comparés.
- **Phase 3 livrée le 22/09** : `GET` et `PUT /negotiation/me/themes`, empreinte sur les codes triés, `If-Match` et le `412`, trois codes stables, seize tests sur base réelle.
- **Phase 4 livrée le 22/09** : la file d'écritures différées, qui n'existait pas — magasin `ecritures`, `file.ts` pur, trois déclencheurs, vidange à la déconnexion —, et le vocabulaire lu avec le drapeau ; quinze tests sans navigateur.
- **Phase 5 livrée le 22/09 — le MVP de l'étape est tenu** : l'écran « Mes thématiques » sert le premier choix et la modification, le choix passe par la file avec son empreinte, la garde de première entrée le propose une seule fois, et le `412` s'affiche avec le message de l'API.
- **Phase 6 livrée le 22/09** : « Ma journée » et ses cinq blocs, chacun vide en une ligne, l'avatar dans l'en-tête, le jour seul en sous-titre.
- **Phase 7 livrée le 22/09** : le profil porte le nom, le pays, « Mon suivi », « Affichage », « Application » ; « Mes téléchargements » mesure la place — la libérer vient avec les documents, à l'étape 1.
- **Phase 8 livrée le 22/09** : `GET /legal/{cle}`, une source pour le site et l'application, les textes **en attente du texte de l'IFDD**, la version servie à la place de `PRIVACY_POLICY_VERSION`, « À propos » et `GnTexteLong`.
- **Phase 9, recette, le 22/09** : quickstart déroulé sur la version construite, **cinq défauts trouvés et corrigés** (code redemandé à un compte admis, `412` dit seulement sur l'écran des thématiques, boucle d'ouverture et choix perdu quand le stockage est refusé, adresse longue qui sortait de sa ligne à 320 px), SC-007 gardé par `check:guide-nego`.

## Ce qui reste

- **Reste sur appareil réel** : T096 à T098, dont T112 de 0b

## Au journal

[22/09](../journal/2026-09-22.md)
