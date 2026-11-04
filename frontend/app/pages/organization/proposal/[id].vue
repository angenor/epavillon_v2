<script setup lang="ts">
import type { ProposalFile } from '~/types/organization-workspace'
import type { ProposalOrganization } from '~/types/programme/proposal'
import type { Organization } from '~/types/org'
import type { AttachableRoleRule } from '~/types/media'
import type { SelectOption } from '~/types/ui'
import type { AssetId, Uuid } from '~/types/shared'
import type { ProposalFileSection } from '~/composables/useProposalFileEditor'
import type { FocalPointInput, ProposalDayPlan, SaveTechnicalNeedsPayload } from '~/types/programme/day-plan'

/**
 * LA FICHE D'UN DOSSIER : `/mon-organisation/dossiers/:id`,
 * `/en/my-organization/submissions/:id`.
 *
 * DIRECTION CONDENSÉE (04/11, maquette `docs/maquettes/espace-organisation/dossier.html`).
 * Le bandeau reprend celui d'une activité. Deux cartes remontent dessus : la
 * frise (et ce qu'elle entraîne) à gauche, le billet « Quand et où » à droite,
 * suivi de la liste d'avancement. Les rubriques sont des sections séparées de
 * filets, dans cet ordre arrêté par le commanditaire : « Votre dossier »
 * (couverture en premier), « Le jour J », « Faire connaître votre activité ».
 *
 * TOUT SE MODIFIE SUR PLACE (29/10) ; un brouillon passe par le formulaire.
 * Retenu (ou annulé), « Format et créneau souhaité » disparaît : le créneau est
 * celui du billet (04/11).
 *
 * L'ORGANISATION PEUT ANNULER son activité retenue tant qu'aucune séance n'a
 * commencé (04/11) : motif obligatoire, puis le dossier se relit, annulé.
 *
 * CE QUE L'ORGANISATION NE VOIT PAS, ET QUI N'EST PAS UN OUBLI : aucune note,
 * aucun rang, aucun nom de membre du comité, aucune liste d'inscrits, aucun
 * nombre d'inscrits. Le filtre est appliqué à la source.
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
const { date, dateTime, intlLocale, timeRangeFull, zoneLabel, zoneOffsetShort } = useDateTime()
const localePath = useLocalePath()
const route = useRoute()
const router = useRouter()
const api = useApi()
const auth = useAuthStore()
const memberships = useMembershipStore()
const requestUrl = useRequestURL()

const dayStyle = trackDayStyle(null)

const proposalId = computed(() => String(route.params.id ?? ''))
useSpaceVisit(() => (proposalId.value ? { proposal_id: proposalId.value } : null))

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
  {
    watch: [proposalId, () => auth.person?.id],
    lazy: true,
    // Relu à chaque arrivée sur la page : en revenant de l'éditeur de l'image, le
    // dossier gardé en mémoire montrait l'ancienne couverture (01/11).
    getCachedData: (key, nuxtApp) => (nuxtApp.isHydrating ? nuxtApp.payload.data[key] : undefined),
  },
)

useHead(() => ({
  title: file.value
    ? `${file.value.tracking.proposal.reference_code} · ${tr(file.value.tracking.proposal.title)}`
    : t('organization.workspace.proposal.notFound.title'),
}))

const editor = useProposalFileEditor(proposalId, async () => {
  await refresh()
})

const loading = computed(() => status.value === 'pending' && file.value?.tracking.proposal.id !== proposalId.value)

// La photo et le logo clair de l'édition, pour le bandeau.
const editionId = computed(() => file.value?.tracking.edition.id ?? null)
const { data: editionImages } = useAsyncData(
  () => `proposal-file-edition-images-${editionId.value ?? 'none'}`,
  () => (editionId.value ? api.events.images(editionId.value) : Promise.resolve(null)),
  { lazy: true, server: false, watch: [editionId] },
)

// ---------------------------------------------------------------------------
// Vues : « Mon dossier », « Historique ». `?vue=sessions` (liens déjà envoyés)
// ouvre le dossier sur le billet « Quand et où ».
// ---------------------------------------------------------------------------

const activeTab = computed<string>(() => {
  const requested = route.query.vue
  const asked = String(Array.isArray(requested) ? requested[0] : requested)
  return asked === 'history' ? asked : 'tracking'
})

function selectTab(value: string): void {
  router.replace({ query: { ...route.query, vue: value } })
}

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
// Frise, décision, annulation, droits de modification
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
        activityCancelled: t('organization.workspace.proposals.timeline.activityCancelled'),
        scheduled: t('organization.workspace.proposals.timeline.scheduled'),
      })
    : [],
)

const timezone = computed(() => file.value?.tracking.edition.timezone ?? 'UTC')

/** « Heure d'Antalya (UTC+3) » : l'historique se lit à l'heure de l'édition. */
const zoneOfEdition = computed(() => {
  const label = t('organization.workspace.proposal.sessions.zone', {
    zone: zoneLabel(timezone.value, file.value?.tracking.edition.city ?? undefined),
    offset: zoneOffsetShort(timezone.value),
  })
  return label.charAt(0).toLocaleUpperCase(locale.value) + label.slice(1)
})
const proposalStatus = computed(() => file.value?.tracking.proposal.status ?? 'draft')
const isDraft = computed(() => proposalStatus.value === 'draft')
const isAccepted = computed(() => proposalStatus.value === 'accepted')
/** Retenu, ou annulé : le créneau est celui de l'IFDD, la demande de format n'a plus d'objet. */
const retained = computed(() => isAccepted.value || proposalStatus.value === 'cancelled')

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
    .join('. ')
})

const cancellation = computed(() =>
  file.value?.tracking.proposal.status === 'cancelled'
    ? proposalCancellation(file.value.tracking.transitions, file.value.history)
    : null,
)

/** Qui a annulé, quand l'historique ne le nomme pas : un auteur du fil, ou soi-même. */
const cancelledByFallback = computed(() => {
  const actorId = cancellation.value?.actorId
  if (!actorId) return ''
  const participant = file.value?.participants.find((person) => person.id === actorId)
  if (participant) return participant.display_name
  return actorId === auth.person?.id ? `${auth.person.first_name} ${auth.person.last_name}`.trim() : ''
})

// ---------------------------------------------------------------------------
// Annulation par l'organisation (04/11)
// ---------------------------------------------------------------------------

const canCancel = computed(() => (file.value ? canOrganizationCancel(file.value.tracking) : false))
const cancelOpen = ref(false)
const cancelling = ref(false)
const cancelError = ref<string | null>(null)

/** Le rappel du créneau dans la fenêtre, fuseau compris. */
const cancelWhen = computed(() => {
  const first = file.value?.tracking.sessions[0]
  if (!first) return ''
  const day = formatDate(first.session.starts_at, { locale: intlLocale.value, timeZone: timezone.value, dateStyle: 'full' })
  const hours = timeRangeFull(first.session.starts_at, first.session.ends_at, timezone.value, file.value?.tracking.edition.city ?? undefined)
  const label = t('organization.workspace.proposal.cancel.when', { day, hours })
  return label.charAt(0).toLocaleUpperCase(locale.value) + label.slice(1)
})

function openCancel(): void {
  cancelError.value = null
  cancelOpen.value = true
}

async function confirmCancel(reason: string): Promise<void> {
  const person = auth.person
  if (!person) return
  cancelling.value = true
  cancelError.value = null
  try {
    await api.proposals.cancel(person.id, proposalId.value, { reason })
    cancelOpen.value = false
    await refresh()
    if (import.meta.client) window.scrollTo({ top: 0, behavior: 'smooth' })
  } catch (thrown) {
    cancelError.value = apiErrorMessage(thrown, (key) => t(key))
  } finally {
    cancelling.value = false
  }
}

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

async function refreshAll(): Promise<void> {
  await Promise.all([refresh(), editor.refresh()])
}

const leadOrganization = computed(() =>
  memberships.submittable.find((entry) => entry.organization.id === file.value?.tracking.proposal.organization_id),
)

const canSetLogo = computed(() => leadOrganization.value?.membership.role === 'manager')

// ---------------------------------------------------------------------------
// Ce qui manque, liste d'avancement
// ---------------------------------------------------------------------------

const facts = computed(() =>
  editor.saved.value && file.value
    ? {
        draft: editor.saved.value,
        hasCover: file.value.tracking.cover !== null,
        hasLogo: file.value.organization_logo !== null,
        dayPlan: dayPlan.value,
        retained: retained.value,
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

type DayPart = 'focal' | 'needs' | 'social' | 'presentations'
const daySaving = ref<DayPart | null>(null)
const dayError = ref<{ part: DayPart; message: string } | null>(null)

async function dayWrite(part: DayPart, write: () => Promise<ProposalDayPlan | void>): Promise<void> {
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

const errorOf = (part: DayPart) => (dayError.value?.part === part ? dayError.value.message : null)

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

/** Le nom complet de la porteuse, pour le bandeau. */
const organizationName = computed(
  () =>
    leadOrganization.value?.organization.legal_name ??
    associated.value.find((entry) => entry.link.role === 'lead')?.organization?.legal_name ??
    leadName.value ??
    '',
)

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

const leadLogo = computed(() => {
  const asset = file.value?.organization_logo
  return asset ? (asset.sources.thumb?.url ?? asset.url) : null
})

function sectionProps(section: ProposalFileSection) {
  return {
    canEdit: canEditSections.value,
    editing: editor.editing.value === section,
    // Les autres rubriques restent ouvrables : passer à l'une enregistre celle en cours.
    locked: editor.saving.value && editor.editing.value !== section,
    saving: editor.saving.value,
    error: editor.editing.value === section ? editor.error.value : null,
  }
}

// ---------------------------------------------------------------------------
// Colonne de droite : le billet d'un dossier retenu, la liste d'avancement
// ---------------------------------------------------------------------------

const showTicket = computed(() => isAccepted.value)
const showChecklist = computed(() => activeTab.value === 'tracking' && checklist.value.length > 0)
const hasAside = computed(() => showTicket.value || showChecklist.value)

// ---------------------------------------------------------------------------
// Les échanges vivent dans l'assistance de chaque écran (02/11) : la page lui
// dit quel dossier elle montre, et suit l'état qu'une correction résolue change.
// « Écrire à l'équipe » l'ouvre (04/11).
// ---------------------------------------------------------------------------

const assistance = useAssistance()
assistance.attach({ proposalId: () => proposalId.value })
onMounted(() => {
  if (route.query.vue === 'exchanges' || route.hash === '#echanges') assistance.open.value = true
})
watch(assistance.revision, () => refresh())

function writeTeam(): void {
  assistance.open.value = true
}

// L'état du dossier change sous les yeux : la saisie en cours vit à part (`editor`).
useRealtime(
  () => [`proposal:${proposalId.value}:status`],
  () => refresh(),
)
</script>

<template>
  <div data-direction="condensee" :style="dayStyle" class="full-bleed -mt-8 -mb-8 min-h-[75vh] bg-surface font-sans text-text sm:-mt-10 sm:-mb-10">
    <!-- Chargement, erreur, introuvable : le bandeau garde son lien de retour. -->
    <template v-if="loading || error || !file">
      <UiMediaBanner class="max-lg:pb-9!" :back-to="localePath('/mon-organisation')" :back-label="t('organization.workspace.proposal.breadcrumb')">
        <div v-if="loading" class="mt-5 flex flex-col gap-2.5" aria-hidden="true">
          <span class="h-12 max-w-160 animate-pulse rounded-md bg-surface-inverse-raised motion-reduce:animate-none" />
          <span class="h-12 max-w-105 animate-pulse rounded-md bg-surface-inverse-raised motion-reduce:animate-none" />
        </div>
      </UiMediaBanner>
      <div class="mx-auto max-w-7xl px-4 pb-24 sm:px-6">
        <div class="relative z-1 pt-6 lg:-mt-18 lg:pt-0">
          <WorkspacePanel v-if="loading" :aria-busy="true">
            <UiLoadingState variant="card" :lines="5" />
          </WorkspacePanel>
          <WorkspacePanel v-else-if="error">
            <UiErrorState :retry-label="t('common.actions.retry')" @retry="refresh()" />
          </WorkspacePanel>
          <WorkspacePanel v-else>
            <UiEmptyState
              icon="document"
              :title="t('organization.workspace.proposal.notFound.title')"
              :description="t('organization.workspace.proposal.notFound.description')"
              :action-to="localePath('/mon-organisation')"
              :action-label="t('organization.workspace.proposal.notFound.action')"
            />
          </WorkspacePanel>
        </div>
      </div>
    </template>

    <template v-else>
      <WorkspaceFileHero
        :file="file"
        :image="editionImages?.cover ?? editionImages?.banner ?? null"
        :edition-logo="editionImages?.logo_inverse ?? null"
        :organization-name="organizationName"
        :organization-acronym="leadOrganization?.organization.acronym ?? associated.find((entry) => entry.link.role === 'lead')?.organization?.acronym ?? null"
        :activity-cancelled="cancellation?.afterAcceptance ?? false"
        :view="activeTab"
        panel-id="dossier-vue"
        @update:view="selectTab"
      />

      <!-- Les marges négatives n'existent qu'à partir de `lg` : sur une colonne, rien ne remonte. -->
      <div class="mx-auto max-w-7xl px-4 pb-24 sm:px-6">
        <div
          id="dossier-vue"
          class="grid grid-cols-1 gap-6 pt-6 lg:gap-x-10 lg:gap-y-0 lg:pt-0"
          :class="hasAside ? 'lg:grid-cols-[minmax(0,1fr)_360px]' : ''"
        >
          <div class="relative z-1 flex min-w-0 flex-col gap-5 lg:col-start-1 lg:row-start-1 lg:-mt-18">
            <div v-if="file.acting_for_organization" class="overflow-hidden rounded-lg shadow-lg">
              <UiAlert
                intent="warning"
                :title="t('organization.workspace.proposal.acting.title', { organization: leadName ?? '' })"
                :message="t('organization.workspace.proposal.acting.description')"
              />
            </div>

            <template v-if="activeTab === 'tracking'">
              <WorkspaceFileProgress
                :steps="steps"
                :timezone="timezone"
                :reference-code="file.tracking.proposal.reference_code"
                :status="file.tracking.proposal.status"
                :decided-at="file.tracking.proposal.decided_at"
                :decision-message="decisionMessage"
                :cancellation="cancellation"
                :cancelled-by-fallback="cancelledByFallback"
                :is-draft="isDraft"
                :edit-blocked-reason="editBlockedReason"
                :draft-to="draftTo"
              />

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
            </template>

            <WorkspacePanel
              v-else
              heading-id="workspace-history-title"
              :title="t('organization.workspace.proposal.history.title')"
              :meta="zoneOfEdition"
            >
              <WorkspaceHistoryList :entries="file.history" :timezone="timezone" condensed />
            </WorkspacePanel>
          </div>

          <aside
            v-if="hasAside"
            class="relative z-1 flex min-w-0 flex-col gap-5 lg:col-start-2 lg:row-span-2 lg:row-start-1 lg:-mt-18 lg:self-stretch"
            :aria-label="t('organization.workspace.proposal.sessions.title')"
          >
            <WorkspaceSessionPanel
              v-if="showTicket"
              :sessions="file.tracking.sessions"
              :timezone="timezone"
              :city="file.tracking.edition.city"
              :public-url="publicUrl"
              :file-name="file.tracking.proposal.reference_code"
              :can-cancel="canCancel"
              @write-team="writeTeam"
              @cancel="openCancel"
            />
            <WorkspaceFileChecklist v-if="showChecklist" :items="checklist" :raised="!showTicket" />
          </aside>

          <div v-if="activeTab === 'tracking'" class="min-w-0 lg:col-start-1 lg:row-start-2">
            <!-- VOTRE DOSSIER : en premier, couverture en tête (04/11). -->
            <section class="mt-12 lg:mt-16" aria-labelledby="dossier-title">
              <WorkspaceRule
                heading-id="dossier-title"
                icon="document"
                :label="t('organization.workspace.proposal.sections.dossier')"
                :meta="canEditSections ? t('organization.workspace.proposal.sections.dossierMeta') : ''"
              />

              <WorkspaceFileCover
                :proposal-id="file.tracking.proposal.id"
                :title="tr(file.tracking.proposal.title)"
                :cover="file.tracking.cover"
                :can-edit="canEditMedia"
                :rule="mediaRules?.cover ?? null"
                @changed="refresh()"
              />

              <UiLoadingState v-if="!editor.saved.value" variant="card" :lines="4" />

              <template v-else>
                <WorkspaceFileSection
                  id="presentation"
                  :title="t('organization.workspace.proposal.sections.presentation')"
                  v-bind="sectionProps('presentation')"
                  @edit="editor.edit('presentation')"
                  @save="editor.save()"
                  @cancel="editor.cancel()"
                >
                  <dl class="grid gap-4.5">
                    <div>
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.fields.title') }}</dt>
                      <dd class="mt-1 font-semibold text-text">{{ editor.saved.value.title }}</dd>
                    </div>
                    <div>
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.fields.objectives') }}</dt>
                      <dd class="mt-1 text-text-secondary">{{ plainTextOf(editor.saved.value.objectives) }}</dd>
                    </div>
                    <div>
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.fields.detailedPresentation') }}</dt>
                      <dd class="mt-1 line-clamp-6 text-text-secondary">{{ plainTextOf(editor.saved.value.detailed_presentation) }}</dd>
                    </div>
                    <div v-if="plainTextOf(editor.saved.value.expected_outcomes).trim()">
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.fields.expectedOutcomes') }}</dt>
                      <dd class="mt-1 text-text-secondary">{{ plainTextOf(editor.saved.value.expected_outcomes) }}</dd>
                    </div>
                  </dl>
                  <template #edit>
                    <ProposalStepPresentation v-if="editor.working.value" v-model="editor.working.value" :issues="editor.shownIssues.value" />
                  </template>
                </WorkspaceFileSection>

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

                <WorkspaceFileSection
                  id="organisations"
                  :title="t('organization.workspace.proposal.sections.organizations')"
                  v-bind="sectionProps('organizations')"
                  @edit="editor.edit('organizations')"
                  @save="editor.save()"
                  @cancel="editor.cancel()"
                >
                  <ul class="flex flex-col">
                    <li
                      v-for="entry in associated"
                      :key="entry.link.organization_id"
                      class="flex flex-wrap items-center gap-x-3.5 gap-y-1.5 border-t border-border-subtle py-3 first:border-t-0 first:pt-0"
                    >
                      <span
                        class="grid h-10 w-14 shrink-0 place-items-center rounded-sm border border-border-subtle bg-surface-inverse-selected p-0.75 font-condensed text-[0.8rem] font-bold text-text-on-inverse-selected"
                        aria-hidden="true"
                      >
                        <img v-if="entry.link.role === 'lead' && leadLogo" :src="leadLogo" alt="" class="max-h-full max-w-full object-contain">
                        <template v-else>{{ entry.organization?.acronym ?? initialsOf(entry.organization?.legal_name ?? '') }}</template>
                      </span>
                      <span class="min-w-0 flex-[1_1_220px] font-semibold text-text">
                        {{ entry.organization?.legal_name ?? entry.link.organization_id }}
                      </span>
                      <UiBadge
                        :intent="entry.link.role === 'lead' ? 'info' : 'neutral'"
                        :label="t(`organization.workspace.proposal.tracking.role.${entry.link.role}`)"
                      />
                      <UiBadge
                        v-if="entry.link.role === 'lead' && !facts?.hasLogo"
                        intent="danger"
                        :label="t('organization.workspace.proposal.sections.noLogo')"
                      />
                      <span v-if="!entry.link.confirmed_at" class="text-[0.85rem] font-semibold text-warning">
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

                <WorkspaceFileSection
                  id="thematiques"
                  :title="t('organization.workspace.proposal.sections.classification')"
                  v-bind="sectionProps('classification')"
                  @edit="editor.edit('classification')"
                  @save="editor.save()"
                  @cancel="editor.cancel()"
                >
                  <dl class="grid gap-x-8 gap-y-4.5 sm:grid-cols-2">
                    <div>
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.fields.themes') }}</dt>
                      <dd class="mt-1 text-text-secondary">{{ termLabels(editor.saved.value.theme_codes, 'themes') }}</dd>
                    </div>
                    <div>
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.fields.categories') }}</dt>
                      <dd class="mt-1 text-text-secondary">{{ termLabels(editor.saved.value.category_codes, 'categories') }}</dd>
                    </div>
                    <div>
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.fields.languages') }}</dt>
                      <dd class="mt-1 text-text-secondary">{{ localeLabels(editor.saved.value.language_codes) }}</dd>
                    </div>
                    <div v-if="editor.saved.value.country_id">
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.fields.country') }}</dt>
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

                <!-- Retenu ou annulé : le créneau est celui du billet, la demande n'a plus d'objet (04/11). -->
                <WorkspaceFileSection
                  v-if="!retained"
                  id="format"
                  :title="t('organization.workspace.proposal.sections.schedule')"
                  :description="t('organization.workspace.proposal.sections.scheduleHint')"
                  v-bind="sectionProps('schedule')"
                  @edit="editor.edit('schedule')"
                  @save="editor.save()"
                  @cancel="editor.cancel()"
                >
                  <dl class="grid gap-x-8 gap-y-4.5 sm:grid-cols-3">
                    <div v-if="editor.saved.value.format">
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.tracking.format') }}</dt>
                      <dd class="mt-1 text-text-secondary">
                        {{ t(`organization.workspace.proposal.tracking.formatValue.${editor.saved.value.format}`) }}
                      </dd>
                    </div>
                    <div v-if="editor.saved.value.duration_minutes">
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.tracking.duration') }}</dt>
                      <dd class="mt-1 text-text-secondary">
                        {{ t('organization.workspace.proposal.tracking.durationValue', { count: editor.saved.value.duration_minutes }) }}
                      </dd>
                    </div>
                    <div v-if="file.tracking.proposal.preferred_start_at">
                      <dt class="text-[0.82rem] text-text-subtle">{{ t('organization.workspace.proposal.tracking.preferredSlot') }}</dt>
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
            </section>

            <!-- LE JOUR J : activité retenue seulement. -->
            <section v-if="dayPlan" class="mt-12 lg:mt-16" aria-labelledby="jourj-title">
              <WorkspaceRule
                heading-id="jourj-title"
                tone="success"
                icon="calendar"
                :label="t('organization.workspace.proposal.dayPlan.title')"
                :meta="t('organization.workspace.proposal.dayPlan.meta')"
              />
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
              <WorkspaceReminderSchedule
                v-for="(tracked, index) in file.tracking.sessions"
                :key="tracked.session.id"
                :section-id="index === 0 ? 'rappels' : `rappels-${index + 1}`"
                :session-label="file.tracking.sessions.length > 1 ? date(tracked.session.starts_at, timezone) : ''"
                :reminders="tracked.reminders"
                :timezone="tracked.session.timezone"
                :zone-label="file.tracking.edition.city ?? undefined"
              />
            </section>

            <!-- FAIRE CONNAÎTRE VOTRE ACTIVITÉ -->
            <section v-if="dayPlan" class="mt-12 lg:mt-16" aria-labelledby="promo-title">
              <WorkspaceRule
                heading-id="promo-title"
                tone="success"
                icon="broadcast"
                :label="t('organization.workspace.proposal.dayPlan.promote')"
              />
              <WorkspaceFileSocialPosts
                :posts="dayPlan.social_posts"
                :can-edit="canPromote"
                :busy="daySaving === 'social'"
                :error="errorOf('social')"
                @add="addPost"
                @remove="removePost"
              />
            </section>
          </div>
        </div>
      </div>

      <WorkspaceFileCancelDialog
        v-model:open="cancelOpen"
        :title="tr(file.tracking.proposal.title)"
        :when="cancelWhen"
        :busy="cancelling"
        :error="cancelError"
        @confirm="confirmCancel"
      />
    </template>
  </div>
</template>
