<script setup lang="ts">
import type { AdminKnowledgeRef } from '~/types/admin-negotiation-savoir'
import type { Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

const props = withDefaults(
  defineProps<{
    modelValue: Uuid[]
    /** Les entrées que l'on peut lier, l'entrée elle-même exclue. */
    options: SelectOption[]
    /** Les liées rendues par l'API : leur libellé, si la liste ne les porte plus. */
    known?: AdminKnowledgeRef[]
    label: string
    hint?: string
    placeholder?: string
    readonly?: boolean
    error?: string
  }>(),
  { known: () => [], hint: undefined, placeholder: undefined, readonly: false, error: undefined },
)

const emit = defineEmits<{ 'update:modelValue': [value: Uuid[]] }>()

const { t } = useI18n()

const libelles = computed(() => {
  const carte = new Map(props.known.map((r) => [r.id, r.label]))
  for (const option of props.options) carte.set(option.value, option.label)
  return carte
})

const disponibles = computed(() => props.options.filter((o) => !props.modelValue.includes(o.value)))

function ajouter(id: string): void {
  if (id && !props.modelValue.includes(id)) emit('update:modelValue', [...props.modelValue, id])
}

function retirer(id: Uuid): void {
  emit('update:modelValue', props.modelValue.filter((x) => x !== id))
}

function deplacer(index: number, sens: -1 | 1): void {
  const liste = [...props.modelValue]
  const [id] = liste.splice(index, 1)
  if (!id) return
  liste.splice(index + sens, 0, id)
  emit('update:modelValue', liste)
}
</script>

<template>
  <div class="space-y-3">
    <UiCombobox
      v-if="!props.readonly"
      model-value=""
      :label="props.label"
      :hint="props.hint"
      :placeholder="props.placeholder"
      :options="disponibles"
      :error="props.error"
      @update:model-value="ajouter"
    />
    <p v-else class="text-sm font-semibold">{{ props.label }}</p>

    <p v-if="props.modelValue.length === 0" class="text-sm text-text-muted">{{ t('admin-related-field.empty') }}</p>
    <ol v-else class="divide-y divide-border rounded-md border border-border">
      <li v-for="(id, index) in props.modelValue" :key="id" class="flex items-center gap-2 px-3 py-2">
        <span class="min-w-0 grow break-words">{{ libelles.get(id) ?? t('admin-related-field.unknown') }}</span>
        <template v-if="!props.readonly">
          <UiButton
            variant="ghost"
            size="sm"
            icon="chevron-up"
            icon-only
            :label="t('admin-related-field.up')"
            :disabled="index === 0"
            @click="deplacer(index, -1)"
          />
          <UiButton
            variant="ghost"
            size="sm"
            icon="chevron-down"
            icon-only
            :label="t('admin-related-field.down')"
            :disabled="index === props.modelValue.length - 1"
            @click="deplacer(index, 1)"
          />
          <UiButton
            variant="ghost"
            size="sm"
            icon="close"
            icon-only
            :label="t('admin-related-field.remove')"
            @click="retirer(id)"
          />
        </template>
      </li>
    </ol>
  </div>
</template>
