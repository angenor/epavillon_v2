/**
 * Les adresses de la v1 (Vue + Supabase), servie sur ce domaine jusqu'au 26/10,
 * et leur équivalent v2 (D11). Pur : la table engendrée par
 * `ops/reprise-v1/redirections/generer.sh` est passée en argument.
 *
 * Ne reconnaît que des formes que la v2 n'a pas : les adresses françaises de la
 * v2 ne portent pas de préfixe, les anglaises sont sous `/en`, et la v1 n'avait
 * ni l'un ni l'autre de ces chemins, sauf `/` et `/programmations`, qui passent.
 */

export interface TableRedirectionsV1 {
  /** Activité v1 → page de sa séance publiée, sinon programmation de l'édition. */
  activites: Record<string, string>
  editions: Record<string, { page: string; programmation: string }>
  /** Nom de la page statique v1 (`journee-jeunesse`) → liste du jour. */
  journees: Record<string, string>
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const ANNEE = /^\d{4}$/

const PROGRAMMATIONS = '/programmations'
const ACCUEIL = '/'
const COMMUNAUTE = '/communaute'
const NEGOCIATIONS = '/negociations'
const MON_ORGANISATION = '/mon-organisation'
const CONNEXION = '/connexion'

/** Paramètres de la v1 qui désignent une adresse v1 : sans objet une fois redirigés. */
const PARAMETRES_V1 = ['redirect', 'from']

const PAGES_SIMPLES: Record<string, string> = {
  '/about': ACCUEIL,
  '/login': CONNEXION,
  '/register': '/inscription',
  // Le lien d'un courriel v1 portait un jeton Supabase, sans valeur ici :
  // le compte repris se connecte, la v2 redemande la vérification si besoin.
  '/verify-email': CONNEXION,
  '/auth/callback': CONNEXION,
  '/forgot-password': '/mot-de-passe-oublie',
  // Même raison : le jeton v1 ne vaut rien, on en redemande un.
  '/reset-password': '/mot-de-passe-oublie',
  '/organization/setup': MON_ORGANISATION,
  '/profile': MON_ORGANISATION,
  '/community': COMMUNAUTE,
  '/organizations': COMMUNAUTE,
  '/directory': COMMUNAUTE,
  '/nego': NEGOCIATIONS,
  '/ai/chatbot': NEGOCIATIONS,
  '/formations': '/maintenance/training',
  '/messages': '/maintenance/messaging',
  '/appointments': '/maintenance/messaging',
  '/notifications': ACCUEIL,
  '/debug-profile': ACCUEIL,
  '/403': ACCUEIL,
  '/fphn2026': ACCUEIL,
  '/activities': PROGRAMMATIONS,
  '/events': PROGRAMMATIONS,
}

/** Préfixes dont tout descendant mène à la même page. */
const ESPACES: Record<string, string> = {
  organizations: COMMUNAUTE,
  directory: COMMUNAUTE,
  nego: NEGOCIATIONS,
  formations: '/maintenance/training',
  fphn2026: ACCUEIL,
}

/** Sections de l'administration v1 : la v2 nomme les siennes en français, sans collision. */
const ADMIN_V1 = new Set([
  'users', 'activities', 'activities-dates-manager', 'organizations', 'events', 'trainings',
  'content', 'communications', 'reports', 'roles', 'negotiations', 'emails',
  'youtube-streams', 'incident-messages',
])

/** Lecture sans prototype : `/programmations/2025/constructor` ne doit rien trouver. */
function propre<T>(table: Record<string, T>, cle: string): T | null {
  return Object.hasOwn(table, cle) ? (table[cle] ?? null) : null
}

function cheminV1(segments: string[], table: TableRedirectionsV1): string | null {
  const [tete, a, b, c] = segments
  const chemin = `/${segments.join('/')}`

  if (tete === 'activities') {
    // /activities/<id>, /activities/<id>/manage|edit, /activities/preview/<id>
    const id = a === 'preview' ? b : a
    if (id && UUID.test(id) && segments.length <= 3) return propre(table.activites, id.toLowerCase()) ?? PROGRAMMATIONS
    return PROGRAMMATIONS
  }

  if (tete === 'events') {
    if (a === 'create' || a === 'dashboard') return MON_ORGANISATION
    if (a && UUID.test(a)) return propre(table.editions, a.toLowerCase())?.page ?? PROGRAMMATIONS
    return PROGRAMMATIONS
  }

  // `/programmations/<slug-édition>/<slug-séance>` est la v2 : seule l'année v1
  // suivie d'un identifiant ou d'une page nommée est reconnue, le reste passe.
  if (tete === 'programmations' && a && ANNEE.test(a) && b && c === undefined) {
    if (UUID.test(b)) return propre(table.editions, b.toLowerCase())?.programmation ?? PROGRAMMATIONS
    const journee = propre(table.journees, b)
    if (journee) return journee
    // Pages statiques non reprises (séminaire de Chypre).
    if (a === '2025' && b === 'seminaire-chypre') return PROGRAMMATIONS
    return null
  }

  if (tete === 'admin' && a && ADMIN_V1.has(a)) return '/admin'

  return propre(PAGES_SIMPLES, chemin) ?? (tete && segments.length > 1 ? propre(ESPACES, tete) : null)
}

/**
 * L'adresse v2 d'une adresse v1, chaîne de requête comprise, ou `null` si le
 * chemin n'est pas une adresse v1 (adresse v2, fichier, API).
 */
export function redirectionV1(pathname: string, search: string, table: TableRedirectionsV1): string | null {
  // La v1 précompilait ses pages en `/activities/<id>/index.html` : barre finale.
  const segments = pathname.split('/').filter((segment) => segment !== '')
  if (segments.length === 0) return null
  // PACO reste servi par la v1 elle-même (D9, D15) : Apache le prend avant le site.
  if (segments[0] === 'paco') return null

  const cible = cheminV1(segments, table)
  if (cible === null) return null

  const [chemin = '', requeteCible = ''] = cible.split('?')
  const parametres = new URLSearchParams(search)
  for (const nom of PARAMETRES_V1) parametres.delete(nom)
  for (const [nom, valeur] of new URLSearchParams(requeteCible)) parametres.set(nom, valeur)
  const requete = parametres.toString()
  return requete ? `${chemin}?${requete}` : chemin
}
