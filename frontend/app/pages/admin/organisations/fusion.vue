<script setup lang="ts">
import type {
  MergeField,
  MergePreview,
  MergeResult,
  MergeSideKey,
} from '~/types/admin-organizations'
import type { EffectivePermission } from '~/types/identity'
import type { SimilarOrganization } from '~/types/org'

/**
 * ÉCRAN DE FUSION — `/admin/organisations/fusion?gauche=…&droite=…`.
 *
 * L'ÉCRAN LE PLUS DÉLICAT DU JALON, et le seul dont un clic déplace des
 * rattachements dans tous les modules à la fois. Tout y est ordonné pour qu'une
 * personne comprenne ce qu'elle fait AVANT de le faire :
 *
 *   1. le SENS — qui absorbe qui, proposé et inversable d'un clic ;
 *   2. la COMPARAISON champ par champ, écarts mis en évidence, avec le choix de
 *      la valeur à conserver pour chaque champ divergent ;
 *   3. le DÉCOMPTE de ce qui sera transféré, lu dans `org.organization_references` ;
 *   4. les AVERTISSEMENTS — jamais bloquants, toujours nommés ;
 *   5. le MOTIF, obligatoire, consigné dans `org.merge_log` ;
 *   6. la CONFIRMATION par saisie du nom de la fiche absorbée.
 *
 * L'URL PORTE LES DEUX FICHES ET LE SENS. `gauche` et `droite` viennent de la
 * paire, dans l'ordre neutre de `ck_duplicate_candidates_ordered` ; `absorbe`
 * désigne la fiche CONSERVÉE. Inverser le sens change ce seul paramètre —
 * l'adresse reste partageable, et le décompte se recalcule, car il n'est pas
 * symétrique.
 *
 * DEUX ENTRÉES POSSIBLES. Depuis la file, les deux fiches sont connues. Depuis la
 * fiche d'une organisation, une seule l'est : l'écran ouvre alors le sélecteur, et
 * la recherche est celle du rattachement — `org.find_similar_organizations()`,
 * jamais une seconde implémentation.
 *
 * PORTÉE GLOBALE EXIGÉE : `org.organization.merge`. Une fusion ne se limite pas à
 * une édition, il n'y a donc pas de version restreinte de cet écran.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.organizations', to: '/admin/organisations' },
    { labelKey: 'admin.organization.duplicates.title', to: '/admin/organisations/doublons' },
    { labelKey: 'admin.organization.merge.title' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.organization.merge.title') }))

await adminScope.ensureLoaded()

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-merge-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canMerge = computed(() => hasPermission(granted.value, 'org.organization.merge'))

// ---------------------------------------------------------------------------
// Les deux fiches et le sens, portés par l'URL
// ---------------------------------------------------------------------------

function queryText(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

const leftId = computed(() => queryText(route.query.gauche))
const rightId = computed(() => queryText(route.query.droite))
const pairId = computed(() => queryText(route.query.paire) || null)
/** Fiche CONSERVÉE. Vide : le sens suggéré s'applique. */
const absorbingId = computed(() => queryText(route.query.absorbe))

/**
 * L'ORIENTATION DEMANDÉE À L'API — `source` est la fiche ABSORBÉE, `target` la
 * fiche CONSERVÉE.
 *
 * Tant que l'URL ne fixe pas `absorbe`, on demande l'aperçu dans l'ordre de la
 * paire ; la suggestion s'applique ensuite (voir le `watch` plus bas) et écrit le
 * sens dans l'URL. Deux appels dans ce seul cas, et c'est le prix de la justesse :
 * LE DÉCOMPTE N'EST PAS SYMÉTRIQUE — ce qui est dédoublonné dépend de ce que la
 * cible porte déjà —, on ne peut donc pas retourner un aperçu à l'affichage.
 */
const orientation = computed(() => {
  if (!leftId.value || !rightId.value) return null
  return absorbingId.value === leftId.value
    ? { source: rightId.value, target: leftId.value }
    : { source: leftId.value, target: rightId.value }
})

const {
  data: rawPreview,
  status,
  error,
  refresh,
} = await useAsyncData<MergePreview | null>(
  'admin-merge-preview',
  async () => {
    const sides = orientation.value
    if (!canMerge.value || !sides) return null
    return api.adminOrganizations.mergePreview(sides.source, sides.target, pairId.value)
  },
  { watch: [canMerge, orientation], lazy: true },
)

const direction = computed(() =>
  rawPreview.value ? { source: rawPreview.value.source, target: rawPreview.value.target } : null,
)

/**
 * LE SENS PROPOSÉ, posé une seule fois, et dans l'URL.
 *
 * `suggestAbsorbingSide()` départage par le sceau, puis par le score, puis par
 * l'ancienneté. Écrire le résultat dans l'adresse plutôt que dans un état local
 * a deux effets voulus : le rechargement rend exactement le même écran, et le
 * bouton d'inversion n'a qu'un paramètre à changer.
 */
watch(
  rawPreview,
  (preview) => {
    if (!preview || absorbingId.value) return
    const absorbing = suggestAbsorbingSide(preview.target, preview.source)
    router.replace({ query: { ...route.query, absorbe: absorbing.organization_id } })
  },
  { immediate: true },
)

function swapDirection(): void {
  if (!rawPreview.value) return
  router.replace({
    query: { ...route.query, absorbe: rawPreview.value.source.organization_id },
  })
}

// ---------------------------------------------------------------------------
// Le choix de la seconde fiche, quand une seule est connue
// ---------------------------------------------------------------------------

const search = ref('')
const { data: candidates, status: searchStatus } = await useAsyncData<SimilarOrganization[]>(
  'admin-merge-candidates',
  async () => {
    if (search.value.trim().length < 2) return []
    const results = await api.organizations.similar({ name: search.value, limit: 8 })
    // Une fiche ne se fusionne pas avec elle-même.
    return results.filter((entry) => entry.organization_id !== leftId.value)
  },
  { default: () => [], watch: [search], lazy: true },
)

function pickSecond(organizationId: string): void {
  router.replace({ query: { ...route.query, droite: organizationId, absorbe: undefined } })
}

// ---------------------------------------------------------------------------
// Les choix de champ
// ---------------------------------------------------------------------------

const choices = ref<Partial<Record<MergeField, MergeSideKey>>>({})

/**
 * Les choix se réinitialisent à chaque nouvel aperçu — donc à chaque inversion du
 * sens. C'est voulu : « conserver la valeur de gauche » ne veut plus rien dire
 * quand la gauche a changé de fiche, et garder les cases cochées ferait valider
 * l'inverse de ce qu'on lit.
 */
watch(
  rawPreview,
  (preview) => {
    choices.value = preview ? defaultMergeChoices(preview.comparisons) : {}
  },
  { immediate: true },
)

function choose(field: MergeField, side: MergeSideKey): void {
  choices.value = { ...choices.value, [field]: side }
}

const unresolved = computed(() =>
  rawPreview.value ? unresolvedFields(rawPreview.value.comparisons, choices.value) : [],
)

// ---------------------------------------------------------------------------
// Motif, confirmation, exécution
// ---------------------------------------------------------------------------

const reason = ref('')
const reasonTouched = ref(false)
const confirmOpen = ref(false)
const busy = ref(false)
const serverMismatch = ref(false)
const errorMessage = ref<string | null>(null)
const result = ref<MergeResult | null>(null)

const reasonError = computed(() =>
  reasonTouched.value && reason.value.trim().length === 0
    ? t('admin.organization.merge.reason.required')
    : undefined,
)

/** Le motif est obligatoire ; les écarts non tranchés bloquent aussi la validation. */
const canSubmit = computed(
  () => reason.value.trim().length > 0 && unresolved.value.length === 0 && !busy.value,
)

function askConfirmation(): void {
  reasonTouched.value = true
  if (!canSubmit.value) return
  serverMismatch.value = false
  confirmOpen.value = true
}

async function submit(typedName: string): Promise<void> {
  if (!rawPreview.value) return
  busy.value = true
  errorMessage.value = null

  try {
    const response = await api.adminOrganizations.merge(
      {
        source_id: rawPreview.value.source.organization_id,
        target_id: rawPreview.value.target.organization_id,
        pair_id: pairId.value,
        reason: reason.value.trim(),
        field_choices: choices.value,
        confirmation_name: typedName,
      },
      auth.person?.id ?? null,
    )

    if (response.status === 'merged') {
      result.value = response
      confirmOpen.value = false
      return
    }
    if (response.status === 'confirmation_mismatch') {
      serverMismatch.value = true
      return
    }

    confirmOpen.value = false
    errorMessage.value = t(
      response.status === 'already_merged'
        ? 'admin.organization.merge.error.alreadyMerged'
        : 'admin.organization.merge.error.generic',
    )
  } finally {
    busy.value = false
  }
}

const movedRows = computed(() =>
  result.value
    ? Object.values(result.value.rows_reassigned).reduce((sum, count) => sum + count, 0)
    : 0,
)
</script>

<template>
  <div class="mx-auto w-full max-w-6xl">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !canMerge"
      :required-scope="t('admin.organization.merge.forbidden.scope')"
      :description="t('admin.organization.merge.forbidden.description')"
      action-to="/admin/organisations"
      :action-label="t('admin.organization.merge.back')"
    />

    <template v-else>
      <header>
        <UiButton variant="link" icon="arrow-left" :to="localePath('/admin/organisations/doublons')">
          {{ t('admin.organization.merge.back') }}
        </UiButton>
        <h1 class="mt-2 text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.organization.merge.title') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.organization.merge.subtitle') }}
        </p>
      </header>

      <!-- FUSION EFFECTUÉE. L'écran ne revient pas à la comparaison : il rend
           compte, chiffres à l'appui, puis renvoie vers la fiche conservée. -->
      <section v-if="result" class="mt-8 rounded-lg border border-success-border bg-success-surface p-6">
        <h2 class="text-lg font-semibold text-success">
          {{ t('admin.organization.merge.result.title') }}
        </h2>
        <p class="mt-1 text-text">
          {{
            t('admin.organization.merge.result.description', {
              source: rawPreview?.source.legal_name ?? '',
              target: rawPreview?.target.legal_name ?? '',
            })
          }}
        </p>
        <p class="mt-2 text-sm text-text-muted">
          {{ t('admin.organization.merge.result.rows', movedRows) }}
          <template v-if="result.fields_applied.length > 0">
            — {{ t('admin.organization.merge.result.fields', result.fields_applied.length) }}
          </template>
        </p>
        <div class="mt-4 flex flex-wrap gap-2">
          <UiButton :to="localePath(`/admin/organisations/${result.target}`)">
            {{ t('admin.organization.merge.result.openTarget') }}
          </UiButton>
          <UiButton variant="secondary" :to="localePath('/admin/organisations/doublons')">
            {{ t('admin.organization.merge.result.backToQueue') }}
          </UiButton>
        </div>
      </section>

      <template v-else>
        <UiErrorState
          v-if="error"
          class="mt-8"
          :retry-label="t('common.actions.retry')"
          @retry="refresh()"
        />

        <!-- UNE SEULE FICHE CONNUE : on choisit la seconde, par la recherche du
             rattachement — la même fonction, jamais une seconde. -->
        <section v-else-if="leftId && !rightId" class="mt-8 rounded-lg border border-border bg-surface-raised p-6">
          <h2 class="text-lg font-semibold text-text">
            {{ t('admin.organization.merge.picker.title') }}
          </h2>
          <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
            {{ t('admin.organization.merge.picker.description') }}
          </p>

          <UiSearchInput
            v-model="search"
            class="mt-4"
            :label="t('admin.organization.merge.picker.search')"
            :placeholder="t('admin.organization.merge.picker.search')"
            :loading="searchStatus === 'pending'"
            :result-count="candidates.length"
            block
          />

          <ul v-if="candidates.length > 0" class="mt-4 flex flex-col gap-2">
            <li
              v-for="candidate in candidates"
              :key="candidate.organization_id"
              class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border px-3 py-2"
            >
              <div class="min-w-0">
                <p class="font-medium text-text">{{ candidate.legal_name }}</p>
                <p class="text-xs text-text-muted">
                  <span v-if="candidate.acronym" class="font-mono">{{ candidate.acronym }}</span>
                  <span v-if="candidate.matched_name && candidate.matched_name !== candidate.legal_name">
                    — {{ t('admin.organization.merge.picker.matched', { name: candidate.matched_name }) }}
                  </span>
                </p>
              </div>
              <UiButton variant="secondary" size="sm" @click="pickSecond(candidate.organization_id)">
                {{ t('admin.organization.merge.picker.select') }}
              </UiButton>
            </li>
          </ul>
          <p
            v-else-if="search.trim().length >= 2 && searchStatus !== 'pending'"
            class="mt-4 text-sm text-text-muted"
          >
            {{ t('admin.organization.merge.picker.empty') }}
          </p>
        </section>

        <UiLoadingState v-else-if="status === 'pending' && !rawPreview" class="mt-8" />

        <!-- FICHE INTROUVABLE OU DÉJÀ ABSORBÉE — `tg_forbid_merge_chains` refuse
             de cibler une fiche elle-même fusionnée. -->
        <UiEmptyState
          v-else-if="!rawPreview"
          class="mt-8"
          icon="ban"
          :title="t('admin.organization.merge.notFound.title')"
          :description="t('admin.organization.merge.notFound.description')"
          :action-label="t('admin.organization.merge.notFound.action')"
          :action-to="localePath('/admin/organisations/doublons')"
        />

        <template v-else-if="direction">
          <UiAlert
            v-if="errorMessage"
            class="mt-6"
            intent="danger"
            live
            :message="errorMessage"
          />

          <!-- 1. LE SENS. La fiche conservée à gauche, l'absorbée à droite, et
               l'inversion d'un clic. -->
          <section class="mt-8 rounded-lg border border-border bg-surface-raised p-4">
            <div class="flex flex-wrap items-start justify-between gap-4">
              <h2 class="text-lg font-semibold text-text">
                {{ t('admin.organization.merge.direction.title') }}
              </h2>
              <UiButton variant="secondary" size="sm" icon="refresh" @click="swapDirection">
                {{ t('admin.organization.merge.direction.swap') }}
              </UiButton>
            </div>

            <div class="mt-3 grid gap-3 sm:grid-cols-2">
              <div class="rounded-md border border-success-border bg-success-surface p-3">
                <p class="text-xs font-semibold tracking-wide text-success uppercase">
                  {{ t('admin.organization.merge.direction.absorbing') }}
                </p>
                <p class="mt-1 font-semibold text-text">{{ direction.target.legal_name }}</p>
                <p class="text-xs text-text-muted">
                  {{ t('admin.organization.merge.direction.absorbingHint') }}
                </p>
              </div>
              <div class="rounded-md border border-border bg-surface-sunken p-3">
                <p class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
                  {{ t('admin.organization.merge.direction.absorbed') }}
                </p>
                <p class="mt-1 font-semibold text-text">{{ direction.source.legal_name }}</p>
                <p class="text-xs text-text-muted">
                  {{ t('admin.organization.merge.direction.absorbedHint') }}
                </p>
              </div>
            </div>

            <p class="mt-2 text-xs text-text-subtle">
              {{ t('admin.organization.merge.direction.suggested') }}
            </p>
          </section>

          <!-- 4. LES AVERTISSEMENTS, avant la comparaison : ils portent sur le
               SENS, et c'est encore le moment de l'inverser. Jamais bloquants. -->
          <UiAlert
            v-if="rawPreview.warnings.length > 0"
            class="mt-6"
            intent="warning"
            :title="t('admin.organization.merge.warnings.title')"
          >
            <ul class="flex list-disc flex-col gap-1 pl-5">
              <li v-for="warning in rawPreview.warnings" :key="warning.code">
                {{ t('admin.organization.merge.warnings.' + warning.code, warning.values ?? {}) }}
              </li>
            </ul>
          </UiAlert>

          <!-- 2. LA COMPARAISON champ par champ. -->
          <AdminOrganizationsMergeCompare
            class="mt-6"
            :comparisons="rawPreview.comparisons"
            :source="rawPreview.source"
            :target="rawPreview.target"
            :choices="choices"
            :disabled="busy"
            @choose="choose"
          />

          <!-- 3. LE DÉCOMPTE de ce qui sera transféré, et ce que la fusion
               préserve. -->
          <AdminOrganizationsMergeTransfer class="mt-6" :preview="rawPreview" />

          <!-- 5. LE MOTIF, obligatoire. -->
          <section class="mt-6 rounded-lg border border-border bg-surface-raised p-4">
            <UiTextarea
              v-model="reason"
              :label="t('admin.organization.merge.reason.label')"
              :hint="t('admin.organization.merge.reason.hint')"
              :placeholder="t('admin.organization.merge.reason.placeholder')"
              :error="reasonError"
              :rows="3"
              required
              block
              @blur="reasonTouched = true"
            />

            <div class="mt-4 flex flex-wrap items-center justify-end gap-3">
              <p v-if="unresolved.length > 0" class="text-sm text-warning">
                {{ t('admin.organization.merge.compare.unresolved', unresolved.length) }}
              </p>
              <UiButton variant="danger" :disabled="unresolved.length > 0" @click="askConfirmation">
                {{ t('admin.organization.merge.submit') }}
              </UiButton>
            </div>
          </section>

          <!-- 6. LA CONFIRMATION par saisie du nom de la fiche absorbée. -->
          <AdminOrganizationsMergeConfirmDialog
            v-model:open="confirmOpen"
            :source="rawPreview.source"
            :target="rawPreview.target"
            :busy="busy"
            :server-mismatch="serverMismatch"
            @confirm="submit"
          />
        </template>
      </template>
    </template>
  </div>
</template>
