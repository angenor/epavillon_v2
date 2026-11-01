<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'

/**
 * L'APPEL À PROPOSITIONS DE L'ÉDITION DU PAVILLON : `id="appel-a-propositions"`.
 *
 * L'ANCRE EXISTE TOUJOURS : le pied de page de chaque écran (`layouts/public.vue`)
 * y renvoie. Sans appel, la section se réduit à une phrase plutôt que de
 * disparaître.
 *
 * L'édition vient des données (`HomeScreen.currentEdition`), jamais d'une
 * constante. L'échéance affichée est l'échéance EFFECTIVE (`call_deadline`,
 * prolongation comprise) : la seule qu'on ait le droit d'annoncer.
 *
 * Aplat institutionnel à gauche, qui ne s'inverse pas en thème sombre ; la
 * couverture de l'édition à droite s'y fond par `.fade-inverse-start` (dégradé
 * arbitré le 16/09). Sur téléphone, la photographie passe au-dessus du texte.
 * Sans image, le libellé de l'édition en filigrane : aucun visuel inventé.
 *
 * Sur fond sombre, l'urgence des dernières 48 heures passe par la pastille
 * (aplat opaque jaune) : `--color-warning` en texte y serait illisible.
 */

interface Props {
  /** `HomeScreen.currentEdition` : `null` quand aucune édition ne tient de pavillon. */
  edition: PublicEditionRow | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeWithZone, dateRange } = useDateTime()
const localePath = useLocalePath()

/** L'édition porte-t-elle réellement un appel ? La ligne peut exister sans. */
const hasCall = computed(() => Boolean(props.edition?.call_id))

const deadline = computed(() => props.edition?.call_deadline ?? null)
const countdown = useCountdown(deadline)

const isOpen = computed(() => Boolean(props.edition?.call_is_open))
const isUrgent = computed(() => isOpen.value && Boolean(countdown.value?.imminent))

const deadlineDay = computed(() => {
  const edition = props.edition
  return edition?.call_deadline ? date(edition.call_deadline, edition.timezone) : ''
})

const deadlineTime = computed(() => {
  const edition = props.edition
  return edition?.call_deadline
    ? timeWithZone(edition.call_deadline, edition.timezone, edition.city ?? undefined)
    : ''
})

const place = computed(() => {
  const edition = props.edition
  if (!edition) return ''
  return [edition.city, tr(edition.country_name)].filter(Boolean).join(', ')
})

const dates = computed(() =>
  props.edition ? dateRange(props.edition.starts_at, props.edition.ends_at, props.edition.timezone) : '',
)

const picture = computed(() => props.edition?.cover ?? props.edition?.banner ?? null)

const stamp = computed(() =>
  props.edition ? (props.edition.edition_label ?? String(props.edition.edition_year)) : '',
)

/** Le rebours n'a de sens qu'ouvert. Absent du rendu serveur, il se remplit à l'hydratation. */
const showCountdown = computed(() => isOpen.value && Boolean(countdown.value) && !countdown.value?.expired)

const editionPath = computed(() =>
  props.edition ? localePath(`/evenements/${props.edition.slug}`) : localePath('/'),
)

const programmePath = computed(() =>
  props.edition ? localePath(`/programmations?edition=${props.edition.slug}`) : localePath('/programmations'),
)

const factLabel = 'block text-xs font-semibold tracking-[0.06em] text-text-on-inverse-muted uppercase'
const factValue = 'mt-1 block font-condensed text-[1.6rem] leading-[1.05] font-bold text-text-on-inverse uppercase'
</script>

<template>
  <section id="appel-a-propositions" class="scroll-mt-24" aria-labelledby="appel-titre">
    <div
      v-if="!props.edition || !hasCall"
      class="rounded-xl border border-border bg-surface-sunken px-5 py-6 sm:px-7"
    >
      <h2 id="appel-titre" class="heading-condensed text-3xl">{{ t('home.call.none.title') }}</h2>
      <p class="mt-2 max-w-(--measure) text-text-secondary">
        {{ t('home.call.none.description') }}
      </p>
      <UiButton
        v-if="props.edition"
        class="mt-4"
        variant="secondary"
        :to="editionPath"
        icon-trailing="arrow-right"
        :label="t('home.call.none.action')"
      />
    </div>

    <article
      v-else
      data-bird-perch
      class="relative isolate grid overflow-hidden rounded-xl bg-surface-inverse text-text-on-inverse lg:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)]"
    >
      <div class="relative min-h-52 lg:order-2 lg:min-h-0" aria-hidden="true">
        <template v-if="picture">
          <UiImage
            :image="picture"
            ratio="auto"
            frame-class="size-full"
            class="absolute inset-0"
            sizes="(min-width: 1024px) 36rem, 100vw"
          />
          <div class="scrim-fade-bottom absolute inset-0 lg:hidden" />
          <div class="fade-inverse-start absolute inset-y-0 left-0 hidden w-4/5 lg:block" />
        </template>
        <div v-else class="flex size-full items-center justify-end overflow-hidden pr-8">
          <span class="font-condensed text-8xl leading-none font-bold whitespace-nowrap text-text-on-inverse/10 sm:text-9xl">
            {{ stamp }}
          </span>
        </div>
      </div>

      <div class="relative px-5 py-7 sm:px-8 lg:py-11 lg:ps-11 lg:pe-6">
        <div class="mb-4 flex flex-wrap items-center gap-2.5">
          <UiBadge :intent="isOpen ? (isUrgent ? 'warning' : 'info') : 'neutral'" size="sm" solid>
            {{ t(isOpen ? 'home.call.state.open' : 'home.call.state.closed') }}
          </UiBadge>
          <span
            v-if="props.edition.series_name"
            class="font-condensed text-base font-semibold tracking-[0.06em] text-text-on-inverse-muted uppercase"
          >
            {{ tr(props.edition.series_name) }}
          </span>
        </div>

        <h2 id="appel-titre" class="heading-condensed text-[clamp(2.2rem,4.2vw,3.4rem)] text-text-on-inverse">
          {{ t('home.call.title') }}
          <span class="block text-accent-on-inverse">{{ props.edition.edition_label ?? tr(props.edition.title) }}</span>
        </h2>
        <p class="mt-3.5 line-clamp-4 max-w-[54ch] text-text-on-inverse-muted">
          {{ richTextToPlain(tr(props.edition.description)) }}
        </p>

        <dl class="mt-6 flex flex-wrap gap-x-10 gap-y-4">
          <div v-if="deadlineDay">
            <dt :class="factLabel">{{ t(isOpen ? 'home.call.deadline' : 'home.call.closedSince') }}</dt>
            <dd class="m-0">
              <span :class="factValue">{{ deadlineDay }}</span>
              <span class="text-[0.8rem] text-text-on-inverse-muted">{{ deadlineTime }}</span>
            </dd>
          </div>
          <div v-if="place">
            <dt :class="factLabel">{{ t('home.call.place') }}</dt>
            <dd class="m-0">
              <span :class="factValue">{{ place }}</span>
              <span class="text-[0.8rem] text-text-on-inverse-muted">{{ dates }}</span>
            </dd>
          </div>
          <div v-if="showCountdown && countdown">
            <dt :class="factLabel">{{ t('home.call.remaining') }}</dt>
            <dd class="m-0">
              <span :class="factValue" class="tabular-nums">
                <template v-if="countdown.days > 0">
                  {{ t('home.call.countdown.days', { count: countdown.days }, countdown.days) }}
                </template>
                <template v-else-if="countdown.hours > 0">
                  {{ t('home.call.countdown.hours', { count: countdown.hours }, countdown.hours) }}
                </template>
                <template v-else>
                  {{ t('home.call.countdown.minutes', { count: countdown.minutes }, countdown.minutes) }}
                </template>
              </span>
            </dd>
          </div>
        </dl>

        <div class="mt-7 flex flex-wrap gap-3">
          <template v-if="isOpen">
            <UiButton
              variant="on-inverse"
              :to="localePath('/deposer-une-proposition')"
              icon-trailing="arrow-right"
              :label="t('home.call.action.submit')"
              :data-bird-say="t('home.bird.submit')"
            />
            <UiButton
              variant="inverse"
              class="border-text-on-inverse!"
              :to="editionPath"
              :label="t('home.call.action.edition')"
              :data-bird-say="t('home.bird.edition')"
            />
          </template>
          <template v-else>
            <UiButton
              variant="on-inverse"
              :to="editionPath"
              :label="t('home.call.action.edition')"
              :data-bird-say="t('home.bird.edition')"
            />
            <UiButton
              variant="inverse"
              class="border-text-on-inverse!"
              :to="programmePath"
              :label="t('home.call.action.programme')"
            />
          </template>
        </div>
      </div>
    </article>
  </section>
</template>
