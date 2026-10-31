<script setup lang="ts">
import type { CoverDesignOptions } from '~/types/programme/cover-design'
import type { CoverScene } from '~/utils/cover-studio'

/** Les modèles de l'atelier, chacun rendu avec les textes et la photo du dossier. */

interface Props {
  scene: CoverScene | null
  options: CoverDesignOptions
  selected: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ select: [code: string] }>()

const { t } = useI18n()

const groups = [
  { key: 'withPhoto', templates: COVER_TEMPLATES.filter((entry) => entry.usesPhoto) },
  { key: 'withoutPhoto', templates: COVER_TEMPLATES.filter((entry) => !entry.usesPhoto) },
]
</script>

<template>
  <div class="flex flex-col gap-6">
    <section v-for="group in groups" :key="group.key" class="flex flex-col gap-3" :aria-labelledby="`cover-templates-${group.key}`">
      <h3 :id="`cover-templates-${group.key}`" class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
        {{ t(`organization.workspace.cover.groups.${group.key}`) }}
      </h3>
      <div class="grid grid-cols-2 gap-3">
        <button
          v-for="entry in group.templates"
          :key="entry.code"
          type="button"
          :aria-pressed="props.selected === entry.code"
          class="flex cursor-pointer flex-col gap-1.5 rounded-lg border-2 p-1 text-left transition-colors duration-(--duration-fast)"
          :class="props.selected === entry.code ? 'border-accent' : 'border-transparent hover:border-border-strong'"
          @click="emit('select', entry.code)"
        >
          <CoverStudioCanvas
            :scene="props.scene"
            :template="entry.code"
            :options="props.options"
            :label="t(`organization.workspace.cover.templates.${entry.code}`)"
            class="rounded"
          />
          <span class="px-0.5 text-sm font-medium text-text">{{ t(`organization.workspace.cover.templates.${entry.code}`) }}</span>
        </button>
      </div>
    </section>
    <p class="text-sm text-text-muted">{{ t('organization.workspace.cover.catalogHint') }}</p>
  </div>
</template>
