/**
 * Les groupes de négociation de la personne (FR-006a) — le patron exact de
 * `useGnThematiques` : vocabulaire lu par la route publique des termes, choix
 * gardé sous `mes-groupes`, écriture **toujours** par la file avec l'empreinte en
 * `If-Match` ; un `412` s'abandonne et l'état vrai est relu.
 *
 * Aucun groupe coché est un choix valable : toutes les coordinations passent alors.
 */
import type { GroupsPayload } from '~/types/negotiation-sessions'
import { ecrireGarde } from '~/utils/guide-nego/garde'
import {
  codesVoulus,
  libelleDe,
  thematiquesGardees,
  type EtatDesThematiques,
  type ThematiqueGardee,
} from '~/utils/guide-nego/thematiques'

export const VOCABULAIRE_GROUPES = 'negotiation_group'
const CLE = 'mes-groupes'
/** La clé de file de l'écran des thématiques, qui lit ses avis. */
export const CLE_FILE_GROUPES = CLE

/** Le même état que les thématiques : des codes triés, et l'empreinte lue. */
export type EtatDesGroupes = EtatDesThematiques
export type GroupeGarde = ThematiqueGardee

export function useGnGroupes() {
  const api = useApi()
  const { locale } = useI18n()
  const file = useGnFile()
  const session = useGnSession()

  const vocabulaire = useGnLecture<GroupeGarde[]>('groupes', async () =>
    thematiquesGardees(await api.reference.terms(VOCABULAIRE_GROUPES)),
  )

  const suivis = useGnLecture<EtatDesGroupes>(CLE, async () => {
    const lu = await api.negotiationSessions.mesGroupes()
    return { codes: codesVoulus(lu.valeur.groups), empreinte: lu.empreinte ?? lu.valeur.etag }
  })

  const groupes = computed<GroupeGarde[]>(() => vocabulaire.etat.value.valeur ?? [])
  const mesCodes = computed<string[]>(() => suivis.etat.value.valeur?.codes ?? [])
  const empreinte = computed<string | null>(() => suivis.etat.value.valeur?.empreinte ?? null)

  function nomDe(code: string): string | null {
    const trouve = groupes.value.find((g) => g.code === code)
    return trouve ? libelleDe(trouve, locale.value) : null
  }

  async function assurerLeVocabulaire(): Promise<void> {
    if (!vocabulaire.etat.value.pret) await vocabulaire.rafraichir()
    else void vocabulaire.rafraichir()
  }

  // Sans compte, la lecture des choix échouerait, et l'échec passerait pour une panne de réseau.
  async function assurer(): Promise<void> {
    const choix = session.connectee.value && !suivis.etat.value.pret ? suivis.rafraichir() : Promise.resolve()
    await Promise.all([assurerLeVocabulaire(), choix])
  }

  /** Le choix s'affiche tout de suite ; l'heure et l'empreinte restent celles du serveur. */
  async function appliquer(codes: string[]): Promise<void> {
    const etat = suivis.etat.value
    const valeur: EtatDesGroupes = { codes, empreinte: etat.valeur?.empreinte ?? null }
    suivis.etat.value = { ...etat, valeur, pret: true }
    await ecrireGarde({ cle: CLE, valeur, lu_a: etat.luA ?? new Date().toISOString(), empreinte: valeur.empreinte })
  }

  file.inscrire(
    CLE,
    (intention) =>
      api.negotiationSessions.suivreDesGroupes((intention.corps as GroupsPayload).groups, intention.empreinte),
    () => suivis.rafraichir(),
  )

  async function enregistrer(codes: Iterable<string>): Promise<void> {
    const voulus = codesVoulus(codes)
    await file.poser(CLE, { groups: voulus } satisfies GroupsPayload, empreinte.value)
    await appliquer(voulus)
    await file.partir()
  }

  return {
    groupes,
    vocabulairePret: computed(() => vocabulaire.etat.value.pret),
    /** Lu une fois : sinon une liste vide ne dit pas « aucun groupe publié ». */
    vocabulaireLu: computed(() => vocabulaire.etat.value.source !== 'aucune'),
    mesCodes,
    empreinte,
    pret: computed(() => suivis.etat.value.pret),
    luA: computed(() => suivis.etat.value.luA),
    nomDe,
    assurer,
    enregistrer,
    rafraichir: suivis.rafraichir,
  }
}
