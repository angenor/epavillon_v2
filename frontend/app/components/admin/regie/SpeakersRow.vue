<script setup lang="ts">
import type { RegieSpeakerSummary } from '~/types/admin-regie'
import type { TimeZoneName } from '~/types/shared'

interface Props {
  summary: RegieSpeakerSummary
  timezone: TimeZoneName
}

const props = defineProps<Props>()
const emit = defineEmits<{ register: []; resend: []; owner: [] }>()

const { t } = useI18n()
const dt = useDateTime()
const k = (key: string, params: Record<string, unknown> = {}, plural?: number) =>
  plural === undefined ? t(`admin.regie.speakers.row.${key}`, params) : t(`admin.regie.speakers.row.${key}`, params, plural)

const at = (value: string) => ({ date: dayMonth(value, props.timezone, dt.intlLocale.value), time: dt.time(value, props.timezone) })

const lead = computed(() => k('online', { count: props.summary.online }, props.summary.online))
const rest = computed(() => {
  const s = props.summary
  const parts = [s.online === 0 ? k('noneTail') : s.to_list > 0 ? k('toList', { count: s.to_list }, s.to_list) : k('end')]
  if (s.links_sent_at) parts.push(` ${k('linksSent', at(s.links_sent_at))}`)
  if (s.owner_link_sent_at) parts.push(` ${k('ownerSent', at(s.owner_link_sent_at))}`)
  return parts.join('')
})
</script>

<template>
  <div
    class="col-span-full flex flex-wrap items-center gap-x-2.5 gap-y-2 rounded-md border border-dashed border-border bg-surface px-3 py-2.5 md:col-span-1 md:col-start-2"
    role="group"
    :aria-label="k('label')"
  >
    <p class="mr-auto min-w-0 text-[0.82rem] text-text-secondary">
      <strong class="font-semibold text-text">{{ lead }}</strong>{{ rest }}
    </p>
    <UiButton class="max-w-full whitespace-normal!" size="sm" variant="secondary" @click="emit('register')">{{ k('register') }}</UiButton>
    <UiButton
      class="max-w-full whitespace-normal!"
      size="sm"
      variant="secondary"
      :disabled="props.summary.reachable === 0"
      :title="props.summary.reachable === 0 ? k('resendNone') : undefined"
      @click="emit('resend')"
    >
      {{ k('resend') }}
    </UiButton>
    <UiButton
      class="max-w-full whitespace-normal!"
      size="sm"
      variant="secondary"
      :disabled="!props.summary.has_owner"
      :title="props.summary.has_owner ? undefined : k('ownerNone')"
      @click="emit('owner')"
    >
      {{ k('owner') }}
    </UiButton>
  </div>
</template>
