<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
import type { InviteMemberPayload, ProposalTracking, WorkspaceOverview } from '~/types/organization-workspace'
import type { Intent, SelectOption } from '~/types/ui'
import type { WorkspaceProposalFilter } from '~/utils/workspace-editions'
import type { Uuid } from '~/types/shared'

/**
 * ESPACE ORGANISATION : `/mon-organisation`, `/en/my-organization`.
 *
 * IL RÉPOND À DEUX QUESTIONS, ET DANS CET ORDRE : « qu'est-ce qui attend une
 * action de ma part ? », puis « où en est chacun de mes dossiers ? ». Une
 * organisation qui ouvre cette page vient d'abord savoir si elle a quelque
 * chose à faire.
 *
 * DIRECTION CONDENSÉE (04/11, maquette `docs/maquettes/espace-organisation/`) :
 * bandeau sur la photo de l'édition ouverte, onglets d'édition posés dessus ;
 * « À faire » et le billet de la prochaine activité remontent sur le bandeau ;
 * « Mes dossiers » et « Membres » ouvrent sur une étiquette posée sur un filet.
 *
 * QUATRE DÉCISIONS, ET CHACUNE SE PAIE SI ON L'INVERSE :
 *
 *  1. LE BLOC D'ACTIONS EST EN TÊTE, ET IL RESTE LISIBLE VIDE.
 *  2. « CORRECTIONS DEMANDÉES » SE VOIT DE LOIN, avec le NOMBRE de points à traiter.
 *  3. UNE ÉDITION À LA FOIS (28/10), un onglet chacune, l'édition vivante
 *     ouverte d'office ; la recherche parcourt toutes les éditions et range ce
 *     qu'elle trouve ailleurs sous le nom de l'édition.
 *  4. L'ÉTAT VIDE MET L'APPEL EN AVANT.
 *
 * LA PAGE EST GARDÉE PAR `requires-organization`, comme le formulaire de dépôt.
 * QUATRE ÉTATS : chargement, erreur (avec reprise), vide, accès refusé.
 */

definePageMeta({
  layout: 'public',
  middleware: ['auth', 'requires-organization'],
  // La raison existe depuis A2 (`organization.join.required.reasons`) : elle a
  // été écrite pour cet écran-ci, il n'y en a pas de seconde à créer.
  organizationReason: 'organization-space',
})

defineI18nRoute({ paths: { fr: '/mon-organisation', en: '/my-organization' } })

const { t } = useI18n()
const { tr } = useI18nText()
const { dateRange } = useDateTime()
const localePath = useLocalePath()
const api = useApi()
const auth = useAuthStore()
const memberships = useMembershipStore()

useHead(() => ({ title: t('organization.workspace.title') }))

const dayStyle = trackDayStyle(null)

// ---------------------------------------------------------------------------
// L'organisation consultée, dans l'URL (`?organisation=…`) pour qu'un lien
// envoyé par courriel ouvre la bonne.
// ---------------------------------------------------------------------------

const route = useRoute()
const router = useRouter()

await memberships.ensureLoaded()

const activeOrganizations = computed(() => memberships.submittable)

const currentOrganizationId = computed<Uuid | null>(() => {
  const requested = route.query.organisation
  const asked = Array.isArray(requested) ? requested[0] : requested
  const known = activeOrganizations.value.find((entry) => entry.organization.id === asked)
  return known?.organization.id ?? activeOrganizations.value[0]?.organization.id ?? null
})

const organizationOptions = computed<SelectOption[]>(() =>
  activeOrganizations.value.map((entry) => ({
    value: entry.organization.id,
    label: entry.organization.acronym
      ? `${entry.organization.legal_name} (${entry.organization.acronym})`
      : entry.organization.legal_name,
  })),
)

function selectOrganization(id: string): void {
  router.replace({ query: { ...route.query, organisation: id } })
}

// ---------------------------------------------------------------------------
// Le contenu
// ---------------------------------------------------------------------------

const {
  data: overview,
  status,
  error,
  refresh,
} = await useAsyncData<WorkspaceOverview | null>(
  'organization-workspace',
  async () => {
    const organizationId = currentOrganizationId.value
    const person = auth.person
    if (!organizationId || !person) return null
    return api.workspace.overview(organizationId, person.id)
  },
  { watch: [currentOrganizationId], lazy: true },
)

const loading = computed(() => status.value === 'pending' && overview.value?.organization.id !== currentOrganizationId.value)
/** L'organisation affichée : nulle tant qu'on charge, en erreur ou refusé. */
const shown = computed(() => (loading.value || error.value ? null : overview.value))

// Un message de l'équipe, un dossier qui change d'état : la liste et ses pastilles suivent.
useRealtime(
  () => (currentOrganizationId.value ? [`organization:${currentOrganizationId.value}`] : []),
  () => refresh(),
)

useSpaceVisit(() => (currentOrganizationId.value ? { organization_id: currentOrganizationId.value } : null))

// ---------------------------------------------------------------------------
// Une édition à la fois (28/10), dans l'URL (`?edition=slug`).
// ---------------------------------------------------------------------------

const tabs = computed(() => (overview.value ? workspaceEditionTabs(overview.value) : []))

const activeTab = computed(() => {
  const requested = route.query.edition
  return defaultEditionTab(tabs.value, Array.isArray(requested) ? requested[0] : requested)
})

// Ouvrir un onglet, c'est l'avoir vu : sa pastille et celle du compte s'éteignent.
const seen = useWorkspaceSeen()
watch(
  activeTab,
  (tab) => {
    if (!tab) return
    seen.markTab(tab)
    seen.markNav(tabs.value)
  },
  { immediate: true },
)

function selectTab(slug: string): void {
  router.replace({ query: { ...route.query, edition: slug }, hash: route.hash })
}

function editionName(edition: EventEdition): string {
  return edition.acronym ?? tr(edition.title)
}

// La photo de l'édition ouverte : changer d'onglet change le bandeau.
const editionId = computed(() => activeTab.value?.edition.id ?? null)
const { data: editionImages } = useAsyncData(
  () => `workspace-edition-images-${editionId.value ?? 'none'}`,
  () => (editionId.value ? api.events.images(editionId.value) : Promise.resolve(null)),
  { lazy: true, server: false, watch: [editionId] },
)
const bannerImage = computed(() => (shown.value && editionId.value ? (editionImages.value?.cover ?? editionImages.value?.banner ?? null) : null))

// ---------------------------------------------------------------------------
// Recherche et filtre : la recherche parcourt TOUTES les éditions.
// ---------------------------------------------------------------------------

const query = ref('')
const filter = ref<WorkspaceProposalFilter>('all')

const actionIndex = computed(() => proposalActionIndex(overview.value?.actions ?? []))

const filterOptions = computed<SelectOption[]>(() =>
  WORKSPACE_PROPOSAL_FILTERS.map((value) => ({ value, label: t(`organization.workspace.filters.${value}`) })),
)

const keep = (tracking: ProposalTracking): boolean =>
  matchesProposalFilter(tracking, filter.value, actionIndex.value) && matchesProposalQuery(tracking, query.value)

const searching = computed(() => query.value.trim().length > 0)

const listed = computed(() => activeTab.value?.proposals.filter(keep) ?? [])

const elsewhere = computed(() =>
  searching.value
    ? tabs.value
        .filter((tab) => tab !== activeTab.value)
        .map((tab) => ({ tab, proposals: tab.proposals.filter(keep) }))
        .filter((group) => group.proposals.length > 0)
    : [],
)

function showMissingReports(): void {
  query.value = ''
  filter.value = 'report'
  document.getElementById('mes-dossiers')?.scrollIntoView({ behavior: 'smooth' })
}

const editionSpan = computed(() => {
  const edition = activeTab.value?.edition
  if (!edition) return ''
  return [editionName(edition), edition.city, dateRange(edition.starts_at, edition.ends_at, edition.timezone)].filter(Boolean).join(', ')
})

// ---------------------------------------------------------------------------
// Ce qui remonte sur le bandeau, à droite : la prochaine activité, l'appel ouvert.
// ---------------------------------------------------------------------------

const next = computed(() => (activeTab.value && !activeTab.value.over ? nextWorkspaceActivity(activeTab.value.proposals) : null))
const callInTab = computed(() => Boolean(shown.value?.call_edition && shown.value.call_edition.id === activeTab.value?.edition.id))
const isEmpty = computed(() => Boolean(shown.value) && tabs.value.length === 0)
const hasAside = computed(() => Boolean(shown.value) && (isEmpty.value || next.value !== null || callInTab.value))
// La liste des dossiers suit « À faire » dans la colonne de gauche, sans attendre la droite (04/11) ; posée sur
// la page, elle ne remonte pas sur le bandeau : la colonne ne monte que si elle s'ouvre sur une carte.
const raised = computed(
  () =>
    loading.value ||
    Boolean(error.value) ||
    !shown.value ||
    shown.value.membership.status === 'pending' ||
    isEmpty.value ||
    Boolean(activeTab.value?.over) ||
    (activeTab.value?.actions.length ?? 0) > 0,
)

const logo = computed(() => {
  const asset = shown.value?.organization_logo
  return asset ? (asset.sources.thumb?.url ?? asset.url) : null
})

const isManager = computed(() => overview.value?.membership.role === 'manager')

// ---------------------------------------------------------------------------
// Invitation d'un membre, décision sur une demande
// ---------------------------------------------------------------------------

const inviting = ref(false)
const memberMessage = ref<{ intent: Intent; text: string } | null>(null)

/** Le refus de l'API s'affiche tel quel ; le site ne parle que si elle s'est tue. */
function refusalOf(thrown: unknown): string {
  return thrown instanceof ForbiddenError ? thrown.message : apiErrorMessage(thrown, (key) => t(key))
}

async function invite(payload: Omit<InviteMemberPayload, 'organization_id'>): Promise<void> {
  const organizationId = currentOrganizationId.value
  const person = auth.person
  if (!organizationId || !person) return

  inviting.value = true
  memberMessage.value = null
  try {
    const result = await api.organizations.invite(person.id, { ...payload, organization_id: organizationId })
    memberMessage.value = {
      // Seule l'invitation réellement partie est une réussite : « déjà membre »
      // et « déjà invitée » sont des réponses, pas des échecs de l'envoi.
      intent: result.status === 'invited' ? 'success' : 'info',
      text: t(`organization.workspace.members.invite.result.${result.status}`, {
        email: result.entry.person.primary_email,
      }),
    }
    await refresh()
  } catch (error) {
    memberMessage.value = { intent: 'danger', text: refusalOf(error) }
  } finally {
    inviting.value = false
  }
}

/** Un référent tranche une demande d'adhésion. Une invitation ne s'approuve pas. */
async function decide(membershipId: string, approved: boolean): Promise<void> {
  const person = auth.person
  if (!person) return
  inviting.value = true
  memberMessage.value = null
  try {
    await api.organizations.decideMembership(person.id, { membership_id: membershipId, approved })
    await refresh()
  } catch (error) {
    memberMessage.value = { intent: 'danger', text: refusalOf(error) }
  } finally {
    inviting.value = false
  }
}
</script>

<template>
  <div data-direction="condensee" :style="dayStyle" class="full-bleed -mt-8 -mb-8 min-h-[75vh] bg-surface font-sans text-text sm:-mt-10 sm:-mb-10">
    <WorkspaceHero
      :organization="shown?.organization ?? null"
      :logo="logo"
      :image="bannerImage"
      :loading="loading"
      :tabs="tabs"
      :active-slug="activeTab?.edition.slug ?? null"
      :organization-options="organizationOptions"
      :organization-id="currentOrganizationId"
      panel-id="mes-dossiers"
      @select-organization="selectOrganization"
      @select-tab="selectTab"
    />

    <!-- Les marges négatives n'existent qu'à partir de `lg` : sur une colonne, rien ne remonte. -->
    <div class="mx-auto max-w-7xl px-4 pb-24 sm:px-6">
      <div class="grid grid-cols-1 gap-6 pt-6 lg:grid-cols-[minmax(0,1fr)_360px] lg:items-start lg:gap-x-10 lg:gap-y-0 lg:pt-0">
        <div class="relative z-1 flex min-w-0 flex-col gap-5" :class="[raised ? 'lg:-mt-18' : 'lg:pt-10', hasAside ? '' : 'lg:col-span-2']">
          <WorkspacePanel v-if="loading" :aria-busy="true">
            <UiLoadingState variant="card" :lines="4" />
          </WorkspacePanel>

          <WorkspacePanel v-else-if="error">
            <UiErrorState :retry-label="t('common.actions.retry')" @retry="refresh()" />
          </WorkspacePanel>

          <!-- L'adhésion a disparu entre la garde et la réponse : le quatrième état. -->
          <WorkspacePanel v-else-if="!shown">
            <UiForbiddenState
              :action-to="localePath('/rattachement-organisation')"
              :action-label="t('organization.workspace.membershipPending.action')"
            />
          </WorkspacePanel>

          <template v-else>
            <div v-if="shown.membership.status === 'pending'" class="overflow-hidden rounded-lg shadow-lg">
              <UiAlert
                intent="info"
                :title="t('organization.workspace.membershipPending.title')"
                :message="t('organization.workspace.membershipPending.description', { organization: shown.organization.legal_name })"
              />
            </div>

            <!-- Aucun dossier ni appel : l'état vide met l'appel en avant, et rien d'autre. -->
            <WorkspacePanel v-if="isEmpty">
              <UiEmptyState
                icon="document"
                :title="t('organization.workspace.proposals.empty.title')"
                :description="t('organization.workspace.proposals.empty.description')"
              />
            </WorkspacePanel>

            <!-- Édition terminée : un nombre et un geste, pas la liste des comptes rendus. -->
            <WorkspacePanel v-else-if="activeTab?.over" :aria-label="t('organization.workspace.reports.label')">
              <div class="flex flex-wrap items-center justify-between gap-x-6 gap-y-3">
                <div>
                  <p class="flex items-baseline gap-3">
                    <b class="font-condensed text-[3.4rem] leading-[0.85] font-bold text-heading tabular-nums">{{ activeTab.missingReports }}</b>
                    <span class="font-semibold text-text">{{ t('organization.workspace.reports.count', activeTab.missingReports) }}</span>
                  </p>
                  <p class="mt-1 text-sm text-text-muted">
                    {{ t('organization.workspace.reports.description', { edition: activeTab.edition.city ?? editionName(activeTab.edition) }, activeTab.missingReports) }}
                  </p>
                </div>
                <UiButton v-if="activeTab.missingReports > 0" variant="secondary" @click="showMissingReports()">
                  {{ t('organization.workspace.editions.showMissingReports') }}
                </UiButton>
              </div>
            </WorkspacePanel>

            <WorkspaceActionList
              v-else-if="activeTab && activeTab.actions.length > 0"
              :actions="activeTab.actions"
              :timezone="activeTab.edition.timezone"
            />
          </template>

          <section
            v-if="shown && activeTab"
            id="mes-dossiers"
            class="scroll-mt-24"
            :class="raised ? 'mt-4 lg:mt-6' : ''"
            aria-labelledby="workspace-proposals-title"
          >
            <WorkspaceRule
              heading-id="workspace-proposals-title"
              :label="t('organization.workspace.proposals.title')"
              :count="activeTab.proposals.length"
              :meta="editionSpan"
            />

            <div class="mt-5 grid grid-cols-1 gap-x-4 gap-y-3 sm:grid-cols-[minmax(0,340px)_minmax(0,240px)] sm:justify-end">
              <UiSearchInput
                id="workspace-search"
                v-model="query"
                :label="t('organization.workspace.search.label')"
                :placeholder="t('organization.workspace.search.placeholder')"
                :min-length="1"
                :debounce="0"
              />
              <UiSelect
                id="workspace-filter"
                :model-value="filter"
                :options="filterOptions"
                :label="t('organization.workspace.filters.label')"
                hide-optional
                @update:model-value="filter = ($event as WorkspaceProposalFilter | null) ?? 'all'"
              />
            </div>

            <p v-if="searching" class="mt-6 text-[0.85rem] font-bold text-text-secondary">
              {{ t('organization.workspace.search.inTab', { edition: editionName(activeTab.edition) }) }}
            </p>
            <ul v-if="listed.length > 0" class="mt-2">
              <li v-for="tracking in listed" :key="tracking.proposal.id" class="border-b border-border-subtle last:border-b-0">
                <WorkspaceProposalRow :tracking="tracking" :index="actionIndex" />
              </li>
            </ul>
            <p v-else class="py-4.5 text-[0.92rem] text-text-muted">
              {{
                activeTab.proposals.length === 0
                  ? t('organization.workspace.proposals.noneInEdition')
                  : t('organization.workspace.search.noneInTab', { edition: editionName(activeTab.edition) })
              }}
            </p>

            <!-- Hors de l'onglet : une étiquette par édition, à son nom, pour qu'aucun
                 dossier passé ne se lise comme un dossier en cours. -->
            <div v-for="group in elsewhere" :key="group.tab.edition.id" class="mt-8">
              <WorkspaceRule
                as="p"
                :label="[editionName(group.tab.edition), group.tab.edition.city].filter(Boolean).join(', ')"
                :meta="t('organization.workspace.search.otherEdition')"
              />
              <ul class="mt-2">
                <li v-for="tracking in group.proposals" :key="tracking.proposal.id" class="border-b border-border-subtle last:border-b-0">
                  <WorkspaceProposalRow :tracking="tracking" :index="actionIndex" />
                </li>
              </ul>
            </div>
          </section>
        </div>

        <aside v-if="hasAside && shown" class="relative z-1 flex min-w-0 flex-col gap-5 lg:-mt-18" :aria-label="t('organization.workspace.aside')">
          <WorkspaceOpenCallCard
            v-if="isEmpty || callInTab"
            :call="shown.open_call"
            :edition="shown.call_edition"
            :hide-when-closed="!isEmpty"
          />
          <WorkspaceNextActivity v-if="next" :activity="next" />
        </aside>

        <div v-if="shown" class="min-w-0 lg:col-span-2">

          <WorkspaceMembersPanel
            class="mt-4 lg:mt-6"
            :members="shown.members"
            :organization-name="shown.organization.legal_name"
            :viewer-id="auth.person?.id ?? ''"
            :is-manager="isManager"
            :pending-validation="shown.organization.status === 'candidate'"
            :busy="inviting"
            :message="memberMessage"
            @invite="invite"
            @decide="decide"
          />
        </div>
      </div>
    </div>
  </div>
</template>
