<script setup lang="ts">
import type { Mailing, MailingRecipientRow } from '~/types/admin-mailings'
import type { ShowcaseSessionOption } from '~/types/admin-showcase'

/**
 * LA FICHE D'UN ENVOI PARTI : ce qui a été écrit, à qui, et ce qu'il est
 * advenu de chaque message. C'est ce que la boîte d'envoi, illisible, ne
 * montrait pas. Le corps est montré tel qu'écrit, variables comprises.
 */
interface Props {
  mailing: Mailing
  timezone: string
  zone: string
  sessions: ShowcaseSessionOption[]
  busy: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ duplicate: []; cancel: []; refresh: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

function at(value: string | null): string {
  return value ? t('admin.mailing.common.atZone', { date: dateTime(value, props.timezone), zone: t(zoneFormulaKey('zoneOf', props.zone), { zone: props.zone }) }) : ''
}

const counts = computed(() => [
  { key: 'sent', value: props.mailing.counts.sent },
  { key: 'queued', value: props.mailing.counts.queued + props.mailing.counts.sending },
  { key: 'failed', value: props.mailing.counts.failed },
  { key: 'suppressed', value: props.mailing.counts.suppressed },
  { key: 'cancelled', value: props.mailing.counts.cancelled },
])

const rows = computed<MailingRecipientRow[]>(() =>
  props.mailing.recipients.map((r) => ({
    email: r.email,
    name: [r.first_name, r.last_name].filter(Boolean).join(' ') || null,
    organization: r.organization,
    sources: r.sources,
    status: r.status,
    suppressed: r.status === 'suppressed',
    error: r.last_error,
    sentAt: r.sent_at,
  })),
)

function sessionLabel(id: string | null): string {
  if (!id) return ''
  const session = props.sessions.find((s) => s.id === id)
  return session ? tr(session.title) : t('admin.mailing.common.oneSession')
}
</script>

<template>
  <div class="space-y-10">
    <section class="space-y-4">
      <div class="flex flex-wrap items-center gap-3">
        <AdminMailingsStatusBadge of="mailing" :status="props.mailing.status" />
        <span class="text-sm text-text-secondary">
          {{
            t('admin.mailing.record.sentBy', {
              name: props.mailing.queued_by_name ?? t('admin.mailing.common.someone'),
              date: at(props.mailing.queued_at),
            })
          }}
        </span>
      </div>
      <p v-if="props.mailing.completed_at" class="text-sm text-text-subtle">
        {{ t('admin.mailing.record.completedAt', { date: at(props.mailing.completed_at) }) }}
      </p>
      <p v-if="props.mailing.cancelled_at" class="text-sm text-text-subtle">
        {{ t('admin.mailing.record.cancelledAt', { date: at(props.mailing.cancelled_at) }) }}
      </p>
      <UiAlert
        v-if="props.mailing.status === 'queued'"
        intent="warning"
        compact
        :message="t('admin.mailing.record.inProgress', { limit: props.mailing.hourly_limit })"
      />

      <dl class="grid grid-cols-2 gap-px overflow-hidden rounded-lg border border-border bg-border sm:grid-cols-5">
        <div v-for="c in counts" :key="c.key" class="bg-surface-raised px-4 py-3 last:col-span-2 sm:last:col-span-1">
          <dt class="text-xs text-text-subtle">{{ t(`admin.mailing.record.counts.${c.key}`) }}</dt>
          <dd class="mt-1 font-display text-2xl font-semibold tabular-nums">{{ c.value }}</dd>
        </div>
      </dl>

      <div class="flex flex-wrap gap-3">
        <UiButton icon="copy" :loading="props.busy" @click="emit('duplicate')">
          {{ t('admin.mailing.record.duplicate') }}
        </UiButton>
        <UiButton variant="secondary" icon="refresh" :disabled="props.busy" @click="emit('refresh')">
          {{ t('admin.mailing.record.refresh') }}
        </UiButton>
        <UiButton
          v-if="props.mailing.status === 'queued'"
          variant="danger"
          icon="ban"
          :disabled="props.busy"
          @click="emit('cancel')"
        >
          {{ t('admin.mailing.record.cancel') }}
        </UiButton>
      </div>
    </section>

    <section>
      <h2 class="font-display text-xl font-semibold">{{ t('admin.mailing.record.message') }}</h2>
      <AdminMailingsPreview
        class="mt-3"
        :subject="props.mailing.subject"
        :body-html="props.mailing.body_html"
        :to="t('admin.mailing.record.eachRecipient')"
        :cc="props.mailing.cc"
        :bcc="props.mailing.bcc"
        :reply-to="props.mailing.reply_to"
        :values="{}"
      />
    </section>

    <section>
      <h2 class="font-display text-xl font-semibold">{{ t('admin.mailing.record.criteria') }}</h2>
      <ul class="mt-3 space-y-1 text-sm text-text-secondary">
        <li v-for="c in props.mailing.audience" :key="c.kind">
          <span class="font-semibold text-text">{{ t(`admin.mailing.common.kind.${c.kind}`) }}</span>
          <template v-if="c.proposal_statuses.length">
            · {{ c.proposal_statuses.map((s) => t(`admin.mailing.common.proposalStatus.${s}`)).join(', ') }}
          </template>
          <template v-if="c.registration_statuses.length">
            · {{ c.registration_statuses.map((s) => t(`admin.mailing.common.registrationStatus.${s}`)).join(', ') }}
          </template>
          <template v-if="c.role_codes.length">
            · {{ c.role_codes.map((s) => t(`admin.mailing.common.role.${s}`)).join(', ') }}
          </template>
          <template v-if="c.session_id"> · {{ sessionLabel(c.session_id) }}</template>
        </li>
        <li v-if="props.mailing.extra_addresses.length">
          <span class="font-semibold text-text">{{ t('admin.mailing.common.source.addresses') }}</span>
          · {{ props.mailing.extra_addresses.join(', ') }}
        </li>
      </ul>
    </section>

    <section>
      <h2 class="font-display text-xl font-semibold">
        {{ t('admin.mailing.record.recipients', { count: props.mailing.counts.total }) }}
      </h2>
      <AdminMailingsRecipientList
        class="mt-3"
        :rows="rows"
        :caption="t('admin.mailing.record.recipients', { count: props.mailing.counts.total })"
        :timezone="props.timezone"
        :zone="props.zone"
      />
    </section>
  </div>
</template>
