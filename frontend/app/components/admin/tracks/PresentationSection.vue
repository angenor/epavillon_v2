<script setup lang="ts">
import type { EditionVenue } from '~/types/admin-events'
import type { I18nText } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { TrackPageDraft } from '~/utils/track-page'

/**
 * Lieu, accroche et publics du fil. Le lieu se choisit parmi ceux de
 * l'édition ; aucun choix veut dire « le pavillon » (`venue_id` nul), le cas
 * de toutes les journées spéciales. L'hôtel de l'atelier se déclare à
 * l'onglet « Lieux et salles », avec son adresse, son secteur et sa consigne.
 */

const props = defineProps<{
  modelValue: TrackPageDraft
  venues: EditionVenue[]
  venuesTo: string
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: TrackPageDraft] }>()

const { t } = useI18n()
const { tr } = useI18nText()

function set<K extends keyof TrackPageDraft>(key: K, value: TrackPageDraft[K]): void {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}

const pavilion = computed(() => props.venues.find((venue) => venue.kind === 'pavilion') ?? null)

const venueOptions = computed<SelectOption[]>(() => [
  {
    value: '',
    label: pavilion.value
      ? t('admin.event.track.place.pavilionNamed', { name: tr(pavilion.value.name) })
      : t('admin.event.track.place.pavilion'),
  },
  ...props.venues
    .filter((venue) => venue.kind !== 'pavilion' || venue.id === props.modelValue.venue_id)
    .map((venue) => ({
      value: venue.id,
      label: tr(venue.name),
      description: venue.address ?? undefined,
    })),
])

const shown = computed<EditionVenue | null>(() =>
  props.modelValue.venue_id
    ? (props.venues.find((venue) => venue.id === props.modelValue.venue_id) ?? null)
    : pavilion.value,
)
</script>

<template>
  <div class="space-y-6">
    <div class="grid gap-4 lg:grid-cols-2">
      <div>
        <UiSelect
          :model-value="props.modelValue.venue_id ?? ''"
          :label="t('admin.event.track.place.venue')"
          :hint="t('admin.event.track.place.venueHint')"
          :options="venueOptions"
          hide-optional
          :disabled="props.disabled"
          @update:model-value="(next: string) => set('venue_id', next || null)"
        />
        <UiButton class="mt-2" variant="link" size="sm" icon="plus" :to="props.venuesTo">
          {{ t('admin.event.track.place.manageVenues') }}
        </UiButton>
      </div>

      <!-- Ce que le plan d'accès de la page affichera. -->
      <div v-if="shown" class="rounded-md border border-border-subtle bg-surface p-4 text-sm">
        <p class="flex flex-wrap items-baseline gap-x-2 font-semibold text-text">
          <UiIcon name="map-pin" size="1rem" class="self-center text-text-muted" />
          {{ tr(shown.name) }}
          <span v-if="shown.area" class="font-normal text-text-muted">{{ tr(shown.area) }}</span>
        </p>
        <p v-if="shown.address" class="mt-1 text-text-secondary">{{ shown.address }}</p>
        <p v-if="shown.access_note" class="mt-1 text-text-muted">{{ tr(shown.access_note) }}</p>
        <p v-if="!shown.address && !shown.map_url" class="mt-1 text-text-subtle">
          {{ t('admin.event.track.place.noAddress') }}
        </p>
        <UiButton
          v-if="shown.map_url"
          class="mt-2"
          variant="link"
          size="sm"
          icon-trailing="external-link"
          :href="shown.map_url"
        >
          {{ t('admin.event.track.place.map') }}
        </UiButton>
      </div>
    </div>

    <AdminEventsI18nField
      :model-value="props.modelValue.tagline"
      :label="t('admin.event.track.place.tagline')"
      :hint="t('admin.event.track.place.taglineHint')"
      :maxlength="120"
      :disabled="props.disabled"
      @update:model-value="(next: I18nText | null) => set('tagline', next)"
    />

    <AdminTracksAudiencesField
      :model-value="props.modelValue.target_audiences"
      :disabled="props.disabled"
      @update:model-value="(next: I18nText[]) => set('target_audiences', next)"
    />
  </div>
</template>
