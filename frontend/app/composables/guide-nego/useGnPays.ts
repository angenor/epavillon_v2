/**
 * Le nom d'un pays, dans la langue de l'écran — **gardé**, pour qu'un profil ouvert
 * sans réseau dise encore « Sénégal » plutôt que rien.
 *
 * Le référentiel se lit par la route publique des pays ; on n'en garde que
 * l'identifiant, le code ISO2 (le pays prérempli d'une inscription au Pavillon) et le nom.
 */
import type { I18nText } from '~/types/shared'

interface PaysGarde {
  id: string
  /** Absent d'une garde d'avant l'étape 5 : elle se relit. */
  iso2?: string
  nom: I18nText
}

export function useGnPays() {
  const api = useApi()
  const { locale } = useI18n()

  const { etat, rafraichir } = useGnLecture<PaysGarde[]>('pays', async () =>
    (await api.reference.countries()).map((pays) => ({ id: pays.id, iso2: pays.iso2, nom: { ...pays.name } })),
  )

  /** Lit le référentiel s'il ne l'a jamais été ; il change rarement, une lecture suffit. */
  async function assurer(): Promise<void> {
    if (!etat.value.pret || etat.value.valeur?.some((p) => !p.iso2)) await rafraichir()
  }

  function nomDuPays(id: string | null): string | null {
    if (!id) return null
    const pays = etat.value.valeur?.find((p) => p.id === id)
    return pays ? (pays.nom[locale.value] ?? pays.nom.fr ?? null) : null
  }

  function iso2DuPays(id: string | null): string | null {
    if (!id) return null
    return etat.value.valeur?.find((p) => p.id === id)?.iso2 ?? null
  }

  return { assurer, nomDuPays, iso2DuPays }
}
