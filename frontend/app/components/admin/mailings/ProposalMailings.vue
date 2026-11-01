<script setup lang="ts">
import type { MailingListScreen, MailingSummary } from '~/types/admin-mailings'
import type { Uuid } from '~/types/shared'

/**
 * LES COURRIELS D'UN DOSSIER (01/11), pour sa fiche : ceux écrits depuis le
 * dossier par « Écrire au déposant », du plus récent au plus ancien. Pour
 * chacun, le texte type choisi (à défaut l'objet), la date d'envoi dans le
 * fuseau de l'édition, le destinataire principal, le nombre d'adresses en copie
 * et les messages en échec. Une ligne ouvre l'envoi (un brouillon, dans la
 * fenêtre de rédaction).
 *
 * Gardé comme la rubrique Courriels : `engagement.mailing.send` sur l'édition du
 * dossier. Hors périmètre, l'API répond comme si le dossier n'existait pas.
 */

interface Props {
  proposalId: Uuid
}

const props = defineProps<Props>()
const { t } = useI18n()
const api = useApi()
const localePath = useLocalePath()
const composer = useMailComposerStore()
const { dateTime } = useDateTime()

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<MailingListScreen | null>(
  `admin-proposal-mailings-${props.proposalId}`,
  () => api.adminMailings.listForProposal(props.proposalId),
  { lazy: true, watch: [() => props.proposalId] },
)

// La fenêtre de rédaction se ferme : un brouillon a pu partir ou disparaître.
watch(() => composer.version, () => void refresh())

const rows = computed<MailingSummary[]>(() => screen.value?.rows ?? [])
const forbidden = computed(() => (status.value === 'success' && screen.value === null) || isForbiddenError(error.value))
const zone = computed(() => {
  const s = screen.value
  return s ? s.city || timeZoneCityLabel(s.timezone) : ''
})

function when(row: MailingSummary): string {
  const s = screen.value
  if (!s) return ''
  const at = row.queued_at ?? row.updated_at
  return t('admin.mailing.common.atZone', {
    date: dateTime(at, s.timezone),
    zone: t(zoneFormulaKey('zoneOf', zone.value), { zone: zone.value }),
  })
}

function open(row: MailingSummary): void {
  if (row.status === 'draft') composer.open(row.id)
  else navigateTo(localePath(`/admin/courriels/${row.id}`))
}
</script>

<template>
  <section class="space-y-3" :aria-label="t('admin-mailing-proposal-mailings.title')">
    <UiLoadingState v-if="status === 'pending' && !screen" variant="text" :lines="3" />

    <UiForbiddenState
      v-else-if="forbidden"
      compact
      :description="t('admin-mailing-proposal-mailings.forbidden')"
    />

    <UiErrorState v-else-if="error" compact :description="apiErrorMessage(error, t)" @retry="refresh()" />

    <UiEmptyState
      v-else-if="rows.length === 0"
      compact
      icon="mail"
      :title="t('admin-mailing-proposal-mailings.empty.title')"
      :description="t('admin-mailing-proposal-mailings.empty.description')"
    />

    <ul v-else class="divide-y divide-border border-y border-border">
      <li v-for="row in rows" :key="row.id">
        <button
          type="button"
          class="flex min-h-(--target-min) w-full cursor-pointer flex-wrap items-start justify-between gap-x-4 gap-y-1 py-3 text-left hover:bg-surface-raised"
          @click="open(row)"
        >
          <span class="min-w-0 flex-1">
            <span class="block font-semibold wrap-break-word text-text">
              {{ row.mail_text_name ?? (row.shown_subject || t('admin.mailing.common.noSubject')) }}
            </span>
            <span v-if="row.mail_text_name && row.shown_subject" class="block text-sm wrap-break-word text-text-secondary">
              {{ row.shown_subject }}
            </span>
            <span class="mt-0.5 block text-xs text-text-subtle">
              <template v-if="row.to">{{ t('admin-mailing-proposal-mailings.to', { to: row.to }) }}</template>
              <template v-if="row.cc_count">
                · {{ t('admin-mailing-proposal-mailings.cc', { count: row.cc_count }, row.cc_count) }}
              </template>
            </span>
          </span>
          <span class="flex shrink-0 flex-col items-end gap-1 text-right">
            <AdminMailingsStatusBadge of="mailing" :status="row.status" />
            <span v-if="row.status !== 'draft'" class="text-xs text-text-secondary">{{ when(row) }}</span>
            <span v-if="row.counts.failed" class="text-xs font-semibold text-danger">
              {{ t('admin.mailing.list.failed', { count: row.counts.failed }, row.counts.failed) }}
            </span>
          </span>
        </button>
      </li>
    </ul>
  </section>
</template>
