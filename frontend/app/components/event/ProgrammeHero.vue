<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'

/**
 * BANDEAU DE `/programmations` — distinct de celui de la page d'édition (16/09).
 *
 * Il ne présente pas la conférence : il dit quel programme on lit, ce qu'il
 * pèse et de quand il date, et permet d'en ouvrir un autre. D'où une hauteur
 * bornée par ce contenu, et la même composition que la section d'appel de
 * l'accueil — aplat institutionnel, couverture fondue à droite.
 */

interface Props {
  edition: PublicEditionRow
  /** Toutes les éditions publiques ; le sélecteur n'apparaît qu'à partir de deux. */
  editions: PublicEditionRow[]
}

const props = defineProps<Props>()
const emit = defineEmits<{ select: [eventId: string] }>()

const { t, n } = useI18n()
const { tr } = useI18nText()
const { dateRange, dateTime, zoneLabel } = useDateTime()

const picture = computed(() => props.edition.cover ?? props.edition.banner ?? null)

const label = (edition: PublicEditionRow) =>
  edition.acronym ?? edition.edition_label ?? tr(edition.title)

const title = computed(() =>
  t(props.edition.has_pavilion ? 'programme.hero.titlePavilion' : 'programme.hero.title', {
    edition: label(props.edition),
  }),
)

const zone = computed(() => zoneLabel(props.edition.timezone, props.edition.city ?? undefined))
const period = computed(() =>
  dateRange(props.edition.starts_at, props.edition.ends_at, props.edition.timezone),
)
const place = computed(() =>
  [props.edition.city, tr(props.edition.country_name)].filter(Boolean).join(', '),
)

const isPublished = computed(() => props.edition.programme_published_at !== null)

const figures = computed(() => [
  { key: 'activities', value: props.edition.published_session_count },
  { key: 'organizations', value: props.edition.organization_count },
  { key: 'countries', value: props.edition.country_count },
])

const updatedAt = computed(() =>
  props.edition.programme_updated_at
    ? dateTime(props.edition.programme_updated_at, props.edition.timezone)
    : '',
)

const choices = computed(() =>
  [...props.editions].sort((a, b) => b.starts_at.localeCompare(a.starts_at)),
)
</script>

<template>
  <!-- `-mt-8 sm:-mt-10` annule le rembourrage haut du `<main>` : le bandeau
       touche le menu, et doit donc rester le premier enfant de la page. -->
  <header
    class="full-bleed relative isolate -mt-8 overflow-hidden bg-surface-inverse text-text-on-inverse sm:-mt-10"
  >
    <div v-if="picture" class="absolute inset-0 -z-10 lg:left-2/5" aria-hidden="true">
      <UiImage
        :image="picture"
        ratio="auto"
        frame-class="size-full"
        class="size-full"
        loading="eager"
        sizes="(min-width: 1024px) 60vw, 100vw"
      />
      <div class="absolute inset-0 bg-scrim/60 lg:bg-scrim/15" />
      <div class="fade-inverse-start absolute inset-y-0 left-0 hidden w-3/5 lg:block" />
    </div>

    <div class="mx-auto w-full max-w-[1280px] px-4 py-8 sm:px-6 sm:py-10">
      <div class="max-w-2xl">
        <p class="flex flex-wrap items-center gap-3 text-sm">
          <span
            class="font-bold uppercase text-text-on-inverse-muted"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ t('programme.hero.overline') }}
          </span>
          <UiStatusBadge
            :state="props.edition.temporal_state"
            size="sm"
            :label="t(`programme.hero.state.${props.edition.temporal_state}`)"
          />
        </p>

        <h1 class="mt-3 font-display text-2xl leading-tight text-balance text-text-on-inverse sm:text-4xl">
          {{ title }}
        </h1>

        <p class="mt-2 text-sm text-text-on-inverse-muted">
          <span class="tabular-nums">{{ period }}</span>
          <template v-if="place"> · {{ place }}</template>
        </p>

        <dl
          v-if="isPublished"
          class="mt-6 flex flex-wrap gap-x-8 gap-y-4"
          :aria-label="t('programme.hero.stats.label')"
        >
          <div v-for="figure in figures" :key="figure.key" class="flex flex-col-reverse">
            <dt class="text-xs uppercase text-text-on-inverse-muted" :style="{ letterSpacing: 'var(--tracking-caps)' }">
              {{ t(`programme.hero.stats.${figure.key}`, figure.value) }}
            </dt>
            <dd class="font-display text-3xl leading-none tabular-nums text-text-on-inverse">
              {{ n(figure.value) }}
            </dd>
          </div>

          <div
            v-if="updatedAt"
            class="basis-full border-t border-border-on-inverse pt-4 sm:basis-auto sm:border-t-0 sm:border-l sm:pt-0 sm:pl-8"
          >
            <dt class="text-xs uppercase text-text-on-inverse-muted" :style="{ letterSpacing: 'var(--tracking-caps)' }">
              {{ t('programme.hero.stats.updated') }}
            </dt>
            <dd class="mt-1 text-sm text-text-on-inverse">
              <span class="font-bold">{{ updatedAt }}</span>
              <span class="block text-xs text-text-on-inverse-muted">{{ zone }}</span>
            </dd>
          </div>
        </dl>

        <nav v-if="choices.length > 1" class="mt-6" :aria-label="t('programme.hero.others')">
          <p class="text-xs uppercase text-text-on-inverse-muted" :style="{ letterSpacing: 'var(--tracking-caps)' }">
            {{ t('programme.hero.others') }}
          </p>
          <div class="mt-2 flex flex-wrap gap-2">
            <UiButton
              v-for="choice in choices"
              :key="choice.id"
              :variant="choice.id === props.edition.id ? 'primary' : 'inverse'"
              :aria-current="choice.id === props.edition.id ? 'true' : undefined"
              :label="`${choice.edition_year} · ${label(choice)}`"
              @click="emit('select', choice.id)"
            />
          </div>
        </nav>
      </div>
    </div>
  </header>
</template>
