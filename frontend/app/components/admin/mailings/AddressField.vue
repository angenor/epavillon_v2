<script setup lang="ts">
import type { MailingCriterion } from '~/types/admin-mailings'
import type { ShowcaseSessionOption } from '~/types/admin-showcase'

/**
 * UN CHAMP D'ADRESSES À PUCES, comme dans une messagerie : on tape ou on colle
 * des adresses (virgule, point-virgule, espace ou Entrée les séparent), chacune
 * devient une puce ; une adresse invalide reste, en rouge, jusqu'à correction.
 *
 * Avec `groups` (le champ « À »), le bouton « + Groupe » ajoute des puces de
 * critères (déposants, intervenants, inscrits, équipe), choisis dans le même
 * éditeur que l'ancienne étape Destinataires.
 */
interface Props {
  label: string
  addresses: string[]
  groups?: MailingCriterion[] | null
  sessions?: ShowcaseSessionOption[]
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), { groups: null, sessions: () => [], disabled: false })
const emit = defineEmits<{
  'update:addresses': [value: string[]]
  'update:groups': [value: MailingCriterion[]]
}>()

const { t } = useI18n()
const inputId = useId()
const draft = ref('')
const groupsOpen = ref(false)

function commit(): void {
  const added = parseAddressList(draft.value).filter((a) => !props.addresses.includes(a))
  draft.value = ''
  if (added.length) emit('update:addresses', [...props.addresses, ...added])
}

function onKeydown(event: KeyboardEvent): void {
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
  <div class="border-b border-border">
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
