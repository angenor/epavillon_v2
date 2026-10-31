<script setup lang="ts">
import type { MailText } from '~/types/admin-mailings'
import type { Uuid } from '~/types/shared'

/**
 * LES TEXTES TYPES DE LA FENÊTRE DE RÉDACTION (01/11), sous les gabarits et sur
 * leur motif : une bande qui défile de côté, rangée par catégorie, nom seul ;
 * « Étendre » ajoute l'objet. Choisir un texte remplace l'objet et le message,
 * après confirmation si le message a été écrit ou modifié depuis.
 */

interface Props {
  eventId: Uuid
  /** Le texte type retenu ; chaîne vide : aucun. */
  chosenId: string
  /** Le message tel qu'il est : il dit si le remplacer perdrait quelque chose. */
  body: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ apply: [text: MailText] }>()

const { t } = useI18n()
const api = useApi()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()
const titleId = useId()

const expanded = ref(false)
const texts = ref<MailText[]>([])
const loadError = ref<string | null>(null)
const pending = ref<MailText | null>(null)

watch(
  () => props.eventId,
  async (eventId) => {
    loadError.value = null
    try {
      texts.value = await api.adminMailings.texts.list(eventId, adminScope.scope)
    } catch {
      texts.value = []
      loadError.value = t('admin-mailing-text-strip.error')
    }
  },
  { immediate: true },
)

interface Group {
  label: string
  texts: MailText[]
}

const groups = computed<Group[]>(() => {
  const byLabel = new Map<string, MailText[]>()
  for (const text of texts.value) {
    const label = text.category ?? t('admin-mailing-text-strip.uncategorized')
    byLabel.set(label, [...(byLabel.get(label) ?? []), text])
  }
  return [...byLabel].map(([label, list]) => ({ label, texts: list }))
})

/** Écrit ou modifié depuis le dernier texte type choisi : le remplacer perdrait ce travail. */
const bodyChanged = computed(() => {
  if (!richTextToPlain(props.body).trim()) return false
  const chosen = texts.value.find((text) => text.id === props.chosenId)
  return props.body !== (chosen?.body_html ?? '')
})

function choose(text: MailText): void {
  if (text.id === props.chosenId && !bodyChanged.value) return
  if (bodyChanged.value) {
    pending.value = text
    return
  }
  emit('apply', text)
}

function confirmReplace(): void {
  if (pending.value) emit('apply', pending.value)
  pending.value = null
}

const usesProposal = (text: MailText) =>
  citedVariables(`${text.subject} ${text.body_html}`).some((name) => MAILING_PROPOSAL_VARIABLES.includes(name))

const cardClass = (chosen: boolean) => [
  expanded.value ? 'w-64 items-start' : 'items-center',
  chosen ? 'border-accent bg-accent-surface' : 'border-border bg-surface-raised hover:border-border-strong',
]
</script>

<template>
  <section v-if="texts.length || loadError" :aria-labelledby="titleId">
    <div class="flex flex-wrap items-center justify-between gap-x-4">
      <h3 :id="titleId" class="text-sm font-semibold text-text">{{ t('admin-mailing-text-strip.title') }}</h3>
      <div class="flex items-center">
        <button
          type="button"
          class="inline-flex min-h-(--target-min) cursor-pointer items-center gap-1 px-2 text-sm text-text-link hover:text-text-link-hover"
          :aria-expanded="expanded"
          @click="expanded = !expanded"
        >
          <UiIcon :name="expanded ? 'chevron-up' : 'chevron-down'" size="1rem" />
          {{ expanded ? t('admin.mailing.composer.options.reduce') : t('admin.mailing.composer.options.expand') }}
        </button>
        <NuxtLink
          :to="localePath('/admin/courriels/textes')"
          class="inline-flex min-h-(--target-min) items-center px-2 text-sm text-text-link underline-offset-2 hover:underline"
        >
          {{ t('admin-mailing-text-strip.manage') }}
        </NuxtLink>
      </div>
    </div>

    <div
      role="radiogroup"
      :aria-labelledby="titleId"
      class="relative flex gap-4 overflow-x-auto overscroll-x-contain px-0.5 pt-0.5 pb-2"
    >
      <div
        v-for="(group, index) in groups"
        :key="group.label"
        role="group"
        :aria-label="group.label"
        class="flex shrink-0 flex-col gap-1"
        :class="index > 0 ? 'border-l border-border pl-4' : ''"
      >
        <p class="text-xs font-semibold text-text-muted">{{ group.label }}</p>
        <div class="flex gap-2">
          <label
            v-for="text in group.texts"
            :key="text.id"
            class="flex min-h-(--target-min) shrink-0 cursor-pointer gap-2.5 rounded-lg border px-2.5 py-2 transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-focus"
            :class="cardClass(text.id === props.chosenId)"
          >
            <input
              type="radio"
              name="mailing-text"
              class="sr-only"
              :value="text.id"
              :checked="text.id === props.chosenId"
              @change="choose(text)"
            >
            <span class="min-w-0">
              <span class="block text-sm font-semibold" :class="expanded ? 'text-balance' : 'whitespace-nowrap'">
                {{ text.name }}
              </span>
              <span v-if="expanded" class="mt-0.5 line-clamp-2 block text-xs text-text-subtle">{{ text.subject }}</span>
              <span v-if="expanded && usesProposal(text)" class="mt-0.5 block text-xs text-text-muted">
                {{ t('admin-mailing-text-strip.usesProposal') }}
              </span>
            </span>
          </label>
        </div>
      </div>
    </div>

    <div v-if="pending" class="mt-1 space-y-2" role="alert">
      <UiAlert intent="warning" compact :message="t('admin-mailing-text-strip.confirm', { name: pending.name })" />
      <div class="flex flex-wrap gap-2">
        <UiButton @click="confirmReplace">{{ t('admin-mailing-text-strip.replace') }}</UiButton>
        <UiButton variant="secondary" @click="pending = null">{{ t('admin-mailing-text-strip.keep') }}</UiButton>
      </div>
    </div>
    <UiAlert v-if="loadError" class="mt-1" intent="warning" compact :message="loadError" />
  </section>
</template>
