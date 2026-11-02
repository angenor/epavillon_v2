<script setup lang="ts">
import type { EditionStatsRow, PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { EventId, IsoDateTime } from '~/types/shared'
import type { AsideFastening } from './AsideSession.vue'

/**
 * LE PANNEAU « À VENIR AU PAVILLON », à droite de la vitrine (maquette du 01/11).
 *
 * Automatique (arbitrage du 24/08) : rien ne s'y compose au back-office. Il
 * montre trois lignes au plus : les prochaines activités retenues, complétées
 * par les prochaines éditions dont le programme n'y figure pas encore
 * (arbitrage du 03/10) ; puis les chiffres clés de l'édition et le lien vers la
 * programmation. Au-delà de trois lignes, le panneau dépassait l'écran et
 * poussait les vignettes sous le pli. Rien n'est re-filtré : l'API sert
 * des séances déjà réduites aux premières à venir ou en cours.
 *
 * Sur la photographie, du verre ; sans photographie (téléphone, vitrine vide),
 * l'aplat institutionnel : le verre ne se pose que sur un média. Le texte est
 * clair dans les deux cas.
 */

/** La maquette en montre trois ; la suite est dans la programmation. */
const SHOWN_SESSIONS = 3

interface Props {
  /** Les prochaines séances, déjà choisies par l'API. */
  sessions: PublicScheduleRow[]
  /** L'historique COMPLET : le panneau n'en montre que les prochaines. */
  editions: PublicEditionRow[]
  /** `programme.v_edition_stats`, indexée par édition. Absent vaut zéro. */
  stats: Record<EventId, EditionStatsRow>
  /** L'édition du pavillon (`HomeScreen.currentEdition`) : ses chiffres à défaut d'une édition commune. */
  edition: PublicEditionRow | null
  /** Instant de composition de la réponse : l'horloge qui fait autorité. */
  now: IsoDateTime
  /** Le panneau flotte sur la photographie de la vitrine (grand écran seulement). */
  onMedia?: boolean
}

const props = withDefaults(defineProps<Props>(), { onMedia: false })

const { t } = useI18n()
const { tr } = useI18nText()
const { zoneLabel } = useDateTime()
const localePath = useLocalePath()

const shown = computed(() => props.sessions.slice(0, SHOWN_SESSIONS))

const upcomingEditions = computed(() =>
  nextEditions(
    props.editions.filter(
      (edition) =>
        !edition.programme_published_at || !nextSessionOfEdition(props.sessions, edition.id),
    ),
    SHOWN_SESSIONS - shown.value.length,
  ),
)

/** L'édition commune aux séances montrées : elle nomme le panneau, et les lignes n'ont pas à la répéter. */
const singleEdition = computed(() => {
  const eventId = commonEditionId(shown.value)
  if (!eventId) return null
  return props.editions.find((edition) => edition.id === eventId) ?? null
})

/** « CdP31, Antalya, heure d'Antalya » */
const subtitle = computed(() => {
  const edition = singleEdition.value
  if (!edition) return ''
  return [
    edition.edition_label ?? edition.acronym ?? tr(edition.title),
    edition.city,
    zoneLabel(edition.timezone, edition.city ?? undefined),
  ]
    .filter(Boolean)
    .join(', ')
})

const statsEdition = computed(() => singleEdition.value ?? props.edition)

/** Le logo de la conférence, en face du titre ; sur une tuile blanche, il garde ses couleurs sur l'aplat sombre. */
const logo = computed(() => {
  const asset = statsEdition.value?.logo
  return asset ? { src: asset.sources.thumb?.url ?? asset.url, alt: tr(asset.alt_text) } : null
})

/**
 * Ruban un jour, épingle le lendemain (demande du 02/11). Le jour se compte dans
 * le fuseau de l'édition, d'après l'horloge de l'API : serveur et navigateur
 * tombent d'accord.
 */
const fastening = computed<AsideFastening>(() => {
  const timeZone = statsEdition.value?.timezone ?? 'UTC'
  const ymd = new Intl.DateTimeFormat('en-CA', { timeZone, year: 'numeric', month: '2-digit', day: '2-digit' }).format(
    new Date(props.now),
  )
  return Math.floor(Date.parse(`${ymd}T00:00:00Z`) / 86_400_000) % 2 === 1 ? 'tape' : 'pin'
})

/** Zéro ne s'affiche pas : une édition sans programme n'a pas à annoncer « 0 activité ». */
const figures = computed(() => {
  const edition = statsEdition.value
  const row = edition ? props.stats[edition.id] : undefined
  if (!row) return []
  return [
    { key: 'sessions', value: row.published_session_count },
    { key: 'organizations', value: row.organization_count },
    { key: 'countries', value: row.country_count },
  ].filter((figure) => figure.value > 0)
})

const programmeTo = computed(() => {
  const edition = statsEdition.value
  return edition ? localePath(`/programmations?edition=${edition.slug}`) : localePath('/programmations')
})

const isEmpty = computed(() => shown.value.length === 0 && upcomingEditions.value.length === 0)
</script>

<template>
  <aside
    class="flex flex-col rounded-xl border p-5 text-text-on-inverse sm:p-6"
    :class="
      props.onMedia
        ? 'border-border-on-inverse bg-surface-inverse-raised lg:border-glass-border lg:bg-glass lg:shadow-glass lg:backdrop-blur-glass-strong'
        : 'border-border-on-inverse bg-surface-inverse-raised'
    "
    aria-labelledby="a-venir-titre"
  >
    <div class="flex items-start gap-3">
      <div class="min-w-0 flex-1">
        <!-- La couleur est explicite : `main.css` peint les titres en `--color-heading`, invisible ici. -->
        <h2
          id="a-venir-titre"
          class="heading-condensed text-[1.6rem] text-text-on-inverse"
          :data-bird-say="t('home.bird.aside')"
        >
          {{ t('home.aside.title') }}
        </h2>
        <p v-if="subtitle" class="mt-0.5 text-sm text-text-on-inverse-muted">{{ subtitle }}</p>
      </div>
      <span
        v-if="logo"
        class="flex h-12 max-w-[26%] shrink-0 items-center justify-center rounded-lg bg-surface-inverse-selected p-1.5 sm:max-w-30"
      >
        <img :src="logo.src" :alt="logo.alt" class="max-h-9 max-w-full object-contain">
      </span>
    </div>

    <UiEmptyState
      v-if="isEmpty"
      class="mt-4"
      compact
      icon="calendar"
      :title="t('home.aside.empty.title')"
      :description="t('home.aside.empty.description')"
    />

    <HomeAsideTimeline
      v-if="shown.length"
      class="mt-3"
      :sessions="shown"
      :editions="props.editions"
      :single-edition="singleEdition"
      :fastening="fastening"
    />

    <section
      v-if="upcomingEditions.length"
      class="mt-3 border-t pt-3"
      :class="props.onMedia ? 'border-border-on-inverse lg:border-glass-border' : 'border-border-on-inverse'"
    >
      <h3 class="font-condensed text-base font-semibold tracking-[0.05em] text-text-on-inverse-muted uppercase">
        {{ t('home.aside.editions.title') }}
      </h3>
      <ul class="mt-1">
        <li v-for="edition in upcomingEditions" :key="edition.id">
          <HomeAsideEdition
            :edition="edition"
            :next-session="nextSessionOfEdition(props.sessions, edition.id)"
            :now="props.now"
          />
        </li>
      </ul>
    </section>

    <dl
      v-if="figures.length"
      class="mt-3 flex flex-wrap gap-x-6 gap-y-2 border-t pt-3.5"
      :class="props.onMedia ? 'border-border-on-inverse lg:border-glass-border' : 'border-border-on-inverse'"
    >
      <div v-for="figure in figures" :key="figure.key" class="flex flex-col-reverse">
        <dt class="text-xs font-semibold tracking-[0.06em] text-text-on-inverse-muted uppercase">
          {{ t(`home.aside.figures.${figure.key}`, figure.value) }}
        </dt>
        <dd class="font-condensed text-[1.8rem] leading-none font-bold text-text-on-inverse tabular-nums">
          {{ figure.value }}
        </dd>
      </div>
    </dl>

    <!-- Même habit que « S'inscrire » sur une activité : c'est l'action qui mène au programme. -->
    <UiButton
      v-if="props.sessions.length || figures.length"
      variant="signup"
      block
      class="mt-4"
      :to="programmeTo"
      :data-bird-say="t('home.bird.programme')"
    >
      {{ t('home.aside.programme.cta') }}
    </UiButton>
  </aside>
</template>
