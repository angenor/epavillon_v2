<script setup lang="ts">
import type { CommentPerson } from '~/types/programme/proposal'
import type { SelectOption } from '~/types/ui'
import type { PersonId } from '~/types/shared'

/**
 * LE CHAMP « À » d'un fil du comité. Vide, il vaut « Tout le comité », et c'est
 * écrit : un champ vide laisserait croire que le message ne part chez personne.
 * Les personnes retenues s'affichent en jetons retirables ; la recherche
 * n'offre que celles qui ne le sont pas encore.
 */

interface Props {
  modelValue: PersonId[]
  /** Membres du comité, soi-même exclu. */
  audience: CommentPerson[]
  label: string
  searchLabel: string
  hint?: string
  disabled?: boolean
  /** Sans le jeton « Tout le comité » : on ajoute à un cercle déjà formé. */
  hideEveryone?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: PersonId[]] }>()

const { t } = useI18n()
const legendId = useId()

const chosen = computed(() =>
  props.modelValue.map(
    (id) => props.audience.find((person) => person.person_id === id) ?? { person_id: id, display_name: '' },
  ),
)

const options = computed<SelectOption[]>(() =>
  props.audience
    .filter((person) => !props.modelValue.includes(person.person_id))
    .map((person) => ({ value: person.person_id, label: person.display_name })),
)

const announcement = ref('')

function add(personId: string): void {
  if (!personId || props.modelValue.includes(personId)) return
  emit('update:modelValue', [...props.modelValue, personId])
  const name = props.audience.find((person) => person.person_id === personId)?.display_name ?? ''
  announcement.value = t('admin.proposal.review.comments.recipients.added', { name })
}

function remove(personId: PersonId): void {
  emit('update:modelValue', props.modelValue.filter((id) => id !== personId))
}
</script>

<template>
  <fieldset :aria-labelledby="legendId" class="min-w-0">
    <legend :id="legendId" class="text-sm font-semibold text-text">{{ props.label }}</legend>

    <ul class="mt-2 flex flex-wrap items-center gap-2" :aria-label="props.label">
      <li v-if="chosen.length === 0 && !props.hideEveryone">
        <UiChip :label="t('admin.proposal.review.comments.recipients.everyone')" fixed />
      </li>
      <li v-for="person in chosen" :key="person.person_id">
        <UiChip :label="person.display_name" :disabled="props.disabled" @remove="remove(person.person_id)" />
      </li>
    </ul>

    <UiCombobox
      class="mt-3 max-w-md"
      :model-value="null"
      :options="options"
      :label="props.searchLabel"
      :hint="props.hint"
      :placeholder="t('admin.proposal.review.comments.recipients.placeholder')"
      :disabled="props.disabled || options.length === 0"
      hide-optional
      @update:model-value="add"
    />

    <p aria-live="polite" class="sr-only">{{ announcement }}</p>
  </fieldset>
</template>
