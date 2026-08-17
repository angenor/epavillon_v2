<script setup lang="ts">
import type { ProposalFile } from '~/types/organization-workspace'
import type { ProposalOrganization } from '~/types/programme/proposal'
import type { Organization } from '~/types/org'
import type { Intent, TabItem } from '~/types/ui'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { Uuid } from '~/types/shared'

/**
 * SUIVI D'UN DOSSIER — `/mon-organisation/dossiers/:id`,
 * `/en/my-organization/submissions/:id`.
 *
 * POURQUOI UNE PAGE ET NON UN PANNEAU. Le fil d'échanges et l'historique champ
 * par champ ne tiennent pas dans une liste, et surtout : un dossier se DÉSIGNE.
 * On s'écrit « regarde COP31-00001 » entre collègues, la confirmation de dépôt y
 * renvoie, un courriel du comité y renverra. Une adresse propre était donc
 * nécessaire — un panneau dépliant n'en a pas.
 *
 * LES ONGLETS SONT DES VUES D'UN MÊME OBJET, ce pour quoi `UiTabs` est fait :
 * le dossier, ses échanges, ses activités, son historique. Ils ne changent pas
 * de sujet, seulement de point de vue.
 *
 * CE QUE L'ORGANISATION NE VOIT PAS, ET QUI N'EST PAS UN OUBLI : aucune note,
 * aucun rang, aucun nom de membre du comité, aucune liste d'inscrits. Le filtre
 * est appliqué à la source (`useApi().workspace.proposalFile`) et non ici — un
 * composant ne doit pas être le dernier rempart entre une note interne et le
 * déposant.
 */

definePageMeta({
  layout: 'public',
  middleware: ['auth', 'requires-organization'],
  organizationReason: 'organization-space',
})

defineI18nRoute({
  paths: { fr: '/mon-organisation/dossiers/[id]', en: '/my-organization/submissions/[id]' },
})

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()
const localePath = useLocalePath()
const route = useRoute()
const api = useApi()
const auth = useAuthStore()
const memberships = useMembershipStore()

const proposalId = computed(() => String(route.params.id ?? ''))

await memberships.ensureLoaded()

/**
 * L'organisation depuis laquelle on consulte. Le dossier appartient à une seule
 * organisation ; on cherche donc, parmi les adhésions actives de la personne,
 * celle qui le porte — plutôt que d'imposer un paramètre d'URL de plus.
 */
const organizationIds = computed(() => memberships.active.map((entry) => entry.organization.id))

const {
  data: file,
  status,
  error,
  refresh,
} = await useAsyncData<ProposalFile | null>(
  () => `workspace-proposal-${proposalId.value}`,
  async () => {
    for (const organizationId of organizationIds.value) {
      const found = await api.workspace.proposalFile(proposalId.value, organizationId)
      if (found) return found
    }
    return null
  },
  { watch: [proposalId], lazy: true },
)

useHead(() => ({
  title: file.value
    ? `${file.value.tracking.proposal.reference_code} — ${tr(file.value.tracking.proposal.title)}`
    : t('organization.workspace.proposal.notFound.title'),
}))

// ---------------------------------------------------------------------------
// Onglets
//
// L'onglet actif vit dans l'URL : un lien envoyé vers les échanges d'un dossier
// doit ouvrir les échanges, pas le suivi.
// ---------------------------------------------------------------------------

const router = useRouter()

const activeTab = computed<string>(() => {
  const requested = route.query.vue
  const asked = Array.isArray(requested) ? requested[0] : requested
  return ['tracking', 'exchanges', 'sessions', 'history'].includes(String(asked))
    ? String(asked)
    : 'tracking'
})

function selectTab(value: string): void {
  router.replace({ query: { ...route.query, vue: value } })
}

const tabs = computed<TabItem[]>(() => [
  { value: 'tracking', label: t('organization.workspace.proposal.tabs.tracking') },
  {
    value: 'exchanges',
    label: t('organization.workspace.proposal.tabs.exchanges'),
    count: file.value?.comments.filter((comment) => comment.parent_id === null).length,
  },
  {
    value: 'sessions',
    label: t('organization.workspace.proposal.tabs.sessions'),
    count: file.value?.tracking.sessions.length,
  },
  { value: 'history', label: t('organization.workspace.proposal.tabs.history') },
])

// ---------------------------------------------------------------------------
// Frise et résumé
// ---------------------------------------------------------------------------

const steps = computed(() =>
  file.value
    ? buildProposalTimeline(file.value.tracking, {
        draft: t('organization.workspace.proposals.timeline.draft'),
        submitted: t('organization.workspace.proposals.timeline.submitted'),
        under_review: t('organization.workspace.proposals.timeline.under_review'),
        changes_requested: t('organization.workspace.proposals.timeline.changes_requested'),
        decision: t('organization.workspace.proposals.timeline.decision'),
        accepted: t('organization.workspace.proposals.timeline.accepted'),
        rejected: t('organization.workspace.proposals.timeline.rejected'),
        withdrawn: t('organization.workspace.proposals.timeline.withdrawn'),
        cancelled: t('organization.workspace.proposals.timeline.cancelled'),
        scheduled: t('organization.workspace.proposals.timeline.scheduled'),
      })
    : [],
)

const STATUS_TONES: Record<ProposalStatus, Intent> = {
  draft: 'neutral',
  submitted: 'info',
  under_review: 'info',
  changes_requested: 'warning',
  accepted: 'success',
  rejected: 'danger',
  withdrawn: 'neutral',
  cancelled: 'danger',
}

const timezone = computed(() => file.value?.tracking.edition.timezone ?? 'UTC')

/**
 * Ce dossier peut-il encore être modifié ? La règle est dans un utilitaire pur
 * (`utils/proposal-edit.ts`) parce que la liste des dossiers pose la même
 * question, et que deux lectures divergentes de « on peut encore » afficheraient
 * un bouton ici et pas là.
 */
const editBlockedReason = computed(() =>
  file.value
    ? proposalEditBlockedReason(file.value.tracking.proposal, file.value.tracking.edition)
    : 'file_closed',
)

const editTo = computed(() =>
  localePath({
    path: localePath('/deposer-une-proposition'),
    query: { dossier: proposalId.value },
  }),
)

/** Les organisations associées au dossier, avec leur fiche pour les nommer. */
const associated = ref<{ link: ProposalOrganization; organization: Organization | null }[]>([])

watchEffect(async () => {
  const id = file.value?.tracking.proposal.id
  if (!id) {
    associated.value = []
    return
  }
  const links = await api.proposals.organizations(id)
  associated.value = await Promise.all(
    links.map(async (link) => ({
      link,
      organization: await api.organizations.byId(link.organization_id),
    })),
  )
})

// ---------------------------------------------------------------------------
// Écritures du fil
// ---------------------------------------------------------------------------

const busy = ref(false)

async function reply(parentId: Uuid, body: string): Promise<void> {
  const person = auth.person
  if (!person || !file.value) return
  busy.value = true
  try {
    await api.workspace.reply(person.id, {
      proposal_id: file.value.tracking.proposal.id,
      parent_id: parentId,
      body,
    })
    await refresh()
  } finally {
    busy.value = false
  }
}

async function resolve(commentId: Uuid, resolved: boolean): Promise<void> {
  const person = auth.person
  if (!person) return
  busy.value = true
  try {
    await api.workspace.resolve(person.id, { comment_id: commentId, resolved })
    await refresh()
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-5xl px-4 py-8 sm:px-6 lg:px-8">
    <UiBreadcrumb
      :items="[
        { label: t('organization.workspace.proposal.breadcrumb'), to: localePath('/mon-organisation') },
        { label: file?.tracking.proposal.reference_code ?? '' },
      ]"
    />

    <UiLoadingState v-if="status === 'pending'" class="mt-10" variant="card" :lines="5" />

    <UiErrorState
      v-else-if="error"
      class="mt-10"
      :retry-label="t('common.actions.retry')"
      @retry="refresh()"
    />

    <UiEmptyState
      v-else-if="!file"
      class="mt-10"
      icon="document"
      :title="t('organization.workspace.proposal.notFound.title')"
      :description="t('organization.workspace.proposal.notFound.description')"
      :action-to="localePath('/mon-organisation')"
      :action-label="t('organization.workspace.proposal.notFound.action')"
    />

    <template v-else>
      <header class="mt-6 flex flex-col gap-3">
        <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
          <p class="font-mono text-sm tracking-wide text-text-subtle">
            {{ file.tracking.proposal.reference_code }}
          </p>
          <UiBadge
            :intent="STATUS_TONES[file.tracking.proposal.status]"
            :label="t(`organization.workspace.proposals.timeline.${file.tracking.proposal.status}`)"
            solid
          />
        </div>
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ tr(file.tracking.proposal.title) }}
        </h1>
        <p class="text-text-muted">{{ tr(file.tracking.edition.title) }}</p>
      </header>

      <UiTabs
        class="mt-8"
        :items="tabs"
        :model-value="activeTab"
        :label="t('organization.workspace.proposal.tabs.label')"
        @update:model-value="selectTab"
      >
        <!-- SUIVI -->
        <template v-if="activeTab === 'tracking'">
          <div class="flex flex-col gap-8">
            <UiStatusTimeline
              :steps="steps"
              :timezone="timezone"
              :label="t('organization.workspace.proposal.tracking.timelineLabel')"
            />

            <!-- La décision, redite hors de la frise : c'est la réponse qu'on
                 vient chercher, et elle ne doit pas se lire en petits caractères
                 sous une pastille. -->
            <UiAlert
              v-if="file.tracking.proposal.decided_at"
              :intent="file.tracking.proposal.status === 'accepted' ? 'success' : 'warning'"
              :title="
                t(`organization.workspace.proposal.tracking.decision.${file.tracking.proposal.status}`)
              "
              :message="
                [
                  t('organization.workspace.proposal.tracking.decision.on', {
                    date: date(file.tracking.proposal.decided_at, timezone),
                  }),
                  file.tracking.proposal.decision_reason
                    ? t('organization.workspace.proposal.tracking.decision.reason', {
                        reason: file.tracking.proposal.decision_reason,
                      })
                    : '',
                ]
                  .filter(Boolean)
                  .join(' — ')
              "
            />

            <!-- MODIFIER SON DOSSIER — « tant que l'événement n'est pas
                 terminé, il peut modifier » (commanditaire, 17/08). Quand ce
                 n'est plus possible, on DIT pourquoi : un bouton absent sans
                 écriteau est ce qui produit les courriels à l'IFDD. -->
            <UiButton
              v-if="editBlockedReason === null"
              variant="primary"
              class="self-start"
              :to="editTo"
              icon="edit"
            >
              {{
                file.tracking.proposal.status === 'draft'
                  ? t('organization.workspace.proposal.tracking.editDraft')
                  : t('organization.workspace.proposal.tracking.edit')
              }}
            </UiButton>

            <p v-else class="flex items-start gap-2 text-sm text-text-muted">
              <UiIcon name="lock" size="1.05rem" class="mt-0.5 shrink-0 text-text-subtle" />
              {{ t(`organization.workspace.proposal.tracking.editBlocked.${editBlockedReason}`) }}
            </p>

            <section aria-labelledby="proposal-summary-title">
              <h2 id="proposal-summary-title" class="text-xl font-semibold">
                {{ t('organization.workspace.proposal.tracking.summary') }}
              </h2>

              <dl class="mt-4 grid gap-x-8 gap-y-4 sm:grid-cols-2">
                <div>
                  <dt class="text-sm text-text-subtle">
                    {{ t('organization.workspace.proposal.tracking.format') }}
                  </dt>
                  <dd class="mt-0.5 text-text-secondary">
                    {{ t(`organization.workspace.proposal.tracking.formatValue.${file.tracking.proposal.format}`) }}
                  </dd>
                </div>

                <div v-if="file.tracking.proposal.duration_minutes">
                  <dt class="text-sm text-text-subtle">
                    {{ t('organization.workspace.proposal.tracking.duration') }}
                  </dt>
                  <dd class="mt-0.5 text-text-secondary">
                    {{
                      t('organization.workspace.proposal.tracking.durationValue', {
                        count: file.tracking.proposal.duration_minutes,
                      })
                    }}
                  </dd>
                </div>

                <div v-if="file.tracking.proposal.preferred_start_at">
                  <dt class="text-sm text-text-subtle">
                    {{ t('organization.workspace.proposal.tracking.preferredSlot') }}
                  </dt>
                  <dd class="mt-0.5 text-text-secondary">
                    {{ dateTime(file.tracking.proposal.preferred_start_at, timezone) }}
                  </dd>
                </div>

                <div v-if="file.tracking.proposal.requested_sessions > 1">
                  <dt class="text-sm text-text-subtle">
                    {{ t('organization.workspace.proposal.tracking.requestedSessions') }}
                  </dt>
                  <dd class="mt-0.5 text-text-secondary">{{ file.tracking.proposal.requested_sessions }}</dd>
                </div>

                <div v-if="file.tracking.proposal.target_audiences.length > 0" class="sm:col-span-2">
                  <dt class="text-sm text-text-subtle">
                    {{ t('organization.workspace.proposal.tracking.audiences') }}
                  </dt>
                  <dd class="mt-1.5 flex flex-wrap gap-2">
                    <UiBadge
                      v-for="(audience, index) in file.tracking.proposal.target_audiences"
                      :key="index"
                      :label="tr(audience)"
                      size="sm"
                    />
                  </dd>
                </div>
              </dl>
            </section>

            <section v-if="associated.length > 1" aria-labelledby="proposal-organizations-title">
              <h2 id="proposal-organizations-title" class="text-xl font-semibold">
                {{ t('organization.workspace.proposal.tracking.organizations') }}
              </h2>
              <ul class="mt-4 flex flex-col divide-y divide-border-subtle rounded-lg border border-border bg-surface-raised">
                <li
                  v-for="entry in associated"
                  :key="entry.link.organization_id"
                  class="flex flex-wrap items-center gap-x-4 gap-y-1 px-4 py-3"
                >
                  <span class="min-w-0 flex-1 font-medium text-text">
                    {{ entry.organization?.legal_name ?? entry.link.organization_id }}
                  </span>
                  <UiBadge
                    :label="t(`organization.workspace.proposal.tracking.role.${entry.link.role}`)"
                    size="sm"
                  />
                  <!-- Une co-organisation annoncée ENGAGE un tiers : tant qu'elle
                       n'est pas confirmée, le dire. -->
                  <span v-if="!entry.link.confirmed_at" class="text-sm text-warning">
                    {{ t('organization.workspace.proposal.tracking.unconfirmed') }}
                  </span>
                </li>
              </ul>
            </section>
          </div>
        </template>

        <!-- ÉCHANGES -->
        <WorkspaceCommentThread
          v-else-if="activeTab === 'exchanges'"
          :comments="file.comments"
          :participants="file.participants"
          :viewer-id="auth.person?.id ?? ''"
          :busy="busy"
          @reply="reply"
          @resolve="resolve"
        />

        <!-- ACTIVITÉS ET RAPPELS -->
        <WorkspaceSessionPanel
          v-else-if="activeTab === 'sessions'"
          :sessions="file.tracking.sessions"
          :timezone="timezone"
        />

        <!-- HISTORIQUE -->
        <WorkspaceHistoryList v-else :entries="file.history" :timezone="timezone" />
      </UiTabs>
    </template>
  </div>
</template>
