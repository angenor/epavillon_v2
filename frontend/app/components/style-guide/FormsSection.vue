<script setup lang="ts">
import type { SelectOption } from '~/types/ui'

/**
 * Section « Formulaires » — les sept états exigés de chaque champ : vide,
 * rempli, focus, erreur avec message, aide contextuelle, désactivé, lecture
 * seule.
 *
 * ILS SONT MONTRÉS CÔTE À CÔTE, ET C'EST TOUT L'INTÉRÊT. Un champ désactivé et
 * un champ en lecture seule se ressemblent tant qu'on ne les compare pas ; les
 * voir ensemble rappelle qu'ils ne font pas la même chose — l'un sort du
 * formulaire, l'autre y reste.
 *
 * Le focus ne se démontre pas dans une capture : il faut tabuler. La note de la
 * section le dit, et l'anneau est celui de toute la plateforme.
 */

const { t } = useI18n()

const empty = ref('')
const filled = ref('Institut de la Francophonie pour le développement durable')
const invalid = ref('contact@')
const withHint = ref('')
const readonlyValue = ref('COP31-00147')

const summary = ref(
  "Un atelier de deux heures réunissant les points focaux nationaux de six pays d'Afrique de l'Ouest autour des mécanismes de financement de l'adaptation côtière.",
)
const longText = ref('')
const richText = ref(
  '<p>La session confronte les <strong>mesures relevées</strong> aux prévisions des plans.</p><ul><li><p>Trois pays, six sites.</p></li></ul>',
)

const selectValue = ref('')
const selectFilled = ref('hybrid')
const radioValue = ref('in_person')
const checkOne = ref(true)
const checkTwo = ref(false)
const checkPartial = ref(false)
const switchOne = ref(true)
const switchBusy = ref(false)
const dateValue = ref('2027-11-14')
const dateTimeValue = ref('2027-11-14T14:30')

const search = ref('')
const searchResults = ref<number | null>(null)
const searching = ref(false)

const formatOptions = computed<SelectOption[]>(() => [
  { value: 'in_person', label: t('session-card.format.in_person') },
  { value: 'online', label: t('session-card.format.online') },
  { value: 'hybrid', label: t('session-card.format.hybrid') },
])

/** Simule l'aller-retour du référentiel d'organisations (écran A2). */
function onSearch(term: string): void {
  if (!term) {
    searchResults.value = null
    return
  }
  searching.value = true
  setTimeout(() => {
    searching.value = false
    searchResults.value = term.length > 4 ? 2 : 7
  }, 500)
}

function toggleSwitchWithWork(): void {
  switchBusy.value = true
  setTimeout(() => (switchBusy.value = false), 1600)
}
</script>

<template>
  <StyleGuideSection
    id="formulaires"
    :title="t('style-guide.forms.title')"
    :description="t('style-guide.forms.description')"
  >
    <!-- LES SEPT ÉTATS D'UN CHAMP -->
    <StyleGuideDemo
      :title="t('style-guide.forms.states.title')"
      :note="t('style-guide.forms.states.note')"
    >
      <div class="grid gap-5 md:grid-cols-2">
        <UiInput
          v-model="empty"
          :label="t('style-guide.forms.states.empty')"
          :placeholder="t('style-guide.forms.placeholder')"
        />
        <UiInput v-model="filled" :label="t('style-guide.forms.states.filled')" />
        <UiInput
          v-model="withHint"
          :label="t('style-guide.forms.states.hint')"
          :hint="t('style-guide.forms.hintText')"
          :placeholder="t('style-guide.forms.placeholder')"
        />
        <UiInput
          v-model="invalid"
          type="email"
          :label="t('style-guide.forms.states.error')"
          :error="t('style-guide.forms.errorText')"
          required
        />
        <UiInput
          v-model="filled"
          :label="t('style-guide.forms.states.disabled')"
          disabled
        />
        <UiInput
          v-model="readonlyValue"
          :label="t('style-guide.forms.states.readonly')"
          :hint="t('style-guide.forms.readonlyHint')"
          readonly
        />
      </div>
      <p class="mt-4 rounded-md border border-info-border bg-info-surface px-3 py-2 text-sm text-text-muted">
        {{ t('style-guide.forms.states.focusNote') }}
      </p>
    </StyleGuideDemo>

    <!-- TAILLES, ICÔNES, COMPTEUR -->
    <StyleGuideDemo
      :title="t('style-guide.forms.variants.title')"
      :note="t('style-guide.forms.variants.note')"
    >
      <div class="grid gap-5 md:grid-cols-2">
        <UiInput v-model="empty" size="sm" :label="t('style-guide.forms.variants.small')" :placeholder="t('style-guide.forms.placeholder')" />
        <UiInput v-model="empty" size="lg" :label="t('style-guide.forms.variants.large')" :placeholder="t('style-guide.forms.placeholder')" />
        <UiInput
          v-model="empty"
          icon="globe"
          type="url"
          :label="t('style-guide.forms.variants.withIcon')"
          placeholder="https://"
        />
        <UiInput
          v-model="empty"
          type="number"
          :label="t('style-guide.forms.variants.number')"
          :min="0"
          :max="300"
          :hint="t('style-guide.forms.variants.numberHint')"
        />
      </div>
    </StyleGuideDemo>

    <!-- ZONE DE TEXTE -->
    <StyleGuideDemo
      :title="t('style-guide.forms.textarea.title')"
      :note="t('style-guide.forms.textarea.note')"
    >
      <div class="grid gap-5 md:grid-cols-2">
        <UiTextarea
          v-model="summary"
          :label="t('style-guide.forms.textarea.summary')"
          :maxlength="400"
          :rows="5"
          :hint="t('style-guide.forms.textarea.summaryHint')"
        />
        <UiTextarea
          v-model="longText"
          :label="t('style-guide.forms.textarea.autoGrow')"
          :placeholder="t('style-guide.forms.textarea.autoGrowPlaceholder')"
          auto-grow
          :rows="2"
        />
      </div>
    </StyleGuideDemo>

    <!-- TEXTE RICHE -->
    <StyleGuideDemo
      :title="t('style-guide.forms.richText.title')"
      :note="t('style-guide.forms.richText.note')"
    >
      <ClientOnly>
        <UiRichText
          v-model="richText"
          :label="t('style-guide.forms.richText.label')"
          :hint="t('style-guide.forms.richText.hint')"
          :maxlength="600"
          :rows="6"
        />
        <template #fallback>
          <UiTextarea :label="t('style-guide.forms.richText.label')" :rows="6" readonly />
        </template>
      </ClientOnly>
    </StyleGuideDemo>

    <!-- LISTES ET CHOIX -->
    <StyleGuideDemo
      :title="t('style-guide.forms.choices.title')"
      :note="t('style-guide.forms.choices.note')"
    >
      <div class="grid gap-6 md:grid-cols-2">
        <div class="space-y-5">
          <UiSelect
            v-model="selectValue"
            :options="formatOptions"
            :label="t('style-guide.forms.choices.selectEmpty')"
            :placeholder="t('style-guide.forms.choices.selectPlaceholder')"
            required
          />
          <UiSelect
            v-model="selectFilled"
            :options="formatOptions"
            :label="t('style-guide.forms.choices.selectFilled')"
          />
          <UiSelect
            v-model="selectFilled"
            :options="formatOptions"
            :label="t('style-guide.forms.choices.selectReadonly')"
            readonly
          />
          <UiSelect
            v-model="selectValue"
            :options="formatOptions"
            :label="t('style-guide.forms.choices.selectError')"
            :placeholder="t('style-guide.forms.choices.selectPlaceholder')"
            :error="t('style-guide.forms.choices.selectErrorText')"
          />
        </div>

        <div class="space-y-6">
          <UiRadio
            v-model="radioValue"
            :options="formatOptions"
            :label="t('style-guide.forms.choices.radio')"
            :hint="t('style-guide.forms.choices.radioHint')"
          />

          <div class="space-y-2.5">
            <UiCheckbox v-model="checkOne" :label="t('style-guide.forms.choices.checkChecked')" />
            <UiCheckbox
              v-model="checkTwo"
              :label="t('style-guide.forms.choices.checkWithHint')"
              :hint="t('style-guide.forms.choices.checkHint')"
            />
            <UiCheckbox
              v-model="checkPartial"
              indeterminate
              :label="t('style-guide.forms.choices.checkPartial')"
            />
            <UiCheckbox
              :model-value="true"
              :label="t('style-guide.forms.choices.checkDisabled')"
              disabled
            />
            <UiCheckbox
              :model-value="false"
              :label="t('style-guide.forms.choices.checkError')"
              :error="t('style-guide.forms.choices.checkErrorText')"
              required
            />
          </div>

          <div class="space-y-3 border-t border-border-subtle pt-4">
            <UiSwitch
              v-model="switchOne"
              :label="t('style-guide.forms.choices.switchOn')"
              :hint="t('style-guide.forms.choices.switchHint')"
            />
            <UiSwitch
              :model-value="false"
              :label="t('style-guide.forms.choices.switchOff')"
            />
            <UiSwitch
              :model-value="true"
              :loading="switchBusy"
              :label="t('style-guide.forms.choices.switchLoading')"
              @update:model-value="toggleSwitchWithWork"
            />
            <UiSwitch
              :model-value="false"
              disabled
              :label="t('style-guide.forms.choices.switchDisabled')"
            />
          </div>
        </div>
      </div>
    </StyleGuideDemo>

    <!-- DATES ET RECHERCHE -->
    <StyleGuideDemo
      :title="t('style-guide.forms.dates.title')"
      :note="t('style-guide.forms.dates.note')"
    >
      <div class="grid gap-5 md:grid-cols-2">
        <UiDatePicker
          v-model="dateValue"
          :label="t('style-guide.forms.dates.dateOnly')"
          min="2027-11-09"
          max="2027-11-20"
          :hint="t('style-guide.forms.dates.bounds')"
        />
        <UiDatePicker
          v-model="dateTimeValue"
          with-time
          :label="t('style-guide.forms.dates.dateTime')"
          timezone-label="Belém"
          min="2027-11-09T08:00"
          max="2027-11-20T20:00"
        />
      </div>

      <div class="mt-5 max-w-xl">
        <UiSearchInput
          v-model="search"
          :label="t('style-guide.forms.search.label')"
          :hint="t('style-guide.forms.search.hint')"
          :loading="searching"
          :result-count="searchResults"
          @search="onSearch"
        />
        <p v-if="searchResults !== null && !searching" class="mt-2 text-sm text-text-muted">
          {{ t('common.pagination.results', searchResults) }}
        </p>
      </div>
    </StyleGuideDemo>
  </StyleGuideSection>
</template>
