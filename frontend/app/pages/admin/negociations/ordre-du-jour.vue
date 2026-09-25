<script setup lang="ts">
import type { EffectivePermission } from '~/types/identity'
import type { AgendaItemAdmin } from '~/types/negotiation-sessions'
import type { TaxonomyTerm } from '~/types/reference'
import type { SelectOption } from '~/types/ui'
import type { PublicEditionRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'ORDRE DU JOUR d'une COP (FR-040) : chaque point, rattaché ou non à une
 * thématique. Les libellés des thématiques viennent du vocabulaire, jamais d'un
 * fichier de traduction.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationAgenda' }],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.negociations.ordre-du-jour.title') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)
const canManage = computed(() => hasPermission(granted.value, 'negotiation.space.manage'))

const { data: editions, status: editionsStatus } = await useAsyncData<PublicEditionRow[]>(
  'admin-negotiation-cop-editions',
  () => api.events.publicList(),
  { default: () => [], lazy: true },
)
const choixEditions = computed(() => editionsDeCop(editions.value, locale.value))
const slug = computed(() =>
  typeof route.query.edition === 'string' && route.query.edition
    ? route.query.edition
    : (choixEditions.value.parDefaut ?? ''),
)
const fuseau = computed(
  () => (editions.value.find((e) => e.slug === slug.value)?.timezone ?? 'UTC') as TimeZoneName,
)

const { data: points, status, error, refresh } = await useAsyncData<AgendaItemAdmin[] | null>(
  'admin-negotiation-agenda-items',
  () => (slug.value ? api.adminNegotiations.pointsDeLOrdreDuJour(slug.value) : Promise.resolve(null)),
  { lazy: true, watch: [slug] },
)

const { data: vocabulaire } = await useAsyncData<TaxonomyTerm[]>(
  'reference-terms-negotiation_theme',
  () => api.reference.terms('negotiation_theme'),
  { default: () => [], lazy: true },
)
const themes = computed<SelectOption[]>(() =>
  vocabulaire.value
    .filter((terme) => terme.is_active)
    .map((terme) => ({ value: terme.code, label: resolveI18nText(terme.label, locale.value) })),
)

const isForbidden = computed(
  () => (!canManage.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)
const sansTheme = computed(() => (points.value ?? []).filter((p) => !p.theme).length)

function choisirLEdition(valeur: string): void {
  router.replace({ query: { ...route.query, edition: valeur } })
}

/** Le point garde sa place : la liste ne se retrie qu'au prochain chargement. */
function remplacer(point: AgendaItemAdmin): void {
  if (points.value) points.value = points.value.map((p) => (p.id === point.id ? point : p))
}
</script>

<template>
  <div class="mx-auto w-full max-w-4xl">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.ordre-du-jour.forbidden.scope')"
      :description="t('admin.negociations.ordre-du-jour.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-4">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.negociations.ordre-du-jour.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.negociations.ordre-du-jour.subtitle') }}
          </p>
        </div>
        <UiSelect
          v-if="choixEditions.options.length > 1"
          class="w-full sm:w-56"
          :label="t('admin.negociations.ordre-du-jour.edition')"
          :options="choixEditions.options"
          :model-value="slug"
          hide-optional
          @update:model-value="choisirLEdition"
        />
      </header>

      <UiEmptyState
        v-if="editionsStatus === 'success' && !choixEditions.options.length"
        class="mt-8"
        icon="calendar"
        :title="t('admin.negociations.ordre-du-jour.noEdition.title')"
        :description="t('admin.negociations.ordre-du-jour.noEdition.description')"
      />

      <UiErrorState v-else-if="error" class="mt-8" :retry-label="t('common.actions.retry')" @retry="refresh()" />

      <UiLoadingState v-else-if="status === 'pending' || !points" class="mt-8" variant="list" />

      <UiEmptyState
        v-else-if="!points.length"
        class="mt-8"
        icon="list"
        :title="t('admin.negociations.ordre-du-jour.empty.title')"
        :description="t('admin.negociations.ordre-du-jour.empty.description')"
        :action-label="t('admin.negociations.ordre-du-jour.importLink')"
        :action-to="localePath({ path: '/admin/negociations/import', query: { edition: slug } })"
      />

      <template v-else>
        <UiAlert
          class="mt-6"
          :intent="sansTheme > 0 ? 'warning' : 'success'"
          compact
          live
          :message="t('admin.negociations.ordre-du-jour.withoutTheme', sansTheme)"
        />
        <ul class="mt-4">
          <AdminNegotiationAgendaItemRow
            v-for="point in points"
            :key="point.id"
            :point="point"
            :themes="themes"
            :timezone="fuseau"
            @updated="remplacer"
          />
        </ul>
      </template>
    </template>
  </div>
</template>
