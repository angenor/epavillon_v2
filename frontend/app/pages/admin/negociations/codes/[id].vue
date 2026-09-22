<script setup lang="ts">
import type { InvitationCodeUseRow } from '~/types/admin-negotiation'
import type { EffectivePermission } from '~/types/identity'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LA FICHE D'UN CODE, ET CE QU'IL A OUVERT.
 *
 * RÉVOQUER N'EST PAS RETIRER, ET L'ÉCRAN DOIT LE MONTRER (ADR-006, FR-039).
 * Deux boutons, deux confirmations, deux phrases : révoquer ferme la porte et
 * **laisse les accès déjà accordés ouverts** — la confirmation dit combien —,
 * retirer sort des personnes sans invalider le code du groupe. Les fondre en un
 * seul geste ferait de chaque fuite de code une exclusion collective.
 *
 * LE NOMBRE D'ACCÈS ENCORE OUVERTS EST AFFICHÉ AVANT LA RÉVOCATION, pas après :
 * c'est ce qui permet à un administrateur de savoir ce qu'il lui restera à
 * faire, au moment où il décide.
 *
 * QUATRE ÉTATS : chargement, introuvable (un code supprimé, ou une adresse
 * forgée — indiscernables, et c'est voulu), erreur avec reprise, accès refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationCodes', to: '/admin/negociations/codes' },
    { labelKey: 'admin.negociations.codes.detail.title' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const localePath = useLocalePath()
const { date, dateTime } = useDateTime()

const codeId = computed(() => String(route.params.id ?? ''))

const timezone = computed<TimeZoneName>(
  () => (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC') as TimeZoneName,
)

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canManage = computed(() => hasPermission(granted.value, 'negotiation.space.manage'))

const { data: code, status, error, refresh } = await useAsyncData(
  'admin-negotiation-code',
  () => api.adminNegotiations.code(codeId.value),
  { default: () => null, watch: [codeId], lazy: true },
)

const { data: uses, refresh: refreshUses } = await useAsyncData(
  'admin-negotiation-code-uses',
  () => api.adminNegotiations.usages(codeId.value),
  { default: () => null, watch: [codeId], lazy: true },
)

useHead(() => ({
  title: code.value
    ? t('admin.negociations.codes.detail.title', { code: code.value.code })
    : t('admin.negociations.codes.list.title'),
}))

const revokeOpen = ref(false)
const revokeAllOpen = ref(false)
const revokeAccessFor = ref<InvitationCodeUseRow | null>(null)
const reason = ref('')
const submitting = ref(false)
const writeError = ref<string | null>(null)
const result = ref<string | null>(null)

/** Le motif se vide à chaque ouverture : celui d'hier ne vaut pas pour aujourd'hui. */
watch([revokeOpen, revokeAllOpen, revokeAccessFor], () => {
  reason.value = ''
  writeError.value = null
})

const rows = computed<InvitationCodeUseRow[]>(() => uses.value?.rows ?? [])
const grantedUses = computed(() => code.value?.granted_uses ?? 0)

const columns = computed<TableColumn[]>(() => [
  { key: 'person', label: t('admin.negociations.codes.detail.uses.columns.person') },
  { key: 'usedAt', label: t('admin.negociations.codes.detail.uses.columns.usedAt'), width: '14rem' },
  { key: 'access', label: t('admin.negociations.codes.detail.uses.columns.access'), width: '14rem' },
  {
    key: 'actions',
    label: t('admin.negociations.codes.detail.uses.columns.actions'),
    align: 'end',
    width: '11rem',
  },
])

const STATE_INTENT = {
  active: 'success',
  revoked: 'danger',
  expired: 'neutral',
  not_yet_valid: 'info',
  exhausted: 'warning',
} as const

async function ecrire(action: () => Promise<string>): Promise<void> {
  if (submitting.value) return
  submitting.value = true
  writeError.value = null
  try {
    result.value = await action()
    await Promise.all([refresh(), refreshUses()])
    revokeOpen.value = false
    revokeAllOpen.value = false
    revokeAccessFor.value = null
  } catch (erreur) {
    writeError.value = erreur instanceof Error ? erreur.message : t('api.unreachable.network')
  } finally {
    submitting.value = false
  }
}

function revoquer(): void {
  void ecrire(async () => {
    await api.adminNegotiations.revoquerLeCode(codeId.value, reason.value || null)
    return t('admin.negociations.codes.result.revoked')
  })
}

function retirerUnAcces(): void {
  const personne = revokeAccessFor.value
  if (!personne) return
  void ecrire(async () => {
    const { revoked } = await api.adminNegotiations.retirerUnAcces(
      codeId.value,
      personne.person_id,
      reason.value || null,
    )
    return revoked > 0
      ? t('admin.negociations.codes.result.accessRevoked')
      : t('admin.negociations.codes.result.accessAlreadyRevoked')
  })
}

function retirerTous(): void {
  void ecrire(async () => {
    const { revoked } = await api.adminNegotiations.retirerTousLesAcces(
      codeId.value,
      reason.value || null,
    )
    return revoked > 0
      ? t('admin.negociations.codes.result.allRevoked', { count: revoked })
      : t('admin.negociations.codes.result.noneRevoked')
  })
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
      <UiLoadingState v-if="status === 'pending'" />

      <UiErrorState
        v-else-if="error"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiEmptyState
        v-else-if="!code"
        icon="search"
        :title="t('admin.negociations.codes.detail.notFound.title')"
        :description="t('admin.negociations.codes.detail.notFound.description')"
        :action-label="t('admin.negociations.codes.detail.notFound.action')"
        :action-to="localePath('/admin/negociations/codes')"
      />

      <template v-else>
        <header class="flex flex-wrap items-start justify-between gap-x-6 gap-y-4">
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-3">
              <h1 class="font-mono text-3xl leading-tight font-semibold tracking-wide">
                {{ code.code }}
              </h1>
              <UiBadge
                :intent="STATE_INTENT[code.state]"
                :label="t(`admin.negociations.codes.state.${code.state}`)"
              />
            </div>
            <p class="mt-1 text-lg">{{ code.label }}</p>
            <p class="mt-1 text-sm text-text-muted">
              {{
                code.created_by_name
                  ? t('admin.negociations.codes.detail.createdBy', {
                      date: date(code.created_at, timezone),
                      author: code.created_by_name,
                    })
                  : t('admin.negociations.codes.detail.createdOn', {
                      date: date(code.created_at, timezone),
                    })
              }}
            </p>
            <p v-if="code.revoked_at" class="mt-1 text-sm text-text-muted">
              {{
                code.revoked_by_name
                  ? t('admin.negociations.codes.detail.revokedBy', {
                      date: date(code.revoked_at, timezone),
                      author: code.revoked_by_name,
                    })
                  : t('admin.negociations.codes.detail.revokedOn', {
                      date: date(code.revoked_at, timezone),
                    })
              }}
              <span v-if="code.revoked_reason">
                — {{ t('admin.negociations.codes.detail.reason', { reason: code.revoked_reason }) }}
              </span>
            </p>
          </div>

          <div class="flex flex-wrap gap-3">
            <UiButton
              v-if="!code.revoked_at"
              variant="danger"
              icon="lock"
              @click="revokeOpen = true"
            >
              {{ t('admin.negociations.codes.detail.revoke') }}
            </UiButton>
            <UiButton
              v-if="grantedUses > 0"
              variant="secondary"
              icon="ban"
              @click="revokeAllOpen = true"
            >
              {{ t('admin.negociations.codes.detail.revokeAll') }}
            </UiButton>
          </div>
        </header>

        <UiAlert v-if="writeError" class="mt-6" intent="danger" live :message="writeError" />
        <UiAlert
          v-else-if="result"
          class="mt-6"
          intent="success"
          live
          compact
          dismissible
          :message="result"
        />

        <UiAlert
          class="mt-6"
          intent="info"
          compact
          :message="t('admin.negociations.codes.detail.grantedUses', { count: grantedUses })"
        />

        <h2 class="mt-8 text-xl font-semibold">
          {{ t('admin.negociations.codes.detail.uses.title') }}
        </h2>

        <UiTable
          class="mt-4"
          :columns="columns"
          :rows="rows"
          row-key="person_id"
          row-label-key="display_name"
          :caption="t('admin.negociations.codes.detail.uses.caption')"
        >
          <template #cell-person="{ row }">
            <span class="font-medium">{{ row.display_name }}</span>
            <span class="mt-0.5 block text-sm text-text-muted">{{ row.email }}</span>
          </template>

          <template #cell-usedAt="{ row }">{{ dateTime(row.used_at, timezone) }}</template>

          <template #cell-access="{ row }">
            <template v-if="row.access_active">
              <UiBadge intent="success" :label="t('admin.negociations.codes.detail.uses.active')" />
            </template>
            <template v-else>
              <UiBadge intent="neutral" :label="t('admin.negociations.codes.state.revoked')" />
              <span v-if="row.access_revoked_at" class="mt-0.5 block text-sm text-text-muted">
                {{
                  t('admin.negociations.codes.detail.uses.revoked', {
                    date: date(row.access_revoked_at, timezone),
                  })
                }}
              </span>
            </template>
          </template>

          <template #cell-actions="{ row }">
            <UiButton
              v-if="row.access_active"
              variant="ghost"
              size="sm"
              @click="revokeAccessFor = row"
            >
              {{ t('admin.negociations.codes.detail.uses.revoke') }}
            </UiButton>
          </template>

          <template #empty>
            <UiEmptyState
              icon="users"
              compact
              :title="t('admin.negociations.codes.detail.uses.empty.title')"
              :description="t('admin.negociations.codes.detail.uses.empty.description')"
            />
          </template>
        </UiTable>
      </template>
    </template>

    <!-- Révoquer le code : la porte se ferme, les accès restent. -->
    <UiModal
      v-model:open="revokeOpen"
      :title="t('admin.negociations.codes.confirm.revoke.title')"
      :description="t('admin.negociations.codes.confirm.revoke.question')"
    >
      <UiAlert
        intent="info"
        compact
        :message="
          grantedUses > 0
            ? t('admin.negociations.codes.confirm.revoke.keeps', { count: grantedUses })
            : t('admin.negociations.codes.confirm.revoke.keepsNone')
        "
      />
      <UiFormField
        class="mt-4"
        :label="t('admin.negociations.codes.confirm.revoke.reason')"
        :hint="t('admin.negociations.codes.confirm.revoke.reasonHint')"
      >
        <template #default="{ control }">
          <UiTextarea v-bind="control" v-model="reason" :rows="2" :maxlength="280" />
        </template>
      </UiFormField>
      <div class="mt-5 flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="revokeOpen = false">
          {{ t('admin.negociations.codes.confirm.revoke.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="submitting" @click="revoquer">
          {{ t('admin.negociations.codes.confirm.revoke.confirm') }}
        </UiButton>
      </div>
    </UiModal>

    <!-- Retirer l'accès d'une personne : le code, lui, reste ce qu'il est. -->
    <UiModal
      :open="revokeAccessFor !== null"
      :title="
        t('admin.negociations.codes.confirm.revokeAccess.title', {
          name: revokeAccessFor?.display_name ?? '',
        })
      "
      :description="t('admin.negociations.codes.confirm.revokeAccess.question')"
      @update:open="(value: boolean) => !value && (revokeAccessFor = null)"
    >
      <UiFormField :label="t('admin.negociations.codes.confirm.revokeAccess.reason')">
        <template #default="{ control }">
          <UiTextarea v-bind="control" v-model="reason" :rows="2" :maxlength="280" />
        </template>
      </UiFormField>
      <div class="mt-5 flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="revokeAccessFor = null">
          {{ t('admin.negociations.codes.confirm.revokeAccess.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="submitting" @click="retirerUnAcces">
          {{ t('admin.negociations.codes.confirm.revokeAccess.confirm') }}
        </UiButton>
      </div>
    </UiModal>

    <!-- Retirer tous les accès : le geste d'un code compromis. -->
    <UiModal
      v-model:open="revokeAllOpen"
      :title="t('admin.negociations.codes.confirm.revokeAll.title')"
      :description="t('admin.negociations.codes.confirm.revokeAll.question', { count: grantedUses })"
    >
      <UiAlert
        intent="warning"
        compact
        :message="t('admin.negociations.codes.confirm.revokeAll.warning')"
      />
      <UiFormField class="mt-4" :label="t('admin.negociations.codes.confirm.revokeAll.reason')">
        <template #default="{ control }">
          <UiTextarea v-bind="control" v-model="reason" :rows="2" :maxlength="280" />
        </template>
      </UiFormField>
      <div class="mt-5 flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="revokeAllOpen = false">
          {{ t('admin.negociations.codes.confirm.revokeAll.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="submitting" @click="retirerTous">
          {{ t('admin.negociations.codes.confirm.revokeAll.confirm') }}
        </UiButton>
      </div>
    </UiModal>
  </div>
</template>
