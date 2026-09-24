/**
 * La lecture en ligne par plages (ADR-022, R15) : `HEAD` pour la taille, puis un
 * `GET` par morceau que pdf.js demande, avec la session. Écrite ici et non laissée
 * à pdf.js, qui lirait le fichier entier avant la première page, et pour savoir ce
 * qui est demandé, reçu, en route — la progression et le délai de bascule en dépendent.
 *
 * Tout échec du réseau rend une `ErreurDeReseau`, jamais une erreur de pdf.js :
 * la bascule ne sanctionne que l'échec de l'affichage.
 */

export class ErreurDeReseau extends Error {
  readonly statut: number | null

  constructor(message: string, statut: number | null = null) {
    super(message)
    this.name = 'ErreurDeReseau'
    this.statut = statut
  }
}

/** Ce que le `PDFDataRangeTransport` de pdf.js offre, et que l'on branche. */
export interface TransportDePdfjs {
  onDataRange(debut: number, morceau: Uint8Array | null): void
  requestDataRange(debut: number, fin: number): void
  abort(): void
}

export type Recuperer = (adresse: string, init?: RequestInit) => Promise<Response>

export interface SuiviDesPlages {
  readonly demande: number
  readonly recu: number
  readonly enRoute: number
  /** Reçu sur demandé, de 0 à 1 ; 0 tant que rien n'est demandé. */
  readonly progression: number
  /** Une plage au moins est en route : le délai de bascule ne court pas. */
  readonly enAttenteDuReseau: boolean
  readonly erreur: ErreurDeReseau | null
  /** Se résout à la première erreur de réseau ; ne se rejette jamais. */
  readonly echec: Promise<ErreurDeReseau>
}

export interface OptionsDuTransport {
  recuperer?: Recuperer
  surChangement?: (suivi: SuiviDesPlages) => void
}

const versErreurDeReseau = (erreur: unknown, contexte: string): ErreurDeReseau =>
  erreur instanceof ErreurDeReseau
    ? erreur
    : new ErreurDeReseau(`${contexte} : ${erreur instanceof Error ? erreur.message : String(erreur)}`)

export async function lireLaTaille(adresse: string, recuperer: Recuperer = fetch): Promise<number> {
  let reponse: Response
  try {
    reponse = await recuperer(adresse, { method: 'HEAD', credentials: 'include' })
  } catch (erreur) {
    throw versErreurDeReseau(erreur, 'taille du document')
  }
  const taille = Number(reponse.headers.get('Content-Length'))
  if (reponse.status !== 200) throw new ErreurDeReseau(`taille du document : ${reponse.status}`, reponse.status)
  if (!Number.isInteger(taille) || taille <= 0) throw new ErreurDeReseau('taille du document : longueur absente')
  return taille
}

/** Remplace `requestDataRange` et `abort` du transport donné, et rend son suivi. */
export function brancherLeTransport(
  transport: TransportDePdfjs,
  adresse: string,
  { recuperer = fetch, surChangement }: OptionsDuTransport = {},
): SuiviDesPlages {
  const controleur = new AbortController()
  let demande = 0
  let recu = 0
  let enRoute = 0
  let erreur: ErreurDeReseau | null = null
  let signalerLEchec: (erreur: ErreurDeReseau) => void = () => undefined
  const echec = new Promise<ErreurDeReseau>((resoudre) => (signalerLEchec = resoudre))

  const suivi: SuiviDesPlages = {
    get demande() { return demande },
    get recu() { return recu },
    get enRoute() { return enRoute },
    get progression() { return demande === 0 ? 0 : recu / demande },
    get enAttenteDuReseau() { return enRoute > 0 },
    get erreur() { return erreur },
    echec,
  }
  const signaler = () => surChangement?.(suivi)

  const echouer = (nouvelle: ErreurDeReseau) => {
    if (erreur || controleur.signal.aborted) return
    erreur = nouvelle
    signalerLEchec(nouvelle)
  }

  async function lireLaPlage(debut: number, fin: number) {
    let octets: Uint8Array
    try {
      const reponse = await recuperer(adresse, {
        headers: { Range: `bytes=${debut}-${fin - 1}` },
        credentials: 'include',
        signal: controleur.signal,
      })
      if (reponse.status !== 206) {
        throw new ErreurDeReseau(`plage ${debut}-${fin - 1} : ${reponse.status}`, reponse.status)
      }
      octets = new Uint8Array(await reponse.arrayBuffer())
      if (octets.byteLength !== fin - debut) {
        throw new ErreurDeReseau(`plage ${debut}-${fin - 1} : ${octets.byteLength} octets reçus`, reponse.status)
      }
    } catch (cause) {
      enRoute--
      echouer(versErreurDeReseau(cause, `plage ${debut}-${fin - 1}`))
      signaler()
      return
    }
    enRoute--
    recu += octets.byteLength
    signaler()
    // Hors du `try` : une erreur de pdf.js n'est pas une erreur de réseau.
    transport.onDataRange(debut, octets)
  }

  transport.requestDataRange = (debut: number, fin: number) => {
    if (erreur || controleur.signal.aborted) return
    demande += fin - debut
    enRoute++
    signaler()
    void lireLaPlage(debut, fin)
  }
  transport.abort = () => controleur.abort()

  return suivi
}
