/**
 * La file des écritures différées : ce que la personne a décidé sans réseau, et qui
 * part à son retour.
 *
 * Tout est pur ici, et c'est délibéré : les règles qui comptent — une seule entrée
 * par clé, l'empreinte de la première, rien ne part sous un autre compte, un 412
 * s'abandonne — se trompent silencieusement, et un test doit pouvoir les prendre
 * sans navigateur. Le magasin et l'envoi sont injectés.
 *
 * L'IDEMPOTENCE PROTÈGE DU REJEU, L'EMPREINTE DE L'ANCIENNETÉ. Le corps porte l'état
 * entier voulu : le rejouer est sûr. Mais un choix parti en retard ne doit pas
 * effacer un choix plus récent fait ailleurs : l'empreinte de l'état sur lequel il a
 * été pris part en `If-Match`, et l'API refuse en 412 ce qui arrive trop tard.
 *
 * Ce que la file n'est pas : un mécanisme général de synchronisation. Une clé, une
 * intention, une empreinte, un compte — et elle repart.
 */
export interface Intention {
  /** Ce qu'elle écrit. Une seule intention par clé. */
  cle: string
  /** L'état entier voulu, jamais un delta. */
  corps: unknown
  /** L'état sur lequel le choix a été pris — envoyée en `If-Match`. */
  empreinte: string | null
  /** La personne qui l'a prise : elle ne part jamais sous un autre compte. */
  personne: string
  /** Instant ISO de la prise, pour le dire si l'envoi tarde. */
  prise_a: string
}

export interface MagasinEcritures {
  lire(): Promise<Intention[]>
  lireUne(cle: string): Promise<Intention | null>
  poser(intention: Intention): Promise<void>
  retirer(cle: string): Promise<void>
  vider(): Promise<void>
}

/** Ce que l'envoi rend : la réponse de l'API, ou son silence. */
export type Reponse =
  | { statut: 'succes' }
  | { statut: 'refus'; code: number; message: string | null }
  | { statut: 'panne' }

export type Sort = 'envoyee' | 'perimee' | 'refusee' | 'reportee' | 'ignoree'

export interface Suite {
  cle: string
  sort: Sort
  /** Le message de l'API, tel quel, quand elle a parlé. */
  message: string | null
}

export interface Expediteur {
  envoyer(intention: Intention): Promise<Reponse>
  /** Relire l'état vrai — après un succès, un 412 ou un refus. */
  relire?(): Promise<void> | void
}

export function magasinEnMemoire(): MagasinEcritures {
  const entrees = new Map<string, Intention>()
  return {
    lire: async () => [...entrees.values()],
    lireUne: async (cle) => entrees.get(cle) ?? null,
    poser: async (intention) => void entrees.set(intention.cle, intention),
    retirer: async (cle) => void entrees.delete(cle),
    vider: async () => entrees.clear(),
  }
}

/**
 * Une nouvelle intention remplace la précédente sur la même clé — mais garde SON
 * empreinte : c'est l'état qu'a vu la personne avant de commencer à changer d'avis.
 */
export function fusionnerLesIntentions(
  existante: Intention | null,
  nouvelle: Intention,
): Intention {
  if (!existante || existante.personne !== nouvelle.personne) return nouvelle
  return { ...nouvelle, empreinte: existante.empreinte, prise_a: existante.prise_a }
}

/** Pose une intention. Ne lève jamais : un stockage refusé ne casse pas l'application. */
export async function poserUneIntention(
  magasin: MagasinEcritures,
  nouvelle: Intention,
): Promise<void> {
  try {
    const existante = await magasin.lireUne(nouvelle.cle)
    await magasin.poser(fusionnerLesIntentions(existante, nouvelle))
  } catch {
    /* Sans garde, le choix a déjà été affiché et parti si le réseau est là. */
  }
}

/**
 * Ce qu'on fait d'une réponse. Un 412 et un refus définitif retirent l'entrée — une
 * intention qui ne peut pas aboutir repartirait à chaque ouverture. Une panne ou un
 * 5xx la gardent : c'est exactement ce pour quoi la file existe.
 */
export function decider(reponse: Reponse): Exclude<Sort, 'ignoree'> {
  if (reponse.statut === 'succes') return 'envoyee'
  if (reponse.statut === 'panne') return 'reportee'
  if (reponse.code === 412) return 'perimee'
  if (reponse.code === 408 || reponse.code === 429 || reponse.code >= 500) return 'reportee'
  return 'refusee'
}

export interface File {
  poser(nouvelle: Intention): Promise<void>
  /** Fait partir ce qui attend. Deux appels simultanés n'envoient qu'une fois. */
  partir(): Promise<Suite[]>
  vider(): Promise<void>
}

export interface DependancesFile {
  magasin: MagasinEcritures
  expediteurs: Map<string, Expediteur> | ((cle: string) => Expediteur | undefined)
  /** La personne connectée à l'instant du départ ; nulle, rien ne part. */
  personne: () => string | null
  /** Ce que la personne doit savoir : un 412, un refus définitif. */
  signaler?: (suite: Suite) => void
}

export function creerFile(deps: DependancesFile): File {
  let departEnCours: Promise<Suite[]> | null = null
  // Posé ou remplacé pendant un départ : le départ suivant le prend aussitôt.
  let aRejouer = false

  const expediteurDe = (cle: string) =>
    typeof deps.expediteurs === 'function' ? deps.expediteurs(cle) : deps.expediteurs.get(cle)

  async function envoyerTout(): Promise<Suite[]> {
    const suites: Suite[] = []
    let intentions: Intention[]
    try {
      intentions = await deps.magasin.lire()
    } catch {
      return suites
    }
    const personne = deps.personne()

    for (const intention of intentions) {
      const expediteur = expediteurDe(intention.cle)
      if (!personne || intention.personne !== personne || !expediteur) {
        suites.push({ cle: intention.cle, sort: 'ignoree', message: null })
        continue
      }

      let reponse: Reponse
      try {
        reponse = await expediteur.envoyer(intention)
      } catch {
        reponse = { statut: 'panne' }
      }

      const sort = decider(reponse)
      const suite: Suite = {
        cle: intention.cle,
        sort,
        message: reponse.statut === 'refus' ? reponse.message : null,
      }
      suites.push(suite)

      if (sort === 'reportee') continue
      // Un choix posé pendant l'envoi a pris la place de celui-ci : il reste, et repart.
      const gardee = await deps.magasin.lireUne(intention.cle).catch(() => intention)
      if (gardee && JSON.stringify(gardee.corps) !== JSON.stringify(intention.corps)) {
        aRejouer = true
        continue
      }
      try {
        await deps.magasin.retirer(intention.cle)
      } catch {
        /* L'entrée repartira ; le remplacement en bloc rend le rejeu sûr. */
      }
      // Après un refus aussi : ce qui s'affichait déjà (un ajout à l'agenda) doit se défaire.
      try {
        await expediteur.relire?.()
      } catch {
        /* La relecture a ses propres gardes. */
      }
      if (sort === 'perimee' || sort === 'refusee') deps.signaler?.(suite)
    }
    return suites
  }

  return {
    async poser(nouvelle) {
      if (departEnCours) aRejouer = true
      await poserUneIntention(deps.magasin, nouvelle)
    },
    partir() {
      if (departEnCours) return departEnCours
      departEnCours = (async () => {
        const suites = await envoyerTout()
        while (aRejouer) {
          aRejouer = false
          suites.push(...(await envoyerTout()))
        }
        return suites
      })().finally(() => (departEnCours = null))
      return departEnCours
    },
    async vider() {
      try {
        await deps.magasin.vider()
      } catch {
        /* Rien à faire de plus : sans stockage, il n'y a rien à vider. */
      }
    },
  }
}
