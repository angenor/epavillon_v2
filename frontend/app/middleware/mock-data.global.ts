/**
 * Vide, à chaque navigation, la liste des données simulées de l'écran précédent.
 *
 * Sans cela, le bandeau d'un écran sans API resterait affiché sur les suivants,
 * qui, eux, lisent bien la plateforme — et il finirait par ne plus rien vouloir
 * dire. Il s'exécute avant le rendu : les appels de la nouvelle page le
 * remplissent ensuite.
 */
export default defineNuxtRouteMiddleware((to, from) => {
  // `to` ET `from` COMPARÉS, ET C'EST INDISPENSABLE. Un middleware global se
  // rejoue à l'HYDRATATION, sur la route déjà rendue par le serveur : sans cette
  // comparaison, il effaçait ce que le rendu serveur venait de marquer, une
  // fraction de seconde avant que le bandeau ne se rende. Résultat, il ne
  // s'affichait jamais — et rien ne le disait, puisque le marquage, lui, avait
  // bien eu lieu.
  if (to.fullPath === from.fullPath) return
  useMockData().reset()
})
