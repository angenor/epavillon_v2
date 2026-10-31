<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { AttachableRoleRule } from '~/types/media'
import type { DraftIssue, DraftSpeaker, ProposalDraft } from '~/types/proposal-form'
import type { AssetId } from '~/types/shared'
import type { OrganizationEditBlockedReason } from '~/utils/proposal-edit'

/**
 * LES TEXTES DE L'IMAGE, MODIFIABLES (31/10) : le titre et les intervenants,
 * avec leur photo. Ils valent pour le dossier ET pour la page de l'activité
 * (arbitrage du commanditaire) : l'éditeur écrit le dossier, l'API recopie sur
 * les séances. La date et le lieu restent ceux que l'équipe a fixés.
 *
 * La liste est celle du dossier (`ProposalStepSpeakers`, variante compacte) et
 * sa fenêtre d'intervenant aussi : un seul geste, une seule implémentation. La
 * photo se pose sous la vignette, sur la ligne enregistrée ; un intervenant
 * ajouté ici la reçoit après l'enregistrement.
 */

interface Props {
  call: CallForProposals | null
  issues: DraftIssue[]
  /** Nul : modifiable. Sinon, le motif dit à l'écran. */
  blockedReason: OrganizationEditBlockedReason | 'draft' | null
  /** Le dossier tel qu'enregistré, montré quand il ne se modifie pas. */
  saved: ProposalDraft | null
  dateLine: string | null
  placeLine: string | null
  photoRule: AttachableRoleRule | null
  canSetPhoto: boolean
  draftTo: string
}

const props = defineProps<Props>()
const working = defineModel<ProposalDraft | null>({ required: true })
const emit = defineEmits<{ photoSet: [] }>()

const { t } = useI18n()
const api = useApi()

const photoError = ref<string | null>(null)
const titleIssue = computed(() => props.issues.find((issue) => issue.field === 'title' && issue.severity === 'error'))

async function attachPhoto(speaker: DraftSpeaker, assetId: AssetId | null): Promise<void> {
  if (!assetId || !speaker.speaker_id) return
  photoError.value = null
  try {
    await api.media.attach({
      owner_schema: 'programme',
      owner_table: 'proposal_speakers',
      owner_id: speaker.speaker_id,
      assignments: [{ role: 'avatar', asset_id: assetId }],
    })
    emit('photoSet')
  } catch (thrown) {
    photoError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

function updateTitle(value: string | number): void {
  if (working.value) working.value = { ...working.value, title: String(value) }
}
</script>

<template>
  <div class="flex flex-col gap-5">
    <div>
      <h2 class="text-lg font-bold">{{ t('organization.workspace.cover.tools.texts') }}</h2>
      <p class="mt-1 text-sm text-text-muted">{{ t('organization.workspace.cover.texts.hint') }}</p>
    </div>

    <UiAlert
      v-if="props.blockedReason"
      :message="
        props.blockedReason === 'draft'
          ? t('organization.workspace.cover.texts.draft')
          : t(`organization.workspace.proposal.tracking.editBlocked.${props.blockedReason}`)
      "
    >
      <UiButton v-if="props.blockedReason === 'draft'" variant="link" :to="props.draftTo">
        {{ t('organization.workspace.proposal.tracking.editDraft') }}
      </UiButton>
    </UiAlert>

    <UiInput
      v-if="working"
      id="cover-text-title"
      :model-value="working.title"
      :label="t('organization.workspace.cover.texts.title')"
      :error="titleIssue ? t(titleIssue.messageKey, titleIssue.params ?? {}) : undefined"
      required
      @update:model-value="updateTitle"
    />
    <div v-else-if="props.saved">
      <p class="text-xs text-text-muted">{{ t('organization.workspace.cover.texts.title') }}</p>
      <p class="mt-0.5 font-medium">{{ props.saved.title }}</p>
    </div>

    <div v-if="props.dateLine || props.placeLine" class="rounded-md bg-surface-sunken px-3.5 py-3 text-sm">
      <template v-if="props.dateLine">
        <p class="text-xs text-text-muted">{{ t('organization.workspace.cover.texts.date') }}</p>
        <p class="font-medium">{{ props.dateLine }}</p>
      </template>
      <template v-if="props.placeLine">
        <p class="mt-2 text-xs text-text-muted">{{ t('organization.workspace.cover.texts.place') }}</p>
        <p class="font-medium">{{ props.placeLine }}</p>
      </template>
      <p class="mt-2 text-xs text-text-secondary">{{ t('organization.workspace.cover.texts.fixedByTeam') }}</p>
    </div>

    <div v-if="working && props.call" class="flex flex-col gap-2">
      <h3 class="text-sm font-semibold">{{ t('organization.workspace.cover.texts.speakers') }}</h3>
      <ProposalStepSpeakers v-model="working" :call="props.call" :issues="props.issues" compact>
        <template #photo="{ speaker, fullName }">
          <div class="flex shrink-0 flex-col items-center gap-1">
            <UiImage
              v-if="speaker.photo_image"
              :image="speaker.photo_image"
              ratio="1 / 1"
              sizes="48px"
              rounded="rounded-full"
              class="w-12"
              aria-hidden="true"
            />
            <span
              v-else
              aria-hidden="true"
              class="grid size-12 place-items-center rounded-full border-2 border-dashed border-border-strong text-sm font-bold text-text-secondary"
            >
              {{ initialsOf(`${speaker.first_name} ${speaker.last_name}`) }}
            </span>
            <MediaImageField
              v-if="props.canSetPhoto && speaker.speaker_id"
              role="avatar"
              :asset-id="null"
              :label="t('organization.workspace.proposal.speakers.photoField', { name: fullName })"
              :button-label="t(speaker.photo_image ? 'organization.workspace.cover.texts.changePhoto' : 'organization.workspace.cover.texts.addPhoto')"
              button-variant="link"
              :rule="props.photoRule"
              :owner="{ schema: 'programme', table: 'proposal_speakers', id: speaker.speaker_id }"
              @update:asset-id="(assetId) => attachPhoto(speaker, assetId)"
            />
            <span v-else-if="props.canSetPhoto" class="max-w-16 text-center text-[0.6875rem] leading-tight text-text-secondary">
              {{ t('organization.workspace.cover.texts.photoAfterSave') }}
            </span>
          </div>
        </template>
      </ProposalStepSpeakers>
      <p class="text-xs text-text-secondary">{{ t('organization.workspace.cover.texts.orderHint') }}</p>
    </div>
    <div v-else-if="props.saved?.speakers.length">
      <p class="text-xs text-text-muted">{{ t('organization.workspace.cover.texts.speakers') }}</p>
      <p class="mt-0.5 font-medium">{{ props.saved.speakers.map((speaker) => speakerName(speaker)).join(', ') }}</p>
    </div>

    <UiAlert v-if="photoError" intent="danger" :message="photoError" live />
  </div>
</template>
