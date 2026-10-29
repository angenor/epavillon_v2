<script setup lang="ts">
import type { MailingAddressSuggestion, MailingCriterion } from '~/types/admin-mailings'
import type { ShowcaseSessionOption } from '~/types/admin-showcase'
import type { Uuid } from '~/types/shared'

/**
 * UN CHAMP D'ADRESSES À PUCES, comme dans une messagerie : on tape ou on colle
 * des adresses (virgule, point-virgule, espace ou Entrée les séparent), chacune
 * devient une puce ; une adresse invalide reste, en rouge, jusqu'à correction.
 *
 * Avec `groups` (le champ « À »), le bouton « + Groupe » ajoute des puces de
 * critères (déposants, intervenants, inscrits, équipe), choisis dans le même
 * éditeur que l'ancienne étape Destinataires.
 *
 * Dès deux caractères, des personnes connues de la plateforme sont proposées
 * (nom et adresse, huit au plus).
 */
interface Props {
  label: string
  addresses: string[]
  groups?: MailingCriterion[] | null
  sessions?: ShowcaseSessionOption[]
  disabled?: boolean
  /** L'édition de l'envoi, qui garde la recherche de personnes ; à défaut, celle du back-office. */
  eventId?: Uuid | null
}

const props = withDefaults(defineProps<Props>(), { groups: null, sessions: () => [], disabled: false, eventId: null })
const emit = defineEmits<{
  'update:addresses': [value: string[]]
  'update:groups': [value: MailingCriterion[]]
}>()

const { t } = useI18n()
const api = useApi()
const adminScope = useAdminScopeStore()
const inputId = useId()
const listId = useId()
const draft = ref('')
const groupsOpen = ref(false)

// Saisie semi-automatique : des personnes connues, mais une adresse inconnue
// reste permise (on écrit volontiers à des non-inscrits).
const suggestions = ref<MailingAddressSuggestion[]>([])
const highlighted = ref(-1)
const listOpen = computed(() => suggestions.value.length > 0)
let timer: ReturnType<typeof setTimeout> | null = null
let lastRequest = 0

function closeList(): void {
  if (timer) clearTimeout(timer)
  lastRequest += 1
  suggestions.value = []
  highlighted.value = -1
}

watch(draft, (value) => {
  if (timer) clearTimeout(timer)
  const query = value.trim()
  const eventId = props.eventId ?? adminScope.currentEventId
  if (query.length < 2 || !eventId || props.disabled) {
    closeList()
    return
  }
  timer = setTimeout(async () => {
    const request = ++lastRequest
    try {
      const found = await api.adminMailings.addressSuggestions(eventId, query)
      if (request !== lastRequest) return
      suggestions.value = found.filter((s) => !props.addresses.includes(s.email))
      highlighted.value = -1
    } catch {
      // Sans suggestions, la saisie libre suffit : rien à signaler.
      if (request === lastRequest) suggestions.value = []
    }
  }, 250)
})

onBeforeUnmount(closeList)

function optionId(index: number): string {
  return `${listId}-${index}`
}

function choose(suggestion: MailingAddressSuggestion): void {
  closeList()
  draft.value = ''
  if (!props.addresses.includes(suggestion.email)) emit('update:addresses', [...props.addresses, suggestion.email])
}

function commit(): void {
  closeList()
  const added = parseAddressList(draft.value).filter((a) => !props.addresses.includes(a))
  draft.value = ''
  if (added.length) emit('update:addresses', [...props.addresses, ...added])
}

/** Flèches, Entrée et Échap pilotent la liste quand elle est ouverte. */
function onListKey(event: KeyboardEvent): boolean {
  if (!listOpen.value) return false
  const total = suggestions.value.length
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault()
    const step = event.key === 'ArrowDown' ? 1 : -1
    highlighted.value = (highlighted.value + step + total) % total
    return true
  }
  if (event.key === 'Enter' && highlighted.value >= 0) {
    event.preventDefault()
    const picked = suggestions.value[highlighted.value]
    if (picked) choose(picked)
    return true
  }
  if (event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    closeList()
    return true
  }
  return false
}

function onKeydown(event: KeyboardEvent): void {
  if (onListKey(event)) return
  if ([',', ';', ' ', 'Enter', 'Tab'].includes(event.key) && draft.value.trim()) {
    if (event.key !== 'Tab') event.preventDefault()
    commit()
  } else if (event.key === 'Backspace' && !draft.value && props.addresses.length) {
    emit('update:addresses', props.addresses.slice(0, -1))
  }
}

function onPaste(event: ClipboardEvent): void {
  const text = event.clipboardData?.getData('text') ?? ''
  if (!/[\s,;]/.test(text)) return
  event.preventDefault()
  draft.value += text
  commit()
}

function removeAddress(address: string): void {
  emit('update:addresses', props.addresses.filter((a) => a !== address))
}

function removeGroup(kind: string): void {
  emit('update:groups', (props.groups ?? []).filter((g) => g.kind !== kind))
}

function groupLabel(group: MailingCriterion): string {
  const filters = group.proposal_statuses.length + group.registration_statuses.length + group.role_codes.length
  const base = t(`admin.mailing.common.kind.${group.kind}`)
  return filters || group.session_id ? t('admin.mailing.composer.groupFiltered', { name: base }) : base
}
</script>

<template>
  <div class="relative border-b border-border">
    <div class="flex flex-wrap items-center gap-1.5 py-1.5">
      <label :for="inputId" class="mr-1 w-12 shrink-0 text-sm text-text-subtle">{{ props.label }}</label>

      <span
        v-for="group in props.groups ?? []"
        :key="group.kind"
        class="inline-flex items-center gap-1 rounded-full border border-accent bg-accent-surface py-0.5 pr-0.5 pl-2.5 text-sm text-accent"
      >
        <button type="button" class="cursor-pointer" @click="groupsOpen = true">
          <UiIcon name="users" size="0.9rem" class="mr-1 inline" />{{ groupLabel(group) }}
        </button>
        <button
          type="button"
          class="flex size-6 cursor-pointer items-center justify-center rounded-full hover:bg-surface-hover"
          :aria-label="t('admin.mailing.composer.remove', { name: groupLabel(group) })"
          :disabled="props.disabled"
          @click="removeGroup(group.kind)"
        >
          <UiIcon name="close" size="0.8rem" />
        </button>
      </span>

      <span
        v-for="address in props.addresses"
        :key="address"
        class="inline-flex max-w-full items-center gap-1 rounded-full border py-0.5 pr-0.5 pl-2.5 text-sm"
        :class="isEmailAddress(address) ? 'border-border bg-surface-sunken text-text' : 'border-danger bg-danger-surface text-danger'"
      >
        <span class="min-w-0 [overflow-wrap:anywhere]">{{ address }}</span>
        <button
          type="button"
          class="flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-full hover:bg-surface-hover"
          :aria-label="t('admin.mailing.composer.remove', { name: address })"
          :disabled="props.disabled"
          @click="removeAddress(address)"
        >
          <UiIcon name="close" size="0.8rem" />
        </button>
      </span>

      <input
        :id="inputId"
        v-model="draft"
        type="text"
        inputmode="email"
        autocomplete="off"
        spellcheck="false"
        role="combobox"
        aria-autocomplete="list"
        :aria-expanded="listOpen"
        :aria-controls="listId"
        :aria-activedescendant="listOpen && highlighted >= 0 ? optionId(highlighted) : undefined"
        class="min-h-(--target-min) min-w-40 flex-1 bg-transparent text-sm text-text outline-none placeholder:text-text-subtle"
        :placeholder="props.addresses.length || props.groups?.length ? '' : t('admin.mailing.composer.addressPlaceholder')"
        :disabled="props.disabled"
        @keydown="onKeydown"
        @paste="onPaste"
        @blur="commit"
      >

      <UiButton
        v-if="props.groups"
        variant="ghost"
        size="sm"
        icon="plus"
        :aria-expanded="groupsOpen"
        :disabled="props.disabled"
        @click="groupsOpen = !groupsOpen"
      >
        {{ t('admin.mailing.composer.group') }}
      </UiButton>
      <slot name="end" />
    </div>

    <ul
      v-show="listOpen"
      :id="listId"
      role="listbox"
      :aria-label="t('admin-mailing-address-field.suggestions')"
      class="absolute inset-x-0 top-full z-30 mt-1 max-h-72 overflow-y-auto rounded-md border border-border-strong bg-surface-raised py-1 shadow-lg"
    >
      <li
        v-for="(suggestion, index) in suggestions"
        :id="optionId(index)"
        :key="suggestion.email"
        role="option"
        :aria-selected="index === highlighted"
        class="flex min-h-(--target-min) cursor-pointer flex-col justify-center px-3 py-1.5 text-sm"
        :class="index === highlighted ? 'bg-accent-surface' : ''"
        @mousedown.prevent="choose(suggestion)"
        @mouseenter="highlighted = index"
      >
        <span class="truncate font-semibold text-text">{{ suggestion.name }}</span>
        <span class="truncate text-xs text-text-muted">{{ suggestion.email }}</span>
      </li>
    </ul>
    <p aria-live="polite" class="sr-only">
      {{ listOpen ? t('admin-mailing-address-field.results', suggestions.length) : '' }}
    </p>

    <div v-if="props.groups && groupsOpen" class="mb-3 rounded-lg border border-border bg-surface-sunken p-3">
      <AdminMailingsAudienceEditor
        :audience="props.groups"
        extra-text=""
        :sessions="props.sessions"
        :show-extra="false"
        :disabled="props.disabled"
        @update:audience="emit('update:groups', $event)"
      />
      <div class="mt-3 flex justify-end">
        <UiButton variant="secondary" size="sm" @click="groupsOpen = false">
          {{ t('admin.mailing.composer.groupDone') }}
        </UiButton>
      </div>
    </div>
  </div>
</template>
