/**
 * L'appareil, tel que Guide Négo le déclare à l'ouverture d'une session.
 *
 * **Rien ici n'accorde de droit.** L'identifiant est engendré par le téléphone
 * et se forge comme un `user-agent` : il sert à nommer une session dans une
 * liste — « Android · Chrome, depuis le 3 novembre » — et à compter les
 * téléphones sans dédoubler personne. L'API ne l'autorise sur rien, et le
 * compteur d'essais de code se tient par personne, pas par appareil.
 *
 * **Gardé en `localStorage`, jamais dans un cookie** : un cookie partirait à
 * chaque requête du site, et le garde-fou de Guide Négo interdit de nommer un
 * cookie du site dans ses fichiers. La clé porte le préfixe `gn.` comme les
 * autres.
 */
// L'extension est écrite, et c'est délibéré : `node --test` résout les modules
// lui-même, sans le résolveur de Vite, et une importation de VALEUR sans
// extension ne s'y trouve pas. Les autres fichiers de ce dossier n'importent que
// des TYPES, effacés à la compilation — le défaut ne s'y voyait pas.
import { lireCle, poserCle } from './stockage.ts'

export const CLE_APPAREIL = 'gn.appareil'

export type PlateformeDeclaree = 'android' | 'ios' | 'other'

export interface AppareilDeclare {
  kind: 'app'
  device_id: string
  label: string
  platform: PlateformeDeclaree
}

/**
 * Un identifiant d'installation, engendré une seule fois.
 *
 * `crypto.randomUUID` n'existe pas hors origine sécurisée — un appareil branché
 * en HTTP sur une adresse locale, le cas du développement. Le repli tire du
 * hasard par `getRandomValues`, et à défaut par l'horloge : un identifiant moins
 * bon reste préférable à une session qui ne s'ouvre pas, puisqu'il n'autorise
 * rien.
 */
function engendrerIdentifiant(): string {
  const hasard = globalThis.crypto
  if (hasard?.randomUUID) return hasard.randomUUID()

  if (hasard?.getRandomValues) {
    const octets = hasard.getRandomValues(new Uint8Array(16))
    return Array.from(octets, (o) => o.toString(16).padStart(2, '0')).join('')
  }

  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

/** L'identifiant de cette installation, posé à la première demande. */
export function identifiantDAppareil(): string {
  const garde = lireCle(CLE_APPAREIL)
  if (garde) return garde

  const neuf = engendrerIdentifiant()
  poserCle(CLE_APPAREIL, neuf)
  return neuf
}

/**
 * La plateforme déclarée. Trois valeurs seulement — c'est ce que la base
 * accepte —, et `other` n'est pas un échec : un ordinateur de bureau qui ouvre
 * l'application est un cas ordinaire pendant le développement.
 */
export function plateformeDe(agent: string): PlateformeDeclaree {
  const minuscules = agent.toLowerCase()
  if (/android/.test(minuscules)) return 'android'
  // iPadOS s'annonce « Macintosh » depuis iOS 13 : le tactile le distingue d'un
  // Mac, et sans ce test tous les iPad seraient comptés « other ».
  if (/iphone|ipad|ipod/.test(minuscules)) return 'ios'
  if (/macintosh/.test(minuscules) && navigatorTactile()) return 'ios'
  return 'other'
}

function navigatorTactile(): boolean {
  return typeof navigator !== 'undefined' && (navigator.maxTouchPoints ?? 0) > 1
}

/**
 * Ce que la personne lit : « Android · Chrome ». Composé côté client parce que
 * lui seul sait ce qu'il est ; l'API ne fait que le ranger.
 *
 * Le navigateur se déduit dans un ordre qui compte : Edge et Opera annoncent
 * tous les deux « Chrome », Chrome annonce « Safari ». Tester Safari en premier
 * nommerait Chrome « Safari » sur tous les Android.
 */
export function libelleDAppareil(agent: string): string {
  return `${nomDePlateforme(plateformeDe(agent))} · ${nomDeNavigateur(agent)}`
}

function nomDePlateforme(plateforme: PlateformeDeclaree): string {
  if (plateforme === 'android') return 'Android'
  if (plateforme === 'ios') return 'iPhone'
  return 'Ordinateur'
}

function nomDeNavigateur(agent: string): string {
  if (/Edg\//.test(agent)) return 'Edge'
  if (/OPR\/|Opera/.test(agent)) return 'Opera'
  if (/SamsungBrowser/.test(agent)) return 'Samsung Internet'
  if (/Firefox\/|FxiOS/.test(agent)) return 'Firefox'
  if (/CriOS|Chrome\//.test(agent)) return 'Chrome'
  if (/Safari\//.test(agent)) return 'Safari'
  return 'Navigateur'
}

/**
 * L'objet `client` à joindre à une connexion ou à une inscription. **Facultatif
 * côté API** : son absence vaut « le site », et c'est pourquoi Guide Négo est le
 * seul à le poser.
 */
export function appareilDeclare(): AppareilDeclare {
  const agent = typeof navigator === 'undefined' ? '' : navigator.userAgent
  return {
    kind: 'app',
    device_id: identifiantDAppareil(),
    label: libelleDAppareil(agent),
    platform: plateformeDe(agent),
  }
}
