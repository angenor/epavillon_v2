/**
 * Middleware `requires-organization` — l'étape intermédiaire qui s'insère toute
 * seule avant une action qu'on ne peut pas faire en son nom propre.
 *
 * LA RÈGLE MÉTIER, TELLE QU'ARRÊTÉE PAR LE COMMANDITAIRE LE 17/08 : rejoindre
 * une organisation n'est PAS obligatoire pour avoir un compte. On s'inscrit pour
 * suivre des séances, lire le programme, recevoir les annonces, sans appartenir
 * à quoi que ce soit. En revanche, certaines actions se font nécessairement AU
 * NOM d'une organisation — déposer une proposition d'activité, pour commencer —
 * et celles-là exigent un rattachement. La liste s'allongera au fil des écrans ;
 * c'est pour cela que la règle est un middleware NOMMÉ, posé écran par écran,
 * plutôt qu'un test recopié dans chaque page.
 *
 *   definePageMeta({ middleware: ['auth', 'requires-organization'] })
 *
 * CE QU'IL FAUT DÉCLARER AVEC LUI. La page annonce POURQUOI elle exige un
 * rattachement, faute de quoi l'écran de rattachement affiche un bandeau
 * générique. La clé est celle d'un message de `organization.join.required.reasons` :
 *
 *   definePageMeta({
 *     middleware: ['auth', 'requires-organization'],
 *     organizationReason: 'proposal',
 *   })
 *
 * CE N'EST PAS UN CONTRÔLE DE SÉCURITÉ — un middleware de navigation s'exécute
 * dans le navigateur. L'API refusera de toute façon un dépôt sans adhésion
 * active ni permission (`identity.has_permission`, rôle `org_member`). Ici on
 * évite qu'un formulaire de sept étapes se remplisse pour être rejeté à la fin.
 *
 * UNE DEMANDE EN ATTENTE NE PASSE PAS, et ce n'est pas une sévérité inutile :
 * `memberships.status = 'pending'` veut dire qu'aucun référent n'a encore
 * accepté. Laisser déposer dans cet état, c'est promettre un dossier qui sera
 * refusé à l'enregistrement. L'écran de rattachement montre alors la demande en
 * cours et explique ce qu'on attend — c'est une information, pas un mur muet.
 */
export default defineNuxtRouteMiddleware(async (to) => {
  const memberships = useMembershipStore()
  await memberships.ensureLoaded()

  if (memberships.hasActiveOrganization) return

  const localePath = useLocalePath()
  return navigateTo({
    path: localePath('organization-join'),
    query: {
      // Le même paramètre que le middleware `auth` et que la page de connexion :
      // une seule convention de retour dans toute l'application.
      redirect: to.fullPath,
      // Pourquoi cette page l'exigeait. Non traduit : c'est une clé, pas un texte.
      reason: typeof to.meta.organizationReason === 'string' ? to.meta.organizationReason : undefined,
    },
  })
})
