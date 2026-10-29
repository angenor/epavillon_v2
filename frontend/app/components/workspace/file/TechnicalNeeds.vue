<script setup lang="ts">
import type { SaveTechnicalNeedsPayload, TechnicalNeeds } from '~/types/programme/day-plan'
import type { IsoDateTime, TimeZoneName } from '~/types/shared'

/**
 * LES BESOINS TECHNIQUES, signalés à l'avance (29/10) : trois grandes tuiles à
 * cocher plutôt qu'un formulaire, pour des personnes peu à l'aise avec les
 * écrans. La date limite est indicative : elle se calcule (dix jours avant la
 * première séance) pour rappeler ce qui manque, rien n'est refusé après.
 */

interface Props {
  needs: TechnicalNeeds
  deadline: IsoDateTime | null
  timezone: TimeZoneName
  canEdit: boolean
  saving: boolean
  error: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ save: [needs: SaveTechnicalNeedsPayload] }>()

const { t } = useI18n()
const { date } = useDateTime()

type NeedKey = 'video' | 'remote' | 'accessibility'
const KEYS: NeedKey[] = ['video', 'remote', 'accessibility']

const editing = ref(false)
const form = ref<SaveTechnicalNeedsPayload>({
  video: false,
  remote: false,
  remote_details: '',
  accessibility: false,
  accessibility_details: '',
  other: '',
})

function start(): void {
  const needs = props.needs
  form.value = {
    video: needs.video,
    remote: needs.remote,
    remote_details: needs.remote_details,
    accessibility: needs.accessibility,
    accessibility_details: needs.accessibility_details,
    other: needs.other,
  }
  editing.value = true
}

watch(
  () => props.needs,
  () => {
    if (!props.saving && !props.error) editing.value = false
  },
)

const declared = computed(() => props.needs.updated_at !== null)
const chosen = computed(() => KEYS.filter((key) => props.needs[key]))
</script>

<template>
  <WorkspaceFileSection
    id="besoins-techniques"
    :title="t('organization.workspace.proposal.needs.title')"
    :description="
      props.deadline
        ? t('organization.workspace.proposal.needs.deadline', { date: date(props.deadline, props.timezone) })
        : t('organization.workspace.proposal.needs.description')
    "
    :can-edit="props.canEdit"
    :editing="editing"
    :saving="props.saving"
    :error="props.error"
    @edit="start"
    @save="emit('save', form)"
    @cancel="editing = false"
  >
    <p v-if="!declared" class="font-semibold text-warning">{{ t('organization.workspace.proposal.needs.notDeclared') }}</p>
    <p v-else-if="chosen.length === 0 && !props.needs.other.trim()" class="text-text-secondary">
      {{ t('organization.workspace.proposal.needs.none') }}
    </p>
    <ul v-else class="flex flex-col gap-2 text-text-secondary">
      <li v-for="key in chosen" :key="key" class="flex items-start gap-2">
        <UiIcon name="check" size="1rem" class="mt-1 shrink-0 text-success" />
        <span>
          <strong class="text-text">{{ t(`organization.workspace.proposal.needs.kinds.${key}.label`) }}</strong>
          <template v-if="key === 'remote' && props.needs.remote_details"> · {{ props.needs.remote_details }}</template>
          <template v-if="key === 'accessibility' && props.needs.accessibility_details"> · {{ props.needs.accessibility_details }}</template>
        </span>
      </li>
      <li v-if="props.needs.other.trim()" class="flex items-start gap-2">
        <UiIcon name="info" size="1rem" class="mt-1 shrink-0 text-text-subtle" />
        <span>{{ props.needs.other }}</span>
      </li>
    </ul>

    <template #edit>
      <div class="grid gap-3 sm:grid-cols-3">
        <button
          v-for="key in KEYS"
          :key="key"
          type="button"
          :aria-pressed="form[key]"
          class="flex min-h-28 cursor-pointer flex-col items-start gap-2 rounded-lg border-2 p-4 text-left transition-colors duration-(--duration-fast)"
          :class="form[key] ? 'border-accent bg-info-surface' : 'border-border bg-surface-raised hover:border-border-strong'"
          @click="form[key] = !form[key]"
        >
          <span class="flex w-full items-center justify-between gap-2">
            <span class="font-semibold text-heading">{{ t(`organization.workspace.proposal.needs.kinds.${key}.label`) }}</span>
            <span
              aria-hidden="true"
              class="grid size-6 shrink-0 place-items-center rounded border-2"
              :class="form[key] ? 'border-accent bg-accent-solid text-accent-contrast' : 'border-border-strong'"
            >
              <UiIcon v-if="form[key]" name="check" size="0.875rem" />
            </span>
          </span>
          <span class="text-sm text-text-muted">{{ t(`organization.workspace.proposal.needs.kinds.${key}.hint`) }}</span>
        </button>
      </div>
      <UiInput
        v-if="form.remote"
        id="needs-remote-details"
        v-model="form.remote_details"
        class="mt-4"
        :label="t('organization.workspace.proposal.needs.remoteDetails')"
      />
      <UiInput
        v-if="form.accessibility"
        id="needs-accessibility-details"
        v-model="form.accessibility_details"
        class="mt-4"
        :label="t('organization.workspace.proposal.needs.accessibilityDetails')"
      />
      <UiTextarea
        id="needs-other"
        v-model="form.other"
        class="mt-4"
        :rows="2"
        :label="t('organization.workspace.proposal.needs.other')"
        :placeholder="t('organization.workspace.proposal.needs.otherPlaceholder')"
      />
    </template>
  </WorkspaceFileSection>
</template>
