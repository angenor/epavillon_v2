/**
 * Les thématiques de négociation — **deux lectures, une écriture, et la file**.
 *
 * Le vocabulaire se lit par la route publique des termes, sans session, et se
 * garde comme le drapeau : à la première ouverture en ligne, depuis la mise en
 * page. La ligne « Mes thématiques » du profil a ainsi toujours ses noms, même
 * sur un appareil qui n'a jamais ouvert l'écran des thématiques.
 *
 * **L'écriture passe par la file, toujours** (FR-009). Même avec du réseau : la
 * file part aussitôt, et le chemin unique est ce qui fait qu'un choix pris dans
 * un tunnel se comporte comme les autres. L'empreinte de l'état lu part en
 * `If-Match` ; un choix qui arrive trop tard reçoit `412`, s'abandonne, et
 * l'état vrai est relu.
 *
 * L'inscription à la file se fait **ici, au premier appel du composable**, donc
 * dès l'ouverture depuis la mise en page : une intention gardée d'hier repart
 * sans que personne n'ouvre l'écran des thématiques.
 */
import type { ThemesPayload } from '~/types/negotiation'
import { ecrireGarde } from '~/utils/guide-nego/garde'
import {
  CLE_FILE_THEMATIQUES,
  codesVoulus,
  etatDesThematiques,
  libelleDe,
  recapitulatif,
  thematiquesGardees,
  VOCABULAIRE_THEMATIQUES,
  type EtatDesThematiques,
  type ThematiqueGardee,
} from '~/utils/guide-nego/thematiques'

const CLE_LECTURE_SUIVIS = 'mes-thematiques'

export function useGnThematiques() {
  const api = useApi()
  const { locale } = useI18n()
  const file = useGnFile()

  const vocabulaire = useGnLecture<ThematiqueGardee[]>('thematiques', async () =>
    thematiquesGardees(await api.reference.terms(VOCABULAIRE_THEMATIQUES)),
  )

  const suivis = useGnLecture<EtatDesThematiques>(CLE_LECTURE_SUIVIS, async () => {
    const lu = await api.guideNego.mesThematiques()
    return etatDesThematiques(lu.valeur, lu.empreinte)
  })

  const thematiques = computed<ThematiqueGardee[]>(() => vocabulaire.etat.value.valeur ?? [])
  const vocabulairePret = computed(() => vocabulaire.etat.value.pret)
  const vocabulaireLuA = computed(() => vocabulaire.etat.value.luA)
  /**
   * Le vocabulaire a-t-il été lu **une fois** ? C'est ce qui sépare les deux
   * états que l'écran ne doit pas confondre : une liste vide parce que l'IFDD
   * n'a rien publié, et une liste vide parce que la lecture a échoué.
   */
  const vocabulaireLu = computed(() => vocabulaire.etat.value.source !== 'aucune')

  const mesCodes = computed<string[]>(() => suivis.etat.value.valeur?.codes ?? [])
  const empreinte = computed<string | null>(() => suivis.etat.value.valeur?.empreinte ?? null)
  const pret = computed(() => suivis.etat.value.pret)
  const luA = computed(() => suivis.etat.value.luA)

  /** Le nom d'un code, dans la langue de la personne ; nul si le vocabulaire ne le porte pas. */
  function nomDe(code: string): string | null {
    const trouvee = thematiques.value.find((t) => t.code === code)
    return trouvee ? libelleDe(trouvee, locale.value) : null
  }

  /** Combien, et lesquelles — le récapitulatif du pied comme la ligne du profil. */
  function resumeDe(codes: string[]) {
    return recapitulatif(codes, thematiques.value, locale.value)
  }

  /** Lit le vocabulaire s'il ne l'a jamais été ; le relit sinon, sans attendre. */
  async function assurerLeVocabulaire(): Promise<void> {
    if (!vocabulairePret.value) await vocabulaire.rafraichir()
    else void vocabulaire.rafraichir()
  }

  /** Les deux lectures dont l'écran a besoin, en parallèle : elles ne se doivent rien. */
  async function assurer(): Promise<void> {
    await Promise.all([assurerLeVocabulaire(), pret.value ? Promise.resolve() : suivis.rafraichir()])
  }

  /**
   * Applique le choix **tout de suite**, sans attendre le réseau : sans cela,
   * les cases reviendraient en arrière sous les doigts de la personne. L'heure
   * de lecture et l'empreinte ne bougent pas — elles disent l'état du serveur,
   * pas ce qu'on veut lui écrire.
   */
  async function appliquer(codes: string[]): Promise<void> {
    const etat = suivis.etat.value
    const valeur: EtatDesThematiques = { codes, empreinte: etat.valeur?.empreinte ?? null }
    suivis.etat.value = { ...etat, valeur, pret: true }
    await ecrireGarde({
      cle: CLE_LECTURE_SUIVIS,
      valeur,
      lu_a: etat.luA ?? new Date().toISOString(),
      empreinte: valeur.empreinte,
    })
  }

  file.inscrire(
    CLE_FILE_THEMATIQUES,
    (intention) => {
      const corps = intention.corps as ThemesPayload
      return api.guideNego.suivreDesThematiques(corps.codes, intention.empreinte)
    },
    () => suivis.rafraichir(),
  )

  /**
   * Enregistrer le choix. Rien ne lève : ce qui n'a pas pu partir attend dans
   * la file, et ce qui a été refusé passe par l'avis de file — le message de
   * l'API, tel quel.
   */
  async function enregistrer(codes: Iterable<string>): Promise<void> {
    const voulus = codesVoulus(codes)
    await file.poser(CLE_FILE_THEMATIQUES, { codes: voulus } satisfies ThemesPayload, empreinte.value)
    await appliquer(voulus)
    await file.partir()
  }

  return {
    thematiques,
    vocabulairePret,
    vocabulaireLuA,
    vocabulaireLu,
    mesCodes,
    empreinte,
    pret,
    luA,
    nomDe,
    resumeDe,
    assurer,
    assurerLeVocabulaire,
    enregistrer,
    rafraichir: suivis.rafraichir,
    rafraichirLeVocabulaire: vocabulaire.rafraichir,
  }
}
