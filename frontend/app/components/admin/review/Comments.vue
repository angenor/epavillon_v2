<script setup lang="ts">
import type { PostCommentPayload } from '~/types/admin-review'
import type { Person } from '~/types/identity'
import type { CommentVisibility, ProposalComment } from '~/types/programme/proposal'
import type { PersonId, TimeZoneName } from '~/types/shared'

/**
 * LES ÉCHANGES AUTOUR DU DOSSIER — et la distinction qui coûte le plus cher de
 * tout cet écran : « interne au comité » contre « partagé avec le
 * soumissionnaire ».
 *
 * SE TROMPER DE VISIBILITÉ EST IRRATTRAPABLE. Un message envoyé à l'organisation
 * ne se retire pas : il est notifié par courriel, lu, parfois transféré. La
 * remarque « dossier faible, à écarter si on manque de créneaux », écrite pour
 * le comité et partie chez le déposant, ne se répare par aucune fonction.
 * L'écran rend donc l'erreur DIFFICILE, par quatre moyens qui se cumulent :
 *
 *  1. LE FOND CHANGE. Un message interne est sur fond creusé, un message partagé
 *     sur fond bleuté et bordé — on distingue les deux fils d'un coup d'œil,
 *     sans lire une étiquette.
 *  2. LE LIBELLÉ EST EXPLICITE, jamais un pictogramme seul : « Interne au comité »
 *     et « Partagé avec le soumissionnaire », en toutes lettres, sur chaque
 *     message et sur le champ de saisie.
 *  3. LE DÉFAUT EST LE PLUS SÛR. La rédaction s'ouvre sur « interne au comité » :
 *     un défaut partagé enverrait dehors le premier message écrit sans y penser.
 *  4. LE PREMIER ENVOI PARTAGÉ DEMANDE CONFIRMATION, en nommant l'organisation
 *     destinataire. Une seule fois par visite : une confirmation qui revient à
 *     chaque message finit par se cliquer sans être lue, ce qui la rend nuisible.
 *
 * LA TROISIÈME VISIBILITÉ EST LA NOTE PERSONNELLE (`private`), visible de son
 * seul auteur. Elle est offerte parce que le modèle la porte et qu'elle sert :
 * on y note ce qu'on doit vérifier avant de trancher.
 *
 * UNE DEMANDE DE CORRECTION EST TOUJOURS PARTAGÉE. Cocher la case bascule la
 * visibilité et la verrouille : une demande que le déposant ne verrait pas
 * bloquerait son dossier sans qu'il sache pourquoi.
 */

interface Props {
  comments: ProposalComment[]
  participants: Person[]
  viewerId: PersonId | null
  /** L'organisation porteuse, nommée dans la confirmation d'envoi partagé. */
  leadOrganizationName: string
  timezone: TimeZoneName
  canWrite: boolean
  busy?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  post: [payload: Omit<PostCommentPayload, 'proposal_id'>]
}>()

const { t } = useI18n()
const { dateTime } = useDateTime()

// ---------------------------------------------------------------------------
// Le fil
// ---------------------------------------------------------------------------

/** Les racines, dans l'ordre où elles ont été écrites — une discussion se lit
 *  dans le sens où elle s'est tenue. */
const threads = computed(() =>
  props.comments
    .filter((comment) => comment.parent_id === null)
    .map((root) => ({
      root,
      replies: props.comments.filter((comment) => comment.parent_id === root.id),
    })),
)

function authorOf(comment: ProposalComment): string {
  if (comment.author_id === props.viewerId) return t('admin.proposal.review.comments.you')
  const person = props.participants.find((entry) => entry.id === comment.author_id)
  return person?.display_name ?? ''
}

/**
 * LA COULEUR D'UN MESSAGE DIT SON DESTINATAIRE, et rien d'autre — ce n'est ni
 * une décoration ni un état d'avancement. Bleuté et bordé pour ce qui sort de
 * l'IFDD, creusé pour ce qui reste entre nous, neutre pour une note personnelle.
 */
const SURFACE: Record<CommentVisibility, string> = {
  submitter: 'border-info-border bg-info-surface',
  committee: 'border-border bg-surface-sunken',
  private: 'border-dashed border-border bg-surface',
}

const BADGE_INTENT: Record<CommentVisibility, 'info' | 'neutral'> = {
  submitter: 'info',
  committee: 'neutral',
  private: 'neutral',
}

// ---------------------------------------------------------------------------
// Rédaction
// ---------------------------------------------------------------------------

const VISIBILITIES: CommentVisibility[] = ['committee', 'submitter', 'private']

// Le défaut est le plus sûr des trois : ce qui part dehors se choisit.
const visibility = ref<CommentVisibility>('committee')
const body = ref('')
const isChangeRequest = ref(false)
const replyingTo = ref<string | null>(null)
const bodyError = ref('')

/** La confirmation d'envoi partagé n'est demandée qu'une fois par visite. */
const sharedConfirmed = ref(false)
const confirmOpen = ref(false)
const confirmChecked = ref(false)

const visibilityOptions = computed(() =>
  VISIBILITIES.map((value) => ({
    value,
    label: t(`admin.proposal.review.comments.visibility.${value}.label`),
    description: t(`admin.proposal.review.comments.visibility.${value}.hint`),
    // Une demande de correction est forcément partagée : les deux autres
    // choix sont fermés tant que la case est cochée.
    disabled: isChangeRequest.value && value !== 'submitter',
  })),
)

watch(isChangeRequest, (value) => {
  if (value) visibility.value = 'submitter'
})

function reset(): void {
  body.value = ''
  isChangeRequest.value = false
  visibility.value = 'committee'
  replyingTo.value = null
  bodyError.value = ''
}

function openReply(commentId: string, parentVisibility: CommentVisibility): void {
  replyingTo.value = commentId
  body.value = ''
  bodyError.value = ''
  // Répondre dans un fil garde le fil : une réponse « interne » sous un message
  // partagé casserait la lecture de la discussion pour le déposant.
  visibility.value = parentVisibility === 'private' ? 'committee' : parentVisibility
  isChangeRequest.value = false
}

function requestSend(): void {
  const text = body.value.trim()
  if (!text) {
    bodyError.value = t('admin.proposal.review.comments.compose.empty')
    return
  }
  if (visibility.value === 'submitter' && !sharedConfirmed.value) {
    confirmChecked.value = false
    confirmOpen.value = true
    return
  }
  send()
}

function send(): void {
  emit('post', {
    parent_id: replyingTo.value,
    visibility: visibility.value,
    body: body.value.trim(),
    is_change_request: isChangeRequest.value,
  })
  if (visibility.value === 'submitter') sharedConfirmed.value = true
  confirmOpen.value = false
  reset()
}
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised" aria-labelledby="review-comments-title">
    <header class="border-b border-border-subtle px-5 py-4">
      <h2 id="review-comments-title" class="text-lg font-semibold">
        {{ t('admin.proposal.review.comments.title') }}
      </h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.proposal.review.comments.description') }}
      </p>
    </header>

    <div class="flex flex-col gap-4 p-5">
      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

      <p v-if="threads.length === 0" class="text-sm text-text-muted">
        {{ t('admin.proposal.review.comments.empty') }}
      </p>

      <ol v-else class="flex flex-col gap-4">
        <li
          v-for="thread in threads"
          :key="thread.root.id"
          class="rounded-md border"
          :class="SURFACE[thread.root.visibility]"
        >
          <article class="p-4">
            <header class="flex flex-wrap items-center gap-x-3 gap-y-1">
              <span class="font-semibold text-text">{{ authorOf(thread.root) }}</span>
              <time :datetime="thread.root.created_at" class="text-sm text-text-subtle">
                {{ dateTime(thread.root.created_at, props.timezone) }}
              </time>
              <span v-if="thread.root.edited_at" class="text-xs text-text-subtle">
                · {{ t('admin.proposal.review.comments.edited') }}
              </span>

              <!-- LE LIBELLÉ DE VISIBILITÉ, EN TOUTES LETTRES, sur chaque
                   message. C'est la seule chose qui empêche de lire un fil
                   interne comme une discussion avec l'organisation. -->
              <UiBadge
                size="sm"
                :intent="BADGE_INTENT[thread.root.visibility]"
                :icon="thread.root.visibility === 'private' ? 'eye-off' : 'users'"
                :label="t(`admin.proposal.review.comments.visibility.${thread.root.visibility}.label`)"
              />
              <UiBadge
                v-if="thread.root.is_change_request"
                size="sm"
                intent="warning"
                icon="warning"
                :label="t('admin.proposal.review.comments.changeRequest')"
              />
            </header>

            <p class="mt-3 max-w-(--measure) whitespace-pre-line text-text-secondary">
              {{ thread.root.body }}
            </p>

            <p v-if="thread.root.is_change_request" class="mt-2 text-sm">
              <span v-if="thread.root.resolved_at" class="text-success">
                {{
                  t('admin.proposal.review.comments.resolved', {
                    date: dateTime(thread.root.resolved_at, props.timezone),
                  })
                }}
              </span>
              <span v-else class="text-warning">
                {{ t('admin.proposal.review.comments.unresolved') }}
              </span>
            </p>
          </article>

          <ol v-if="thread.replies.length > 0" class="flex flex-col border-t border-border-subtle">
            <li
              v-for="reply in thread.replies"
              :key="reply.id"
              class="border-b border-border-subtle px-4 py-3 last:border-b-0 sm:pl-9"
            >
              <header class="flex flex-wrap items-center gap-x-3 gap-y-1">
                <span class="font-semibold text-text">{{ authorOf(reply) }}</span>
                <time :datetime="reply.created_at" class="text-sm text-text-subtle">
                  {{ dateTime(reply.created_at, props.timezone) }}
                </time>
              </header>
              <p class="mt-2 max-w-(--measure) whitespace-pre-line text-text-secondary">{{ reply.body }}</p>
            </li>
          </ol>

          <footer v-if="props.canWrite" class="border-t border-border-subtle px-4 py-2">
            <UiButton
              v-if="replyingTo !== thread.root.id"
              variant="ghost"
              size="sm"
              icon="mail"
              @click="openReply(thread.root.id, thread.root.visibility)"
            >
              {{ t('admin.proposal.review.comments.compose.reply') }}
            </UiButton>
            <p v-else class="py-1 text-sm text-text-muted">
              {{ t('admin.proposal.review.comments.compose.label') }} ↓
            </p>
          </footer>
        </li>
      </ol>

      <!-- RÉDACTION. Le choix du destinataire est AU-DESSUS du champ, pas
           au-dessous d'un bouton d'envoi : on décide à qui l'on parle avant
           d'écrire, pas après. -->
      <div
        v-if="props.canWrite"
        class="rounded-md border p-4"
        :class="SURFACE[visibility]"
      >
        <UiRadio
          v-model="visibility"
          :label="t('admin.proposal.review.comments.visibility.label')"
          :options="visibilityOptions"
          :disabled="props.busy"
        />

        <UiTextarea
          v-model="body"
          class="mt-4"
          :label="t('admin.proposal.review.comments.compose.label')"
          :placeholder="t('admin.proposal.review.comments.compose.placeholder')"
          :error="bodyError || undefined"
          :rows="3"
          auto-grow
          block
          :disabled="props.busy"
        />

        <UiCheckbox
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
          <UiButton v-if="replyingTo || body" variant="ghost" :disabled="props.busy" @click="reset()">
            {{ t('admin.proposal.review.comments.compose.cancel') }}
          </UiButton>
        </div>
      </div>
    </div>

    <!-- LA CONFIRMATION DU PREMIER ENVOI PARTAGÉ. Elle NOMME l'organisation
         destinataire : « ce message part chez … » est ce qui arrête la main,
         pas « êtes-vous sûr ? ». La case à cocher force une seconde lecture. -->
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
        <UiCheckbox
          v-model="confirmChecked"
          :label="t('admin.proposal.review.comments.confirm.checkbox')"
        />
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
  </section>
</template>
