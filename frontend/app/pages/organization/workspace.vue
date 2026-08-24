<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
import type { InviteMemberPayload, ProposalTracking, WorkspaceOverview } from '~/types/organization-workspace'
import type { Intent, SelectOption } from '~/types/ui'
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
 *  3. LES DOSSIERS SONT GROUPÉS PAR ÉDITION. Une organisation fidèle en a
 *     déposé plusieurs années de suite, et lire un dossier de la COP30 comme
 *     s'il était en cours est le contresens le plus facile à produire.
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

const activeOrganizations = computed(() => memberships.active)

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

/**
 * Les dossiers, groupés par édition et rangés de la plus récente à la plus
 * ancienne. Le groupement se fait ICI et non dans la réponse : c'est un choix
 * d'affichage, et la réponse doit rester une liste que d'autres écrans peuvent
 * trier autrement.
 */
interface EditionGroup {
  edition: EventEdition
  proposals: ProposalTracking[]
}

const groups = computed<EditionGroup[]>(() => {
  const byEdition = new Map<string, EditionGroup>()
  for (const tracking of overview.value?.proposals ?? []) {
    const existing = byEdition.get(tracking.edition.id)
    if (existing) {
      existing.proposals.push(tracking)
      continue
    }
    byEdition.set(tracking.edition.id, { edition: tracking.edition, proposals: [tracking] })
  }
  return [...byEdition.values()].sort((a, b) => b.edition.starts_at.localeCompare(a.edition.starts_at))
})

/** Fuseau des échéances affichées : celui de l'édition qui reçoit les dossiers. */
const deadlineTimezone = computed(() => overview.value?.call_edition?.timezone ?? 'UTC')

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
      <p class="text-sm text-text-subtle">{{ t('organization.workspace.title') }}</p>
      <div class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ overview?.organization.legal_name ?? t('organization.workspace.title') }}
          </h1>
          <p class="mt-1 text-text-muted">{{ t('organization.workspace.subtitle') }}</p>
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
      <WorkspaceActionList class="mt-10" :actions="overview.actions" :timezone="deadlineTimezone" />

      <section class="mt-12" aria-labelledby="workspace-proposals-title">
        <div class="mb-4 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
          <h2 id="workspace-proposals-title" class="text-xl font-semibold">
            {{ t('organization.workspace.proposals.title') }}
          </h2>
          <p class="text-sm text-text-subtle">
            {{ t('organization.workspace.proposals.count', overview.proposals.length) }}
          </p>
        </div>

        <!-- L'état vide met l'appel en avant, et rien d'autre. -->
        <div v-if="overview.proposals.length === 0" class="grid gap-6 lg:grid-cols-[2fr_1fr]">
          <UiEmptyState
            icon="document"
            :title="t('organization.workspace.proposals.empty.title')"
            :description="t('organization.workspace.proposals.empty.description')"
          />
          <WorkspaceOpenCallCard :call="overview.open_call" :edition="overview.call_edition" />
        </div>

        <div v-else class="grid gap-8 lg:grid-cols-[2fr_1fr] lg:items-start">
          <div class="flex flex-col gap-8">
            <div v-for="(group, index) in groups" :key="group.edition.id" class="flex flex-col gap-4">
              <!-- Le titre d'édition n'apparaît qu'à partir de deux groupes :
                   une seule campagne n'a pas besoin qu'on la nomme deux fois. -->
              <!-- Le SIGLE plutôt que le titre complet : « COP31 » tient sur
                   une ligne là où « COP31 — Conférence des Nations unies sur les
                   changements climatiques » en prend deux, en capitales, pour ne
                   rien dire de plus à cet endroit. -->
              <h3 v-if="groups.length > 1" class="text-sm font-bold tracking-wide text-text-subtle uppercase">
                {{ group.edition.acronym ?? tr(group.edition.title) }}
                <span class="font-normal normal-case">
                  ·
                  {{
                    index === 0
                      ? t('organization.workspace.editions.current')
                      : t('organization.workspace.editions.past')
                  }}
                </span>
              </h3>

              <WorkspaceProposalCard
                v-for="tracking in group.proposals"
                :key="tracking.proposal.id"
                :tracking="tracking"
              />
            </div>
          </div>

          <WorkspaceOpenCallCard
            class="lg:sticky lg:top-24"
            :call="overview.open_call"
            :edition="overview.call_edition"
          />
        </div>
      </section>

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
