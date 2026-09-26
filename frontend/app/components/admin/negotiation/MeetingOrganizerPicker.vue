<script setup lang="ts">
import type { SimilarOrganization } from '~/types/org'
import type { Uuid } from '~/types/shared'

/**
 * L'organisation qui tient la réunion, cherchée par la recherche unique de la
 * plateforme — toutes dénominations confondues (règle métier n° 1).
 */

const props = defineProps<{
  modelValue: Uuid | null
  /** Le nom rendu par l'API pour la fiche déjà retenue. */
  currentName?: string | null
  error?: string
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: Uuid | null] }>()

const { t } = useI18n()
const api = useApi()

const query = ref('')
const results = ref<SimilarOrganization[]>([])
const searching = ref(false)
const searchFailed = ref(false)
const searched = ref(false)
const chosenName = ref<string | null>(null)

const shownName = computed(() => (props.modelValue ? (chosenName.value ?? props.currentName ?? null) : null))

let sequence = 0
async function search(value: string): Promise<void> {
  const term = value.trim()
  query.value = term
  if (term.length < 2) {
    results.value = []
    searched.value = false
    return
  }
  const current = ++sequence
  searching.value = true
  searchFailed.value = false
  try {
    const found = await api.organizations.similar({ name: term, limit: 8 })
    if (current !== sequence) return
    results.value = found
    searched.value = true
  } catch {
    if (current === sequence) searchFailed.value = true
  } finally {
    if (current === sequence) searching.value = false
  }
}

function choose(match: SimilarOrganization): void {
  chosenName.value = match.acronym ? `${match.legal_name} (${match.acronym})` : match.legal_name
  emit('update:modelValue', match.organization_id)
  results.value = []
  searched.value = false
  query.value = ''
}

function clear(): void {
  chosenName.value = null
  emit('update:modelValue', null)
}
</script>

<template>
  <div class="space-y-3">
    <div
      v-if="props.modelValue"
      class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border px-4 py-3"
    >
      <p class="min-w-0 font-semibold">
        {{ shownName ?? t('admin.negociations.reunions.form.organizer.unnamed') }}
      </p>
      <UiButton variant="ghost" size="sm" icon="close" :disabled="props.disabled" @click="clear">
        {{ t('admin.negociations.reunions.form.organizer.change') }}
      </UiButton>
    </div>

    <template v-else>
      <UiSearchInput
        :model-value="query"
        :label="t('admin.negociations.reunions.form.organizer.search')"
        :hint="t('admin.negociations.reunions.form.organizer.searchHint')"
        :error="props.error"
        :loading="searching"
        :disabled="props.disabled"
        :result-count="searched ? results.length : null"
        required
        @search="search"
      />

      <UiAlert
        v-if="searchFailed"
        intent="danger"
        compact
        :message="t('admin.negociations.reunions.form.organizer.searchError')"
      />

      <p v-else-if="searched && !results.length" class="text-sm text-text-muted">
        {{ t('admin.negociations.reunions.form.organizer.none') }}
      </p>

      <ul v-else-if="results.length" class="grid gap-2">
        <li
          v-for="match in results"
          :key="match.organization_id"
          class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border px-4 py-3"
        >
          <div class="min-w-0">
            <p class="font-semibold">{{ match.legal_name }}</p>
            <p v-if="match.acronym || match.matched_name" class="text-sm text-text-muted">
              {{ [match.acronym, match.matched_name !== match.legal_name ? match.matched_name : null].filter(Boolean).join(' · ') }}
            </p>
          </div>
          <UiButton variant="secondary" size="sm" :disabled="props.disabled" @click="choose(match)">
            {{ t('admin.negociations.reunions.form.organizer.choose') }}
          </UiButton>
        </li>
      </ul>
    </template>

    <p v-if="props.modelValue && props.error" role="alert" class="text-sm font-bold text-danger">
      {{ props.error }}
    </p>
  </div>
</template>
