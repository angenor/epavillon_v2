<script setup lang="ts">
import type { CoverDesignOptions } from '~/types/programme/cover-design'
import type { CoverFormat, CoverScene } from '~/utils/cover-studio'

/**
 * LES SORTIES DE L'IMAGE, toujours en vue sous l'aperçu. Le serveur tire
 * `partage` de l'image enregistrée (`media/domain/variants.rs`), la même image
 * en 1200 px. La bande des courriels n'est plus montrée : les courriels prennent
 * leurs visuels aux gabarits du back-office (31/10).
 */

interface Props {
  scene: CoverScene | null
  template: string
  options: CoverDesignOptions
  selected: CoverFormat
}

const props = defineProps<Props>()
const emit = defineEmits<{ select: [format: CoverFormat] }>()

const { t } = useI18n()

const FORMATS: { key: CoverFormat; width: string }[] = [
  { key: 'cover', width: 'w-36' },
  { key: 'share', width: 'w-36' },
]
</script>

<template>
  <div role="group" :aria-label="t('organization.workspace.cover.formats.label')" class="flex flex-wrap items-end justify-center gap-4">
    <button
      v-for="format in FORMATS"
      :key="format.key"
      type="button"
      :aria-pressed="props.selected === format.key"
      class="flex cursor-pointer flex-col gap-1.5 text-left"
      :class="format.width"
      @click="emit('select', format.key)"
    >
      <span
        class="relative block overflow-hidden rounded-md ring-offset-2 ring-offset-surface-sunken"
        :class="props.selected === format.key ? 'ring-2 ring-accent' : 'ring-1 ring-border'"
      >
        <CoverStudioCanvas
          :scene="props.scene"
          :template="props.template"
          :options="props.options"
          :label="t(`organization.workspace.cover.formats.${format.key}`)"
        />
      </span>
      <span class="flex items-baseline gap-1.5">
        <span class="text-sm font-semibold text-text">{{ t(`organization.workspace.cover.formats.${format.key}`) }}</span>
        <span class="text-xs text-text-subtle">{{ t(`organization.workspace.cover.formats.${format.key}Size`) }}</span>
      </span>
    </button>
  </div>
</template>
