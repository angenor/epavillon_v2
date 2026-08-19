# A15 — Accueil public et vitrine administrable

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 19/08. **Le modèle a été écrit d'abord** : nouveau schéma `content` (`115_content.sql`), les vues `event.v_public_editions` et `programme.v_edition_stats`, et le rôle `video` ajouté à `media.attachment_role`. 3 pages (`index.vue` — qui **remplace la redirection** du 17/08 —, `admin/vitrine/{index,nouveau,[id]}.vue`), 11 composants `app/components/home/`, 6 composants `app/components/admin/showcase/`, 4 utilitaires purs (`showcase.ts`, `edition-history.ts`, `showcase-form.ts`, plus les fabriques de mocks), 3 fichiers de contrats (`types/content.ts`, `types/home.ts`, `types/admin-showcase.ts`), 3 fichiers de mocks, 2 fabriques d'API (`api/home.ts`, `api/admin-showcase.ts`), 6 fichiers de traduction (3 × 2 locales) et 18 espaces réservés d'image. L'aperçu du back-office **réutilise le composant du bandeau public** — pas une seconde mise en page. Deux compléments hors périmètre du prompt : `content.highlight.manage` ajoutée à `mocks/permissions.ts` et le module `content` à `mocks/platform.ts`, sans quoi l'écran des permissions effectives (A12) affichait un code technique. **L'historique des éditions a été refondu le 19/08** : le rail-affiche `min-h-[calc(100svh-var(--nav-height))]`, `--radius-xl` sorti de sa réserve pour les affiches, et les groupes par millésime aplatis à l'affichage — l'ordre reste celui de `groupEditionsByYear()`.
