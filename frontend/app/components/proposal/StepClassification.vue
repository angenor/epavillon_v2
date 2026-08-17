<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { ParticipationMode } from '~/types/event/edition'
import type { Locale, TaxonomyTerm } from '~/types/reference'
import type { DraftIssue, ProposalDraft } from '~/types/proposal-form'
import type { SelectOption } from '~/types/ui'

/**
 * ÉTAPE 3 — CLASSER L'ACTIVITÉ : thématiques, catégorie, format, langues, pays.
 *
 * TOUT CE QUI EST PROPOSÉ ICI VIENT DE LA BASE, sans exception. Les thématiques
 * et les catégories sont des lignes de `reference.taxonomy_terms`, avec leur
 * libellé traduit et leur couleur ; les langues viennent de `reference.locales`,
 * les pays de `reference.countries`. Aucun de ces libellés n'a le droit
 * d'apparaître dans un fichier i18n — c'est le défaut n°1 de la v1, dont
 * l'ENUM `activity_theme` à quinze valeurs et ses libellés recopiés dans le
 * frontend ont fini désynchronisés.
 *
 * LES FORMATS PROPOSÉS SONT CEUX DE L'APPEL (`calls_for_proposals.allowed_formats`),
 * pas les trois valeurs de l'ENUM. Un appel qui n'accepte que le présentiel ne
 * doit pas offrir « en ligne » pour le refuser ensuite.
 *
 * TROIS THÉMATIQUES SUFFISENT. Le guide de style l'impose côté affichage — au-delà
 * de trois pastilles, une carte cesse d'informer — et c'est aussi vrai du fond :
 * une activité qui coche huit thématiques n'en traite aucune. La quatrième n'est
 * pas refusée, elle est signalée : c'est un conseil de rédaction, pas une règle
 * de la base.
 */

const draft = defineModel<ProposalDraft>({ required: true })

interface Props {
  call: CallForProposals
  themes: TaxonomyTerm[]
  categories: TaxonomyTerm[]
  locales: Locale[]
  /** Options de pays, déjà triées dans la langue affichée. */
  countryOptions: SelectOption[]
  issues: DraftIssue[]
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

/** Au-delà, la carte de programmation replie en « +N » — et le dossier se dilue. */
const RECOMMENDED_THEMES = 3

function errorOf(field: string): string | undefined {
  const issue = props.issues.find((entry) => entry.field === field && entry.severity === 'error')
  return issue ? t(issue.messageKey, issue.params ?? {}) : undefined
}

function warningOf(field: string): string | undefined {
  const issue = props.issues.find((entry) => entry.field === field && entry.severity === 'warning')
  return issue ? t(issue.messageKey) : undefined
}

// ---------------------------------------------------------------------------
// Thématiques
// ---------------------------------------------------------------------------

function isThemeSelected(code: string): boolean {
  return draft.value.theme_codes.includes(code)
}

function toggleTheme(code: string, selected: boolean): void {
  draft.value.theme_codes = selected
    ? [...draft.value.theme_codes, code]
    : draft.value.theme_codes.filter((entry) => entry !== code)
}

const themeCount = computed(() => draft.value.theme_codes.length)

// ---------------------------------------------------------------------------
// Catégorie, format, langues
// ---------------------------------------------------------------------------

const categoryOptions = computed<SelectOption[]>(() =>
  props.categories.map((term) => ({ value: term.code, label: tr(term.label) })),
)

/** Les formats OUVERTS PAR L'APPEL, dans l'ordre de l'ENUM. */
const formatOptions = computed<SelectOption[]>(() =>
  props.call.allowed_formats.map((mode) => ({
    value: mode,
    label: t(`proposal.form.step-classification.formats.${mode}.label`),
    description: t(`proposal.form.step-classification.formats.${mode}.hint`),
  })),
)

function isLanguageSelected(code: string): boolean {
  return draft.value.language_codes.includes(code)
}

function toggleLanguage(code: string, selected: boolean): void {
  draft.value.language_codes = selected
    ? [...draft.value.language_codes, code]
    : draft.value.language_codes.filter((entry) => entry !== code)
}
</script>

<template>
  <div class="grid gap-8">
    <!-- THÉMATIQUES -->
    <section class="grid gap-3">
      <header>
        <h2 class="font-display text-xl text-text">
          {{ t('proposal.form.step-classification.themes.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('proposal.form.step-classification.themes.description') }}
        </p>
      </header>

      <p
        class="text-sm"
        :class="themeCount > RECOMMENDED_THEMES ? 'text-warning' : 'text-text-muted'"
        role="status"
      >
        {{ t('proposal.form.step-classification.themes.selected', { count: themeCount }, themeCount) }}
        <span v-if="themeCount > RECOMMENDED_THEMES">
          — {{ t('proposal.form.step-classification.themes.tooMany', { max: RECOMMENDED_THEMES }) }}
        </span>
      </p>

      <p v-if="warningOf('theme_codes')" class="text-sm text-warning">
        {{ warningOf('theme_codes') }}
      </p>

      <ul class="grid gap-2 sm:grid-cols-2">
        <li v-for="term in props.themes" :key="term.code">
          <label
            class="flex min-h-(--target-min) cursor-pointer items-center gap-3 rounded-md border px-3 py-2 transition-colors duration-(--duration-fast)"
            :class="
              isThemeSelected(term.code)
                ? 'border-accent bg-accent-surface'
                : 'border-border hover:bg-surface-hover'
            "
          >
            <UiCheckbox
              :model-value="isThemeSelected(term.code)"
              @update:model-value="toggleTheme(term.code, $event)"
            />
            <span class="flex min-w-0 items-center gap-2">
              <!-- La couleur vient de la base : point coloré, jamais fond de texte. -->
              <span
                v-if="term.color_hex"
                class="size-2.5 shrink-0 rounded-full"
                :style="{ backgroundColor: term.color_hex }"
                aria-hidden="true"
              />
              <span class="text-sm text-text">{{ tr(term.label) }}</span>
            </span>
          </label>
        </li>
      </ul>
    </section>

    <!-- CATÉGORIE, FORMAT, LANGUES, PAYS -->
    <section class="grid gap-6 border-t border-border pt-8">
      <UiSelect
        v-model="draft.activity_type_code"
        :options="categoryOptions"
        :label="t('proposal.form.step-classification.category.label')"
        :hint="warningOf('activity_type_code') ?? t('proposal.form.step-classification.category.hint')"
        :placeholder="t('proposal.form.step-classification.category.placeholder')"
      />

      <!-- `:model-value` et non `v-model` : le composant émet une chaîne, la
           colonne attend l'ENUM `event.participation_mode`. La conversion est
           sûre — les options sortent de `call.allowed_formats`. -->
      <UiRadio
        :model-value="draft.format"
        :options="formatOptions"
        :label="t('proposal.form.step-classification.format.label')"
        :hint="t('proposal.form.step-classification.format.hint')"
        :error="errorOf('format')"
        required
        @update:model-value="draft.format = $event as ParticipationMode"
      />

      <fieldset>
        <legend class="mb-1.5 text-sm font-bold text-text">
          {{ t('proposal.form.step-classification.languages.label') }}
          <span class="ml-0.5 text-danger" aria-hidden="true">*</span>
        </legend>
        <p class="mb-2 max-w-(--measure) text-sm text-text-muted">
          {{ t('proposal.form.step-classification.languages.hint') }}
        </p>
        <div class="flex flex-wrap gap-4">
          <UiCheckbox
            v-for="locale in props.locales"
            :key="locale.code"
            :model-value="isLanguageSelected(locale.code)"
            :label="locale.native_label"
            @update:model-value="toggleLanguage(locale.code, $event)"
          />
        </div>
        <p v-if="errorOf('language_codes')" role="alert" class="mt-1.5 text-sm font-bold text-danger">
          {{ errorOf('language_codes') }}
        </p>
      </fieldset>

      <UiSelect
        v-model="draft.country_id"
        :options="props.countryOptions"
        :label="t('proposal.form.step-classification.country.label')"
        :hint="t('proposal.form.step-classification.country.hint')"
        :placeholder="t('proposal.form.step-classification.country.placeholder')"
      />
    </section>
  </div>
</template>
