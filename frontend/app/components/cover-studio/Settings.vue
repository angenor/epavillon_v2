<script setup lang="ts">
import type { CoverDesignOptions } from '~/types/programme/cover-design'

/**
 * CE QUI FIGURE SUR L'IMAGE et SA COULEUR, la colonne de droite de l'atelier.
 * Les logos de l'IFDD et de l'OIF se règlent à part de celui de l'organisation
 * (31/10) : elle peut ne montrer que le sien. Une couleur libre s'ajoute aux
 * cinq de la charte par la fenêtre de choix.
 */

type OptionKey = Exclude<keyof CoverDesignOptions, 'show_event_banner'>

interface Props {
  /** Précision sous chaque case. Une clé absente : la case n'est pas proposée. */
  hints: Partial<Record<OptionKey, string>>
  /** Code d'une couleur de la charte, ou `#rrggbb`. */
  accent: string
  /** La couleur libre déjà choisie, gardée pour y revenir. */
  customAccent: string | null
}

const props = defineProps<Props>()
const options = defineModel<CoverDesignOptions>('options', { required: true })
const emit = defineEmits<{ 'update:accent': [value: string]; openPicker: [] }>()

const { t } = useI18n()

const ORDER: OptionKey[] = ['show_date', 'show_speakers', 'show_partner_logos', 'show_org_logo', 'show_event_icon']
const shown = computed(() => ORDER.filter((key) => props.hints[key] !== undefined))
const accents = Object.entries(COVER_ACCENTS)

const accentName = computed(() =>
  CUSTOM_ACCENT.test(props.accent)
    ? t('organization.workspace.cover.customColor', { hex: props.accent.toUpperCase() })
    : t(`organization.workspace.cover.accents.${props.accent}`),
)

function toggle(key: OptionKey, value: boolean): void {
  options.value = { ...options.value, [key]: value }
}

const swatch = (selected: boolean) =>
  `size-11 cursor-pointer rounded-full border-4 border-surface-raised ring-2 transition-shadow duration-(--duration-fast) ${
    selected ? 'ring-text' : 'ring-transparent hover:ring-border-strong'
  }`
</script>

<template>
  <div class="flex flex-col">
    <div class="border-b border-border-subtle px-5 py-5">
      <fieldset>
        <legend class="mb-3 font-semibold text-heading">{{ t('organization.workspace.cover.onImage') }}</legend>
        <div class="flex flex-col gap-3">
          <UiCheckbox
            v-for="key in shown"
            :id="`cover-${key}`"
            :key="key"
            :model-value="options[key]"
            :label="t(`organization.workspace.cover.options.${key}`)"
            :hint="props.hints[key]"
            @update:model-value="toggle(key, $event)"
          />
        </div>
      </fieldset>
    </div>

    <div class="px-5 py-5">
      <fieldset>
        <legend class="mb-3 font-semibold text-heading">{{ t('organization.workspace.cover.color') }}</legend>
        <p class="-mt-2 mb-3 text-sm text-text-secondary">{{ accentName }}</p>
        <div class="flex flex-wrap gap-2.5">
        <button
          v-for="[code, token] in accents"
          :key="code"
          type="button"
          :aria-pressed="props.accent === code"
          :aria-label="t(`organization.workspace.cover.accents.${code}`)"
          :class="swatch(props.accent === code)"
          :style="{ background: `var(${token})` }"
          @click="emit('update:accent', code)"
        />
        <button
          v-if="props.customAccent"
          type="button"
          :aria-pressed="props.accent === props.customAccent"
          :aria-label="t('organization.workspace.cover.customColor', { hex: props.customAccent.toUpperCase() })"
          :class="swatch(props.accent === props.customAccent)"
          :style="{ background: props.customAccent }"
          @click="emit('update:accent', props.customAccent)"
        />
        <button
          type="button"
          :aria-label="t('organization.workspace.cover.addColor')"
          class="grid size-11 cursor-pointer place-items-center rounded-full border-2 border-dashed border-border-strong text-text-secondary hover:border-accent hover:text-accent"
          @click="emit('openPicker')"
        >
            <UiIcon name="plus" size="1.125rem" />
          </button>
        </div>
      </fieldset>
    </div>
  </div>
</template>
