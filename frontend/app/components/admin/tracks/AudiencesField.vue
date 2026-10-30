<script setup lang="ts">
import type { I18nText } from '~/types/shared'

/**
 * Les publics cibles d'un fil : un public par ligne, en français et en anglais.
 * Texte libre comme sur un dossier (`target_audiences`), pas une taxonomie.
 */

const props = defineProps<{
  modelValue: I18nText[]
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: I18nText[]] }>()

const { t } = useI18n()

function update(index: number, locale: 'fr' | 'en', value: string): void {
  emit(
    'update:modelValue',
    props.modelValue.map((entry, i) => {
      if (i !== index) return entry
      const next: I18nText = { ...entry, [locale]: value }
      if (locale === 'en' && !value.trim()) delete next.en
      return next
    }),
  )
}

function add(): void {
  emit('update:modelValue', [...props.modelValue, { fr: '' }])
}

function remove(index: number): void {
  emit('update:modelValue', props.modelValue.filter((_, i) => i !== index))
}
</script>

<template>
  <fieldset>
    <legend class="text-sm font-bold text-text">{{ t('admin.event.track.audiences.label') }}</legend>
    <p class="mt-1 text-xs text-text-muted">{{ t('admin.event.track.audiences.hint') }}</p>

    <ol v-if="props.modelValue.length" class="mt-3 space-y-3">
      <li
        v-for="(audience, index) in props.modelValue"
        :key="index"
        class="flex flex-col gap-2 border-b border-border-subtle pb-3 last:border-b-0 sm:flex-row sm:items-start"
      >
        <div class="grid min-w-0 flex-1 gap-2 sm:grid-cols-2">
          <UiInput
            :model-value="audience.fr"
            :label="t('admin.event.track.audiences.fr', { n: index + 1 })"
            :maxlength="120"
            :disabled="props.disabled"
            :error="!audience.fr.trim() && audience.en ? t('admin.event.track.audiences.frRequired') : undefined"
            @update:model-value="(next: string) => update(index, 'fr', next)"
          />
          <UiInput
            :model-value="audience.en ?? ''"
            :label="t('admin.event.track.audiences.en', { n: index + 1 })"
            :maxlength="120"
            :disabled="props.disabled"
            @update:model-value="(next: string) => update(index, 'en', next)"
          />
        </div>
        <div class="flex items-center gap-1 sm:mt-6">
          <UiMoveButtons
            :item="audience.fr || t('admin.event.track.audiences.fr', { n: index + 1 })"
            :first="index === 0"
            :last="index === props.modelValue.length - 1"
            :disabled="props.disabled"
            @up="emit('update:modelValue', moved(props.modelValue, index, -1))"
            @down="emit('update:modelValue', moved(props.modelValue, index, 1))"
          />
          <UiButton
            variant="ghost"
            icon="trash"
            icon-only
            :label="t('admin.event.track.audiences.remove', { audience: audience.fr || index + 1 })"
            :disabled="props.disabled"
            @click="remove(index)"
          />
        </div>
      </li>
    </ol>

    <UiButton class="mt-3" variant="secondary" icon="plus" :disabled="props.disabled" @click="add">
      {{ t('admin.event.track.audiences.add') }}
    </UiButton>
  </fieldset>
</template>
