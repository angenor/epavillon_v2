<script setup lang="ts">
import type { CommitteePayload, EditionDetail } from '~/types/admin-events'
import type { SelectOption, TableColumn } from '~/types/ui'

/**
 * ONGLET « COMITÉ DE SÉLECTION ».
 *
 * ── CETTE TABLE DIT LA COMPOSITION, PAS LE DROIT D'ACCÈS ────────────────────
 *
 * `event.call_reviewers` dit qui siège ; l'autorisation reste portée par
 * `identity.role_assignments` sur la portée de l'édition. Ajouter quelqu'un ici ne
 * lui ouvre aucun dossier — et c'est exactement le piège. L'écran affiche donc, pour
 * chaque membre, s'il détient bien `programme.review.write` sur cette édition : sans
 * cette colonne, on confierait douze dossiers à quelqu'un qui se heurterait à un
 * écran « accès refusé ». Les candidats proposés sont filtrés par la même
 * PERMISSION, jamais par un nom de rôle.
 *
 * ── LA COMPOSITION S'ENREGISTRE D'UN SEUL GESTE ─────────────────────────────
 *
 * Ajouts, retraits et plafonds partent ensemble : un comité se compose en le
 * regardant en entier, parce que répartir la charge suppose de voir tout le monde.
 * Trois appels séparés auraient permis d'enregistrer un plafond sur quelqu'un qu'on
 * vient de retirer.
 *
 * ── `workload_cap` VIDE SIGNIFIE « AUCUN PLAFOND », PAS « AUCUN DOSSIER » ────
 *
 * La colonne est nullable en base, et la confusion coûterait cher : afficher « 0 »
 * ferait croire qu'un membre n'a rien à évaluer. On écrit « non renseigné ».
 */

interface Props {
  detail: EditionDetail
  canManage: boolean
  busy?: boolean
  notice?: string | null
  /** Membres retirés qui portaient encore des dossiers, déjà nommés. */
  removedWithAssignments?: string[]
}

const props = defineProps<Props>()
const emit = defineEmits<{ save: [payload: CommitteePayload] }>()

const { t } = useI18n()

const call = computed(() => props.detail.call)

/** Brouillon local : la composition ne part qu'à l'enregistrement. */
type DraftMember = {
  person_id: string
  full_name: string
  email: string
  organization_name: string | null
  is_lead: boolean
  workload_cap: number | null
  assigned_count: number
  submitted_count: number
  has_review_permission: boolean
  added_at: string
}

const members = ref<DraftMember[]>([])

function reset(): void {
  members.value = props.detail.committee.map((member) => ({ ...member }))
}
reset()

watch(() => props.detail.committee, reset)

const dirty = computed(() => {
  const before = props.detail.committee
  if (before.length !== members.value.length) return true
  return members.value.some((member) => {
    const original = before.find((entry) => entry.person_id === member.person_id)
    return (
      !original ||
      original.is_lead !== member.is_lead ||
      original.workload_cap !== member.workload_cap
    )
  })
})

const columns = computed<TableColumn[]>(() => [
  { key: 'member', label: t('admin.event.tabs.committeeTab.columns.member') },
  { key: 'lead', label: t('admin.event.tabs.committeeTab.columns.lead'), width: '9rem' },
  { key: 'cap', label: t('admin.event.tabs.committeeTab.columns.cap'), width: '10rem' },
  { key: 'load', label: t('admin.event.tabs.committeeTab.columns.load'), hideBelow: 'lg' },
  { key: 'actions', label: t('admin.event.tabs.committeeTab.columns.actions'), width: '6rem' },
])

// ---------------------------------------------------------------------------
// Ajout
// ---------------------------------------------------------------------------

const addOpen = ref(false)
const candidateId = ref('')

const candidateOptions = computed<SelectOption[]>(() =>
  props.detail.committee_candidates
    .filter((candidate) => !members.value.some((m) => m.person_id === candidate.person_id))
    .map((candidate) => ({
      value: candidate.person_id,
      label: candidate.full_name,
      description: candidate.organization_name ?? candidate.email,
    })),
)

function addMember(): void {
  const candidate = props.detail.committee_candidates.find(
    (entry) => entry.person_id === candidateId.value,
  )
  if (!candidate) return
  members.value.push({
    person_id: candidate.person_id,
    full_name: candidate.full_name,
    email: candidate.email,
    organization_name: candidate.organization_name,
    is_lead: false,
    workload_cap: null,
    assigned_count: 0,
    submitted_count: 0,
    has_review_permission: candidate.has_review_permission,
    added_at: new Date().toISOString(),
  })
  candidateId.value = ''
  addOpen.value = false
}

function removeMember(personId: string): void {
  members.value = members.value.filter((member) => member.person_id !== personId)
}

/** Un seul président : le désigner le retire du précédent, sans le faire chercher. */
function setLead(personId: string, isLead: boolean): void {
  for (const member of members.value) {
    member.is_lead = isLead && member.person_id === personId
  }
}

function setCap(personId: string, raw: string): void {
  const member = members.value.find((entry) => entry.person_id === personId)
  if (member) member.workload_cap = raw ? Number(raw) : null
}

function submit(): void {
  if (!call.value) return
  emit('save', {
    call_id: call.value.id,
    members: members.value.map((member) => ({
      person_id: member.person_id,
      is_lead: member.is_lead,
      workload_cap: member.workload_cap,
    })),
  })
}
</script>

<template>
  <section>
    <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div class="min-w-0">
        <h2 class="font-display text-xl font-semibold">
          {{ t('admin.event.tabs.committeeTab.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.event.tabs.committeeTab.intro') }}
        </p>
      </div>

      <UiButton
        v-if="props.canManage && call"
        icon="plus"
        :disabled="props.busy || candidateOptions.length === 0"
        @click="addOpen = true"
      >
        {{ t('admin.event.tabs.committeeTab.add') }}
      </UiButton>
    </header>

    <!-- LE COMITÉ SE COMPOSE SUR UN APPEL : sans appel, il n'y a rien à composer. -->
    <UiEmptyState
      v-if="!call"
      class="mt-5"
      icon="users"
      :title="t('admin.event.tabs.committeeTab.noCall.title')"
      :description="t('admin.event.tabs.committeeTab.noCall.description')"
    />

    <template v-else>
      <UiAlert
        v-if="props.notice"
        class="mt-4"
        intent="success"
        live
        :message="props.notice"
      />

      <!-- Retirer quelqu'un qui portait des dossiers laisse ces dossiers sans
           lecteur : on nomme les personnes, pas un décompte. -->
      <UiAlert
        v-if="props.removedWithAssignments && props.removedWithAssignments.length > 0"
        class="mt-4"
        intent="warning"
        live
        :message="t('admin.event.tabs.committeeTab.removedWithAssignments', {
          names: props.removedWithAssignments.join(' · '),
        })"
      />

      <UiTable
        class="mt-5"
        :columns="columns"
        :rows="members"
        row-key="person_id"
        row-label-key="full_name"
        :caption="t('admin.event.tabs.committeeTab.title')"
        visually-hidden-caption
        :loading="props.busy"
      >
        <template #cell-member="{ row }">
          <div class="min-w-0">
            <p class="truncate font-medium text-text">{{ row.full_name }}</p>
            <p class="truncate text-xs text-text-muted">
              {{ row.organization_name ?? row.email }}
            </p>
            <!-- SIÉGER N'ACCORDE RIEN : sans ce signal, on confierait des dossiers
                 à quelqu'un qui se heurterait à un « accès refusé ». -->
            <p v-if="!row.has_review_permission" class="mt-0.5 text-xs text-danger">
              {{ t('admin.event.tabs.committeeTab.missingPermission') }}
            </p>
          </div>
        </template>

        <template #cell-lead="{ row }">
          <UiCheckbox
            :model-value="row.is_lead"
            :label="t('admin.event.tabs.committeeTab.lead')"
            :disabled="!props.canManage"
            @update:model-value="(next: boolean) => setLead(row.person_id, next)"
          />
        </template>

        <template #cell-cap="{ row }">
          <UiInput
            :model-value="row.workload_cap ?? ''"
            type="number"
            :min="1"
            size="sm"
            :label="t('admin.event.tabs.committeeTab.columns.cap')"
            hide-label
            hide-optional
            :placeholder="t('admin.event.tabs.committeeTab.capNone')"
            :disabled="!props.canManage"
            @update:model-value="(next: string) => setCap(row.person_id, next)"
          />
        </template>

        <template #cell-load="{ row }">
          <p class="text-sm text-text">
            {{ t('admin.event.tabs.committeeTab.load', {
              assigned: row.assigned_count,
              submitted: row.submitted_count,
            }) }}
          </p>
          <!-- Le plafond dépassé demande attention, ce n'est pas une erreur :
               jaune, jamais rouge. -->
          <p
            v-if="row.workload_cap !== null && row.assigned_count > row.workload_cap"
            class="text-xs text-warning"
          >
            {{ t('admin.event.tabs.committeeTab.overCap') }}
          </p>
        </template>

        <template #cell-actions="{ row }">
          <UiButton
            v-if="props.canManage"
            variant="ghost"
            size="sm"
            icon="trash"
            icon-only
            :label="t('admin.event.tabs.committeeTab.remove')"
            @click="removeMember(row.person_id)"
          />
        </template>

        <template #empty>
          <UiEmptyState
            icon="users"
            :title="t('admin.event.tabs.committeeTab.empty.title')"
            :description="t('admin.event.tabs.committeeTab.empty.description')"
          />
        </template>
      </UiTable>

      <div v-if="props.canManage" class="mt-4 flex flex-wrap items-center gap-3">
        <UiButton :disabled="!dirty" :loading="props.busy" @click="submit">
          {{ t('common.actions.save') }}
        </UiButton>
        <UiButton variant="ghost" :disabled="!dirty || props.busy" @click="reset">
          {{ t('common.actions.reset') }}
        </UiButton>
        <p v-if="dirty" class="text-sm text-warning">
          {{ t('admin.event.tabs.committeeTab.unsaved') }}
        </p>
        <p v-if="props.detail.committee.length > 0" class="ml-auto text-xs text-text-subtle">
          {{ t('admin.event.tabs.committeeTab.capHint') }}
        </p>
      </div>
    </template>

    <UiModal
      v-model:open="addOpen"
      :title="t('admin.event.tabs.committeeTab.addTitle')"
    >
      <UiSelect
        v-if="candidateOptions.length > 0"
        :model-value="candidateId"
        :label="t('admin.event.tabs.committeeTab.addField')"
        :hint="t('admin.event.tabs.committeeTab.addHint')"
        :options="candidateOptions"
        :placeholder="t('common.actions.select')"
        @update:model-value="(next: string) => (candidateId = next)"
      />
      <p v-else class="text-sm text-text-muted">
        {{ t('admin.event.tabs.committeeTab.noCandidates') }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="addOpen = false">
          {{ t('admin.event.tabs.confirm.cancel') }}
        </UiButton>
        <UiButton :disabled="!candidateId" @click="addMember">
          {{ t('common.actions.add') }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>
