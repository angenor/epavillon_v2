<script setup lang="ts">
import type { MailingAudienceEntry, MailingPreview } from '~/types/admin-mailings'
import type { Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * L'APERÇU EN DIRECT de la fenêtre de rédaction (29/10), à droite sur grand
 * écran. Rendu par l'API, par la fonction même du worker : ce qu'on voit est ce
 * qui part. Il se relit après chaque enregistrement (`refreshKey`).
 *
 * `allow-same-origin` sans `allow-scripts` : aucun script ne s'exécute dans le
 * courriel, mais la fenêtre lit sa position de défilement et la rend après
 * chaque relecture. Sans quoi l'aperçu remonterait en haut à chaque pause de
 * frappe.
 */

interface Props {
  mailingId: Uuid
  entries: MailingAudienceEntry[]
  refreshKey: number
}

const props = defineProps<Props>()
const { t } = useI18n()
const api = useApi()

// « Vu par » : une personne avec compte et, s'il y en a, une sans, pour voir le secours.
const viewers = computed<SelectOption[]>(() => {
  const sendable = props.entries.filter((e) => !e.is_suppressed)
  const withAccount = sendable.filter((e) => e.person_id).slice(0, 30)
  const without = sendable.filter((e) => !e.person_id).slice(0, 30)
  return [...without.slice(0, 1), ...withAccount, ...without.slice(1)].map((e) => ({
    value: e.email,
    label: [e.first_name, e.last_name].filter(Boolean).join(' ') || e.email,
    description: e.person_id ? e.email : t('admin.mailing.composer.options.noAccount', { email: e.email }),
  }))
})
const viewer = ref('')
watch(viewers, (list) => {
  if (!list.some((v) => v.value === viewer.value)) viewer.value = list[0]?.value ?? ''
}, { immediate: true })

const frame = ref<HTMLIFrameElement | null>(null)
let scrollTop = 0

function rememberScroll(): void {
  try {
    scrollTop = frame.value?.contentWindow?.scrollY ?? 0
  } catch {
    scrollTop = 0
  }
}

function restoreScroll(): void {
  try {
    frame.value?.contentWindow?.scrollTo(0, scrollTop)
  } catch {
    // Fenêtre devenue inaccessible : l'aperçu reste en haut.
  }
}

const preview = ref<MailingPreview | null>(null)
const previewError = ref<string | null>(null)
watch(
  () => [props.mailingId, viewer.value, props.refreshKey] as const,
  async ([id, email]) => {
    try {
      previewError.value = null
      const fresh = await api.adminMailings.preview(id, email || undefined)
      if (fresh?.html !== preview.value?.html) rememberScroll()
      preview.value = fresh
    } catch (thrown) {
      previewError.value = apiErrorMessage(thrown, t)
    }
  },
  { immediate: true },
)
</script>

<template>
  <section class="flex min-h-0 flex-col gap-2" :aria-label="t('admin.mailing.composer.options.preview')">
    <div class="flex flex-wrap items-end justify-between gap-x-3 gap-y-1">
      <h3 class="text-sm font-semibold text-text">{{ t('admin.mailing.composer.options.preview') }}</h3>
      <UiSelect
        v-if="viewers.length > 1"
        v-model="viewer"
        class="w-full sm:w-72"
        :options="viewers"
        :label="t('admin.mailing.composer.options.seenBy')"
        hide-optional
      />
    </div>
    <p v-if="preview && !preview.has_account" class="text-xs text-text-subtle">
      {{ t('admin.mailing.composer.options.previewNoAccount') }}
    </p>
    <UiAlert v-if="previewError" intent="danger" compact :message="previewError" />
    <iframe
      v-if="preview"
      ref="frame"
      class="h-112 w-full rounded-lg border border-border bg-white xl:h-auto xl:min-h-0 xl:flex-1"
      :title="t('admin.mailing.composer.options.preview')"
      sandbox="allow-same-origin"
      :srcdoc="preview.html"
      @load="restoreScroll"
    />
    <UiLoadingState v-else-if="!previewError" variant="text" :lines="6" />
  </section>
</template>
