<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'

/**
 * FICHE D'UNE ACTIVITÉ — le contenu du dialogue ouvert depuis l'une ou l'autre
 * des deux vues.
 *
 * POURQUOI UN DIALOGUE ET NON UNE PAGE. La page publique d'une activité viendra
 * (elle a besoin des intervenants, des documents, de l'inscription et de la
 * rediffusion — rien de tout cela n'est dans `v_public_schedule`). D'ici là, un
 * dialogue rend ce que la vue porte déjà, SANS quitter la programmation ni
 * perdre les filtres. Il n'invente aucun champ : tout ce qui est affiché ici
 * vient d'une colonne de la vue.
 *
 * LA SÉLECTION EST PARTAGÉE par les deux vues, comme les filtres : on ouvre une
 * activité depuis la grille, on bascule en calendrier, elle y reste désignée.
 */

interface Props {
  session: PublicScheduleRow
  zoneLabel?: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const specialDays = computed(() => props.session.tracks.filter((track) => track.kind === 'special_day'))
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex flex-wrap items-center gap-2">
      <UiLiveBadge :session-id="props.session.id" size="sm" />
      <UiStatusBadge
        :state="props.session.temporal_state"
        size="sm"
        :label="t(`session-card.state.${props.session.temporal_state}`)"
      />
      <UiBadge size="sm">{{ t(`session-card.format.${props.session.format}`) }}</UiBadge>
    </div>

    <UiImage
      v-if="props.session.cover"
      :image="props.session.cover"
      ratio="16 / 9"
      rounded="rounded-lg"
      sizes="(min-width: 640px) 32rem, 100vw"
    />

    <dl class="grid gap-3 sm:grid-cols-2">
      <div>
        <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('programme.detail.slot') }}
        </dt>
        <dd class="mt-1">
          <p class="text-sm text-text">{{ date(props.session.starts_at, props.session.timezone) }}</p>
          <UiZonedTime
            :start="props.session.starts_at"
            :end="props.session.ends_at"
            :timezone="props.session.timezone"
            :zone-label="props.zoneLabel"
            format="full"
          />
        </dd>
      </div>

      <div v-if="props.session.room_name">
        <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('programme.detail.room') }}
        </dt>
        <dd class="mt-1 text-sm text-text">{{ tr(props.session.room_name) }}</dd>
      </div>

      <div v-if="props.session.organization_name">
        <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('programme.detail.organization') }}
        </dt>
        <dd class="mt-1 text-sm text-text">
          {{ props.session.organization_name }}
          <span v-if="props.session.organization_acronym" class="text-text-muted">
            ({{ props.session.organization_acronym }})
          </span>
          <span v-if="props.session.organization_country" class="block text-text-muted">
            {{ tr(props.session.organization_country) }}
          </span>
        </dd>
      </div>

      <div v-if="props.session.capacity !== null">
        <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('programme.detail.capacity') }}
        </dt>
        <dd class="mt-1">
          <UiCapacityMeter
            :registered="props.session.registered_count"
            :capacity="props.session.capacity"
            compact
          />
        </dd>
      </div>
    </dl>

    <p v-if="props.session.summary" class="text-sm text-text-secondary">
      {{ tr(props.session.summary) }}
    </p>

    <div v-if="specialDays.length || props.session.themes.length" class="flex flex-wrap items-center gap-2">
      <UiBadge
        v-for="track in specialDays"
        :key="track.slug"
        intent="info"
        size="sm"
        :dot-color="track.color"
      >
        {{ tr(track.title) }}
      </UiBadge>
      <UiThemeTagList
        :themes="props.session.themes.map((theme) => ({
          code: theme.code,
          label: theme.label,
          color: theme.color,
        }))"
        :max="6"
        size="sm"
      />
    </div>

    <p v-if="props.session.is_streamed" class="inline-flex items-center gap-1.5 text-sm text-text-muted">
      <UiIcon name="video" size="0.95rem" />
      {{ t('session-card.streamed') }}
    </p>
  </div>
</template>
