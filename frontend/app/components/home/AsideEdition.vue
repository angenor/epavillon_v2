<script setup lang="ts">
import type { IsoDateTime } from '~/types/shared'
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'

/**
 * UN ÉVÉNEMENT À VENIR — le premier bloc du panneau « À venir ».
 *
 * DEUX DESSINS POUR UNE SEULE CARTE. Le prochain rendez-vous est ce que la page
 * d'accueil a de plus utile à dire : il prend une carte pleine, avec son
 * compte à rebours, son lieu et le volume de son programme. Ceux qui suivent
 * tiennent en trois lignes — nom, état, prochaine séance. Deux composants pour
 * cela auraient donné deux dessins à tenir alignés.
 *
 * `edition_label` PLUTÔT QUE `title` EN TÊTE quand il existe : « COP31 » est ce
 * que les gens disent, l'intitulé complet vient après.
 *
 * LE COMPTE À REBOURS SE COMPTE EN JOURNÉES CIVILES, dans le fuseau de
 * l'édition, et non en millisecondes divisées : « dans 3 jours » doit vouloir
 * dire ce que le calendrier dit, pas ce que l'arrondi décide.
 */

interface Props {
  edition: PublicEditionRow
  /** Le prochain rendez-vous, celui qui prend la carte pleine. */
  featured?: boolean
  /** `programme.v_edition_stats` — absent vaut zéro, la ligne s'efface alors. */
  sessionCount?: number
  /** La prochaine séance connue de cette édition, s'il y en a une en main. */
  nextSession?: PublicScheduleRow | null
  /** Instant de composition de la réponse — l'horloge qui fait autorité. */
  now: IsoDateTime
}

const props = withDefaults(defineProps<Props>(), { featured: false, sessionCount: 0 })

const { t } = useI18n()
const { tr } = useI18nText()
const { dateRange, zoneLabel, date, time } = useDateTime()
const localePath = useLocalePath()

const name = computed(() => props.edition.edition_label ?? tr(props.edition.title))

/**
 * L'intitulé complet, DÉBARRASSÉ DU LIBELLÉ QUI LE PRÉCÈDE DÉJÀ.
 *
 * Les titres du modèle s'écrivent « COP31 — Conférence des Nations unies… » :
 * affichés sous « COP31 », ils répètent le nom qu'on vient de lire et volent
 * une ligne dans une colonne de 340 px. On retire le préfixe quand il est
 * exactement le libellé, jamais autrement — un titre qui ne commence pas par
 * son libellé reste intact.
 */
const subtitle = computed(() => {
  const label = props.edition.edition_label
  const title = tr(props.edition.title)
  if (!label || !title.startsWith(label)) return title
  return title.slice(label.length).replace(/^\s*[—–-]\s*/, '') || title
})

const dates = computed(() =>
  dateRange(props.edition.starts_at, props.edition.ends_at, props.edition.timezone),
)

const zone = computed(() => zoneLabel(props.edition.timezone, props.edition.city ?? undefined))

const place = computed(() =>
  [props.edition.city, tr(props.edition.country_name)].filter(Boolean).join(', '),
)

/**
 * LA DURÉE NE SE DIT QUE D'UN PAVILLON. « Douze jours » décrit une COP, qui se
 * tient d'une traite ; appliqué à un cycle de webinaires étalé de février à
 * décembre, le même calcul annonce « 302 jours », ce qui ne veut rien dire.
 * Zéro efface la mention.
 */
const dayCount = computed(() => (props.edition.has_pavilion ? editionDayCount(props.edition) : 0))

/** Le décompte n'a de sens qu'avant l'ouverture ; une édition en cours l'annonce autrement. */
const daysAhead = computed(() =>
  props.edition.temporal_state === 'upcoming'
    ? daysBetweenInZone(props.now, props.edition.starts_at, props.edition.timezone)
    : null,
)

const countdown = computed(() => {
  const days = daysAhead.value
  if (days === null || days < 0) return ''
  if (days === 0) return t('home.aside.editions.startsToday')
  return t('home.aside.editions.inDays', days)
})

const nextSessionLabel = computed(() => {
  const session = props.nextSession
  if (!session) return ''
  return t('home.aside.editions.nextSession', {
    date: date(session.starts_at, session.timezone),
    time: time(session.starts_at, session.timezone),
    // La ville de l'ÉDITION, sans quoi le repli rend « Belem » sans accent.
    zone: zoneLabel(session.timezone, props.edition.city ?? undefined),
  })
})
</script>

<template>
  <!-- CARTE PLEINE — le prochain rendez-vous. -->
  <article
    v-if="props.featured"
    class="rounded-lg border border-glass-border bg-glass-raised p-3.5 shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover"
  >
    <div class="flex items-center justify-between gap-2">
      <UiStatusBadge
        :state="props.edition.temporal_state"
        size="sm"
        :label="t(`home.history.state.${props.edition.temporal_state}`)"
      />
      <span v-if="countdown" class="text-xs text-text-on-inverse-muted">{{ countdown }}</span>
    </div>

    <h4 class="mt-2 font-display text-xl leading-tight font-bold" :style="{ letterSpacing: 'var(--tracking-title)' }">
      <NuxtLink
        :to="localePath(`/evenements/${props.edition.slug}`)"
        class="text-text-on-inverse no-underline hover:underline"
      >
        {{ name }}
      </NuxtLink>
    </h4>
    <p v-if="props.edition.edition_label" class="mt-0.5 text-[0.8125rem] leading-snug text-text-on-inverse">
      {{ subtitle }}
    </p>

    <p class="mt-2 text-xs text-text-on-inverse-muted">{{ dates }} ({{ zone }})</p>
    <p v-if="place" class="text-xs text-text-on-inverse-muted">{{ place }}</p>

    <!-- LE VOLUME DU PROGRAMME, quand il est publié. Absent vaut zéro, et zéro
         ne s'affiche pas : une édition dont le programme n'est pas encore
         arrêté n'a pas à annoncer « 0 activité ». -->
    <div
      v-if="props.sessionCount"
      class="mt-3 flex items-center gap-2 border-t border-glass-border pt-2.5 text-xs text-text-on-inverse"
    >
      <span>{{ t('home.aside.editions.sessions', props.sessionCount) }}</span>
      <template v-if="dayCount">
        <span class="size-[3px] shrink-0 rounded-full bg-glass-border-strong" aria-hidden="true" />
        <span>{{ t('home.aside.editions.days', dayCount) }}</span>
      </template>
    </div>
  </article>

  <!-- LIGNE COMPACTE — les rendez-vous suivants. -->
  <article
    v-else
    class="rounded-lg border border-glass-border bg-glass-raised p-3 shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover"
  >
    <div class="flex items-start justify-between gap-2">
      <h4 class="min-w-0 text-sm leading-snug font-bold">
        <NuxtLink
          :to="localePath(`/evenements/${props.edition.slug}`)"
          class="text-text-on-inverse no-underline hover:underline"
        >
          {{ name }}
        </NuxtLink>
      </h4>
      <UiStatusBadge
        :state="props.edition.temporal_state"
        size="sm"
        :label="t(`home.history.state.${props.edition.temporal_state}`)"
      />
    </div>

    <p class="mt-1 text-xs text-text-on-inverse-muted">
      {{ dates }} <span>({{ zone }})</span>
    </p>
    <p v-if="nextSessionLabel" class="mt-1 text-xs text-text-on-inverse-muted">{{ nextSessionLabel }}</p>
    <p v-else-if="place" class="mt-1 text-xs text-text-on-inverse-muted">{{ place }}</p>
  </article>
</template>
