<script setup lang="ts">
import type { PersonLookup } from '~/types/proposal-form'
import type { PersonId } from '~/types/shared'

/**
 * CHANGER LE DÉPOSANT : la personne qui suit le dossier et en reçoit les avis.
 *
 * La recherche est celle des intervenants (`/people/suggest`, puis l'adresse
 * exacte) : la clé est l'adresse, jamais le nom, deux homonymes ne se
 * confondent pas. Seule une personne qui a un compte peut déposer.
 */

interface Props {
  open: boolean
  currentId: PersonId
  currentName: string
  busy?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; submit: [personId: PersonId] }>()

const { t } = useI18n()
const api = useApi()

const query = ref('')
const results = ref<PersonLookup[]>([])
const searching = ref(false)
const searched = ref(false)
const failed = ref(false)
const chosen = ref<PersonLookup | null>(null)

watch(
  () => props.open,
  (open) => {
    if (!open) return
    query.value = ''
    results.value = []
    searched.value = false
    chosen.value = null
  },
)

let sequence = 0
async function search(value: string): Promise<void> {
  const term = value.trim()
  query.value = term
  if (term.length < 3) {
    results.value = []
    searched.value = false
    return
  }
  const current = ++sequence
  searching.value = true
  failed.value = false
  try {
    const suggested = await api.proposals.suggestSpeakers(term)
    // Une adresse complète que les suggestions écartent (annuaire refusé) se
    // retrouve par son adresse exacte.
    const exact = term.includes('@') && !suggested.some((entry) => entry.email === term)
      ? await api.proposals.lookupSpeaker(term)
      : null
    if (current !== sequence) return
    results.value = [...(exact ? [exact] : []), ...suggested].filter((entry) => entry.has_account)
    searched.value = true
  } catch {
    if (current === sequence) failed.value = true
  } finally {
    if (current === sequence) searching.value = false
  }
}

function nameOf(entry: PersonLookup): string {
  return `${entry.first_name} ${entry.last_name}`.trim()
}
</script>

<template>
  <UiDrawer
    :open="props.open"
    :title="t('admin.proposal.dashboard.submitterPanel.title')"
    width="40rem"
    @update:open="emit('update:open', $event)"
  >
    <div class="flex flex-col gap-5">
      <p class="text-sm text-text-secondary">{{ t('admin.proposal.dashboard.submitterPanel.intro') }}</p>

      <p class="flex flex-wrap items-baseline justify-between gap-2 border-y border-border-subtle py-3 text-sm">
        <span class="font-semibold text-text">{{ props.currentName }}</span>
        <span class="text-text-muted">{{ t('admin.proposal.dashboard.submitterPanel.current') }}</span>
      </p>

      <div
        v-if="chosen"
        class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border px-4 py-3"
      >
        <div class="min-w-0">
          <p class="font-semibold">{{ nameOf(chosen) }}</p>
          <p class="font-mono text-sm break-all text-text-muted">{{ chosen.email }}</p>
        </div>
        <UiButton variant="ghost" icon="close" :disabled="props.busy" @click="chosen = null">
          {{ t('admin.proposal.dashboard.submitterPanel.other') }}
        </UiButton>
      </div>

      <template v-else>
        <UiSearchInput
          :model-value="query"
          :label="t('admin.proposal.dashboard.submitterPanel.search')"
          :hint="t('admin.proposal.dashboard.submitterPanel.hint')"
          :min-length="3"
          :loading="searching"
          :disabled="props.busy"
          :result-count="searched ? results.length : null"
          @search="search"
        />
        <UiAlert v-if="failed" intent="danger" compact :message="t('admin.proposal.dashboard.submitterPanel.failed')" />
        <p v-else-if="searched && !results.length" class="text-sm text-text-muted">
          {{ t('admin.proposal.dashboard.submitterPanel.none') }}
        </p>
        <ul v-else-if="results.length" class="grid gap-2">
          <li
            v-for="entry in results"
            :key="entry.person_id"
            class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border px-4 py-3"
          >
            <div class="min-w-0">
              <p class="font-semibold">{{ nameOf(entry) }}</p>
              <p class="font-mono text-sm break-all text-text-muted">{{ entry.email }}</p>
              <p v-if="entry.organization_name" class="text-sm text-text-muted">{{ entry.organization_name }}</p>
            </div>
            <UiButton
              variant="secondary"
              :disabled="props.busy || entry.person_id === props.currentId"
              @click="chosen = entry"
            >
              {{ entry.person_id === props.currentId
                ? t('admin.proposal.dashboard.submitterPanel.current')
                : t('admin.proposal.dashboard.submitterPanel.choose') }}
            </UiButton>
          </li>
        </ul>
      </template>

      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />
      <p class="text-xs text-text-muted">{{ t('admin.proposal.dashboard.submitterPanel.membershipNote') }}</p>
    </div>

    <template #footer>
      <div class="flex flex-wrap justify-end gap-2">
        <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
          {{ t('common.actions.cancel') }}
        </UiButton>
        <UiButton :loading="props.busy" :disabled="!chosen" @click="chosen && emit('submit', chosen.person_id)">
          {{ t('admin.proposal.dashboard.submitterPanel.confirm') }}
        </UiButton>
      </div>
    </template>
  </UiDrawer>
</template>
