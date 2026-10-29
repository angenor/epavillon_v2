<script setup lang="ts">
import type { FocalPoint, FocalPointInput } from '~/types/programme/day-plan'

/**
 * LE POINT FOCAL SUR PLACE (29/10) : la personne que l'équipe de l'IFDD appelle
 * le jour J. Par défaut celle qui a déposé le dossier, intervenante ou non.
 * Toujours au moins une : la carte refuse de retirer la dernière, et dit
 * pourquoi plutôt que de griser un bouton sans explication.
 */

interface Props {
  points: FocalPoint[]
  canEdit: boolean
  saving: boolean
  error: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ save: [points: FocalPointInput[]] }>()

const { t } = useI18n()

interface Row extends FocalPointInput {
  key: string
  sameNumber: boolean
}

const editing = ref(false)
const rows = ref<Row[]>([])
const lastWarning = ref(false)

const toRow = (point: FocalPoint | FocalPointInput, index: number): Row => ({
  key: `focal-${index}-${point.speaker_id ?? point.person_id ?? 'free'}`,
  speaker_id: point.speaker_id,
  person_id: point.person_id,
  full_name: point.full_name,
  phone: point.phone,
  whatsapp: point.whatsapp,
  sameNumber: point.whatsapp === '' || point.whatsapp === point.phone,
})

function start(): void {
  rows.value = props.points.map(toRow)
  lastWarning.value = false
  editing.value = true
}

function add(): void {
  rows.value.push(toRow({ speaker_id: null, person_id: null, full_name: '', phone: '', whatsapp: '' }, rows.value.length))
}

function remove(index: number): void {
  if (rows.value.length === 1) {
    lastWarning.value = true
    return
  }
  rows.value.splice(index, 1)
}

function save(): void {
  emit(
    'save',
    rows.value.map(({ speaker_id, person_id, full_name, phone, whatsapp, sameNumber }) => ({
      speaker_id,
      person_id,
      full_name: full_name.trim(),
      phone: phone.trim(),
      whatsapp: (sameNumber ? phone : whatsapp).trim(),
    })),
  )
}

// L'enregistrement réussi rend une nouvelle liste : la carte se referme.
watch(
  () => props.points,
  () => {
    if (!props.saving && !props.error) editing.value = false
  },
)

function originOf(point: FocalPoint): string {
  const key = 'organization.workspace.proposal.focal.origin'
  if (point.is_default) return t(`${key}.submitter`)
  if (point.speaker_id) return t(`${key}.speaker`)
  return t(`${key}.colleague`)
}
</script>

<template>
  <WorkspaceFileSection
    id="point-focal"
    :title="t('organization.workspace.proposal.focal.title')"
    :description="t('organization.workspace.proposal.focal.description')"
    :can-edit="props.canEdit"
    :editing="editing"
    :saving="props.saving"
    :error="props.error"
    @edit="start"
    @save="save"
    @cancel="editing = false"
  >
    <ul class="flex flex-col gap-3">
      <li
        v-for="(point, index) in props.points"
        :key="point.id ?? `default-${index}`"
        class="grid gap-3 rounded-md bg-info-surface p-4 sm:grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)_minmax(0,1fr)]"
      >
        <div>
          <p class="font-semibold text-text">{{ point.full_name }}</p>
          <p class="mt-0.5 text-sm text-text-muted">{{ originOf(point) }}</p>
        </div>
        <div>
          <p class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.focal.phone') }}</p>
          <p v-if="point.phone" class="mt-0.5 text-text">{{ point.phone }}</p>
          <p v-else class="mt-0.5 font-semibold text-warning">{{ t('organization.workspace.proposal.focal.missingPhone') }}</p>
        </div>
        <div>
          <p class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.focal.whatsapp') }}</p>
          <p class="mt-0.5 text-text">{{ point.whatsapp || point.phone || '·' }}</p>
        </div>
      </li>
    </ul>
    <p class="mt-3 text-sm text-text-muted">{{ t('organization.workspace.proposal.focal.hint') }}</p>
    <UiAlert v-if="props.error && !editing" class="mt-3" intent="warning" :message="props.error" live />

    <template #edit>
      <ul class="flex flex-col gap-4">
        <li v-for="(row, index) in rows" :key="row.key" class="rounded-md border border-border p-4">
          <div class="grid gap-3 sm:grid-cols-2">
            <UiInput
              :id="`${row.key}-name`"
              v-model="row.full_name"
              :label="t('organization.workspace.proposal.focal.name')"
              :readonly="row.speaker_id !== null || row.person_id !== null"
              required
            />
            <UiInput :id="`${row.key}-phone`" v-model="row.phone" type="tel" :label="t('organization.workspace.proposal.focal.phone')" required />
          </div>
          <UiCheckbox
            :id="`${row.key}-same`"
            v-model="row.sameNumber"
            class="mt-3"
            :label="t('organization.workspace.proposal.focal.sameNumber')"
          />
          <UiInput
            v-if="!row.sameNumber"
            :id="`${row.key}-whatsapp`"
            v-model="row.whatsapp"
            class="mt-3 sm:max-w-sm"
            type="tel"
            :label="t('organization.workspace.proposal.focal.whatsapp')"
          />
          <UiButton variant="ghost" class="mt-3" icon="trash" @click="remove(index)">
            {{ t('organization.workspace.proposal.focal.remove') }}
          </UiButton>
        </li>
      </ul>
      <p v-if="lastWarning" role="status" class="mt-3 text-sm font-semibold text-warning">
        {{ t('organization.workspace.proposal.focal.atLeastOne') }}
      </p>
      <UiButton variant="secondary" class="mt-4" icon="plus" @click="add">
        {{ t('organization.workspace.proposal.focal.add') }}
      </UiButton>
    </template>
  </WorkspaceFileSection>
</template>
