<script setup lang="ts">
import type { PlannerScreen, PlannerSession } from '~/types/admin-planner'
import type { EventId, ProposalId } from '~/types/shared'

/**
 * RÉUNIR OU SÉPARER une séance commune. L'écriture se fait ici, comme la
 * suppression d'un envoi : la fenêtre reste ouverte sur un refus de l'API, dont
 * le message s'affiche tel quel (des inscrits seraient perdus, un dossier est
 * déjà réuni).
 */
interface Props {
  mode: 'join' | 'split' | null
  /** La séance d'où part le geste : son créneau devient celui de la séance commune. */
  session: PlannerSession | null
  /** Toutes les séances de l'édition, pour proposer les activités réunissables. */
  sessions: PlannerSession[]
  eventId: EventId | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ close: []; done: [screen: PlannerScreen, notice: string] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()

const busy = ref(false)
const error = ref<string | null>(null)
const titleFr = ref('')
const titleEn = ref('')
/** Dans l'ordre où l'équipe coche : c'est l'ordre d'affichage public. */
const picked = ref<ProposalId[]>([])

watch(
  () => [props.mode, props.session?.id] as const,
  () => {
    error.value = null
    picked.value = []
    titleFr.value = props.session ? tr(props.session.title) : ''
    titleEn.value = ''
  },
)

const open = computed({
  get: () => props.mode !== null && props.session !== null,
  set: (value: boolean) => {
    if (!value && !busy.value) emit('close')
  },
})

/** Activités retenues, avec un dossier propre et pas déjà réunies. */
const candidates = computed(() =>
  props.sessions.filter(
    (s) =>
      s.id !== props.session?.id &&
      s.proposal_id !== null &&
      s.proposal_status === 'accepted' &&
      s.joint_proposals.length === 0 &&
      s.status !== 'cancelled',
  ),
)

function toggle(proposalId: ProposalId, checked: boolean): void {
  picked.value = checked
    ? [...picked.value.filter((id) => id !== proposalId), proposalId]
    : picked.value.filter((id) => id !== proposalId)
}

function rank(proposalId: ProposalId): number | null {
  const index = picked.value.indexOf(proposalId)
  return index < 0 ? null : index + 2
}

async function join(): Promise<void> {
  const session = props.session
  if (!session?.proposal_id || !props.eventId) return
  if (!titleFr.value.trim()) {
    error.value = t('admin.planner.joint.titleRequired')
    return
  }
  if (!picked.value.length) {
    error.value = t('admin.planner.joint.pickOne')
    return
  }
  busy.value = true
  error.value = null
  try {
    const title = titleEn.value.trim()
      ? { fr: titleFr.value.trim(), en: titleEn.value.trim() }
      : { fr: titleFr.value.trim() }
    const screen = await api.planner.join({
      event_id: props.eventId,
      proposal_ids: [session.proposal_id, ...picked.value],
      title,
    })
    emit('done', screen, t('admin.planner.joint.joined', { title: title.fr }))
  } catch (thrown) {
    error.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}

async function split(): Promise<void> {
  const session = props.session
  if (!session || !props.eventId) return
  busy.value = true
  error.value = null
  try {
    const screen = await api.planner.split(props.eventId, session.id)
    emit('done', screen, t('admin.planner.joint.separated'))
  } catch (thrown) {
    error.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <UiModal
    v-model:open="open"
    size="lg"
    :title="t(props.mode === 'split' ? 'admin.planner.joint.splitTitle' : 'admin.planner.joint.joinTitle')"
    :description="props.session
      ? t(props.mode === 'split' ? 'admin.planner.joint.splitDescription' : 'admin.planner.joint.joinDescription', { title: tr(props.session.title) })
      : undefined"
  >
    <div v-if="props.mode === 'join'" class="space-y-4">
      <UiInput v-model="titleFr" :label="t('admin.planner.joint.titleFr')" required :disabled="busy" />
      <UiInput v-model="titleEn" :label="t('admin.planner.joint.titleEn')" :disabled="busy" />

      <fieldset class="space-y-2">
        <legend class="text-sm font-semibold text-text">{{ t('admin.planner.joint.candidates') }}</legend>
        <p class="text-xs text-text-muted">{{ t('admin.planner.joint.candidatesHint') }}</p>
        <p v-if="!candidates.length" class="text-sm text-text-secondary">
          {{ t('admin.planner.joint.noCandidates') }}
        </p>
        <ul v-else class="max-h-72 space-y-1 overflow-y-auto">
          <li v-for="candidate in candidates" :key="candidate.id" class="flex items-start gap-2">
            <span class="mt-3 w-6 shrink-0 text-right text-sm font-semibold text-accent tabular-nums">
              {{ candidate.proposal_id && rank(candidate.proposal_id) ? `${rank(candidate.proposal_id)}.` : '' }}
            </span>
            <UiCheckbox
              class="min-w-0 flex-1"
              :model-value="candidate.proposal_id ? picked.includes(candidate.proposal_id) : false"
              :label="tr(candidate.title)"
              :hint="[candidate.reference_code, candidate.organization_acronym || candidate.organization_name]
                .filter(Boolean)
                .join(' · ')"
              :disabled="busy"
              @update:model-value="(checked: boolean) => candidate.proposal_id && toggle(candidate.proposal_id, checked)"
            />
          </li>
        </ul>
      </fieldset>
    </div>

    <ol v-else-if="props.session" class="space-y-1 text-sm">
      <li v-for="part in props.session.joint_proposals" :key="part.proposal_id" class="flex gap-2">
        <span class="font-semibold text-accent tabular-nums">{{ part.sort_order }}.</span>
        <span class="min-w-0">
          {{ tr(part.title) }}
          <span class="text-text-muted">· {{ part.organization_acronym || part.organization_name }}</span>
        </span>
      </li>
    </ol>

    <UiAlert v-if="error" class="mt-4" intent="danger" live :message="error" />

    <template #footer>
      <UiButton variant="secondary" :disabled="busy" @click="emit('close')">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton v-if="props.mode === 'join'" icon="users" :loading="busy" @click="join">
        {{ t('admin.planner.joint.confirmJoin') }}
      </UiButton>
      <UiButton v-else variant="danger" :loading="busy" @click="split">
        {{ t('admin.planner.joint.confirmSplit') }}
      </UiButton>
    </template>
  </UiModal>
</template>
