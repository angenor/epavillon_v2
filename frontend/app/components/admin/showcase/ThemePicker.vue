<script setup lang="ts">
import type { ScheduleThemeBadge } from '~/types/views'
import type { TaxonomyTermCode } from '~/types/shared'

/**
 * LES THÉMATIQUES D'UNE DIAPOSITIVE — `reference.entity_terms`, taxonomie
 * `activity_theme`.
 *
 * VOCABULAIRE OUVERT, VENU DE LA BASE. Libellé et couleur sont des DONNÉES
 * (`reference.taxonomy_terms`), résolues par `tr()` : les figer dans la feuille
 * de style ou dans un fichier i18n est le défaut n° 1 de la v1, et il a coûté
 * des libellés désynchronisés pendant deux COP.
 *
 * DES CASES, PAS UNE LISTE DÉROULANTE MULTIPLE. Un `<select multiple>` natif est
 * invisible à qui ne le connaît pas, impraticable au tactile, et il cache le
 * nombre de choix faits — or c'est justement ce qu'il faut voir ici.
 *
 * TROIS PASTILLES S'AFFICHENT, LES SUIVANTES SE REPLIENT EN « +N ». On n'en
 * INTERDIT pas davantage — la thématique sert aussi à filtrer, et une activité
 * peut légitimement en porter quatre —, mais on le DIT : au-delà de trois, les
 * suivantes cessent d'informer sur une carte.
 */

interface Props {
  themes: ScheduleThemeBadge[]
  modelValue: TaxonomyTermCode[]
  label: string
  hint?: string
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: TaxonomyTermCode[]] }>()

const { t } = useI18n()
const { tr } = useI18nText()

const selected = computed(() => new Set(props.modelValue))

function toggle(code: TaxonomyTermCode, checked: boolean): void {
  const next = new Set(selected.value)
  if (checked) next.add(code)
  else next.delete(code)
  // L'ordre du référentiel, pas celui des clics : c'est lui qui décide de la
  // pastille affichée en premier, et deux saisies doivent donner le même rail.
  emit(
    'update:modelValue',
    props.themes.map((theme) => theme.code).filter((code) => next.has(code)),
  )
}
</script>

<template>
  <fieldset :disabled="props.disabled">
    <legend class="text-sm font-bold text-text">{{ props.label }}</legend>
    <p v-if="props.hint" class="mt-1 max-w-(--measure) text-sm text-text-muted">{{ props.hint }}</p>

    <div class="mt-2 grid gap-x-4 sm:grid-cols-2 lg:grid-cols-3">
      <UiCheckbox
        v-for="theme in props.themes"
        :key="theme.code"
        :model-value="selected.has(theme.code)"
        :label="tr(theme.label)"
        :value="theme.code"
        @update:model-value="(checked: boolean) => toggle(theme.code, checked)"
      />
    </div>

    <!-- Ce que le public verra vraiment, dans l'ordre du référentiel. -->
    <div v-if="props.modelValue.length" class="mt-3 flex flex-wrap items-center gap-2">
      <span class="text-xs text-text-subtle">{{ t('admin.showcase.form.themes.preview') }}</span>
      <UiThemeTagList
        :themes="props.themes.filter((theme) => selected.has(theme.code))"
        :max="3"
        size="sm"
      />
    </div>
  </fieldset>
</template>
