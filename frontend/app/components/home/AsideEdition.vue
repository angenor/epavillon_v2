<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'

/**
 * UN PROCHAIN RENDEZ-VOUS DU PANNEAU « À VENIR ».
 *
 * Une ligne, pas une carte : le panneau en montre trois, sous six séances, et la
 * carte d'historique existe déjà plus bas dans la page pour qui veut le détail.
 * Ce qu'on retient ici, c'est le nom de l'édition, quand elle se tient, et où.
 *
 * `edition_label` PLUTÔT QUE `title` EN TÊTE quand il existe : « COP31 » est ce
 * que les gens disent, le titre complet vient après.
 */

interface Props {
  edition: PublicEditionRow
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateRange, zoneLabel } = useDateTime()
const localePath = useLocalePath()

const dates = computed(() =>
  dateRange(props.edition.starts_at, props.edition.ends_at, props.edition.timezone),
)

const zone = computed(() =>
  zoneLabel(props.edition.timezone, props.edition.city ?? undefined),
)

const place = computed(() =>
  [props.edition.city, tr(props.edition.country_name)].filter(Boolean).join(', '),
)
</script>

<template>
  <article class="rounded-lg border border-glass-border bg-glass-raised p-3 shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover">
    <div class="flex items-start justify-between gap-2">
      <h4 class="min-w-0 text-sm leading-snug font-bold">
        <NuxtLink
          :to="localePath(`/evenements/${props.edition.slug}`)"
          class="text-text-on-inverse no-underline hover:underline"
        >
          {{ props.edition.edition_label ?? tr(props.edition.title) }}
        </NuxtLink>
      </h4>
      <UiStatusBadge
        :state="props.edition.temporal_state"
        size="sm"
        :label="t(`home.history.state.${props.edition.temporal_state}`)"
      />
    </div>

    <p class="mt-1 text-xs text-text-on-inverse-muted">
      {{ dates }} <span class="text-text-on-inverse-muted">({{ zone }})</span>
    </p>
    <p v-if="place" class="text-xs text-text-on-inverse-muted">{{ place }}</p>
  </article>
</template>
