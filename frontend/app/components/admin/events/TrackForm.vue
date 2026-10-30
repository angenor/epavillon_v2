<script setup lang="ts">
import type { EditionDetail, EditionTrackPayload } from '~/types/admin-events'
import type { TrackKind } from '~/types/event/edition'
import type { TaxonomyTermCode } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * La fiche d'un fil : genre, titres, période annoncée, couleur, thématiques,
 * responsable, ouverture de la page publique. Partagée par l'onglet « Journées
 * spéciales » et la page d'un fil : un seul formulaire pour un seul geste.
 */

interface Props {
  initial: EditionTrackPayload
  detail: EditionDetail
  busy?: boolean
  /** Erreur de la dernière écriture, déjà traduite. */
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ submit: [payload: EditionTrackPayload]; cancel: [] }>()

const { t } = useI18n()

const KINDS: TrackKind[] = ['special_day', 'thematic_track', 'side_programme']
const PALETTE = ['#0B6C9E', '#1F5F8B', '#8C6D1F', '#2E6B45', '#7A3E6B', '#8B3A2E']

const draft = ref<EditionTrackPayload>({ ...props.initial })
watch(
  () => props.initial,
  (next) => (draft.value = { ...next }),
)

const kindOptions = computed<SelectOption[]>(() =>
  KINDS.map((kind) => ({ value: kind, label: t('admin.event.tabs.tracksTab.kind.' + kind) })),
)

const curatorOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.event.tabs.tracksTab.form.curatedByNone') },
  ...props.detail.curators.map((person) => ({
    value: person.person_id,
    label: person.full_name,
    description: person.organization_name ?? undefined,
  })),
])
</script>

<template>
  <form class="space-y-5" novalidate @submit.prevent="emit('submit', { ...draft })">
    <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

    <AdminEventsI18nField
      :model-value="draft.title"
      :label="t('admin.event.tabs.tracksTab.form.titleField')"
      required
      @update:model-value="(next) => (draft.title = next ?? { fr: '' })"
    />

    <AdminEventsI18nField
      :model-value="draft.subtitle"
      :label="t('admin.event.tabs.tracksTab.form.subtitleField')"
      @update:model-value="(next) => (draft.subtitle = next)"
    />

    <div class="grid gap-4 sm:grid-cols-2">
      <UiInput
        :model-value="draft.code"
        :label="t('admin.event.tabs.tracksTab.form.code')"
        :hint="t('admin.event.tabs.tracksTab.form.codeHint')"
        required
        @update:model-value="(next: string) => (draft.code = next)"
      />
      <UiInput
        :model-value="draft.slug"
        :label="t('admin.event.tabs.tracksTab.form.slug')"
        :hint="t('admin.event.tabs.tracksTab.form.slugHint')"
        required
        @update:model-value="(next: string) => (draft.slug = next)"
      />
    </div>

    <UiSelect
      :model-value="draft.kind"
      :label="t('admin.event.tabs.tracksTab.form.kindField')"
      :options="kindOptions"
      hide-optional
      @update:model-value="(next: string) => (draft.kind = next as TrackKind)"
    />

    <div class="grid gap-4 sm:grid-cols-2">
      <UiDatePicker
        :model-value="draft.starts_on"
        :label="t('admin.event.tabs.tracksTab.form.startsOn')"
        :hint="t('admin.event.tabs.tracksTab.form.periodHint')"
        @update:model-value="(next: string) => (draft.starts_on = next || null)"
      />
      <UiDatePicker
        :model-value="draft.ends_on"
        :label="t('admin.event.tabs.tracksTab.form.endsOn')"
        :min="draft.starts_on ?? undefined"
        @update:model-value="(next: string) => (draft.ends_on = next || null)"
      />
    </div>

    <AdminEventsI18nField
      :model-value="draft.description"
      :label="t('admin.event.tabs.tracksTab.form.descriptionField')"
      multiline
      :rows="4"
      @update:model-value="(next) => (draft.description = next)"
    />

    <!-- La liste envoyée remplace celle du fil : décocher retire vraiment. -->
    <AdminThemePicker
      v-if="props.detail.available_themes.length > 0"
      :model-value="draft.theme_codes"
      :themes="props.detail.available_themes"
      :label="t('admin.event.tabs.tracksTab.form.themes')"
      :hint="t('admin.event.tabs.tracksTab.form.themesHint')"
      :preview-label="t('admin.showcase.form.themes.preview')"
      @update:model-value="(next: TaxonomyTermCode[]) => (draft.theme_codes = next)"
    />

    <UiSelect
      :model-value="draft.curated_by ?? ''"
      :label="t('admin.event.tabs.tracksTab.form.curatedBy')"
      :options="curatorOptions"
      hide-optional
      @update:model-value="(next: string) => (draft.curated_by = next || null)"
    />

    <fieldset>
      <legend class="mb-2 text-sm font-bold text-text">
        {{ t('admin.event.tabs.tracksTab.form.color') }}
      </legend>
      <div class="flex flex-wrap items-center gap-2">
        <button
          v-for="color in PALETTE"
          :key="color"
          type="button"
          class="size-(--target-min) cursor-pointer rounded-md border-2 transition-transform hover:scale-105"
          :class="draft.color_hex === color ? 'border-text' : 'border-border'"
          :style="{ backgroundColor: color }"
          :aria-pressed="draft.color_hex === color"
          :aria-label="color"
          @click="draft.color_hex = color"
        />
      </div>
    </fieldset>

    <UiSwitch
      :model-value="draft.is_published"
      :label="t('admin.event.tabs.tracksTab.form.publish')"
      @update:model-value="(next: boolean) => (draft.is_published = next)"
    />

    <div class="flex flex-wrap gap-3 border-t border-border pt-4">
      <UiButton type="submit" :loading="props.busy">
        {{ t('common.actions.save') }}
      </UiButton>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('cancel')">
        {{ t('admin.event.tabs.confirm.cancel') }}
      </UiButton>
    </div>
  </form>
</template>
