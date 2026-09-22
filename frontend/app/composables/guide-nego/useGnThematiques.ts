/**
 * Les thématiques de négociation — **le vocabulaire, et ce que la personne suit**.
 *
 * Le vocabulaire se lit par la route publique des termes, sans session, et se garde
 * comme le drapeau : à la première ouverture en ligne, depuis la mise en page. La
 * ligne « Mes thématiques » du profil a ainsi toujours ses noms, même sur un appareil
 * qui n'a jamais ouvert l'écran des thématiques.
 *
 * **Les libellés viennent d'ici et de nulle part ailleurs** : le corps des suivis ne
 * porte que des codes. Le libellé se résout dans la langue de la personne, avec repli
 * sur le français (FR-002).
 */
import type { TaxonomyTerm } from '~/types/reference'
import type { I18nText } from '~/types/shared'

export const VOCABULAIRE_THEMATIQUES = 'negotiation_theme'

/** Ce qui se garde d'un terme : le code et ses libellés, rien d'administratif. */
export interface ThematiqueGardee {
  code: string
  label: I18nText
  sort_order: number
}

export function thematiquesGardees(termes: TaxonomyTerm[]): ThematiqueGardee[] {
  return termes
    .filter((t) => t.is_active)
    .sort((a, b) => a.sort_order - b.sort_order || a.code.localeCompare(b.code))
    .map((t) => ({ code: t.code, label: { ...t.label }, sort_order: t.sort_order }))
}

/** Le libellé dans la langue demandée, en français à défaut, le code en dernier recours. */
export function libelleDe(thematique: ThematiqueGardee, locale: string): string {
  return thematique.label[locale] ?? thematique.label.fr
}

export function useGnThematiques() {
  const api = useApi()
  const { locale } = useI18n()

  const vocabulaire = useGnLecture<ThematiqueGardee[]>('thematiques', async () =>
    thematiquesGardees(await api.reference.terms(VOCABULAIRE_THEMATIQUES)),
  )

  const thematiques = computed<ThematiqueGardee[]>(() => vocabulaire.etat.value.valeur ?? [])
  const vocabulairePret = computed(() => vocabulaire.etat.value.pret)
  const vocabulaireLuA = computed(() => vocabulaire.etat.value.luA)

  /** Le nom d'un code, dans la langue de la personne ; nul si le vocabulaire ne le porte pas. */
  function nomDe(code: string): string | null {
    const trouvee = thematiques.value.find((t) => t.code === code)
    return trouvee ? libelleDe(trouvee, locale.value) : null
  }

  /** Lit le vocabulaire s'il ne l'a jamais été ; le relit sinon, sans attendre. */
  async function assurerLeVocabulaire(): Promise<void> {
    if (!vocabulairePret.value) await vocabulaire.rafraichir()
    else void vocabulaire.rafraichir()
  }

  return {
    thematiques,
    vocabulairePret,
    vocabulaireLuA,
    nomDe,
    assurerLeVocabulaire,
    rafraichirLeVocabulaire: vocabulaire.rafraichir,
  }
}
