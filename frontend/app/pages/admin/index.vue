<script setup lang="ts">
import type { AdminDashboard } from '~/types/admin-dashboard'

/**
 * TABLEAU DE BORD DU BACK-OFFICE — `/admin`.
 *
 * LA PAGE D'ACCUEIL DE L'ÉQUIPE DE L'IFDD dans les semaines qui précèdent une
 * COP. Elle répond à une question et une seule quand on l'ouvre : « qu'est-ce
 * qui attend l'équipe aujourd'hui ? » Le reste — les chiffres, la santé du
 * système — se consulte, et la consultation peut attendre le second regard.
 *
 * TROIS ZONES, ET LEUR ORDRE EST LE SUJET :
 *
 *  1. CE QUI DEMANDE UNE ACTION, en tête, le plus visible. Il RESTE LISIBLE
 *     VIDE : un back-office où tout va bien ne doit pas ressembler à un écran
 *     cassé. C'est la contrainte explicite du prompt, et c'est celle qu'on
 *     trahit le plus facilement — une zone rouge vide se lit comme une panne,
 *     une zone absente comme un écran incomplet.
 *  2. LES CHIFFRES, en deux temps. D'abord SIX INDICATEURS DE TÊTE — où en
 *     est-on : dépôts, jours avant la clôture, avancement du comité, sélectivité,
 *     séances créées, inscriptions. Puis les graphiques — comment cela évolue :
 *     entonnoir, courbe des dépôts avec l'échéance marquée, courbe des
 *     inscriptions, répartitions par pays et par thématique. Un graphique ne
 *     répond pas à « où en est-on », et c'est pourtant la question qu'on se pose
 *     en ouvrant l'écran.
 *  3. LA SANTÉ OPÉRATIONNELLE, en bas : ce qui casse en silence.
 *
 * LES CHIFFRES DE LA ZONE 2 SONT MATÉRIALISÉS, PAS INSTANTANÉS. Ils viennent des
 * `analytics.mv_*`, rafraîchies par le worker. L'écran affiche donc l'âge du
 * dernier rafraîchissement, sans détour : un chiffre matérialisé présenté comme
 * instantané est un chiffre faux, et « le tableau de bord affiche les chiffres
 * d'hier » doit rester vérifiable. La zone 1 et la zone 3, elles, sont lues en
 * temps réel — d'où leur place aux deux extrémités.
 *
 * RÈGLE MÉTIER N° 8 — LE PÉRIMÈTRE D'ADMINISTRATION. Un administrateur peut
 * n'avoir accès qu'à une seule édition. Le sélecteur, posé en tête de page par
 * le layout, n'affiche alors que la sienne, sans liste, sans mention, sans
 * compteur : rien dans cette page ne laisse deviner l'existence des autres. Le
 * filtrage définitif appartient à l'API ; ce que fait l'écran, c'est REFUSER
 * plutôt qu'afficher un tableau de bord vide, qui se lirait « il ne se passe
 * rien » au lieu de « ceci ne vous regarde pas ».
 *
 * QUATRE ÉTATS, comme partout : chargement, erreur avec reprise, vide (aucune
 * édition sélectionnée), accès refusé (aucun droit d'administration).
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
})

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()
const api = useApi()
const adminScope = useAdminScopeStore()

useHead(() => ({ title: t('admin.dashboard.title') }))

await adminScope.ensureLoaded()

const {
  data: dashboard,
  status,
  error,
  refresh,
} = await useAsyncData<AdminDashboard | null>(
  'admin-dashboard',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return null
    return api.admin.dashboard(eventId, adminScope.scope)
  },
  { watch: [() => adminScope.currentEventId], lazy: true },
)

const figures = computed(() => dashboard.value?.figures ?? null)
const timezone = computed(() => dashboard.value?.timezone ?? 'UTC')

/**
 * Les repères de la courbe des dépôts : ouverture de l'appel et échéance.
 * L'échéance est celle qui FAIT FOI — `event.effective_deadline()`, donc la
 * prolongation quand il y en a une, et non la clôture initiale que plus personne
 * ne regarde.
 */
const submissionMarkers = computed(() => {
  const rows = []
  if (figures.value?.call_opens_at) {
    rows.push({ at: figures.value.call_opens_at, label: t('admin.dashboard.charts.callOpens') })
  }
  if (figures.value?.deadline) {
    rows.push({
      at: figures.value.deadline,
      label: t('admin.dashboard.charts.deadline'),
      kind: 'deadline' as const,
    })
  }
  return rows
})

function total(points: { cumul: number }[]): number {
  return points.at(-1)?.cumul ?? 0
}
</script>

<template>
  <div class="mx-auto w-full max-w-6xl">
    <!-- ACCÈS REFUSÉ — aucun droit d'administration. Distinct d'un écran vide :
         l'un dit « vous n'avez pas ce droit », l'autre « il n'y a rien à voir »,
         et les confondre envoie chercher une panne là où il faut demander un
         accès. -->
    <UiForbiddenState
      v-if="!adminScope.isLoading && !adminScope.canAdminister"
      :required-scope="t('admin.dashboard.forbidden.scope')"
      :action-to="'/'"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-2">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.dashboard.title') }}
          </h1>
          <p class="mt-1 text-text-muted">{{ t('admin.dashboard.subtitle') }}</p>
        </div>

        <!-- L'ÂGE DES CHIFFRES, dit sans détour. Les projections analytiques
             sont rafraîchies par le worker : les donner pour instantanées serait
             mentir sur ce qu'on regarde. -->
        <p v-if="figures?.refreshed_at" class="text-sm text-text-subtle">
          {{ t('admin.dashboard.refreshedAt', { date: dateTime(figures.refreshed_at, timezone) }) }}
        </p>
      </header>

      <UiLoadingState
        v-if="status === 'pending'"
        class="mt-8"
        variant="card"
        :lines="3"
        :label="t('admin.dashboard.loading')"
      />

      <UiErrorState
        v-else-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <!-- Aucune édition retenue : l'écran le dit, il n'affiche pas des zéros. -->
      <UiEmptyState
        v-else-if="!dashboard"
        class="mt-8"
        icon="chart"
        :title="t('admin.dashboard.empty.title')"
        :description="t('admin.dashboard.empty.description')"
      />

      <template v-else>
        <!-- ZONE 1 -->
        <AdminActionQueue class="mt-8" :actions="dashboard.actions" :timezone="timezone" />

        <!-- ZONE 2 -->
        <section class="mt-12" aria-labelledby="admin-figures-title">
          <h2 id="admin-figures-title" class="text-xl font-semibold">
            {{ t('admin.dashboard.figures.title') }}
          </h2>

          <!-- DEUX SOUS-BLOCS, ET L'ORDRE EST LE SUJET : « où en est-on » avant
               « comment cela évolue ». Six chiffres répondent d'un coup d'œil à la
               première question ; les graphiques, qui demandent qu'on les lise,
               répondent à la seconde. -->
          <h3 class="mt-5 mb-3 text-sm font-semibold tracking-wide text-text-subtle uppercase">
            {{ t('admin.dashboard.figures.kpiTitle') }}
          </h3>

          <AdminKeyFigures :kpis="figures?.kpis ?? []" :timezone="timezone" />

          <h3 class="mt-10 mb-4 text-sm font-semibold tracking-wide text-text-subtle uppercase">
            {{ t('admin.dashboard.figures.chartsTitle') }}
          </h3>

          <!-- `items-start` : sans lui, une carte de courbe s'étire à la hauteur
               de la carte de répartitions voisine et laisse un grand vide sous son
               graphique. -->
          <div class="grid items-start gap-x-8 gap-y-10 lg:grid-cols-2">
            <UiCard v-if="figures?.funnel">
              <AdminProposalFunnel :funnel="figures.funnel" />
            </UiCard>
            <UiCard v-else sunken>
              <p class="text-sm text-text-muted">{{ t('admin.dashboard.funnel.noCall') }}</p>
            </UiCard>

            <UiCard>
              <AdminTrendChart
                :title="t('admin.dashboard.charts.submissions.title')"
                :series-label="t('admin.dashboard.charts.submissions.series')"
                :points="figures?.submissions ?? []"
                :markers="submissionMarkers"
                :total-label="
                  t('admin.dashboard.charts.submissions.total', total(figures?.submissions ?? []))
                "
              />
            </UiCard>

            <UiCard>
              <AdminTrendChart
                :title="t('admin.dashboard.charts.registrations.title')"
                :series-label="t('admin.dashboard.charts.registrations.series')"
                :points="figures?.registrations ?? []"
                tone="postponed"
                :total-label="
                  t(
                    'admin.dashboard.charts.registrations.total',
                    total(figures?.registrations ?? []),
                  )
                "
              />
            </UiCard>

            <!-- Deux cartes et non une : empilées dans le même cadre, les deux
                 répartitions donnaient une carte deux fois plus haute que sa
                 voisine, et la grille se déséquilibrait. -->
            <UiCard>
              <AdminBreakdown
                :title="t('admin.dashboard.breakdown.country.title')"
                :note="t('admin.dashboard.breakdown.country.note')"
                :slices="figures?.by_country ?? []"
              />
            </UiCard>

            <UiCard>
              <AdminBreakdown
                :title="t('admin.dashboard.breakdown.theme.title')"
                :note="t('admin.dashboard.breakdown.theme.note')"
                :slices="figures?.by_theme ?? []"
                fallback-tone="postponed"
              />
            </UiCard>
          </div>
        </section>

        <!-- ZONE 3 -->
        <AdminHealthPanel class="mt-12" :rows="dashboard.health" />
      </template>
    </template>
  </div>
</template>
