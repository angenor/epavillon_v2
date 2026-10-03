import type { MaybeRefOrGetter } from 'vue'
import type { PublicEditionRow } from '~/types/views'

/** Ce qu'affichent l'affiche et la ligne d'une édition — une seule source pour les deux vues. */
export function useEditionSummary(source: MaybeRefOrGetter<PublicEditionRow>) {
  const { tr } = useI18nText()
  const { dateRange, zoneLabel } = useDateTime()
  const localePath = useLocalePath()

  const edition = computed(() => toValue(source))

  const to = computed(() => localePath(`/evenements/${edition.value.slug}`))

  const image = computed(() => edition.value.cover ?? edition.value.banner)

  const dates = computed(() =>
    dateRange(edition.value.starts_at, edition.value.ends_at, edition.value.timezone),
  )

  const zone = computed(() => zoneLabel(edition.value.timezone, edition.value.city ?? undefined))

  const place = computed(() =>
    [edition.value.city, tr(edition.value.country_name)].filter(Boolean).join(', '),
  )

  /** Le repli sans image : aucun visuel inventé, le millésime suffit à repérer l'édition. */
  const stamp = computed(() => edition.value.edition_label ?? String(edition.value.edition_year))

  return { to, image, dates, zone, place, stamp }
}
