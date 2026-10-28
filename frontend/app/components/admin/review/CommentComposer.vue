<script setup lang="ts">
import type { PostCommentPayload } from '~/types/admin-review'
import type { CommentPerson, CommentVisibility, ProposalCommentView } from '~/types/programme/proposal'
import type { PersonId } from '~/types/shared'

/**
 * RÉDIGER, et choisir d'abord à qui l'on parle.
 *
 * Se tromper de visibilité est irrattrapable : un message parti chez
 * l'organisation est notifié, lu, parfois transféré. D'où le défaut le plus sûr
 * (« interne au comité »), la surface qui change de couleur, et la confirmation
 * du premier envoi partagé, une fois par visite (répétée, elle se clique sans
 * être lue).
 *
 * Le champ « À » n'existe que pour un message du comité qui ouvre un fil : une
 * réponse reste dans le cercle de son fil, une note personnelle n'a qu'un
 * lecteur, et le partagé va à toute l'organisation.
 */

interface Props {
  /** La racine du fil auquel on répond. */
  replyTo: ProposalCommentView | null
  /** Qui lira la réponse, quand le fil est restreint. */
  replyCircle: string | null
  audience: CommentPerson[]
  leadOrganizationName: string
  submit: (payload: Omit<PostCommentPayload, 'proposal_id'>) => Promise<boolean>
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ cancel: [] }>()

const { t } = useI18n()

const SURFACE: Record<CommentVisibility, string> = {
  submitter: 'border-info-border bg-info-surface',
  committee: 'border-border bg-surface-sunken',
  private: 'border-dashed border-border bg-surface',
}

const VISIBILITIES: CommentVisibility[] = ['committee', 'submitter', 'private']

const visibility = ref<CommentVisibility>('committee')
const body = ref('')
const isChangeRequest = ref(false)
const recipients = ref<PersonId[]>([])
const bodyError = ref('')

const sharedConfirmed = ref(false)
const confirmOpen = ref(false)
const confirmChecked = ref(false)

const restrictedReply = computed(() => (props.replyTo?.recipients.length ?? 0) > 0)
const canAddress = computed(
  () => visibility.value === 'committee' && props.replyTo === null && !isChangeRequest.value,
)

const visibilityOptions = computed(() =>
  VISIBILITIES.map((value) => ({
    value,
    label: t(`admin.proposal.review.comments.visibility.${value}.label`),
    description: t(`admin.proposal.review.comments.visibility.${value}.hint`),
    // Une demande de correction est forcément partagée ; une réponse à un fil
    // restreint reste dans son cercle.
    disabled:
      (isChangeRequest.value && value !== 'submitter') || (restrictedReply.value && value !== 'committee'),
  })),
)

watch(isChangeRequest, (value) => {
  if (value) visibility.value = 'submitter'
})

watch(
  () => props.replyTo,
  (root) => {
    bodyError.value = ''
    isChangeRequest.value = false
    recipients.value = []
    // Répondre garde le fil : une réponse « interne » sous un message partagé
    // casserait la lecture de la discussion pour le déposant.
    if (root) visibility.value = root.visibility === 'private' ? 'committee' : root.visibility
  },
)

function addMentioned(person: CommentPerson): void {
  if (!recipients.value.includes(person.person_id)) recipients.value = [...recipients.value, person.person_id]
}

function reset(): void {
  body.value = ''
  isChangeRequest.value = false
  visibility.value = 'committee'
  recipients.value = []
  bodyError.value = ''
}

function cancel(): void {
  reset()
  emit('cancel')
}

function requestSend(): void {
  if (!body.value.trim()) {
    bodyError.value = t('admin.proposal.review.comments.compose.empty')
    return
  }
  if (visibility.value === 'submitter' && !sharedConfirmed.value) {
    confirmChecked.value = false
    confirmOpen.value = true
    return
  }
  void send()
}

async function send(): Promise<void> {
  const shared = visibility.value === 'submitter'
  const ok = await props.submit({
    parent_id: props.replyTo?.id ?? null,
    visibility: visibility.value,
    body: body.value.trim(),
    is_change_request: isChangeRequest.value,
    ...(canAddress.value && recipients.value.length > 0 ? { recipient_ids: recipients.value } : {}),
  })
  confirmOpen.value = false
  if (!ok) return
  if (shared) sharedConfirmed.value = true
  cancel()
}
</script>

<template>
  <div class="rounded-md border p-4" :class="SURFACE[visibility]">
    <p v-if="props.replyTo" class="mb-3 text-sm font-semibold text-text">
      {{ t('admin.proposal.review.comments.compose.replying') }}
    </p>

    <UiRadio
      v-model="visibility"
      :label="t('admin.proposal.review.comments.visibility.label')"
      :options="visibilityOptions"
      :disabled="props.busy"
    />

    <AdminReviewCommentRecipients
      v-if="canAddress"
      v-model="recipients"
      class="mt-4"
      :audience="props.audience"
      :label="t('admin.proposal.review.comments.recipients.label')"
      :search-label="t('admin.proposal.review.comments.recipients.search')"
      :hint="t('admin.proposal.review.comments.recipients.hint')"
      :disabled="props.busy"
    />

    <p v-else-if="props.replyCircle" class="mt-4 text-sm text-text-muted">
      {{ t('admin.proposal.review.comments.recipients.replyCircle', { names: props.replyCircle }) }}
    </p>

    <AdminReviewCommentMentionField
      v-model="body"
      class="mt-4"
      :people="props.audience"
      :enabled="canAddress"
      :label="t('admin.proposal.review.comments.compose.label')"
      :placeholder="t('admin.proposal.review.comments.compose.placeholder')"
      :hint="canAddress ? t('admin.proposal.review.comments.mention.hint') : undefined"
      :error="bodyError || undefined"
      :disabled="props.busy"
      @mention="addMentioned"
    />

    <!-- Une demande de correction part chez l'organisation : elle n'a pas sa place dans un fil restreint. -->
    <UiCheckbox
      v-if="!restrictedReply"
      v-model="isChangeRequest"
      class="mt-3"
      :label="t('admin.proposal.review.comments.compose.changeRequest')"
      :hint="t('admin.proposal.review.comments.compose.changeRequestHint')"
      :disabled="props.busy"
    />

    <div class="mt-4 flex flex-wrap gap-2">
      <UiButton
        :variant="visibility === 'submitter' ? 'primary' : 'secondary'"
        :loading="props.busy"
        @click="requestSend()"
      >
        {{ t('admin.proposal.review.comments.compose.send') }}
      </UiButton>
      <UiButton v-if="props.replyTo || body" variant="ghost" :disabled="props.busy" @click="cancel()">
        {{ t('admin.proposal.review.comments.compose.cancel') }}
      </UiButton>
    </div>

    <!-- La confirmation NOMME l'organisation destinataire : « ce message part
         chez … » arrête la main, pas « êtes-vous sûr ? ». -->
    <UiModal
      :open="confirmOpen"
      :title="t('admin.proposal.review.comments.confirm.title')"
      size="md"
      @update:open="(value: boolean) => (confirmOpen = value)"
    >
      <div class="space-y-4">
        <UiAlert
          intent="warning"
          :message="
            t('admin.proposal.review.comments.confirm.description', {
              organization: props.leadOrganizationName,
            })
          "
        />
        <blockquote class="rounded-md border border-info-border bg-info-surface p-3 text-sm whitespace-pre-line text-text-secondary">
          {{ body }}
        </blockquote>
        <UiCheckbox v-model="confirmChecked" :label="t('admin.proposal.review.comments.confirm.checkbox')" />
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="confirmOpen = false">
          {{ t('admin.proposal.review.comments.confirm.back') }}
        </UiButton>
        <UiButton variant="primary" :disabled="!confirmChecked" :loading="props.busy" @click="send()">
          {{ t('admin.proposal.review.comments.confirm.submit') }}
        </UiButton>
      </template>
    </UiModal>
  </div>
</template>
