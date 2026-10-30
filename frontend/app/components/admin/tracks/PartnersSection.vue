<script setup lang="ts">
import type { SimilarOrganization } from '~/types/org'
import type { TaxonomyTerm } from '~/types/reference'
import type { I18nText } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { TrackPartnerDraft, TrackPageDraft } from '~/utils/track-page'

/**
 * Les partenaires de la journée (« Avec » : OIF, GIZ…), distincts des
 * organisations de ses activités. Une vraie fiche du référentiel, cherchée sur
 * toutes ses dénominations (règle n° 1) : une organisation n'y figure qu'une fois.
 * L'ordre de la liste est l'ordre d'affichage.
 */

const props = defineProps<{
  modelValue: TrackPageDraft
  roles: TaxonomyTerm[]
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: TrackPageDraft] }>()

const { t } = useI18n()
const { tr } = useI18nText()

const duplicate = ref<string | null>(null)

const roleOptions = computed<SelectOption[]>(() =>
  props.roles.map((role) => ({ value: role.code, label: tr(role.label) })),
)

function setPartners(partners: TrackPartnerDraft[]): void {
  emit('update:modelValue', { ...props.modelValue, partners })
}

function add(match: SimilarOrganization): void {
  if (props.modelValue.partners.some((p) => p.organization_id === match.organization_id)) {
    duplicate.value = t('admin.event.track.partners.duplicate', { name: match.legal_name })
    return
  }
  duplicate.value = null
  setPartners([
    ...props.modelValue.partners,
    {
      organization_id: match.organization_id,
      name: match.legal_name,
      acronym: match.acronym,
      logo: null,
      role_code: 'partner',
    },
  ])
}

function setRole(index: number, role_code: string): void {
  setPartners(props.modelValue.partners.map((p, i) => (i === index ? { ...p, role_code } : p)))
}
</script>

<template>
  <div class="space-y-5">
    <ol v-if="props.modelValue.partners.length" class="divide-y divide-border-subtle rounded-md border border-border-subtle">
      <li
        v-for="(partner, index) in props.modelValue.partners"
        :key="partner.organization_id"
        class="flex flex-col gap-3 p-3 sm:flex-row sm:items-center"
      >
        <div class="flex min-w-0 flex-1 items-center gap-3">
          <span class="flex size-11 shrink-0 items-center justify-center overflow-hidden rounded-md border border-border-subtle bg-surface-raised">
            <UiImage v-if="partner.logo" :image="partner.logo" ratio="1" frame-class="size-full" fit="contain" />
            <span v-else class="text-xs font-semibold text-text-muted">{{ initialsOf(partner.name) }}</span>
          </span>
          <div class="min-w-0">
            <p class="truncate font-semibold text-text">{{ partner.name }}</p>
            <p v-if="partner.acronym" class="text-sm text-text-muted">{{ partner.acronym }}</p>
          </div>
        </div>
        <div class="flex items-end gap-1">
          <UiSelect
            class="min-w-44 flex-1"
            :model-value="partner.role_code"
            :options="roleOptions"
            :label="t('admin.event.track.partners.role', { name: partner.acronym ?? partner.name })"
            hide-label
            hide-optional
            :disabled="props.disabled || roleOptions.length === 0"
            @update:model-value="(next: string) => setRole(index, next)"
          />
          <UiMoveButtons
            :item="partner.acronym ?? partner.name"
            :first="index === 0"
            :last="index === props.modelValue.partners.length - 1"
            :disabled="props.disabled"
            @up="setPartners(moved(props.modelValue.partners, index, -1))"
            @down="setPartners(moved(props.modelValue.partners, index, 1))"
          />
          <UiButton
            variant="ghost"
            icon="trash"
            icon-only
            :label="t('admin.event.track.partners.remove', { name: partner.acronym ?? partner.name })"
            :disabled="props.disabled"
            @click="setPartners(props.modelValue.partners.filter((_, i) => i !== index))"
          />
        </div>
      </li>
    </ol>
    <p v-else class="text-sm text-text-muted">{{ t('admin.event.track.partners.empty') }}</p>

    <AdminNegotiationMeetingOrganizerPicker
      :model-value="null"
      :label="t('admin.event.track.partners.add')"
      optional
      :disabled="props.disabled"
      @chosen="add"
    />
    <UiAlert v-if="duplicate" intent="warning" compact live :message="duplicate" />

    <AdminEventsI18nField
      :model-value="props.modelValue.support_note"
      :label="t('admin.event.track.partners.supportNote')"
      :hint="t('admin.event.track.partners.supportNoteHint')"
      multiline
      :rows="2"
      :disabled="props.disabled"
      @update:model-value="(next: I18nText | null) => emit('update:modelValue', { ...props.modelValue, support_note: next })"
    />
  </div>
</template>
