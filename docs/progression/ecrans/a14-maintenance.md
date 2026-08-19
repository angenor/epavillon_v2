# A14 — Page « En cours de maintenance »

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 18/08. **Le modèle a été complété d'un drapeau** : `directory.enabled` dans `900_seed.sql` § 2 — l'annuaire était le seul des six modules fermés sans drapeau de module. 1 page `app/pages/maintenance/[module].vue` (UNE seule, paramétrée par la clé du module), 1 middleware **global** `feature-flag.global.ts`, 1 registre `utils/feature-modules.ts`, 1 store `stores/features.ts`, 1 fichier de mocks `mocks/feature-flags.ts`, 1 méthode d'API (`platform.featureFlags()`), 4 fichiers de traduction (2 × 2 locales). Le routage détourne **dans les deux sens** : drapeau éteint → page de maintenance, drapeau allumé → retour à l'espace. `UiMaintenanceState` gagne un créneau `actions` et son propre fichier de traduction ; `community.vue` et `negotiations.vue` sont conservées telles quelles — le middleware les précède désormais

---

## Écarts relevés en écrivant la page « En cours de maintenance » (A14, 18/08)

**Un complément du MODÈLE, fait avant d'écrire l'écran** (voir le tableau plus haut) : le drapeau `directory.enabled`.

1. **`CLAUDE.md` DÉCRIVAIT UN ÉTAT PÉRIMÉ DES DRAPEAUX.** « Formations et messagerie n'ont aucun drapeau semé — ils sont à
   créer » : faux depuis A10, qui avait semé `training.enabled`, `messaging.enabled`, `negotiation.enabled` et
   `tools.enabled` sans que le fichier d'instructions le note. La session a donc commencé par créer ce qui existait déjà.
   **Corrigé dans `CLAUDE.md`** ; la leçon est que le paragraphe « Périmètre actuel » se relit quand on touche au semis.

2. **UN DRAPEAU DE MODULE N'EST PAS UN DRAPEAU DE FONCTIONNALITÉ, et `900_seed.sql` § 2 le dit déjà.**
   `negotiation.channels` ne couvre que les canaux d'échange à l'intérieur de l'espace ; le fermer ne ferme pas
   l'espace. Le registre du front n'accepte donc que les clés `<module>.enabled` — c'est une règle écrite dans
   `utils/feature-modules.ts`, pas une convention implicite.

3. **LE MODÈLE SAIT RÉSOUDRE UN DÉPLOIEMENT PROGRESSIF, LE FRONT NON — ET C'EST VOULU.**
   `platform.is_feature_enabled(clé, personne)` calcule `md5(clé || personne) % 100 < pourcentage` pour qu'une même
   personne voie toujours la même chose. Le store ne considère ouvert qu'un drapeau à 100 % : rejouer un MD5 dans le
   navigateur pour un cas qu'aucun des treize drapeaux semés n'utilise aurait été du code mort et une seconde vérité.
   **À trancher côté API (B1)** : soit `/platform/feature-flags` rend un booléen DÉJÀ RÉSOLU pour la session, soit le
   déploiement progressif reste hors de portée du front. La première solution est la bonne — c'est la base qui a la
   fonction.

4. **QUATRE DES SIX MODULES FERMÉS N'ONT AUCUNE PAGE.** `/publications`, `/formations`, `/outils` et `/messagerie`
   répondent 404, et le middleware ne peut détourner qu'une route qui existe. Leur créer une page vide pour la faire
   aussitôt détourner n'aurait rien apporté : leurs noms de route sont inscrits au registre, inertes, et le jour où l'un
   d'eux aura une page elle sera fermée sans qu'on y pense. **Ce n'est pas un écart du modèle, c'est un fait de
   périmètre** — le jalon ne construit pas ces modules.

5. **`community.vue` ET `negotiations.vue` SONT DEVENUES INATTEIGNABLES, et elles restent.** Le middleware les détourne
   tant que leur drapeau est éteint ; elles gardent leur `defineI18nRoute` — c'est la cible des entrées de barre, et le
   détournement a besoin d'une route source — et leur `UiMaintenanceState` en attendant qu'un vrai écran les remplisse.
   **La conséquence à ne pas oublier** : le jour où l'on ouvre `directory.enabled` ou `negotiation.enabled` sans avoir
   écrit l'écran, ces deux pages afficheront un état de maintenance que plus rien ne commande.
