<script setup lang="ts">
import type { CommentsDeskMessage } from '~/components/workspace/CommentsDesk.vue'
import type { ProposalFile } from '~/types/organization-workspace'
import type { ProposalCommentView, ProposalOrganization, ProposalStatus } from '~/types/programme/proposal'
import type { Organization } from '~/types/org'
import type { AttachableRoleRule } from '~/types/media'
import type { Intent, SelectOption, TabItem } from '~/types/ui'
import type { AssetId, Uuid } from '~/types/shared'
import type { ProposalFileSection } from '~/composables/useProposalFileEditor'
import type { FocalPointInput, ProposalDayPlan, SaveTechnicalNeedsPayload } from '~/types/programme/day-plan'

/**
 * LA FICHE D'UN DOSSIER — `/mon-organisation/dossiers/:id`,
 * `/en/my-organization/submissions/:id`.
 *
 * TOUT SE MODIFIE SUR PLACE (29/10, direction 1 arbitrée avec le commanditaire) :
 * la frise d'abord, puis ce qui manque, l'image de couverture, et une carte par
 * rubrique avec son bouton « Modifier ». Plus de détour par le formulaire de
 * dépôt, sauf pour un brouillon qui n'est pas encore parti.
 *
 * TROIS ONGLETS, ET NON QUATRE. « Activités programmées » est rentré dans le
 * dossier (carte « Quand et où ») : pour l'organisation, c'est la même activité
 * à un autre moment de sa vie, et deux onglets l'obligeaient à deviner où
 * chercher l'heure de son passage. `?vue=sessions` (liens déjà envoyés,
 * actions de l'espace) ouvre le dossier sur cette carte.
 *
 * CE QUE L'ORGANISATION NE VOIT PAS, ET QUI N'EST PAS UN OUBLI : aucune note,
 * aucun rang, aucun nom de membre du comité, aucune liste d'inscrits. Le filtre
 * est appliqué à la source (`useApi().workspace.proposalFile`).
 */

definePageMeta({
  layout: 'public',
  middleware: ['auth', 'requires-organization'],
  organizationReason: 'organization-space',
  actingForOrganization: true,
})

defineI18nRoute({
  paths: { fr: '/mon-organisation/dossiers/[id]', en: '/my-organization/submissions/[id]' },
})

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()
const localePath = useLocalePath()
const route = useRoute()
const router = useRouter()
const api = useApi()
const auth = useAuthStore()
const memberships = useMembershipStore()
const requestUrl = useRequestURL()

const proposalId = computed(() => String(route.params.id ?? ''))

const {
  data: file,
  status,
  error,
  refresh,
} = await useAsyncData<ProposalFile | null>(
  () => `workspace-proposal-${proposalId.value}`,
  () => {
    const person = auth.person
    return person ? api.workspace.proposalFile(proposalId.value, person.id) : Promise.resolve(null)
  },
  { watch: [proposalId, () => auth.person?.id], lazy: true },
)

useHead(() => ({
  title: file.value
    ? `${file.value.tracking.proposal.reference_code} · ${tr(file.value.tracking.proposal.title)}`
    : t('organization.workspace.proposal.notFound.title'),
}))

const editor = useProposalFileEditor(proposalId, async () => {
  await refresh()
})

// ---------------------------------------------------------------------------
// Onglets
// ---------------------------------------------------------------------------

const activeTab = computed<string>(() => {
  const requested = route.query.vue
  const asked = String(Array.isArray(requested) ? requested[0] : requested)
  return asked === 'history' ? asked : 'tracking'
})

function selectTab(value: string): void {
  router.replace({ query: { ...route.query, vue: value } })
}

const tabs = computed<TabItem[]>(() => [
  { value: 'tracking', label: t('organization.workspace.proposal.tabs.tracking') },
  { value: 'history', label: t('organization.workspace.proposal.tabs.history') },
])

// L'ancien onglet des activités mène désormais à la carte « Quand et où ».
onMounted(async () => {
  if (route.query.vue !== 'sessions') return
  await until(() => Boolean(file.value))
  await nextTick()
  document.getElementById('quand-ou')?.scrollIntoView({ behavior: 'smooth' })
})

async function until(ready: () => boolean): Promise<void> {
  if (ready()) return
  await new Promise<void>((resolve) => {
    const stop = watch(ready, (value) => {
      if (!value) return
      stop()
      resolve()
    })
  })
}

// ---------------------------------------------------------------------------
// Frise, droits de modification
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
const proposalStatus = computed(() => file.value?.tracking.proposal.status ?? 'draft')
const isDraft = computed(() => proposalStatus.value === 'draft')

const editBlockedReason = computed(() =>
  file.value ? organizationEditBlockedReason(file.value.tracking.proposal, file.value.tracking.edition) : 'file_closed',
)

/** Les rubriques du dossier : modifiables quand la règle le permet, un brouillon passe par le formulaire. */
const canEditSections = computed(() => editBlockedReason.value === null && !isDraft.value && Boolean(editor.saved.value))

/** Couverture, photos, logo : des images, pas du contenu évalué. Fermées seulement pour un dossier clos. */
const canEditMedia = computed(
  () => file.value !== null && !['rejected', 'withdrawn', 'cancelled'].includes(proposalStatus.value),
)

const draftTo = computed(() =>
  localePath({ path: localePath('/deposer-une-proposition'), query: { dossier: proposalId.value } }),
)

const decisionMessage = computed(() => {
  const proposal = file.value?.tracking.proposal
  if (!proposal?.decided_at) return ''
  return [
    t('organization.workspace.proposal.tracking.decision.on', { date: date(proposal.decided_at, timezone.value) }),
    proposal.decision_reason
      ? t('organization.workspace.proposal.tracking.decision.reason', { reason: proposal.decision_reason })
      : '',
  ]
    .filter(Boolean)
    .join(' · ')
})

// ---------------------------------------------------------------------------
// Images : couverture, photos, logo
// ---------------------------------------------------------------------------

const { data: mediaRules } = useAsyncData(
  'proposal-file-media-rules',
  async () => {
    const [proposalRules, speakerRules, organizationRules] = await Promise.all([
      api.media.roles('programme', 'proposals'),
      api.media.roles('programme', 'proposal_speakers'),
      api.media.roles('org', 'organizations'),
    ])
    const find = (rules: AttachableRoleRule[], role: string) => rules.find((rule) => rule.role === role) ?? null
    return {
      cover: find(proposalRules, 'cover'),
      avatar: find(speakerRules, 'avatar'),
      logo: find(organizationRules, 'logo'),
    }
  },
  { lazy: true, server: false },
)

const coverError = ref<string | null>(null)

async function attachCover(assetId: AssetId | null): Promise<void> {
  if (!assetId || !file.value) return
  coverError.value = null
  try {
    await api.media.attach({
      owner_schema: 'programme',
      owner_table: 'proposals',
      owner_id: file.value.tracking.proposal.id,
      assignments: [{ role: 'cover', asset_id: assetId }],
    })
    await refresh()
  } catch (thrown) {
    coverError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

/** L'atelier a-t-il déjà servi ? « Modifier » le rouvre là où on l'a laissé. */
const { data: coverStudio } = useAsyncData(
  () => `proposal-file-cover-studio-${proposalId.value}`,
  () => api.proposals.coverStudio(proposalId.value),
  { lazy: true, server: false, watch: [proposalId] },
)

const studioTo = computed(() => localePath(`/mon-organisation/dossiers/${proposalId.value}/couverture`))

async function refreshAll(): Promise<void> {
  await Promise.all([refresh(), editor.refresh()])
}

const leadOrganization = computed(() =>
  memberships.submittable.find((entry) => entry.organization.id === file.value?.tracking.proposal.organization_id),
)

const canSetLogo = computed(() => leadOrganization.value?.membership.role === 'manager')

// ---------------------------------------------------------------------------
// Ce qui manque
// ---------------------------------------------------------------------------

const facts = computed(() =>
  editor.saved.value && file.value
    ? {
        draft: editor.saved.value,
        hasCover: file.value.tracking.cover !== null,
        hasLogo: file.value.organization_logo !== null,
        dayPlan: dayPlan.value,
      }
    : null,
)

/** Un dossier clos ou d'une édition passée n'attend plus rien : « il manque » n'y a plus de sens. */
const isFileOpen = computed(() => {
  const edition = file.value?.tracking.edition
  if (!edition) return false
  return !['rejected', 'withdrawn', 'cancelled'].includes(proposalStatus.value) && !isEditionOver(edition)
})

const gaps = computed(() => (facts.value && isFileOpen.value ? proposalFileGaps(facts.value) : []))
const checklist = computed(() => (facts.value && isFileOpen.value ? proposalFileChecklist(facts.value) : []))

// ---------------------------------------------------------------------------
// Le jour J : activité retenue seulement
// ---------------------------------------------------------------------------

const isAccepted = computed(() => proposalStatus.value === 'accepted')

const { data: dayPlan } = useAsyncData<ProposalDayPlan | null>(
  () => `proposal-day-plan-${proposalId.value}`,
  () => (isAccepted.value ? api.proposals.dayPlan(proposalId.value) : Promise.resolve(null)),
  { lazy: true, watch: [proposalId, isAccepted] },
)

/** Point focal et besoins techniques : jusqu'à la fin de l'édition. Présentations et publications : après aussi, le bilan se fait après. */
const canPrepare = computed(() => {
  const current = file.value
  return isAccepted.value && current != null && !isEditionOver(current.tracking.edition)
})
const canPromote = computed(() => isAccepted.value)

const daySaving = ref<'focal' | 'needs' | 'social' | 'presentations' | null>(null)
const dayError = ref<{ part: 'focal' | 'needs' | 'social' | 'presentations'; message: string } | null>(null)

async function dayWrite(part: NonNullable<typeof daySaving.value>, write: () => Promise<ProposalDayPlan | void>): Promise<void> {
  daySaving.value = part
  dayError.value = null
  try {
    const plan = await write()
    if (plan) dayPlan.value = plan
  } catch (thrown) {
    dayError.value = { part, message: apiErrorMessage(thrown, (key) => t(key)) }
  } finally {
    daySaving.value = null
  }
}

const errorOf = (part: NonNullable<typeof daySaving.value>) => (dayError.value?.part === part ? dayError.value.message : null)

const focalInputs = computed<FocalPointInput[]>(() =>
  (dayPlan.value?.focal_points ?? []).map(({ speaker_id, person_id, full_name, phone, whatsapp }) => ({
    speaker_id,
    person_id,
    full_name,
    phone,
    whatsapp,
  })),
)

const focalSpeakerIds = computed(() =>
  (dayPlan.value?.focal_points ?? []).flatMap((point) => (point.speaker_id ? [point.speaker_id] : [])),
)

function saveFocal(points: FocalPointInput[]): Promise<void> {
  return dayWrite('focal', () => api.proposals.saveFocalPoints(proposalId.value, { focal_points: points }))
}

/** Cocher un intervenant l'ajoute aux points focaux, décocher le retire ; le dernier reste. */
function toggleFocal(speakerId: string, on: boolean): Promise<void> {
  const speaker = editor.saved.value?.speakers.find((entry) => entry.speaker_id === speakerId)
  const next = on
    ? [...focalInputs.value, { speaker_id: speakerId, person_id: null, full_name: speaker ? speakerName(speaker) : '', phone: '', whatsapp: '' }]
    : focalInputs.value.filter((point) => point.speaker_id !== speakerId)
  if (next.length === 0) {
    dayError.value = { part: 'focal', message: t('organization.workspace.proposal.focal.atLeastOne') }
    return Promise.resolve()
  }
  return saveFocal(next)
}

function saveNeeds(needs: SaveTechnicalNeedsPayload): Promise<void> {
  return dayWrite('needs', () => api.proposals.saveTechnicalNeeds(proposalId.value, needs))
}

function addPost(url: string): Promise<void> {
  return dayWrite('social', () => api.proposals.addSocialPost(proposalId.value, { url }))
}

function removePost(postId: Uuid): Promise<void> {
  return dayWrite('social', () => api.proposals.removeSocialPost(proposalId.value, postId))
}

const presentationCount = computed(
  () => (editor.data.value?.documents ?? []).filter((entry) => entry.document.document_type_code === 'presentation').length,
)

function addPresentation(assetId: AssetId, title: string): Promise<void> {
  return dayWrite('presentations', async () => {
    await api.proposals.attachDocument(proposalId.value, {
      asset_id: assetId,
      title: title || t('organization.workspace.proposal.presentations.untitled'),
      document_type_code: 'presentation',
      is_public: false,
      sort_order: presentationCount.value,
    })
    await editor.refresh()
  })
}

function removePresentation(documentId: Uuid): Promise<void> {
  return dayWrite('presentations', async () => {
    await api.proposals.removeDocument(proposalId.value, documentId)
    await editor.refresh()
  })
}

// ---------------------------------------------------------------------------
// Lien public et QR code : à la publication du programme seulement
// ---------------------------------------------------------------------------

const publicUrl = computed(() => {
  const tracking = file.value?.tracking
  if (!tracking?.edition.programme_published_at) return null
  const session = tracking.sessions
    .map((entry) => entry.session)
    .find((entry) => entry.status !== 'planned' && entry.status !== 'cancelled')
  if (!session) return null
  return new URL(localePath(`/programmations/${tracking.edition.slug}/${session.slug}`), requestUrl.origin).href
})

// ---------------------------------------------------------------------------
// Rubriques : libellés et options venus de la base
// ---------------------------------------------------------------------------

const references = computed(() => editor.data.value?.references ?? null)

function countryNameOf(countryId: string | null): string | null {
  if (!countryId) return null
  const country = references.value?.countries.find((entry) => entry.id === countryId)
  return country ? tr(country.name) : null
}

const countryOptions = computed<SelectOption[]>(() =>
  (references.value?.countries ?? [])
    .filter((country) => country.is_active)
    .map((country) => ({ value: country.id, label: tr(country.name) }))
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
)

// Pour l'organisation, la porteuse reste la seule candidate : on n'en change pas à sa place.
const leadCandidates = computed<Organization[]>(() =>
  file.value?.acting_for_organization
    ? associated.value.flatMap(({ link, organization }) =>
        organization && link.organization_id === file.value?.tracking.proposal.organization_id ? [organization] : [],
      )
    : memberships.submittable.map((entry) => entry.organization),
)

const leadName = computed(() => {
  const lead = leadCandidates.value.find((organization) => organization.id === file.value?.tracking.proposal.organization_id)
  return lead ? (lead.acronym ?? lead.legal_name) : null
})

function termLabels(codes: string[], kind: 'themes' | 'categories'): string {
  const terms = references.value?.[kind] ?? []
  return codes.map((code) => {
    const term = terms.find((entry) => entry.code === code)
    return term ? tr(term.label) : code
  }).join(', ')
}

function localeLabels(codes: string[]): string {
  const locales = references.value?.locales ?? []
  return codes.map((code) => {
    const entry = locales.find((candidate) => candidate.code === code)
    return entry?.native_label ?? code
  }).join(', ')
}

/** Les organisations associées au dossier, avec leur fiche pour les nommer. */
const { data: associatedData } = useAsyncData(
  () => `proposal-file-organizations-${proposalId.value}`,
  async () => {
    const links = await api.proposals.organizations(proposalId.value)
    return Promise.all(
      links.map(async (link) => ({ link, organization: await api.organizations.byId(link.organization_id) })),
    )
  },
  // Relue après chaque enregistrement : la rubrique Organisations change la liste.
  { lazy: true, watch: [proposalId, () => file.value?.tracking.proposal.updated_at] },
)

const associated = computed<{ link: ProposalOrganization; organization: Organization | null }[]>(
  () => associatedData.value ?? [],
)

function sectionProps(section: ProposalFileSection) {
  return {
    canEdit: canEditSections.value,
    editing: editor.editing.value === section,
    locked: editor.editing.value !== null && editor.editing.value !== section,
    saving: editor.saving.value,
    error: editor.editing.value === section ? editor.error.value : null,
  }
}

// ---------------------------------------------------------------------------
// Les échanges, en fenêtre flottante : relus toutes les 15 s fenêtre ouverte,
// toutes les minutes sinon, pour que la pastille du bouton reste juste.
// ---------------------------------------------------------------------------

const deskOpen = ref(route.query.vue === 'exchanges' || route.hash === '#echanges')

const {
  data: comments,
  status: commentsStatus,
  error: commentsError,
  refresh: refreshComments,
} = useAsyncData<ProposalCommentView[]>(
  () => `workspace-proposal-comments-${proposalId.value}`,
  () => {
    const person = auth.person
    return person ? api.proposals.comments(proposalId.value, person.id) : Promise.resolve([])
  },
  { watch: [proposalId, () => auth.person?.id], lazy: true, server: false, default: () => [] },
)

usePolling(refreshComments, () => (deskOpen.value ? 15_000 : 60_000))

const busy = ref(false)
const sendError = ref<string | null>(null)

async function sendMessage(message: CommentsDeskMessage): Promise<boolean> {
  const person = auth.person
  if (!person || !file.value) return false
  busy.value = true
  sendError.value = null
  try {
    await api.workspace.reply(person.id, {
      proposal_id: file.value.tracking.proposal.id,
      parent_id: message.parentId,
      body: message.body,
      topic: message.topic,
      topic_other: message.topicOther,
    })
    await refreshComments()
    return true
  } catch (thrown) {
    sendError.value = apiErrorMessage(thrown, (key) => t(key))
    return false
  } finally {
    busy.value = false
  }
}

/** Lecture silencieuse : un échec laisse le message à reprendre à sa prochaine vue. */
async function markCommentsRead(ids: Uuid[]): Promise<void> {
  const person = auth.person
  if (!person || !file.value) return
  await api.proposals.markCommentsRead(person.id, file.value.tracking.proposal.id, ids)
  for (const comment of comments.value) if (ids.includes(comment.id)) comment.is_read = true
}

async function resolve(commentId: Uuid, resolved: boolean): Promise<void> {
  const person = auth.person
  if (!person) return
  busy.value = true
  try {
    await api.workspace.resolve(person.id, { comment_id: commentId, resolved })
    await Promise.all([refreshComments(), refresh()])
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-6xl px-4 py-8 sm:px-6 lg:px-8">
    <UiBreadcrumb
      :items="[
        { label: t('organization.workspace.proposal.breadcrumb'), to: localePath('/mon-organisation') },
        { label: file?.tracking.proposal.reference_code ?? '' },
      ]"
    />

    <UiLoadingState v-if="status === 'pending'" class="mt-10" variant="card" :lines="5" />

    <UiErrorState v-else-if="error" class="mt-10" :retry-label="t('common.actions.retry')" @retry="refresh()" />

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
      <UiAlert
        v-if="file.acting_for_organization"
        class="mt-6"
        intent="warning"
        :title="t('organization.workspace.proposal.acting.title', { organization: leadName ?? '' })"
        :message="t('organization.workspace.proposal.acting.description')"
      />

      <header class="mt-6 flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div class="flex min-w-0 flex-col gap-3">
          <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
            <p class="font-mono text-sm tracking-wide text-text-subtle">{{ file.tracking.proposal.reference_code }}</p>
            <UiBadge
              :intent="STATUS_TONES[file.tracking.proposal.status]"
              :label="t(`organization.workspace.proposals.timeline.${file.tracking.proposal.status}`)"
              solid
            />
          </div>
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ tr(file.tracking.proposal.title) }}</h1>
          <p class="text-text-muted">{{ tr(file.tracking.edition.title) }}</p>
        </div>
        <WorkspaceFilePublicLink v-if="publicUrl" :url="publicUrl" :file-name="file.tracking.proposal.reference_code" />
      </header>

      <UiTabs
        class="mt-8"
        :items="tabs"
        :model-value="activeTab"
        :label="t('organization.workspace.proposal.tabs.label')"
        @update:model-value="selectTab"
      >
        <!-- MON DOSSIER -->
        <div v-if="activeTab === 'tracking'" class="grid gap-8 lg:grid-cols-[minmax(0,1fr)_18rem] lg:items-start">
          <div class="flex min-w-0 flex-col gap-6">
            <section
              :aria-label="t('organization.workspace.proposal.tracking.timelineLabel')"
              class="rounded-lg border border-border bg-surface-raised p-5"
            >
              <UiStatusTimeline
                :steps="steps"
                orientation="horizontal"
                hide-details
                :timezone="timezone"
                :label="t('organization.workspace.proposal.tracking.timelineLabel')"
              />
            </section>

            <UiAlert
              v-if="file.tracking.proposal.decided_at"
              :intent="file.tracking.proposal.status === 'accepted' ? 'success' : 'warning'"
              :title="t(`organization.workspace.proposal.tracking.decision.${file.tracking.proposal.status}`)"
              :message="decisionMessage"
            />

            <div
              v-if="isDraft && editBlockedReason === null"
              class="flex flex-col gap-3 rounded-lg border border-border bg-surface-raised p-5 sm:flex-row sm:items-center sm:justify-between"
            >
              <p class="text-text-secondary">{{ t('organization.workspace.proposal.draftNotice') }}</p>
              <UiButton variant="primary" class="shrink-0" :to="draftTo" icon="edit">
                {{ t('organization.workspace.proposal.tracking.editDraft') }}
              </UiButton>
            </div>

            <p v-else-if="editBlockedReason" class="flex items-start gap-2 text-sm text-text-muted">
              <UiIcon name="lock" size="1.05rem" class="mt-0.5 shrink-0 text-text-subtle" />
              {{ t(`organization.workspace.proposal.tracking.editBlocked.${editBlockedReason}`) }}
            </p>

            <WorkspaceFileGaps
              v-if="facts && isFileOpen"
              :gaps="gaps"
              :has-logo="facts.hasLogo"
              :organization-id="file.tracking.proposal.organization_id"
              :organization-name="leadOrganization?.organization.legal_name ?? leadName ?? ''"
              :can-set-logo="canSetLogo"
              :logo-rule="mediaRules?.logo ?? null"
              @logo-set="refresh()"
            />

            <!-- IMAGE DE COUVERTURE -->
            <section
              id="couverture"
              aria-labelledby="couverture-title"
              class="scroll-mt-24 overflow-hidden rounded-lg border border-border bg-surface-raised"
            >
              <UiImage
                v-if="file.tracking.cover"
                :image="file.tracking.cover"
                ratio="16 / 9"
                sizes="(min-width: 1024px) 800px, 100vw"
                loading="eager"
              />
              <div v-else class="grid aspect-video place-items-center bg-surface-sunken text-text-subtle">
                <UiIcon name="image" size="2.5rem" />
              </div>
              <div class="p-5">
                <h2 id="couverture-title" class="text-lg font-semibold text-heading">
                  {{ t('organization.workspace.proposal.cover.title') }}
                </h2>
                <p class="mt-1 text-sm text-text-muted">{{ t('organization.workspace.proposal.cover.description') }}</p>
                <div v-if="canEditMedia" class="mt-4 flex flex-wrap gap-3">
                  <MediaImageField
                    role="cover"
                    :asset-id="null"
                    :label="t('organization.workspace.proposal.cover.title')"
                    :alt-fallback="tr(file.tracking.proposal.title)"
                    :button-label="t('organization.workspace.proposal.cover.change')"
                    :rule="mediaRules?.cover ?? null"
                    :owner="{ schema: 'programme', table: 'proposals', id: file.tracking.proposal.id }"
                    @update:asset-id="attachCover"
                  />
                  <UiButton v-if="coverStudio?.design" variant="secondary" icon="edit" :to="studioTo">
                    {{ t('organization.workspace.proposal.cover.edit') }}
                  </UiButton>
                  <UiButton variant="primary" :to="`${studioTo}?nouveau=1`">
                    {{ t('organization.workspace.proposal.cover.create') }}
                  </UiButton>
                </div>
                <UiAlert v-if="coverError" class="mt-3" intent="danger" :message="coverError" live />
              </div>
            </section>

            <!-- QUAND ET OÙ : remonte sous la couverture dès que l'activité est programmée -->
            <WorkspaceSessionPanel
              v-if="file.tracking.sessions.length > 0"
              id="quand-ou"
              class="scroll-mt-24 rounded-lg border border-border bg-surface-raised p-5 sm:p-6"
              :sessions="file.tracking.sessions"
              :timezone="timezone"
              :edition-over="isEditionOver(file.tracking.edition)"
            />

            <UiLoadingState v-if="!editor.saved.value" variant="card" :lines="4" />

            <template v-else>
              <!-- PRÉSENTATION -->
              <WorkspaceFileSection
                id="presentation"
                :title="t('organization.workspace.proposal.sections.presentation')"
                v-bind="sectionProps('presentation')"
                @edit="editor.edit('presentation')"
                @save="editor.save()"
                @cancel="editor.cancel()"
              >
                <dl class="flex flex-col gap-4">
                  <div>
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.fields.title') }}</dt>
                    <dd class="mt-1 font-medium text-text">{{ editor.saved.value.title }}</dd>
                  </div>
                  <div>
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.fields.objectives') }}</dt>
                    <dd class="mt-1 text-text-secondary">{{ plainTextOf(editor.saved.value.objectives) }}</dd>
                  </div>
                  <div>
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.fields.detailedPresentation') }}</dt>
                    <dd class="mt-1 line-clamp-6 text-text-secondary">{{ plainTextOf(editor.saved.value.detailed_presentation) }}</dd>
                  </div>
                  <div v-if="plainTextOf(editor.saved.value.expected_outcomes).trim()">
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.fields.expectedOutcomes') }}</dt>
                    <dd class="mt-1 text-text-secondary">{{ plainTextOf(editor.saved.value.expected_outcomes) }}</dd>
                  </div>
                </dl>
                <template #edit>
                  <ProposalStepPresentation v-if="editor.working.value" v-model="editor.working.value" :issues="editor.shownIssues.value" />
                </template>
              </WorkspaceFileSection>

              <!-- INTERVENANTS -->
              <WorkspaceFileSection
                id="intervenants"
                :title="t('organization.workspace.proposal.sections.speakers', editor.saved.value.speakers.length)"
                :description="t('organization.workspace.proposal.sections.speakersHint')"
                v-bind="sectionProps('speakers')"
                @edit="editor.edit('speakers')"
                @save="editor.save()"
                @cancel="editor.cancel()"
              >
                <WorkspaceFileSpeakers
                  :speakers="editor.saved.value.speakers"
                  :can-set-photo="canEditMedia"
                  :photo-rule="mediaRules?.avatar ?? null"
                  :focal-speaker-ids="dayPlan && canPrepare ? focalSpeakerIds : null"
                  :focal-busy="daySaving === 'focal'"
                  @photo-set="refreshAll()"
                  @toggle-focal="toggleFocal"
                />
                <template #edit>
                  <ProposalStepSpeakers
                    v-if="editor.working.value && references?.call"
                    v-model="editor.working.value"
                    :call="references.call"
                    :issues="editor.shownIssues.value"
                  />
                </template>
              </WorkspaceFileSection>

              <!-- LE JOUR J -->
              <template v-if="dayPlan">
                <p class="-mb-2 text-sm font-bold tracking-wide text-success uppercase">
                  {{ t('organization.workspace.proposal.dayPlan.prepare') }}
                </p>
                <WorkspaceFileFocalPoints
                  :points="dayPlan.focal_points"
                  :can-edit="canPrepare"
                  :saving="daySaving === 'focal'"
                  :error="errorOf('focal')"
                  @save="saveFocal"
                />
                <WorkspaceFileTechnicalNeeds
                  :needs="dayPlan.technical_needs"
                  :deadline="dayPlan.needs_deadline"
                  :timezone="timezone"
                  :can-edit="canPrepare"
                  :saving="daySaving === 'needs'"
                  :error="errorOf('needs')"
                  @save="saveNeeds"
                />
                <WorkspaceFilePresentations
                  :proposal-id="file.tracking.proposal.id"
                  :documents="editor.data.value?.documents ?? []"
                  :can-edit="canPromote"
                  :busy="daySaving === 'presentations'"
                  :error="errorOf('presentations')"
                  @add="addPresentation"
                  @remove="removePresentation"
                />
                <p class="-mb-2 text-sm font-bold tracking-wide text-success uppercase">
                  {{ t('organization.workspace.proposal.dayPlan.promote') }}
                </p>
                <WorkspaceFileSocialPosts
                  :posts="dayPlan.social_posts"
                  :can-edit="canPromote"
                  :busy="daySaving === 'social'"
                  :error="errorOf('social')"
                  @add="addPost"
                  @remove="removePost"
                />
              </template>

              <!-- ORGANISATIONS -->
              <WorkspaceFileSection
                id="organisations"
                :title="t('organization.workspace.proposal.sections.organizations')"
                v-bind="sectionProps('organizations')"
                @edit="editor.edit('organizations')"
                @save="editor.save()"
                @cancel="editor.cancel()"
              >
                <ul class="flex flex-col divide-y divide-border-subtle">
                  <li
                    v-for="entry in associated"
                    :key="entry.link.organization_id"
                    class="flex flex-wrap items-center gap-x-4 gap-y-1 py-3 first:pt-0 last:pb-0"
                  >
                    <span class="min-w-0 flex-1 font-medium text-text">
                      {{ entry.organization?.legal_name ?? entry.link.organization_id }}
                    </span>
                    <UiBadge :label="t(`organization.workspace.proposal.tracking.role.${entry.link.role}`)" size="sm" />
                    <UiBadge
                      v-if="entry.link.role === 'lead' && !facts?.hasLogo"
                      intent="danger"
                      :label="t('organization.workspace.proposal.sections.noLogo')"
                      size="sm"
                    />
                    <span v-if="!entry.link.confirmed_at" class="text-sm text-warning">
                      {{ t('organization.workspace.proposal.tracking.unconfirmed') }}
                    </span>
                  </li>
                </ul>
                <template #edit>
                  <ProposalStepOrganizations
                    v-if="editor.working.value"
                    v-model="editor.working.value"
                    :lead-candidates="leadCandidates"
                    :issues="editor.shownIssues.value"
                    :country-name-of="countryNameOf"
                  />
                </template>
              </WorkspaceFileSection>

              <!-- THÉMATIQUES ET LANGUES -->
              <WorkspaceFileSection
                id="thematiques"
                :title="t('organization.workspace.proposal.sections.classification')"
                v-bind="sectionProps('classification')"
                @edit="editor.edit('classification')"
                @save="editor.save()"
                @cancel="editor.cancel()"
              >
                <dl class="grid gap-x-8 gap-y-4 sm:grid-cols-2">
                  <div>
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.fields.themes') }}</dt>
                    <dd class="mt-1 text-text-secondary">{{ termLabels(editor.saved.value.theme_codes, 'themes') }}</dd>
                  </div>
                  <div>
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.fields.categories') }}</dt>
                    <dd class="mt-1 text-text-secondary">{{ termLabels(editor.saved.value.category_codes, 'categories') }}</dd>
                  </div>
                  <div>
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.fields.languages') }}</dt>
                    <dd class="mt-1 text-text-secondary">{{ localeLabels(editor.saved.value.language_codes) }}</dd>
                  </div>
                  <div v-if="editor.saved.value.country_id">
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.fields.country') }}</dt>
                    <dd class="mt-1 text-text-secondary">{{ countryNameOf(editor.saved.value.country_id) }}</dd>
                  </div>
                </dl>
                <template #edit>
                  <ProposalStepClassification
                    v-if="editor.working.value && references?.call"
                    v-model="editor.working.value"
                    :call="references.call"
                    :themes="references.themes"
                    :categories="references.categories"
                    :locales="references.locales"
                    :country-options="countryOptions"
                    :issues="editor.shownIssues.value"
                  />
                </template>
              </WorkspaceFileSection>

              <!-- FORMAT ET CRÉNEAU SOUHAITÉ -->
              <WorkspaceFileSection
                id="format"
                :title="t('organization.workspace.proposal.sections.schedule')"
                :description="t('organization.workspace.proposal.sections.scheduleHint')"
                v-bind="sectionProps('schedule')"
                @edit="editor.edit('schedule')"
                @save="editor.save()"
                @cancel="editor.cancel()"
              >
                <dl class="grid gap-x-8 gap-y-4 sm:grid-cols-3">
                  <div v-if="editor.saved.value.format">
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.tracking.format') }}</dt>
                    <dd class="mt-1 text-text-secondary">
                      {{ t(`organization.workspace.proposal.tracking.formatValue.${editor.saved.value.format}`) }}
                    </dd>
                  </div>
                  <div v-if="editor.saved.value.duration_minutes">
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.tracking.duration') }}</dt>
                    <dd class="mt-1 text-text-secondary">
                      {{ t('organization.workspace.proposal.tracking.durationValue', { count: editor.saved.value.duration_minutes }) }}
                    </dd>
                  </div>
                  <div v-if="file.tracking.proposal.preferred_start_at">
                    <dt class="text-sm text-text-subtle">{{ t('organization.workspace.proposal.tracking.preferredSlot') }}</dt>
                    <dd class="mt-1 text-text-secondary">{{ dateTime(file.tracking.proposal.preferred_start_at, timezone) }}</dd>
                  </div>
                </dl>
                <template #edit>
                  <ProposalStepSchedule
                    v-if="editor.working.value && references?.call && references.edition"
                    v-model="editor.working.value"
                    :call="references.call"
                    :edition="references.edition"
                    :issues="editor.shownIssues.value"
                  />
                </template>
              </WorkspaceFileSection>
            </template>

            <!-- Retenu mais pas encore programmé : la carte dit ce qui va venir. -->
            <WorkspaceSessionPanel
              v-if="file.tracking.sessions.length === 0 && file.tracking.proposal.status === 'accepted'"
              id="quand-ou"
              class="scroll-mt-24 rounded-lg border border-border bg-surface-raised p-5 sm:p-6"
              :sessions="file.tracking.sessions"
              :timezone="timezone"
              :edition-over="isEditionOver(file.tracking.edition)"
            />
          </div>

          <WorkspaceFileChecklist v-if="checklist.length > 0" :items="checklist" />
        </div>

        <!-- HISTORIQUE -->
        <WorkspaceHistoryList v-else :entries="file.history" :timezone="timezone" />
      </UiTabs>

      <!-- L'équipe qui agit pour l'organisation lit ses échanges dans la fiche du back-office. -->
      <WorkspaceCommentsDesk
        v-if="!file.acting_for_organization"
        v-model:open="deskOpen"
        :reference-code="file.tracking.proposal.reference_code"
        :comments="comments"
        :participants="file.participants"
        :viewer-id="auth.person?.id ?? ''"
        :loading="commentsStatus === 'pending'"
        :failed="Boolean(commentsError) && comments.length === 0"
        :busy="busy"
        :error="sendError"
        :submit="sendMessage"
        :mark-read="markCommentsRead"
        @resolve="resolve"
        @retry="refreshComments()"
      />
    </template>
  </div>
</template>
