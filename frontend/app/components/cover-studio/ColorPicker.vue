<script setup lang="ts">
/**
 * LA COULEUR LIBRE de l'image (31/10) : teinte, intensité, clarté, ou un code
 * collé. Les titres de l'image sont blancs : la fenêtre dit quand une couleur
 * trop claire les rendrait illisibles, sans l'interdire.
 */

interface Props {
  open: boolean
  /** La couleur de départ, `#rrggbb`. */
  initial: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; apply: [hex: string] }>()

const { t } = useI18n()

const hue = ref(0)
const saturation = ref(0)
const lightness = ref(0)
const typed = ref('')

function start(hex: string): void {
  const hsl = hexToHsl(hex)
  hue.value = hsl.hue
  saturation.value = hsl.saturation
  lightness.value = hsl.lightness
  typed.value = hex.toUpperCase()
}

watch(
  () => props.open,
  (open) => {
    if (open) start(props.initial)
  },
  { immediate: true },
)

const color = computed(() => hslToHex(hue.value, saturation.value, lightness.value))
const readable = computed(() => whiteContrast(color.value) >= 4.5)

function slide(): void {
  typed.value = color.value.toUpperCase()
}

function onTyped(value: string): void {
  typed.value = value.toUpperCase()
  const hex = (value.trim().startsWith('#') ? value.trim() : `#${value.trim()}`).toLowerCase()
  if (CUSTOM_ACCENT.test(hex)) start(hex)
}

// Douze teintes foncées : sur chacune, le texte blanc reste lisible.
const suggestions = Array.from({ length: 12 }, (_, index) => hslToHex(index * 30, 62, 26))

const saturationTrack = computed(
  () => `linear-gradient(to right, ${hslToHex(hue.value, 0, lightness.value)}, ${hslToHex(hue.value, 100, lightness.value)})`,
)
const lightnessTrack = computed(
  () =>
    `linear-gradient(to right, ${hslToHex(hue.value, saturation.value, 5)}, ${hslToHex(hue.value, saturation.value, 50)}, ${hslToHex(hue.value, saturation.value, 95)})`,
)

function apply(): void {
  emit('apply', color.value)
  emit('update:open', false)
}
</script>

<template>
  <UiModal :open="props.open" :title="t('organization.workspace.cover.picker.title')" size="sm" @update:open="emit('update:open', $event)">
    <div class="flex flex-col gap-5">
      <div class="flex items-stretch gap-4">
        <div
          class="flex w-28 shrink-0 flex-col items-center justify-center gap-0.5 rounded-lg"
          :style="{ background: color }"
          aria-hidden="true"
        >
          <span class="text-xl font-bold text-text-on-inverse">Aa</span>
          <span class="text-xs font-semibold tracking-wide text-text-on-inverse uppercase">{{ t('organization.workspace.cover.picker.sample') }}</span>
        </div>
        <div class="flex min-w-0 flex-1 flex-col justify-center gap-2">
          <UiInput
            id="cover-picker-hex"
            :model-value="typed"
            :label="t('organization.workspace.cover.picker.hex')"
            :maxlength="7"
            hide-optional
            @update:model-value="onTyped(String($event))"
          />
          <p class="text-xs font-medium" :class="readable ? 'text-success' : 'text-warning'" role="status">
            {{ t(readable ? 'organization.workspace.cover.picker.readable' : 'organization.workspace.cover.picker.unreadable') }}
          </p>
        </div>
      </div>

      <div class="flex flex-col gap-4">
        <label class="flex flex-col gap-2 text-sm font-semibold">
          {{ t('organization.workspace.cover.picker.hue') }}
          <input
            v-model.number="hue"
            type="range"
            min="0"
            max="360"
            class="cover-slider"
            style="background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)"
            @input="slide"
          />
        </label>
        <label class="flex flex-col gap-2 text-sm font-semibold">
          {{ t('organization.workspace.cover.picker.saturation') }}
          <input v-model.number="saturation" type="range" min="0" max="100" class="cover-slider" :style="{ background: saturationTrack }" @input="slide" />
        </label>
        <label class="flex flex-col gap-2 text-sm font-semibold">
          {{ t('organization.workspace.cover.picker.lightness') }}
          <input v-model.number="lightness" type="range" min="5" max="95" class="cover-slider" :style="{ background: lightnessTrack }" @input="slide" />
        </label>
      </div>

      <fieldset class="flex flex-col gap-2">
        <legend class="mb-2 text-sm font-semibold">{{ t('organization.workspace.cover.picker.suggestions') }}</legend>
        <div class="grid grid-cols-6 gap-2">
          <button
            v-for="hex in suggestions"
            :key="hex"
            type="button"
            :aria-label="hex.toUpperCase()"
            :aria-pressed="hex === color"
            class="h-11 cursor-pointer rounded-md ring-offset-2 ring-offset-surface-raised"
            :class="hex === color ? 'ring-2 ring-text' : ''"
            :style="{ background: hex }"
            @click="start(hex)"
          />
        </div>
      </fieldset>
    </div>

    <template #footer>
      <UiButton variant="secondary" @click="emit('update:open', false)">{{ t('organization.workspace.cover.picker.cancel') }}</UiButton>
      <UiButton variant="primary" @click="apply">{{ t('organization.workspace.cover.picker.apply') }}</UiButton>
    </template>
  </UiModal>
</template>

<style scoped>
.cover-slider {
  appearance: none;
  width: 100%;
  height: 0.875rem;
  margin: 0.75rem 0;
  border-radius: 999px;
  cursor: pointer;
}
.cover-slider::-webkit-slider-thumb {
  appearance: none;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: 50%;
  background: var(--color-surface-raised);
  border: 2px solid var(--color-text);
  box-shadow: var(--shadow-sm);
}
.cover-slider::-moz-range-thumb {
  width: 1.25rem;
  height: 1.25rem;
  border-radius: 50%;
  background: var(--color-surface-raised);
  border: 2px solid var(--color-text);
}
.cover-slider:focus-visible {
  outline: 2px solid var(--color-accent);
  outline-offset: 4px;
}
</style>
