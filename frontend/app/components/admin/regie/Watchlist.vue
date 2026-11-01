<script setup lang="ts">
import type { TimeZoneName } from '~/types/shared'
import type { RegieAlert, RegieAlertKind } from '~/utils/regie'

interface Props {
  alerts: RegieAlert[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()
const { t, locale } = useI18n()
const dt = useDateTime()

const ICONS: Record<RegieAlertKind, string> = {
  presence: 'clock',
  documents: 'document',
  no_meeting: 'video',
  meeting_not_ready: 'video',
}

const rows = computed(() =>
  props.alerts.map((alert) => {
    const orgs = alert.session.organizations
    const count = Math.max(1, orgs.length)
    const when =
      alert.kind === 'presence' && alert.minutesUntil > 0
        ? t('admin.regie.watch.in', { minutes: alert.minutesUntil })
        : dt.time(alert.session.starts_at, props.timezone)
    return {
      key: `${alert.kind}:${alert.session.session_id}`,
      icon: ICONS[alert.kind],
      when,
      text: t(
        `admin.regie.watch.${alert.kind}`,
        { orgs: orgs.length ? regieOrganizationsLabel(orgs, locale.value) : t('admin.regie.watch.theOrganizers') },
        count,
      ),
    }
  }),
)
</script>

<template>
  <section class="rounded-lg border border-border-subtle bg-surface-raised shadow-sm" aria-labelledby="regie-watch">
    <div class="border-b border-border-subtle px-4 py-3.5 sm:px-5">
      <h2 id="regie-watch" class="heading-condensed text-[1.35rem]">{{ t('admin.regie.watch.title') }}</h2>
    </div>
    <p v-if="!rows.length" class="px-4 py-4 text-sm text-text-muted sm:px-5">{{ t('admin.regie.watch.empty') }}</p>
    <ul v-else class="m-0 list-none px-4 pb-2 sm:px-5">
      <li v-for="row in rows" :key="row.key" class="flex gap-2.5 border-b border-dashed border-border-subtle py-2.5 text-sm last:border-b-0">
        <UiIcon :name="row.icon" size="1rem" class="mt-0.5 shrink-0 text-warning" />
        <i18n-t keypath="admin.regie.watch.line" tag="span" scope="global">
          <template #when><b class="font-semibold">{{ row.when }}</b></template>
          <template #text>{{ row.text }}</template>
        </i18n-t>
      </li>
    </ul>
  </section>
</template>
