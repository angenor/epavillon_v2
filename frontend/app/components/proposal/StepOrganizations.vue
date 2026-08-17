<script setup lang="ts">
import type { Membership, Organization, SimilarOrganization } from '~/types/org'
import type { DraftIssue, DraftOrganization, ProposalDraft } from '~/types/proposal-form'
import type { OrganizationRole } from '~/types/programme/proposal'
import type { SelectOption } from '~/types/ui'

/**
 * ÉTAPE 1 — ORGANISATION PORTEUSE ET CO-ORGANISATEURS.
 *
 * DEUX CHOSES DIFFÉRENTES, ET LE MODÈLE LES SÉPARE. Le PORTEUR est
 * `proposals.organization_id` : celui qui soumet, répond aux demandes de
 * correction et est notifié de la décision. Un trigger le recopie en ligne de
 * rôle `lead` dans `proposal_organizations` — une seule vérité, deux points
 * d'accès —, et un index unique partiel garantit qu'il n'y en a jamais deux.
 * Les CO-ORGANISATEURS sont les autres lignes de cette table, avec leur rôle.
 *
 * LE PORTEUR N'EST PAS UN CHOIX LIBRE : ce sont les organisations dont la
 * personne est membre ACTIF. Une adhésion en attente ne donne aucun droit — la
 * garde de la page l'a déjà écarté. Quand il n'y en a qu'une, elle est retenue
 * d'office et affichée, pas cachée : le dossier engage cette organisation, elle
 * doit être lisible avant la première ligne de texte.
 *
 * LES CO-ORGANISATEURS PASSENT PAR LA RECHERCHE DE L'ÉCRAN A2, et c'est
 * essentiel : `find_similar_organizations()` interroge TOUTES les dénominations
 * — nom légal, sigle, traduction, ancien nom, faute de frappe connue. Une
 * seconde recherche écrite ici rapprocherait les fiches dans un écran et pas
 * dans l'autre, ce qui est exactement la façon dont naissent les doublons.
 *
 * ON NE CRÉE PAS D'ORGANISATION DEPUIS CE FORMULAIRE. Une fiche créée au
 * chausse-pied pendant une saisie de dossier est une fiche sans référent et sans
 * vérification. Qui ne trouve pas son co-organisateur passe par l'écran de
 * rattachement, qui est fait pour cela et qui montre les doublons avant de créer.
 */

const draft = defineModel<ProposalDraft>({ required: true })

interface Props {
  /** Adhésions ACTIVES de la personne — le porteur se choisit parmi elles. */
  memberships: { membership: Membership; organization: Organization }[]
  issues: DraftIssue[]
  /** Résolution des libellés venus de la base. */
  countryNameOf: (countryId: string | null) => string | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const api = useApi()

const errorOf = (field: string) =>
  props.issues.find((issue) => issue.field === field && issue.severity === 'error')

const leadError = computed(() => {
  const issue = errorOf('organization_id')
  return issue ? t(issue.messageKey) : undefined
})

// ---------------------------------------------------------------------------
// Le porteur principal
// ---------------------------------------------------------------------------

const leadOptions = computed<SelectOption[]>(() =>
  props.memberships.map(({ organization }) => ({
    value: organization.id,
    label: organization.acronym
      ? `${organization.legal_name} (${organization.acronym})`
      : organization.legal_name,
    description: props.countryNameOf(organization.country_id) ?? undefined,
  })),
)

const leadOrganization = computed(
  () => props.memberships.find((entry) => entry.organization.id === draft.value.organization_id)?.organization ?? null,
)

// ---------------------------------------------------------------------------
// Les co-organisateurs
// ---------------------------------------------------------------------------

/**
 * Les trois rôles ouverts ici. `lead` en est absent par construction : il est
 * porté par la colonne du dossier, pas par cette liste.
 */
const CO_ROLES: Exclude<OrganizationRole, 'lead'>[] = ['co_organizer', 'partner', 'sponsor']

const roleOptions = computed<SelectOption[]>(() =>
  CO_ROLES.map((role) => ({
    value: role,
    label: t(`proposal.form.step-organizations.roles.${role}`),
    description: t(`proposal.form.step-organizations.roleHints.${role}`),
  })),
)

const query = ref('')
const results = ref<SimilarOrganization[]>([])
const isSearching = ref(false)
const searchError = ref<Error | null>(null)
const hasSearched = ref(false)

/** Seule la dernière recherche écrit : même garde qu'à l'écran A2. */
let sequence = 0

async function runSearch(value: string): Promise<void> {
  const term = value.trim()
  query.value = term

  if (term.length < 2) {
    results.value = []
    hasSearched.value = false
    return
  }

  const current = ++sequence
  isSearching.value = true
  searchError.value = null
  try {
    const found = await api.organizations.similar({ name: term, limit: 8 })
    if (current !== sequence) return
    /**
     * On écarte ce qui ne ressemble pas à ce qui a été tapé : la fonction du
     * modèle fait aussi remonter les fiches partageant le domaine de l'adresse,
     * ce qui est juste pour le back-office et hors sujet ici. Même arbitrage
     * qu'à l'écran A2, écart n° 23.
     */
    results.value = found.filter((row) => row.match_reasons.includes('name_similarity'))
    hasSearched.value = true
  } catch (error) {
    if (current !== sequence) return
    searchError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    if (current === sequence) isSearching.value = false
  }
}

/** Déjà retenue — porteur compris : une organisation ne figure qu'une fois. */
function isTaken(organizationId: string): boolean {
  return (
    draft.value.organization_id === organizationId ||
    draft.value.co_organizations.some((entry) => entry.organization_id === organizationId)
  )
}

function addCoOrganization(match: SimilarOrganization): void {
  if (isTaken(match.organization_id)) return
  const entry: DraftOrganization = {
    organization_id: match.organization_id,
    role: 'co_organizer',
    legal_name: match.legal_name,
    acronym: match.acronym,
    country_id: match.country_id,
  }
  draft.value.co_organizations = [...draft.value.co_organizations, entry]
  query.value = ''
  results.value = []
  hasSearched.value = false
}

function removeCoOrganization(organizationId: string): void {
  draft.value.co_organizations = draft.value.co_organizations.filter(
    (entry) => entry.organization_id !== organizationId,
  )
}

function setRole(organizationId: string, role: string): void {
  draft.value.co_organizations = draft.value.co_organizations.map((entry) =>
    entry.organization_id === organizationId
      ? { ...entry, role: role as Exclude<OrganizationRole, 'lead'> }
      : entry,
  )
}
</script>

<template>
  <div class="grid gap-8">
    <!-- LE PORTEUR -->
    <section class="grid gap-4">
      <header>
        <h2 class="font-display text-xl text-text">
          {{ t('proposal.form.step-organizations.lead.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('proposal.form.step-organizations.lead.description') }}
        </p>
      </header>

      <!-- Une seule adhésion : on l'affiche, on ne la fait pas choisir. Un menu
           à une entrée demande un geste pour une décision déjà prise. -->
      <UiCard v-if="props.memberships.length === 1 && leadOrganization" sunken>
        <div class="flex items-start gap-3">
          <UiIcon name="building" size="1.4rem" class="mt-0.5 text-text-muted" />
          <div class="min-w-0">
            <p class="font-bold text-text">{{ leadOrganization.legal_name }}</p>
            <p class="text-sm text-text-muted">
              <span v-if="leadOrganization.acronym">{{ leadOrganization.acronym }}</span>
              <span v-if="leadOrganization.acronym && props.countryNameOf(leadOrganization.country_id)"> · </span>
              <span>{{ props.countryNameOf(leadOrganization.country_id) }}</span>
            </p>
            <p class="mt-2 text-sm text-text-secondary">
              {{ t('proposal.form.step-organizations.lead.single') }}
            </p>
          </div>
        </div>
      </UiCard>

      <UiRadio
        v-else
        v-model="draft.organization_id"
        :options="leadOptions"
        :label="t('proposal.form.step-organizations.lead.field')"
        :hint="t('proposal.form.step-organizations.lead.fieldHint')"
        :error="leadError"
        required
      />
    </section>

    <!-- LES CO-ORGANISATEURS -->
    <section class="grid gap-4 border-t border-border pt-8">
      <header>
        <h2 class="font-display text-xl text-text">
          {{ t('proposal.form.step-organizations.co.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('proposal.form.step-organizations.co.description') }}
        </p>
      </header>

      <!-- Ce qui est déjà retenu, en premier : c'est l'état du dossier. -->
      <ul v-if="draft.co_organizations.length > 0" class="grid gap-3">
        <li
          v-for="entry in draft.co_organizations"
          :key="entry.organization_id"
          class="rounded-md border border-border bg-surface-raised px-4 py-3"
        >
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div class="min-w-0">
              <p class="font-bold text-text">{{ entry.legal_name }}</p>
              <p class="text-sm text-text-muted">
                <span v-if="entry.acronym">{{ entry.acronym }}</span>
                <span v-if="entry.acronym && props.countryNameOf(entry.country_id)"> · </span>
                <span>{{ props.countryNameOf(entry.country_id) }}</span>
              </p>
            </div>

            <div class="flex w-full items-end gap-2 sm:w-auto">
              <UiSelect
                class="w-full sm:w-56"
                :model-value="entry.role"
                :options="roleOptions"
                :label="t('proposal.form.step-organizations.co.roleLabel')"
                size="sm"
                hide-optional
                @update:model-value="setRole(entry.organization_id, $event)"
              />
              <UiButton
                variant="ghost"
                icon="trash"
                icon-only
                :label="t('proposal.form.step-organizations.co.remove', { organization: entry.legal_name })"
                @click="removeCoOrganization(entry.organization_id)"
              />
            </div>
          </div>

          <!-- Une co-organisation ENGAGE UN TIERS : `confirmed_at` reste nul tant
               que l'organisation citée n'a pas confirmé, et le back-office
               l'affiche « en attente ». Le dire ici évite la promesse tacite. -->
          <p class="mt-2 flex items-start gap-1.5 text-sm text-text-muted">
            <UiIcon name="info" size="1rem" class="mt-0.5 shrink-0" />
            {{ t('proposal.form.step-organizations.co.pendingConfirmation') }}
          </p>
        </li>
      </ul>

      <UiSearchInput
        :model-value="query"
        :label="t('proposal.form.step-organizations.co.searchLabel')"
        :hint="t('proposal.form.step-organizations.co.searchHint')"
        :placeholder="t('proposal.form.step-organizations.co.searchPlaceholder')"
        :loading="isSearching"
        :result-count="hasSearched ? results.length : null"
        @search="runSearch"
      />

      <UiErrorState
        v-if="searchError"
        compact
        :title="t('common.states.error.title')"
        :description="t('proposal.form.step-organizations.co.searchError')"
        :retry-label="t('common.actions.retry')"
        @retry="runSearch(query)"
      />

      <ul v-else-if="results.length > 0" class="grid gap-2">
        <li
          v-for="match in results"
          :key="match.organization_id"
          class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border px-4 py-3"
        >
          <div class="min-w-0">
            <p class="font-bold text-text">
              {{ match.legal_name }}
              <UiIcon
                v-if="match.verified_at"
                name="shield-check"
                size="0.95rem"
                class="ml-1 inline text-success"
                :aria-label="t('proposal.form.step-organizations.co.verified')"
              />
            </p>
            <p class="text-sm text-text-muted">
              <span v-if="match.acronym">{{ match.acronym }}</span>
              <span v-if="match.acronym && props.countryNameOf(match.country_id)"> · </span>
              <span>{{ props.countryNameOf(match.country_id) }}</span>
              <span v-if="match.member_count > 0">
                · {{ t('proposal.form.step-organizations.co.members', { count: match.member_count }, match.member_count) }}
              </span>
            </p>
          </div>

          <UiButton
            variant="secondary"
            size="sm"
            icon="plus"
            :disabled="isTaken(match.organization_id)"
            :label="
              isTaken(match.organization_id)
                ? t('proposal.form.step-organizations.co.alreadyAdded')
                : t('common.actions.add')
            "
            @click="addCoOrganization(match)"
          />
        </li>
      </ul>

      <UiEmptyState
        v-else-if="hasSearched"
        compact
        filtered
        icon="building"
        :title="t('proposal.form.step-organizations.co.emptyTitle', { query })"
        :description="t('proposal.form.step-organizations.co.emptyDescription')"
      />
    </section>
  </div>
</template>
