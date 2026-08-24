<script setup lang="ts">
import type { DisplayedPerson } from '~/types/organization-workspace'
import type { ProposalComment } from '~/types/programme/proposal'
import type { PersonId } from '~/types/shared'

/**
 * LE FIL D'ÉCHANGES AVEC LE COMITÉ — demandes de correction, réponses,
 * marquage « résolu ».
 *
 * CE QU'ON VOIT ICI EST CE QUI NOUS EST ADRESSÉ, et rien d'autre. Le modèle
 * porte trois visibilités (`programme.comment_visibility`) : `committee` pour ce
 * que le comité s'écrit, `private` pour les notes personnelles, `submitter` pour
 * ce qui est partagé avec l'organisation. Le filtre est appliqué à la SOURCE et
 * non ici — un composant ne doit pas être le dernier rempart entre une note
 * interne et le déposant.
 *
 * LA DEMANDE DE CORRECTION EST LA RACINE D'UN FIL, jamais un message parmi
 * d'autres : c'est elle qui pilote l'état `changes_requested`, et sa résolution
 * est ce que le comité attend. Les réponses se rangent sous elle, en retrait, et
 * l'ordre de lecture est CHRONOLOGIQUE — la discussion se lit dans le sens où
 * elle s'est tenue, pas du plus récent au plus ancien.
 *
 * « RÉSOLU » SE POSE ET SE RETIRE. Le modèle porte `resolved_at` et
 * `resolved_by` sans dire qui les écrit : l'écran l'ouvre au soumissionnaire —
 * lui seul sait qu'il a corrigé — et permet de revenir en arrière. Une case
 * cochée trop vite ne doit pas exiger un courriel à l'IFDD pour être décochée.
 */

interface Props {
  comments: ProposalComment[]
  participants: DisplayedPerson[]
  /** Personne connectée — pour distinguer ses propres messages. */
  viewerId: PersonId
  /** Une écriture est en vol : les boutons attendent. */
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  reply: [parentId: string, body: string]
  resolve: [commentId: string, resolved: boolean]
}>()

const { t } = useI18n()
const { dateTime } = useDateTime()

/** Fuseau d'affichage : celui de la personne, pour un échange de messages. */
const viewerTimezone = computed(
  () => Intl.DateTimeFormat().resolvedOptions().timeZone || 'Europe/Paris',
)

/** Les racines, dans l'ordre où elles ont été écrites. */
const threads = computed(() =>
  props.comments
    .filter((comment) => comment.parent_id === null)
    .map((root) => ({
      root,
      replies: props.comments.filter((comment) => comment.parent_id === root.id),
    })),
)

const openRequests = computed(
  () => threads.value.filter((thread) => thread.root.is_change_request && thread.root.resolved_at === null).length,
)

function authorOf(comment: ProposalComment): string {
  if (comment.author_id === props.viewerId) return t('organization.workspace.proposal.exchanges.you')
  const person = props.participants.find((p) => p.id === comment.author_id)
  return person ? `${person.first_name} ${person.last_name}` : ''
}

// --- Rédaction d'une réponse ------------------------------------------------
//
// Un seul champ ouvert à la fois : deux zones de saisie simultanées dans un fil
// font perdre ce qu'on écrivait dans l'autre.
const replyingTo = ref<string | null>(null)
const draft = ref('')
const draftError = ref('')

function openReply(commentId: string): void {
  replyingTo.value = commentId
  draft.value = ''
  draftError.value = ''
}

function cancelReply(): void {
  replyingTo.value = null
  draft.value = ''
  draftError.value = ''
}

function submitReply(parentId: string): void {
  const body = draft.value.trim()
  if (body.length === 0) {
    draftError.value = t('organization.workspace.proposal.exchanges.emptyReply')
    return
  }
  emit('reply', parentId, body)
  cancelReply()
}
</script>

<template>
  <section aria-labelledby="workspace-exchanges-title">
    <h2 id="workspace-exchanges-title" class="text-xl font-semibold">
      {{ t('organization.workspace.proposal.exchanges.title') }}
    </h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('organization.workspace.proposal.exchanges.description') }}
    </p>

    <UiAlert
      v-if="openRequests > 0"
      intent="warning"
      class="mt-4"
      :message="t('organization.workspace.proposal.exchanges.openRequests', openRequests)"
    />

    <UiEmptyState
      v-if="threads.length === 0"
      class="mt-6"
      icon="mail"
      :title="t('organization.workspace.proposal.exchanges.empty.title')"
      :description="t('organization.workspace.proposal.exchanges.empty.description')"
    />

    <ol v-else class="mt-6 flex flex-col gap-6">
      <li
        v-for="thread in threads"
        :key="thread.root.id"
        class="rounded-lg border bg-surface-raised"
        :class="
          thread.root.is_change_request && thread.root.resolved_at === null
            ? 'border-warning-border'
            : 'border-border'
        "
      >
        <!-- La racine. Une demande de correction porte son étiquette : sans
             elle, un message du comité ressemble à une remarque, et personne ne
             sait que le dossier attend une réponse pour repartir. -->
        <article class="p-5">
          <header class="flex flex-wrap items-center gap-x-3 gap-y-1">
            <span class="font-semibold text-text">{{ authorOf(thread.root) }}</span>
            <time :datetime="thread.root.created_at" class="text-sm text-text-subtle">
              {{ dateTime(thread.root.created_at, viewerTimezone) }}
            </time>
            <span v-if="thread.root.edited_at" class="text-xs text-text-subtle">
              · {{ t('organization.workspace.proposal.exchanges.edited') }}
            </span>
            <UiBadge
              v-if="thread.root.is_change_request"
              intent="warning"
              :label="t('organization.workspace.proposal.exchanges.changeRequest')"
              icon="warning"
              size="sm"
            />
          </header>

          <p class="mt-3 max-w-(--measure) text-text-secondary whitespace-pre-line">{{ thread.root.body }}</p>
        </article>

        <!-- Les réponses, en retrait et sur fond creusé : on distingue d'un
             coup d'œil ce que le comité demande de ce qu'on lui a répondu. -->
        <ol v-if="thread.replies.length > 0" class="flex flex-col border-t border-border-subtle">
          <li
            v-for="reply in thread.replies"
            :key="reply.id"
            class="border-b border-border-subtle bg-surface-sunken px-5 py-4 last:border-b-0 sm:pl-10"
          >
            <header class="flex flex-wrap items-center gap-x-3 gap-y-1">
              <span class="font-semibold text-text">{{ authorOf(reply) }}</span>
              <time :datetime="reply.created_at" class="text-sm text-text-subtle">
                {{ dateTime(reply.created_at, viewerTimezone) }}
              </time>
            </header>
            <p class="mt-2 max-w-(--measure) text-text-secondary whitespace-pre-line">{{ reply.body }}</p>
          </li>
        </ol>

        <!-- Les deux actions du fil. « Résolu » n'est proposé que sur une
             demande de correction : marquer résolu un message d'information
             n'aurait aucun sens, et le proposer partout viderait le geste. -->
        <footer
          class="flex flex-wrap items-center gap-3 border-t border-border-subtle px-5 py-3"
        >
          <template v-if="replyingTo === thread.root.id">
            <div class="w-full">
              <UiTextarea
                :id="`reply-${thread.root.id}`"
                v-model="draft"
                :label="t('organization.workspace.proposal.exchanges.reply')"
                :error="draftError"
                :rows="3"
                auto-grow
                :placeholder="t('organization.workspace.proposal.exchanges.replyPlaceholder')"
              />
              <div class="mt-3 flex flex-wrap gap-2">
                <UiButton
                  variant="primary"
                  size="sm"
                  :loading="props.busy"
                  @click="submitReply(thread.root.id)"
                >
                  {{ t('organization.workspace.proposal.exchanges.send') }}
                </UiButton>
                <UiButton variant="ghost" size="sm" @click="cancelReply()">
                  {{ t('organization.workspace.proposal.exchanges.cancel') }}
                </UiButton>
              </div>
            </div>
          </template>

          <template v-else>
            <UiButton variant="secondary" size="sm" icon="mail" @click="openReply(thread.root.id)">
              {{ t('organization.workspace.proposal.exchanges.reply') }}
            </UiButton>

            <template v-if="thread.root.is_change_request">
              <UiButton
                v-if="thread.root.resolved_at === null"
                variant="ghost"
                size="sm"
                icon="check"
                :loading="props.busy"
                @click="emit('resolve', thread.root.id, true)"
              >
                {{ t('organization.workspace.proposal.exchanges.markResolved') }}
              </UiButton>

              <span v-else class="flex flex-wrap items-center gap-3">
                <UiBadge
                  intent="success"
                  icon="check"
                  :label="
                    t('organization.workspace.proposal.exchanges.resolved', {
                      date: dateTime(thread.root.resolved_at, viewerTimezone),
                    })
                  "
                  size="sm"
                />
                <UiButton
                  variant="ghost"
                  size="sm"
                  :loading="props.busy"
                  @click="emit('resolve', thread.root.id, false)"
                >
                  {{ t('organization.workspace.proposal.exchanges.unmarkResolved') }}
                </UiButton>
              </span>
            </template>
          </template>
        </footer>
      </li>
    </ol>
  </section>
</template>
