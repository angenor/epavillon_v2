<script setup lang="ts">
import type {
  OrganizationDetail,
  OrganizationDomainRow,
  OrganizationNameRow,
  OrganizationWriteResult,
} from '~/types/admin-organizations'
import type { EffectivePermission } from '~/types/identity'
import type { TabItem } from '~/types/ui'

/**
 * FICHE D'UNE ORGANISATION — `/admin/organisations/:id`.
 *
 * L'ÉCRAN QUI REND VISIBLE CE QU'UNE ORGANISATION EST DANS LE MODÈLE : non pas
 * une ligne avec un nom, mais un faisceau de dénominations, de domaines de
 * messagerie, d'adhésions et d'activités portées. La v1 n'avait qu'un nom et un
 * sigle côte à côte — et c'est exactement pour cela qu'elle fabriquait des
 * doublons.
 *
 * ELLE S'OUVRE AUSSI POUR UNE FICHE ABSORBÉE. `merge_organizations()` conserve la
 * fiche source en statut `merged` « pour que les URL et identifiants externes
 * déjà diffusés continuent de résoudre » : rendre 404 casserait très exactement
 * ce que la fusion promet de préserver. L'écran la coiffe d'un renvoi vers la
 * fiche vivante, et c'est tout.
 *
 * CINQ ONGLETS, PORTÉS PAR L'URL — dénominations, domaines, membres, activités,
 * historique. Le premier n'est pas un choix d'ordre alphabétique : c'est le
 * panneau qui porte la règle métier n° 1, et celui qu'on vient consulter quand on
 * se demande pourquoi une recherche n'a rien trouvé.
 *
 * DEUX ÉCRITURES, ET ELLES NE DEMANDENT PAS LE MÊME DROIT. Poser le sceau,
 * vérifier un domaine, confirmer une dénomination relèvent d'`org.organization.manage` ;
 * lancer une fusion d'`org.organization.merge`, sur la portée globale. Une
 * personne peut tenir le référentiel à jour sans avoir le droit de fusionner.
 *
 * QUATRE ÉTATS : chargement, introuvable, erreur avec reprise, accès refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.organizations', to: '/admin/organisations' },
    { labelKey: 'nav.admin.organizations' },
  ],
})

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const { date } = useDateTime()

await adminScope.ensureLoaded()

const organizationId = computed(() => String(route.params.id ?? ''))
/** Une organisation n'a pas de fuseau : ses dates se lisent dans celui du lecteur. */
const timezone = computed(() => auth.person?.timezone ?? 'UTC')

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-organization-detail-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/**
 * PÉRIMÈTRE NON VIDE, en plus de la permission : `org.organization.read` est
 * accordée au rôle d'utilisateur ORDINAIRE, et l'API refuse alors sur le
 * périmètre. Sans ce second test, l'écran affichait une panne au lieu d'un accès
 * refusé.
 */
const hasScope = computed(
  () => adminScope.scope.is_global || adminScope.scope.event_ids.length > 0,
)
const canRead = computed(
  () => hasPermissionOnAnyScope(granted.value, 'org.organization.read') && hasScope.value,
)
/**
 * LES TROIS ÉCRITURES DE LA FICHE NE DEMANDENT PAS LA PORTÉE GLOBALE. L'API se
 * contente de la permission de gestion sur n'importe quelle portée : c'est le
 * PÉRIMÈTRE qui borne ce qu'on voit, pas la portée du droit. Une coordonnatrice
 * détachée sur la COP31 doit pouvoir poser le sceau et vérifier un domaine sur
 * les organisations qui y déposent.
 */
const canManage = computed(() => hasPermissionOnAnyScope(granted.value, 'org.organization.manage'))
/** La fusion, elle, déplace des rattachements partout : portée GLOBALE exigée. */
const canMerge = computed(() => hasPermission(granted.value, 'org.organization.merge'))

const {
  data: detail,
  status,
  error,
  refresh,
} = await useAsyncData<OrganizationDetail | null>(
  `admin-organization-${organizationId.value}`,
  async () => (canRead.value ? api.adminOrganizations.detail(organizationId.value) : null),
  { watch: [canRead, organizationId], lazy: true },
)

useHead(() => ({ title: detail.value?.legal_name ?? t('nav.admin.organizations') }))

/**
 * LE RENVOI DE LA FICHE ABSORBÉE.
 *
 * La date accompagne toujours le pointeur en base — `ck_organizations_merge_consistency`
 * l'exige — mais le contrat de l'API la rend facultative. À défaut, le renvoi
 * s'affiche quand même : perdre le lien vers la fiche vivante serait bien pire
 * que de ne pas savoir quel jour la fusion a eu lieu.
 */
const mergedNotice = computed(() => {
  const merged = detail.value?.merged_into
  if (!merged) return ''
  return t('admin.organization.detail.merged.notice', {
    name: merged.legal_name,
    date: merged.merged_at ? date(merged.merged_at, timezone.value) : t('common.labels.unknown'),
  })
})

// ---------------------------------------------------------------------------
// Onglets — portés par l'URL
// ---------------------------------------------------------------------------

const TAB_PARAM: Record<string, string> = {
  denominations: 'names',
  domaines: 'domains',
  membres: 'members',
  activites: 'activities',
  historique: 'history',
}
const PARAM_BY_TAB = Object.fromEntries(
  Object.entries(TAB_PARAM).map(([param, tab]) => [tab, param]),
) as Record<string, string>

const activeTab = computed(() => TAB_PARAM[String(route.query.onglet ?? '')] ?? 'names')

const tabs = computed<TabItem[]>(() => [
  {
    value: 'names',
    label: t('admin.organization.detail.tabs.names'),
    count: detail.value?.names.length,
  },
  {
    value: 'domains',
    label: t('admin.organization.detail.tabs.domains'),
    count: detail.value?.domains.length,
  },
  {
    value: 'members',
    label: t('admin.organization.detail.tabs.members'),
    count: detail.value?.members.length,
  },
  {
    value: 'activities',
    label: t('admin.organization.detail.tabs.activities'),
    count: detail.value?.activities.length,
  },
  { value: 'history', label: t('admin.organization.detail.tabs.history') },
])

function selectTab(value: string): void {
  const next = { ...route.query }
  if (value === 'names') delete next.onglet
  else next.onglet = PARAM_BY_TAB[value] ?? value
  router.replace({ query: next })
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

const busy = ref(false)
const notice = ref<string | null>(null)
const writeError = ref<string | null>(null)

/**
 * Toute écriture rend la fiche ENTIÈRE recomposée : vérifier un domaine change le
 * score de confiance, qui change le rang de la fiche dans la liste. Rendre le
 * seul objet modifié laisserait trois panneaux afficher des valeurs fausses.
 */
async function apply(action: () => Promise<OrganizationWriteResult>): Promise<void> {
  busy.value = true
  notice.value = null
  writeError.value = null

  try {
    const result = await action()

    if (result.status === 'domain_taken') {
      writeError.value = t('admin.organization.detail.domains.taken', {
        name: result.conflict_with.legal_name,
      })
      return
    }
    if (result.status === 'not_found') return

    detail.value = result.detail
    notice.value = t('admin.organization.detail.saved')
  } catch (thrown) {
    // Sans ce rattrapage, une écriture refusée ne laissait AUCUNE trace à
    // l'écran. Le message vient de l'API et s'affiche tel quel : elle seule sait
    // pourquoi elle refuse, et son catalogue est déjà en français.
    writeError.value =
      thrown instanceof ForbiddenError ? thrown.message : apiErrorMessage(thrown, (key) => t(key))
  } finally {
    busy.value = false
  }
}

function toggleVerification(): void {
  if (!detail.value) return
  const verified = detail.value.verified_at === null
  apply(() =>
    api.adminOrganizations.setVerification(
      { organization_id: detail.value!.organization_id, verified },
      auth.person?.id ?? null,
    ),
  )
}

function toggleDomain(domain: OrganizationDomainRow, verified: boolean): void {
  apply(() =>
    api.adminOrganizations.setDomainVerification(
      {
        organization_id: organizationId.value,
        domain_id: domain.id,
        verified,
        // Retirer la vérification retire le rattachement automatique :
        // `ck_domain_autojoin_requires_verification` refuserait l'inverse.
        auto_join: verified && domain.auto_join,
      },
      auth.person?.id ?? null,
    ),
  )
}

function toggleAutoJoin(domain: OrganizationDomainRow, autoJoin: boolean): void {
  apply(() =>
    api.adminOrganizations.setDomainVerification(
      {
        organization_id: organizationId.value,
        domain_id: domain.id,
        verified: domain.verified_at !== null,
        auto_join: autoJoin,
      },
      auth.person?.id ?? null,
    ),
  )
}

function toggleName(name: OrganizationNameRow): void {
  apply(() =>
    api.adminOrganizations.setNameConfirmation(
      {
        organization_id: organizationId.value,
        name_id: name.id,
        is_confirmed: !name.is_confirmed,
      },
      auth.person?.id ?? null,
    ),
  )
}

/** Fusionner depuis la fiche : la seconde organisation reste à choisir. */
const mergeLink = computed(() =>
  localePath({
    path: '/admin/organisations/fusion',
    query: { gauche: organizationId.value },
  }),
)

function pairLink(pairId: string, otherId: string): string {
  return localePath({
    path: '/admin/organisations/fusion',
    query: { gauche: organizationId.value, droite: otherId, paire: pairId },
  })
}
</script>

<template>
  <div class="mx-auto w-full max-w-6xl">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !canRead"
      :required-scope="t('admin.organization.detail.forbidden.scope')"
      action-to="/admin/organisations"
      :action-label="t('admin.organization.detail.back')"
    />

    <template v-else>
      <UiButton variant="link" icon="arrow-left" :to="localePath('/admin/organisations')">
        {{ t('admin.organization.detail.back') }}
      </UiButton>

      <UiErrorState
        v-if="error"
        class="mt-6"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiLoadingState v-else-if="status === 'pending' && !detail" class="mt-6" />

      <UiEmptyState
        v-else-if="!detail"
        class="mt-6"
        icon="search"
        :title="t('admin.organization.detail.notFound.title')"
        :description="t('admin.organization.detail.notFound.description')"
        :action-label="t('admin.organization.detail.notFound.action')"
        :action-to="localePath('/admin/organisations')"
      />

      <template v-else>
        <!-- FICHE ABSORBÉE : le renvoi, et la promesse tenue. Ni 404, ni page
             vide — c'est ce que garantit `org.resolve_organization()`. -->
        <UiAlert
          v-if="detail.merged_into"
          class="mt-4"
          intent="info"
          :message="mergedNotice"
        >
          <template #actions>
            <UiButton
              size="sm"
              variant="secondary"
              :to="localePath(`/admin/organisations/${detail.merged_into.organization_id}`)"
            >
              {{ t('admin.organization.detail.merged.action') }}
            </UiButton>
          </template>
        </UiAlert>

        <header class="mt-4 flex flex-wrap items-start justify-between gap-x-6 gap-y-4">
          <div class="min-w-0">
            <h1 class="flex flex-wrap items-center gap-3 text-3xl leading-tight font-semibold text-balance">
              {{ detail.legal_name }}
              <UiBadge
                v-if="detail.verified_at"
                intent="success"
                icon="shield-check"
                :label="t('admin.organization.detail.header.verified')"
              />
              <UiBadge
                v-else
                intent="neutral"
                :label="t('admin.organization.detail.header.notVerified')"
              />
            </h1>

            <p class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-text-muted">
              <span v-if="detail.acronym" class="font-mono">{{ detail.acronym }}</span>
              <span v-if="detail.organization_type_label">
                {{ tr(detail.organization_type_label) }}
              </span>
              <span v-if="detail.country_name">{{ tr(detail.country_name) }}</span>
              <span v-if="detail.city">{{ detail.city }}</span>
              <a
                v-if="detail.website"
                :href="detail.website"
                target="_blank"
                rel="noopener noreferrer"
                class="text-accent"
              >{{ detail.website }}</a>
            </p>

            <p class="mt-1 text-xs text-text-subtle">
              {{
                detail.created_by_name
                  ? t('admin.organization.detail.header.createdBy', {
                      name: detail.created_by_name,
                      date: date(detail.created_at, timezone),
                    })
                  : t('admin.organization.detail.header.createdOn', {
                      date: date(detail.created_at, timezone),
                    })
              }}
              · {{ t('admin.organization.detail.header.trust', { score: detail.trust_score }) }}
            </p>
          </div>

          <div class="flex flex-wrap gap-2">
            <UiButton
              v-if="canManage"
              :variant="detail.verified_at ? 'ghost' : 'primary'"
              :loading="busy"
              @click="toggleVerification"
            >
              {{
                t(
                  detail.verified_at
                    ? 'admin.organization.detail.actions.unverify'
                    : 'admin.organization.detail.actions.verify',
                )
              }}
            </UiButton>
            <UiButton v-if="canMerge && !detail.merged_into" variant="secondary" :to="mergeLink">
              {{ t('admin.organization.detail.actions.merge') }}
            </UiButton>
          </div>
        </header>

        <UiAlert
          v-if="notice"
          class="mt-4"
          intent="success"
          live
          dismissible
          :message="notice"
          @close="notice = null"
        />
        <UiAlert v-if="writeError" class="mt-4" intent="danger" live :message="writeError" />

        <!-- LES DOUBLONS PRÉSUMÉS, en tête de fiche : c'est l'information qui
             change ce qu'on va faire de la page. -->
        <section
          v-if="detail.duplicates.length > 0 && canMerge"
          class="mt-6 rounded-lg border border-warning-border bg-warning-surface p-4"
        >
          <h2 class="text-sm font-semibold text-warning">
            {{ t('admin.organization.detail.duplicates.title') }}
          </h2>
          <p class="mt-1 text-sm text-text">
            {{ t('admin.organization.detail.duplicates.description', detail.duplicates.length) }}
          </p>
          <ul class="mt-2 flex flex-col gap-2">
            <li
              v-for="pair in detail.duplicates"
              :key="pair.id"
              class="flex flex-wrap items-center justify-between gap-3"
            >
              <span class="text-sm text-text">
                {{
                  t('admin.organization.detail.duplicates.with', {
                    name:
                      pair.left.organization_id === detail.organization_id
                        ? pair.right.legal_name
                        : pair.left.legal_name,
                  })
                }}
              </span>
              <UiButton
                size="sm"
                variant="secondary"
                :to="
                  pairLink(
                    pair.id,
                    pair.left.organization_id === detail.organization_id
                      ? pair.right.organization_id
                      : pair.left.organization_id,
                  )
                "
              >
                {{ t('admin.organization.detail.duplicates.action') }}
              </UiButton>
            </li>
          </ul>
        </section>

        <!-- LES CHIFFRES DE LA FICHE — la même fiche de performance que la liste,
             pour n'avoir jamais deux comptes différents du même fait. -->
        <dl class="mt-6 grid gap-3 sm:grid-cols-3 lg:grid-cols-6">
          <div
            v-for="figure in [
              { key: 'members', value: String(detail.scorecard.membres_actifs) },
              { key: 'proposals', value: String(detail.scorecard.propositions_deposees) },
              { key: 'accepted', value: String(detail.scorecard.propositions_acceptees) },
              {
                key: 'ratio',
                value:
                  detail.scorecard.ratio_acceptation === null
                    ? t('admin.organization.detail.figures.noRatio')
                    : t('common.formats.percent', {
                        value: Math.round(detail.scorecard.ratio_acceptation * 100),
                      }),
              },
              { key: 'sessions', value: String(detail.scorecard.sessions_programmees) },
              { key: 'events', value: String(detail.scorecard.evenements_couverts) },
            ]"
            :key="figure.key"
            class="rounded-lg border border-border bg-surface-raised p-3"
          >
            <dt class="text-xs text-text-subtle">
              {{ t('admin.organization.detail.figures.' + figure.key) }}
            </dt>
            <dd class="mt-1 font-mono text-xl tabular-nums text-text">{{ figure.value }}</dd>
          </div>
        </dl>

        <UiTabs
          class="mt-8"
          :items="tabs"
          :model-value="activeTab"
          :label="t('admin.organization.detail.tabs.names')"
          @update:model-value="selectTab"
        />

        <div class="mt-6">
          <AdminOrganizationsDetailNames
            v-if="activeTab === 'names'"
            :names="detail.names"
            :timezone="timezone"
            :can-manage="canManage"
            :busy="busy"
            @toggle="toggleName"
          />

          <AdminOrganizationsDetailDomains
            v-else-if="activeTab === 'domains'"
            :domains="detail.domains"
            :timezone="timezone"
            :can-manage="canManage"
            :busy="busy"
            @verify="toggleDomain"
            @auto-join="toggleAutoJoin"
          />

          <AdminOrganizationsDetailMembers
            v-else-if="activeTab === 'members'"
            :members="detail.members"
            :timezone="timezone"
          />

          <AdminOrganizationsDetailActivities
            v-else-if="activeTab === 'activities'"
            :activities="detail.activities"
            :timezone="timezone"
          />

          <AdminOrganizationsDetailHistory
            v-else
            :history="detail.history"
            :merges="detail.merges"
            :timezone="timezone"
          />
        </div>
      </template>
    </template>
  </div>
</template>
