<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'

/**
 * UNE ÉDITION DANS L'HISTORIQUE DE L'ACCUEIL.
 *
 * ── LA BANNIÈRE EST SOUVENT NULLE, ET LA CARTE DOIT RESTER ENTIÈRE ──────────
 *
 * `v_public_editions.banner` vient du rôle `banner` de `event.events` ; la
 * plupart des éditions passées n'en ont pas. Le repli n'invente donc AUCUNE
 * image : un aplat institutionnel portant le millésime, ce qui reste un repère
 * là où un dégradé ou un pictogramme générique n'en serait pas un. C'est la même
 * règle que `UiImage`, qui ne rend rien plutôt qu'une image de remplacement.
 *
 * ── « EN COURS » EST JAUNE, PAS VERT ────────────────────────────────────────
 *
 * Le jaune signale ce qui demande attention ; le vert, ce qui est confirmé. Une
 * édition en cours n'est pas une réussite, c'est un événement qui se tient en ce
 * moment. `UiStatusBadge` porte déjà cette table de couleurs — la carte lui
 * passe l'état temporel et le libellé, elle ne choisit pas de teinte.
 *
 * ── LE NOMBRE D'ACTIVITÉS : ABSENT VAUT ZÉRO ────────────────────────────────
 *
 * Une édition sans programme publié n'a AUCUNE ligne dans
 * `programme.v_edition_stats`. Le compte arrive donc déjà résolu par
 * `publishedSessionCount()`, et la carte affiche « 0 activité publiée » plutôt
 * qu'un tiret : zéro est une information, un tiret est un aveu.
 */

interface Props {
  edition: PublicEditionRow
  /** Séances publiées — déjà résolu, l'absence de ligne valant zéro. */
  sessionCount: number
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateRange, zoneLabel } = useDateTime()
const localePath = useLocalePath()

const to = computed(() => localePath(`/evenements/${props.edition.slug}`))

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
  <article
    class="flex flex-col overflow-hidden rounded-lg border border-border bg-surface-raised transition-shadow hover:shadow-md"
  >
    <UiImage
      v-if="props.edition.banner"
      :image="props.edition.banner"
      ratio="16 / 9"
      sizes="(min-width: 1280px) 380px, (min-width: 640px) 45vw, 92vw"
    />
    <!-- REPLI : aucun visuel inventé, le millésime suffit à repérer la carte. -->
    <div
      v-else
      class="flex items-center justify-center bg-surface-inverse px-4 text-text-on-inverse"
      style="aspect-ratio: 16 / 9"
      aria-hidden="true"
    >
      <span class="font-display text-2xl tabular-nums">
        {{ props.edition.edition_label ?? props.edition.edition_year }}
      </span>
    </div>

    <div class="flex flex-1 flex-col gap-2 p-4">
      <p class="flex flex-wrap items-center gap-2">
        <span
          v-if="props.edition.series_name"
          class="text-xs uppercase text-text-subtle"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ tr(props.edition.series_name) }}
        </span>
        <UiStatusBadge
          :state="props.edition.temporal_state"
          size="sm"
          :label="t(`home.history.state.${props.edition.temporal_state}`)"
        />
      </p>

      <h4 class="text-lg leading-snug">
        <NuxtLink :to="to" class="text-heading no-underline hover:underline">
          {{ tr(props.edition.title) }}
        </NuxtLink>
      </h4>

      <p v-if="props.edition.edition_label" class="text-sm font-medium text-text-secondary">
        {{ props.edition.edition_label }}
      </p>

      <p class="flex items-start gap-2 text-sm text-text-muted">
        <UiIcon name="calendar" size="1rem" class="mt-0.5 shrink-0" />
        <span>
          {{ dates }}
          <span class="block text-xs text-text-subtle">{{ zone }}</span>
        </span>
      </p>

      <p v-if="place" class="flex items-center gap-2 text-sm text-text-muted">
        <UiIcon name="map-pin" size="1rem" class="shrink-0" />
        {{ place }}
      </p>

      <p class="flex items-center gap-2 text-sm text-text-muted">
        <UiIcon name="grid" size="1rem" class="shrink-0" />
        {{ t('home.history.sessions', { count: props.sessionCount }, props.sessionCount) }}
      </p>

      <UiThemeTagList
        v-if="props.edition.themes.length"
        class="mt-1"
        :themes="props.edition.themes"
        size="sm"
      />

      <NuxtLink
        :to="to"
        class="mt-auto inline-flex items-center gap-1 pt-2 text-sm font-medium no-underline hover:underline"
      >
        {{ t('home.history.card.open') }}
        <UiIcon name="arrow-right" size="0.9rem" />
      </NuxtLink>
    </div>
  </article>
</template>
