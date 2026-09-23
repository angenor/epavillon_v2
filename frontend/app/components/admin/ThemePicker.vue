<script setup lang="ts">
import type { I18nText, TaxonomyTermCode } from '~/types/shared'

/**
 * CHOISIR DES THÉMATIQUES — celles d'une diapositive, d'un fil, d'un document de
 * Guide Négo.
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

export interface PickableTheme {
  code: TaxonomyTermCode
  label: I18nText
  color?: string | null
}

interface Props {
  themes: PickableTheme[]
  modelValue: TaxonomyTermCode[]
  label: string
  hint?: string
  error?: string
  disabled?: boolean
  readonly?: boolean
  /** Libellé de l'aperçu des pastilles ; absent, pas d'aperçu. */
  previewLabel?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: TaxonomyTermCode[]] }>()

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
    <p v-if="props.error" role="alert" class="mt-1 text-sm font-bold text-danger">{{ props.error }}</p>

    <div class="mt-2 grid gap-x-4 sm:grid-cols-2 lg:grid-cols-3">
      <UiCheckbox
        v-for="theme in props.themes"
        :key="theme.code"
        :model-value="selected.has(theme.code)"
        :label="tr(theme.label)"
        :value="theme.code"
        :readonly="props.readonly"
        @update:model-value="(checked: boolean) => toggle(theme.code, checked)"
      />
    </div>

    <!-- Ce que le public verra vraiment, dans l'ordre du référentiel. -->
    <div v-if="props.previewLabel && props.modelValue.length" class="mt-3 flex flex-wrap items-center gap-2">
      <span class="text-xs text-text-subtle">{{ props.previewLabel }}</span>
      <UiThemeTagList
        :themes="props.themes.filter((theme) => selected.has(theme.code))"
        :max="3"
        size="sm"
      />
    </div>
  </fieldset>
</template>
