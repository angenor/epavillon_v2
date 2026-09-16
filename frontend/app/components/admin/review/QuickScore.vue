<script setup lang="ts">
interface Props {
  modelValue: number | null
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: number] }>()

const { t } = useI18n()
const inputId = useId()

const passing = computed(() => props.modelValue !== null && props.modelValue >= QUICK_PASS_MARK)

function onInput(event: Event): void {
  emit('update:modelValue', Number((event.target as HTMLInputElement).value))
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <label :for="inputId" class="text-sm font-bold">{{ t('admin.proposal.review.quick.label') }}</label>

    <p class="text-center tabular-nums" aria-live="polite">
      <span class="text-4xl font-semibold" :class="props.modelValue === null ? 'text-text-subtle' : 'text-text'">
        {{ props.modelValue ?? '—' }}
      </span>
      <span class="text-xl text-text-muted"> / 20</span>
    </p>

    <!-- Un curseur jamais touché n'est pas un zéro : il part du milieu, grisé. -->
    <input
      :id="inputId"
      type="range"
      min="0"
      max="20"
      step="0.5"
      :value="props.modelValue ?? QUICK_PASS_MARK"
      :disabled="props.disabled"
      :aria-valuetext="props.modelValue === null ? t('admin.proposal.review.quick.unset') : `${props.modelValue} / 20`"
      class="h-(--target-min) w-full cursor-pointer disabled:cursor-not-allowed"
      :class="props.modelValue === null ? 'accent-(--color-border-strong)' : 'accent-(--color-accent-solid)'"
      @input="onInput"
    >

    <div class="flex justify-between text-xs text-text-subtle tabular-nums" aria-hidden="true">
      <span>0</span><span>5</span><span class="font-semibold text-text-muted">10</span><span>15</span><span>20</span>
    </div>

    <p
      v-if="props.modelValue !== null"
      class="flex flex-wrap items-center justify-between gap-2 rounded-md border px-3 py-2 text-sm font-medium"
      :class="passing ? 'border-success-border bg-success-surface text-success' : 'border-danger-border bg-danger-surface text-danger'"
    >
      <span class="flex items-center gap-1.5">
        <UiIcon :name="passing ? 'check-circle' : 'ban'" size="1.1rem" />
        {{ t(`admin.proposal.review.quick.verdict.${passing ? 'accept' : 'reject'}`) }}
      </span>
      <span>{{ t(`admin.proposal.review.quick.relevance.${quickRelevance(props.modelValue)}`) }}</span>
    </p>
    <p v-else class="text-sm text-text-muted">{{ t('admin.proposal.review.quick.hint') }}</p>
  </div>
</template>
