<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
import type { InviteMemberPayload, ProposalTracking, WorkspaceOverview } from '~/types/organization-workspace'
import type { Intent, SelectOption, TabItem } from '~/types/ui'
import type { WorkspaceEditionTab, WorkspaceProposalFilter } from '~/utils/workspace-editions'
import type { Uuid } from '~/types/shared'

/**
 * ESPACE ORGANISATION — `/mon-organisation`, `/en/my-organization`.
 *
 * IL RÉPOND À DEUX QUESTIONS, ET DANS CET ORDRE : « qu'est-ce qui attend une
 * action de ma part ? », puis « où en est chacun de mes dossiers ? ». L'ordre
 * n'est pas un détail de mise en page — une organisation qui ouvre cette page
 * vient d'abord savoir si elle a quelque chose à faire. Le reste est de la
 * consultation, et la consultation peut attendre le second écran.
 *
 * QUATRE DÉCISIONS, ET CHACUNE SE PAIE SI ON L'INVERSE :
 *
 *  1. LE BLOC D'ACTIONS EST EN TÊTE, ET IL RESTE LISIBLE VIDE. « Rien ne vous
 *     attend » est une réponse ; une zone absente laisse croire à un écran
 *     incomplet, et une zone rouge vide à une panne.
 *  2. « CORRECTIONS DEMANDÉES » SE VOIT DE LOIN, avec le NOMBRE de points à
 *     traiter. C'est le seul état qui coûte un dossier s'il passe inaperçu : le
 *     comité attend une réponse, et l'échéance de l'appel court toujours.
 *  3. UNE ÉDITION À LA FOIS (28/10), un onglet chacune, l'édition vivante
 *     ouverte d'office. Chaque onglet porte SES actions : les comptes rendus de
 *     la CdP30 reprise de la v1 ne masquent plus le travail de la COP31. La
 *     recherche, elle, parcourt toutes les éditions et range ce qu'elle trouve
 *     ailleurs sous le nom de l'édition.
 *  4. L'ÉTAT VIDE MET L'APPEL EN AVANT. Qui n'a rien déposé n'a pas besoin
 *     qu'on lui décrive ce qu'il ne voit pas : il a besoin de savoir où déposer
 *     et jusqu'à quand.
 *
 * LA PAGE EST GARDÉE PAR `requires-organization`, comme le formulaire de dépôt :
 * suivre des dossiers suppose d'agir au nom d'une organisation, et une adhésion
 * EN ATTENTE ne suffit pas — aucun référent n'a encore accepté. Ce test n'est pas
 * réécrit ici.
 *
 * QUATRE ÉTATS, comme partout : chargement, erreur (avec reprise), vide (aucun
 * dossier), accès refusé (adhésion perdue en cours de route).
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
const localePath = useLocalePath()
const api = useApi()
const auth = useAuthStore()
const memberships = useMembershipStore()

useHead(() => ({ title: t('organization.workspace.title') }))

// ---------------------------------------------------------------------------
// L'organisation consultée
//
// Une personne peut appartenir à plusieurs structures. L'organisation courante
// vit dans l'URL (`?organisation=…`) pour qu'un lien envoyé par courriel ouvre
// la bonne : sans cela, deux référents d'organisations différentes se
// renverraient des adresses qui n'affichent pas la même chose.
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

function selectOrganization(id: string | null): void {
  if (!id) return
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

// ---------------------------------------------------------------------------
// Une édition à la fois (28/10), dans l'URL (`?edition=slug`) pour qu'un lien
// envoyé ouvre le même onglet.
// ---------------------------------------------------------------------------

const tabs = computed(() => (overview.value ? workspaceEditionTabs(overview.value) : []))

const activeTab = computed(() => {
  const requested = route.query.edition
  return defaultEditionTab(tabs.value, Array.isArray(requested) ? requested[0] : requested)
})

const tabItems = computed<TabItem[]>(() =>
  tabs.value.map((tab) => ({ value: tab.edition.slug, label: editionName(tab.edition) })),
)

function selectTab(slug: string): void {
  router.replace({ query: { ...route.query, edition: slug }, hash: route.hash })
}

function editionName(edition: EventEdition): string {
  return edition.acronym ?? tr(edition.title)
}

function editionState(tab: WorkspaceEditionTab): 'over' | 'ongoing' | 'upcoming' {
  if (tab.over) return 'over'
  return Date.parse(tab.edition.starts_at) <= Date.now() ? 'ongoing' : 'upcoming'
}

function editionMeta(tab: WorkspaceEditionTab): string {
  const parts = [
    t(`organization.workspace.editions.state.${editionState(tab)}`),
    t('organization.workspace.proposals.count', tab.proposals.length),
  ]
  if (tab.over && tab.missingReports > 0) {
    parts.push(t('organization.workspace.editions.missingReports', tab.missingReports).toLowerCase())
  } else if (!tab.over && tab.actions.length > 0) {
    parts.push(t('organization.workspace.editions.todo', tab.actions.length))
  }
  return parts.join(' · ')
}

// ---------------------------------------------------------------------------
// Recherche et filtre : la recherche parcourt TOUTES les éditions, et ce
// qu'elle trouve hors de l'onglet se range sous le nom de son édition.
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

const shown = computed(() => activeTab.value?.proposals.filter(keep) ?? [])

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

const logo = computed(() => {
  const asset = overview.value?.organization_logo
  return asset ? (asset.sources.thumb?.url ?? asset.url) : null
})


const isManager = computed(() => overview.value?.membership.role === 'manager')

// ---------------------------------------------------------------------------
// Invitation d'un membre
// ---------------------------------------------------------------------------

const inviting = ref(false)
const memberMessage = ref<{ intent: Intent; text: string } | null>(null)

/**
 * Le texte d'un refus. L'API refuse en nommant sa raison — « seul un référent
 * peut inviter », « cette adhésion est une invitation » — et son catalogue est
 * déjà français : on l'affiche tel quel. Le site ne parle que si elle s'est tue.
 */
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
      // Seule l'invitation réellement partie est une réussite. « Déjà membre »
      // et « déjà invitée » ne sont pas des erreurs — ce sont des réponses, et
      // les peindre en rouge ferait croire à un échec de l'envoi.
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
  <div class="mx-auto w-full max-w-6xl px-4 py-8 sm:px-6 lg:px-8">
    <header class="flex flex-col gap-3">
      <div class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="flex min-w-0 items-center gap-4 sm:gap-5">
          <span
            v-if="overview"
            class="grid size-16 shrink-0 place-items-center overflow-hidden rounded-lg border border-border bg-surface-raised p-2 sm:size-20"
          >
            <img v-if="logo" :src="logo" alt="" class="max-h-full max-w-full object-contain">
            <span v-else class="text-lg font-bold text-heading" aria-hidden="true">
              {{ overview.organization.acronym ?? initialsOf(overview.organization.legal_name) }}
            </span>
          </span>
          <div class="min-w-0">
            <p class="text-sm text-text-subtle">{{ t('organization.workspace.title') }}</p>
            <h1 class="mt-1 text-3xl leading-tight font-semibold text-balance">
              {{ overview?.organization.legal_name ?? t('organization.workspace.title') }}
            </h1>
          </div>
        </div>

        <!-- Le sélecteur n'apparaît qu'à qui a plusieurs organisations : une
             liste déroulante à une entrée n'est pas un choix, c'est un obstacle. -->
        <UiSelect
          v-if="organizationOptions.length > 1"
          id="workspace-organization"
          class="w-full sm:w-80"
          :model-value="currentOrganizationId"
          :options="organizationOptions"
          :label="t('organization.workspace.organizationPicker.label')"
          :hint="t('organization.workspace.organizationPicker.hint')"
          hide-optional
          @update:model-value="selectOrganization"
        />
      </div>
    </header>

    <UiLoadingState v-if="status === 'pending'" class="mt-10" variant="card" :lines="4" />

    <UiErrorState
      v-else-if="error"
      class="mt-10"
      :retry-label="t('common.actions.retry')"
      @retry="refresh()"
    />

    <!-- L'adhésion a disparu entre la garde et la réponse : c'est le quatrième
         état, et il se distingue d'une page vide. -->
    <UiForbiddenState
      v-else-if="!overview"
      class="mt-10"
      :action-to="localePath('/rattachement-organisation')"
      :action-label="t('organization.workspace.membershipPending.action')"
    />

    <template v-else>
      <UiAlert
        v-if="overview.membership.status === 'pending'"
        class="mt-10"
        intent="info"
        :title="t('organization.workspace.membershipPending.title')"
        :message="t('organization.workspace.membershipPending.description', { organization: overview.organization.legal_name })"
      />

      <!-- Aucun dossier ni appel : l'état vide met l'appel en avant, et rien d'autre. -->
      <div v-if="tabs.length === 0" class="mt-10 grid gap-6 lg:grid-cols-[2fr_1fr]">
        <UiEmptyState
          icon="document"
          :title="t('organization.workspace.proposals.empty.title')"
          :description="t('organization.workspace.proposals.empty.description')"
        />
        <WorkspaceOpenCallCard :call="overview.open_call" :edition="overview.call_edition" />
      </div>

      <UiTabs
        v-else-if="activeTab"
        class="mt-8"
        :items="tabItems"
        :model-value="activeTab.edition.slug"
        :label="t('organization.workspace.editions.label')"
        @update:model-value="selectTab"
      >
        <template #tab="{ item }">
          <span v-for="tab in tabs.filter((entry) => entry.edition.slug === item.value)" :key="tab.edition.id" class="flex items-center gap-3 py-2">
            <!-- Décoratives : leur texte alternatif précéderait le nom de l'édition dans l'onglet. -->
            <span
              v-if="tab.logo"
              aria-hidden="true"
              class="grid size-11 shrink-0 place-items-center overflow-hidden rounded-md border border-border bg-surface-raised p-1"
            >
              <img :src="tab.logo.sources.thumb?.url ?? tab.logo.url" alt="" class="max-h-full max-w-full object-contain">
            </span>
            <UiImage
              v-else-if="tab.thumbnail"
              :image="tab.thumbnail"
              ratio="1 / 1"
              sizes="44px"
              rounded="rounded-md"
              class="w-11 shrink-0"
              aria-hidden="true"
            />
            <span
              v-else
              aria-hidden="true"
              class="grid size-11 shrink-0 place-items-center rounded-md border border-border bg-surface-sunken text-xs font-bold text-text-muted"
            >
              {{ initialsOf(editionName(tab.edition)) }}
            </span>
            <span class="flex flex-col items-start text-left">
              <span>{{ editionName(tab.edition) }}</span>
              <span class="text-xs font-normal text-text-muted">{{ editionMeta(tab) }}</span>
            </span>
          </span>
        </template>

        <div class="flex flex-col gap-8">
          <WorkspaceOpenCallCard
            v-if="overview.call_edition?.id === activeTab.edition.id"
            variant="bar"
            :call="overview.open_call"
            :edition="overview.call_edition"
          />

          <!-- Édition terminée : un nombre et un geste, pas la liste des comptes rendus. -->
          <button
            v-if="activeTab.over && activeTab.missingReports > 0"
            type="button"
            class="flex min-h-(--target-min) cursor-pointer flex-col gap-1 rounded-lg border border-border bg-surface-raised px-4 py-3 text-left transition-colors duration-(--duration-fast) hover:border-border-strong sm:flex-row sm:items-center sm:justify-between"
            @click="showMissingReports()"
          >
            <span class="font-semibold text-text">
              {{ t('organization.workspace.editions.missingReports', activeTab.missingReports) }}
            </span>
            <span class="text-sm font-semibold text-accent">{{ t('organization.workspace.editions.showMissingReports') }}</span>
          </button>

          <WorkspaceActionList
            v-if="!activeTab.over"
            :actions="activeTab.actions"
            :timezone="activeTab.edition.timezone"
          />

          <section id="mes-dossiers" class="scroll-mt-24" aria-labelledby="workspace-proposals-title">
            <div class="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
              <h2 id="workspace-proposals-title" class="text-xl font-semibold">
                {{ t('organization.workspace.proposals.title') }}
                <span class="font-normal text-text-subtle">· {{ activeTab.proposals.length }}</span>
              </h2>
              <div class="flex flex-col gap-3 sm:flex-row sm:items-end">
                <UiSearchInput
                  id="workspace-search"
                  v-model="query"
                  class="sm:w-80"
                  :label="t('organization.workspace.search.label')"
                  :placeholder="t('organization.workspace.search.placeholder')"
                  :min-length="1"
                  :debounce="0"
                />
                <UiSelect
                  id="workspace-filter"
                  class="sm:w-56"
                  :model-value="filter"
                  :options="filterOptions"
                  :label="t('organization.workspace.filters.label')"
                  hide-optional
                  @update:model-value="filter = ($event as WorkspaceProposalFilter | null) ?? 'all'"
                />
              </div>
            </div>

            <div class="mt-4 overflow-hidden rounded-lg border border-border bg-surface-raised">
              <p
                v-if="searching"
                class="bg-surface-sunken px-4 py-2 text-sm font-bold text-heading"
              >
                {{ t('organization.workspace.search.inTab', { edition: editionName(activeTab.edition) }) }}
              </p>
              <ul v-if="shown.length > 0" class="divide-y divide-border-subtle">
                <li v-for="tracking in shown" :key="tracking.proposal.id">
                  <WorkspaceProposalRow :tracking="tracking" :index="actionIndex" />
                </li>
              </ul>
              <p v-else class="px-4 py-4 text-sm text-text-muted">
                {{
                  activeTab.proposals.length === 0
                    ? t('organization.workspace.proposals.noneInEdition')
                    : t('organization.workspace.search.noneInTab', { edition: editionName(activeTab.edition) })
                }}
              </p>

              <!-- Hors de l'onglet : un bandeau par édition, à son nom, pour qu'aucun
                   dossier passé ne se lise comme un dossier en cours. -->
              <template v-for="group in elsewhere" :key="group.tab.edition.id">
                <p class="flex flex-wrap items-baseline justify-between gap-x-3 bg-surface-inverse px-4 py-2 text-sm font-bold text-text-on-inverse">
                  <span>{{ [editionName(group.tab.edition), group.tab.edition.city].filter(Boolean).join(' · ') }}</span>
                  <span class="font-normal text-text-on-inverse-muted">{{ t('organization.workspace.search.otherEdition') }}</span>
                </p>
                <ul class="divide-y divide-border-subtle">
                  <li v-for="tracking in group.proposals" :key="tracking.proposal.id">
                    <WorkspaceProposalRow :tracking="tracking" :index="actionIndex" />
                  </li>
                </ul>
              </template>
            </div>
          </section>
        </div>
      </UiTabs>

      <UiAlert
        v-if="memberMessage"
        class="mt-12"
        :intent="memberMessage.intent"
        :message="memberMessage.text"
        live
      />

      <WorkspaceMembersPanel
        class="mt-6"
        :members="overview.members"
        :organization-name="overview.organization.legal_name"
        :viewer-id="auth.person?.id ?? ''"
        :is-manager="isManager"
        :busy="inviting"
        @invite="invite"
        @decide="decide"
      />
    </template>
  </div>
</template>
