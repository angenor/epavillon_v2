/**
 * La garde `auth` ne renvoie à la connexion que sur une session FINIE.
 *
 * Non tranchée — rendu serveur avec un témoin, qui ne peut pas tourner le jeton,
 * ou API injoignable —, la page passe : le navigateur tournera le jeton et
 * tranchera. La renvoyer à la connexion faisait passer par elle toute page
 * rechargée un quart d'heure après la connexion.
 */
export function authGuardOutcome(session: {
  isAuthenticated: boolean
  isResolved: boolean
}): 'allow' | 'sign-in' {
  return session.isAuthenticated || !session.isResolved ? 'allow' : 'sign-in'
}
