/**
 * Middleware `guest` — charge la session avant d'afficher un écran
 * d'authentification.
 *
 * IL NE REDIRIGE PAS, et c'est une décision. Renvoyer d'autorité une personne
 * déjà connectée vers l'accueil produit un aller-retour que personne ne
 * comprend : on a cliqué sur « Se connecter », on se retrouve ailleurs sans
 * explication. Les cinq écrans d'A1 affichent à la place un état « vous êtes
 * déjà connecté », qui nomme le compte en cours et propose les deux suites
 * possibles — continuer, ou se déconnecter pour changer de compte.
 *
 * Son seul travail est donc de garantir que `useAuthStore().isResolved` soit
 * vrai avant le rendu, y compris au rendu serveur : sans cela, chaque page
 * afficherait son formulaire une fraction de seconde avant de se raviser.
 */
export default defineNuxtRouteMiddleware(async () => {
  await useAuthStore().ensureLoaded()
})
