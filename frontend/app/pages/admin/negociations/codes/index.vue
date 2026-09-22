<script setup lang="ts">
import type { InvitationCodeRow, NetworkSummary } from '~/types/admin-negotiation'
import type { EffectivePermission } from '~/types/identity'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES CODES D'INVITATION DE GUIDE NÉGO (0b).
 *
 * LE PÉRIMÈTRE N'EST PAS CELUI DES AUTRES ÉCRANS DU BACK-OFFICE. Partout
 * ailleurs, un administrateur détaché sur une édition voit la sienne ; ici, la
 * permission `negotiation.space.manage` est exigée **sur la portée globale**, et
 * `hasPermission(granted, code)` — sans identifiant d'édition — est exactement ce
 * test. Un espace de négociation n'est rattaché à aucune édition : filtrer par
 * édition n'aurait rien à filtrer, et laisserait croire à une garde là où il n'y
 * en aurait pas.
 *
 * LE CODE EST AFFICHÉ EN CLAIR, ET C'EST VOULU. Il circule sur WhatsApp, recopié
 * à la main par tout un réseau : c'est un secret PARTAGÉ, pas un secret
 * nominatif (FR-037). Le masquer empêcherait de répondre à la seule question
 * qu'on pose à cet écran — « quel est le code en cours ? ».
 *
 * L'ÉTAT VIENT DE L'API, JAMAIS D'UN CALCUL ICI. `negotiation.v_invitation_codes`
 * le dérive une fois, et c'est la même expression qui refuse un code à
 * l'application. Le recalculer ferait diverger la liste de ce que la personne
 * voit sur son téléphone au même instant.
 *
 * QUATRE ÉTATS : chargement (lignes squelettes), vide (aucun code, ce qui veut
 * dire que personne ne peut entrer), erreur avec reprise, accès refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationCodes' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const { date } = useDateTime()

useHead(() => ({ title: t('admin.negociations.codes.list.title') }))

/** Les dates d'un code n'appartiennent à aucune édition : le fuseau est celui de qui lit. */
const timezone = computed<TimeZoneName>(
  () => (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC') as TimeZoneName,
)

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/**
 * **Sur la portée globale, et elle seule.** `hasPermission` sans identifiant
 * d'édition n'accepte qu'une attribution `global` : c'est précisément la garde
 * que l'API applique, et la raison pour laquelle un administrateur d'une seule
 * édition ne voit rien de cet écran (FR-044, SC-008).
 */
const canManage = computed(() => hasPermission(granted.value, 'negotiation.space.manage'))

/** Les paramètres sont en FRANÇAIS : ils apparaissent dans une URL qu'on partage. */
const filters = computed(() => ({
  etat: typeof route.query.etat === 'string' ? route.query.etat : '',
  espace: typeof route.query.espace === 'string' ? route.query.espace : '',
  q: typeof route.query.q === 'string' ? route.query.q : '',
}))

function setFilter(patch: Record<string, string>): void {
  const next = { ...route.query }
  for (const [key, value] of Object.entries(patch)) {
    if (value) next[key] = value
    else delete next[key]
  }
  router.replace({ query: next })
}

const { data: screen, status, error, refresh } = await useAsyncData(
  'admin-negotiation-codes',
  () => api.adminNegotiations.codes(filters.value),
  { watch: [filters], lazy: true },
)

const rows = computed<InvitationCodeRow[]>(() => screen.value?.rows ?? [])
const filtered = computed(() => Boolean(filters.value.etat || filters.value.espace || filters.value.q))

/**
 * **Le compte des appartenances, pas celui des usages** (SC-007). Une personne
 * peut entrer avec deux codes, et son appartenance survit à la révocation du
 * code qui l'a apportée : sommer les `used_count` donnerait un autre chiffre, et
 * le faux. Il reste donc hors du tableau, qui est celui des codes.
 */
const networks = computed<NetworkSummary[]>(() => screen.value?.networks ?? [])

const columns = computed<TableColumn[]>(() => [
  { key: 'code', label: t('admin.negociations.codes.list.columns.code'), width: '9rem' },
  { key: 'label', label: t('admin.negociations.codes.list.columns.label') },
  { key: 'scope', label: t('admin.negociations.codes.list.columns.scope'), hideBelow: 'lg', width: '13rem' },
  { key: 'uses', label: t('admin.negociations.codes.list.columns.uses'), width: '9rem' },
  { key: 'validity', label: t('admin.negociations.codes.list.columns.validity'), hideBelow: 'xl', width: '15rem' },
  { key: 'state', label: t('admin.negociations.codes.list.columns.state'), width: '9rem' },
])

const stateOptions = computed(() => [
  { value: '', label: t('admin.negociations.codes.list.filters.allStates') },
  ...(['active', 'revoked', 'expired', 'not_yet_valid', 'exhausted'] as const).map((etat) => ({
    value: etat,
    label: t(`admin.negociations.codes.state.${etat}`),
  })),
])

const spaceOptions = computed(() => [
  { value: '', label: t('admin.negociations.codes.list.filters.allSpaces') },
  { value: 'global', label: t('admin.negociations.codes.list.scope.global') },
  ...(screen.value?.spaces ?? []).map((espace) => ({ value: espace.id, label: espace.name })),
])

const STATE_INTENT = {
  active: 'success',
  revoked: 'danger',
  expired: 'neutral',
  not_yet_valid: 'info',
  exhausted: 'warning',
} as const

function scopeLabel(row: InvitationCodeRow): string {
  return row.scope.type === 'global'
    ? t('admin.negociations.codes.list.scope.global')
    : t('admin.negociations.codes.list.scope.space', { name: row.scope.name ?? '' })
}

function usesLabel(row: InvitationCodeRow): string {
  return row.max_uses === null
    ? t('admin.negociations.codes.list.uses.unlimited', { count: row.used_count })
    : t('admin.negociations.codes.list.uses.limited', { used: row.used_count, max: row.max_uses })
}

function validityLabel(row: InvitationCodeRow): string {
  const from = date(row.valid_from, timezone.value)
  const until = row.valid_until ? date(row.valid_until, timezone.value) : null
  if (until) return t('admin.negociations.codes.list.validity.between', { from, until })
  return t('admin.negociations.codes.list.validity.always')
}

function openCode(row: InvitationCodeRow): void {
  void navigateTo(localePath(`/admin/negociations/codes/${row.id}`))
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="!canManage && permissionStatus !== 'pending'"
      :required-scope="t('admin.negociations.codes.list.forbidden.scope')"
      :description="t('admin.negociations.codes.list.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.negociations.codes.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.negociations.codes.list.subtitle') }}
          </p>
          <ul v-if="networks.length" class="mt-3 flex flex-wrap gap-x-4 gap-y-1 text-sm">
            <li v-for="network in networks" :key="network.id" class="text-text-muted">
              {{ network.label }} —
              <strong class="font-semibold text-text">
                {{ t('admin.negociations.codes.list.members', { count: network.members_count }) }}
              </strong>
            </li>
          </ul>
        </div>
        <UiButton icon="plus" :to="localePath('/admin/negociations/codes/nouveau')">
          {{ t('admin.negociations.codes.list.new') }}
        </UiButton>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiEmptyState
        v-else-if="rows.length === 0 && !filtered && status !== 'pending'"
        class="mt-8"
        icon="lock"
        :title="t('admin.negociations.codes.list.empty.title')"
        :description="t('admin.negociations.codes.list.empty.description')"
        :action-label="t('admin.negociations.codes.list.new')"
        @action="navigateTo(localePath('/admin/negociations/codes/nouveau'))"
      />

      <template v-else>
        <section
          class="mt-6 flex flex-wrap items-end gap-3"
          :aria-label="t('admin.negociations.codes.list.filters.label')"
        >
          <UiSearchInput
            class="min-w-60 grow"
            :model-value="filters.q"
            :placeholder="t('admin.negociations.codes.list.filters.search')"
            :disabled="status === 'pending'"
            @update:model-value="(value: string) => setFilter({ q: value })"
          />
          <UiFormField :label="t('admin.negociations.codes.list.filters.state')" hide-optional>
            <template #default="{ control }">
              <UiSelect
                v-bind="control"
                :model-value="filters.etat"
                :options="stateOptions"
                :disabled="status === 'pending'"
                @update:model-value="(value: string) => setFilter({ etat: value })"
              />
            </template>
          </UiFormField>
          <UiFormField :label="t('admin.negociations.codes.list.filters.space')" hide-optional>
            <template #default="{ control }">
              <UiSelect
                v-bind="control"
                :model-value="filters.espace"
                :options="spaceOptions"
                :disabled="status === 'pending'"
                @update:model-value="(value: string) => setFilter({ espace: value })"
              />
            </template>
          </UiFormField>
        </section>

        <UiTable
          class="mt-4"
          :columns="columns"
          :rows="rows"
          row-key="id"
          row-label-key="label"
          :caption="t('admin.negociations.codes.list.caption')"
          :loading="status === 'pending'"
          @row-click="openCode"
        >
          <template #cell-code="{ row }">
            <span class="font-mono text-sm font-semibold tracking-wide">{{ row.code }}</span>
          </template>

          <template #cell-label="{ row }">
            <span class="font-medium">{{ row.label }}</span>
            <span v-if="row.network" class="mt-0.5 block text-sm text-text-muted">
              {{ t('admin.negociations.codes.list.network') }} — {{ row.network.label }}
            </span>
          </template>

          <template #cell-scope="{ row }">{{ scopeLabel(row) }}</template>
          <template #cell-uses="{ row }">{{ usesLabel(row) }}</template>
          <template #cell-validity="{ row }">{{ validityLabel(row) }}</template>

          <template #cell-state="{ row }">
            <UiBadge
              :intent="STATE_INTENT[row.state]"
              :label="t(`admin.negociations.codes.state.${row.state}`)"
            />
          </template>

          <template #empty>
            <UiEmptyState
              icon="search"
              filtered
              :title="t('admin.negociations.codes.list.noResults.title')"
              :description="
                t('admin.negociations.codes.list.noResults.description', {
                  total: screen?.total ?? 0,
                })
              "
              :action-label="t('admin.negociations.codes.list.noResults.action')"
              @action="setFilter({ etat: '', espace: '', q: '' })"
            />
          </template>
        </UiTable>
      </template>
    </template>
  </div>
</template>
