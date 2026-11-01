<script setup lang="ts">
import type { IncidentPayload } from '~/types/admin-incidents'
import type { RegieSession } from '~/types/admin-regie'
import type { TaxonomyTerm } from '~/types/reference'
import type { EventId, TimeZoneName } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { ComposerCommand, RegieTemplateId } from '~/utils/regie'

interface Props {
  eventId: EventId
  sessions: RegieSession[]
  focusId: string | null
  kinds: TaxonomyTerm[]
  timezone: TimeZoneName
  nowMs: number
  submitting: boolean
  command: ComposerCommand | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ publish: [payload: IncidentPayload] }>()

const { t } = useI18n()
const dt = useDateTime()

const EVENT_TARGET = 'event'
const template = ref<RegieTemplateId>('delay')
const target = ref<string>(props.focusId ?? EVENT_TARGET)
const textFr = ref('')
const textEn = ref('')
const edited = ref(false)
const textarea = ref<HTMLElement | null>(null)

const targets = computed(() =>
  props.sessions.filter(
    (session) => session.status !== 'cancelled' && session.status !== 'postponed' && Date.parse(session.ends_at) > props.nowMs,
  ),
)
const targetSession = computed(() => targets.value.find((session) => session.session_id === target.value) ?? null)

const targetOptions = computed<SelectOption[]>(() => [
  ...targets.value.map((session) => ({
    value: session.session_id,
    label: t('admin.regie.composer.target.session', { time: dt.time(session.starts_at, props.timezone) }),
  })),
  { value: EVENT_TARGET, label: t('admin.regie.composer.target.event') },
])

/** Le texte du modèle dans une langue donnée : la base exige le français et l'anglais. */
function templateText(id: RegieTemplateId, locale: 'fr' | 'en'): string {
  if (id === 'free') return ''
  const session = targetSession.value
  const time = session ? formatTime(session.starts_at, { timeZone: props.timezone, locale: locale === 'fr' ? 'fr-FR' : 'en-GB' }) : ''
  const variant = session ? 'session' : 'event'
  return t(`admin.regie.composer.text.${id}.${variant}`, { time }, { locale })
}

function refill(): void {
  textFr.value = templateText(template.value, 'fr')
  textEn.value = templateText(template.value, 'en')
  edited.value = false
}

function chooseTemplate(id: RegieTemplateId): void {
  template.value = id
  refill()
}

watch(target, () => {
  if (!edited.value) refill()
})

watch(
  () => props.command?.nonce,
  () => {
    const command = props.command
    if (!command) return
    target.value = command.sessionId && targets.value.some((s) => s.session_id === command.sessionId) ? command.sessionId : (props.focusId ?? EVENT_TARGET)
    template.value = command.template
    refill()
    if (command.template !== 'delay') nextTick(() => textarea.value?.querySelector('textarea')?.focus())
  },
)

watch(
  () => props.focusId,
  (id) => {
    if (!targets.value.some((s) => s.session_id === target.value) && target.value !== EVENT_TARGET) target.value = id ?? EVENT_TARGET
  },
)

// Le texte se prépare dans les deux langues : l'autre locale n'est pas chargée d'office.
const nuxtApp = useNuxtApp()
onMounted(async () => {
  refill()
  await Promise.all((['fr', 'en'] as const).map((code) => nuxtApp.$i18n.loadLocaleMessages(code)))
  if (!edited.value) refill()
})

const knownKinds = computed(() => props.kinds.map((kind) => kind.code))
const canSubmit = computed(() => textFr.value.trim().length > 0 && !props.submitting)

function submit(): void {
  if (!canSubmit.value) return
  const chosen = REGIE_TEMPLATES.find((entry) => entry.id === template.value) ?? REGIE_TEMPLATES[0]!
  const session = targetSession.value
  const fr = textFr.value.trim()
  emit('publish', {
    scope: session ? 'session' : 'event',
    event_id: session ? null : props.eventId,
    event_day_id: null,
    session_id: session?.session_id ?? null,
    organization_id: null,
    incident_kind_code: templateKind(chosen, knownKinds.value),
    severity: chosen.severity,
    title: null,
    // L'anglais laissé vide reprend le français : un message publié vaut mieux qu'un refus pendant que la salle attend.
    message: { fr, en: textEn.value.trim() || fr },
    action_url: null,
    is_dismissible: chosen.severity === 'info',
    display_from: new Date(props.nowMs).toISOString(),
    display_until: session ? session.ends_at : null,
    publish: true,
  })
}
</script>

<template>
  <form class="grid gap-2.5 px-4 pt-3.5 pb-4.5 sm:px-5" @submit.prevent="submit">
    <span class="text-xs font-bold tracking-caps text-text-muted uppercase">{{ t('admin.regie.composer.title') }}</span>
    <div class="flex flex-wrap gap-1.5" role="group" :aria-label="t('admin.regie.composer.template')">
      <button
        v-for="entry in REGIE_TEMPLATES"
        :key="entry.id"
        type="button"
        class="min-h-(--target-min) cursor-pointer rounded-full border px-3 text-sm transition-colors duration-(--duration-fast)"
        :class="template === entry.id ? 'border-accent-solid bg-accent-surface font-semibold text-accent' : 'border-border-strong bg-surface-raised text-text hover:bg-surface-hover'"
        :aria-pressed="template === entry.id"
        @click="chooseTemplate(entry.id)"
      >
        {{ t(`admin.regie.composer.templates.${entry.id}`) }}
      </button>
    </div>
    <div ref="textarea">
      <UiTextarea
        v-model="textFr"
        :label="t('admin.regie.composer.textFr')"
        :rows="3"
        block
        @update:model-value="edited = true"
      />
    </div>
    <details class="text-sm">
      <summary class="min-h-(--target-min) cursor-pointer py-2 text-text-secondary">{{ t('admin.regie.composer.english') }}</summary>
      <UiTextarea v-model="textEn" :label="t('admin.regie.composer.textEn')" :hint="t('admin.regie.composer.englishHint')" :rows="3" block />
    </details>
    <div class="flex flex-wrap items-end justify-between gap-2">
      <UiSelect v-model="target" class="min-w-0 flex-1" :label="t('admin.regie.composer.where')" :options="targetOptions" hide-optional />
      <UiButton type="submit" :disabled="!canSubmit" :loading="props.submitting">{{ t('admin.regie.composer.publish') }}</UiButton>
    </div>
  </form>
</template>
