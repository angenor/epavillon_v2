<script setup lang="ts">
import type { DuplicatePair, DuplicateQueueScreen } from '~/types/admin-organizations'
import type { EffectivePermission } from '~/types/identity'

/**
 * FILE DES DOUBLONS PRÉSUMÉS — `/admin/organisations/doublons`.
 *
 * CE QUE LA V1 N'AVAIT PAS. Elle disposait de `organization_aliases` et de
 * `is_duplicate` ; ce qui manquait était la CHAÎNE COMPLÈTE — détecter, montrer,
 * arbitrer, fusionner. Cet écran est le troisième maillon : le worker remplit
 * `org.duplicate_candidates`, la file présente, l'humain tranche.
 *
 * TRIÉES PAR SIMILARITÉ DÉCROISSANTE, comme le demande le prompt et comme le sert
 * l'index partiel `ix_duplicate_candidates_pending (score DESC) WHERE reviewed_at
 * IS NULL`. La paire la plus ressemblante d'abord : c'est celle qui coûte le plus
 * cher à laisser en place.
 *
 * TROIS ISSUES, ET LA FUSION N'EST PAS LA SEULE. « Ce ne sont pas des doublons »
 * retire la paire pour de bon (`decision = 'distinct'`) ; « plus tard » la reporte
 * (`deferred`) ; la fusion a son écran. Sans les deux premières, la file se
 * remplirait de paires qu'on a déjà regardées, et personne ne l'ouvrirait plus.
 *
 * RIEN N'EST DÉFINITIF. Les paires arbitrées restent listées sous la file, et une
 * paire écartée par erreur se remet en circulation d'un clic. C'est la différence
 * entre un outil de travail et un bouton qui fait disparaître les choses.
 *
 * PORTÉE GLOBALE EXIGÉE. Une paire de doublons ne relève d'aucune édition, et sa
 * résolution déplace des rattachements partout : `org.organization.merge` sur la
 * portée globale, ou accès refusé — pas une file partielle.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.organizations', to: '/admin/organisations' },
    { labelKey: 'admin.organization.duplicates.title' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.organization.duplicates.title') }))

await adminScope.ensureLoaded()

const timezone = computed(() => auth.person?.timezone ?? 'UTC')

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-duplicates-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canMerge = computed(() => hasPermission(granted.value, 'org.organization.merge'))

const {
  data: queue,
  status,
  error,
  refresh,
} = await useAsyncData<DuplicateQueueScreen | null>(
  'admin-duplicate-queue',
  async () => (canMerge.value ? api.adminOrganizations.duplicates() : null),
  { watch: [canMerge], lazy: true },
)

// ---------------------------------------------------------------------------
// Arbitrages
// ---------------------------------------------------------------------------

const busyPairId = ref<string | null>(null)
const notice = ref<string | null>(null)

/** Paire dont on s'apprête à dire qu'elle n'en est pas une. */
const distinctTarget = ref<DuplicatePair | null>(null)
const distinctNote = ref('')

function askDistinct(pair: DuplicatePair): void {
  distinctTarget.value = pair
  distinctNote.value = ''
}

async function decide(
  pair: DuplicatePair,
  decision: 'distinct' | 'deferred',
  note: string | null,
  message: string,
): Promise<void> {
  busyPairId.value = pair.id
  notice.value = null
  try {
    await api.adminOrganizations.decideDuplicate(
      { pair_id: pair.id, decision, note },
      auth.person?.id ?? null,
    )
    await refresh()
    notice.value = message
  } finally {
    busyPairId.value = null
    distinctTarget.value = null
  }
}

/**
 * REMETTRE UNE PAIRE DANS LA FILE. Le modèle ne porte pas de valeur « à
 * réexaminer » : on repasse la décision à `deferred`, et l'API reconnaît le
 * geste à ceci que la paire est DÉJÀ SORTIE de la file — elle efface alors la
 * décision et sa date, ce qui la ramène parmi les dossiers ouverts.
 *
 * Effacer la DATE est le point qui compte : l'écran range sur elle, pas sur la
 * décision. La laisser posée laissait la paire parmi les tranchées, et le
 * bouton disait le contraire de ce qu'il faisait (corrigé le 20/08).
 */
async function reopen(pair: DuplicatePair): Promise<void> {
  await decide(pair, 'deferred', null, t('admin.organization.duplicates.reopened'))
}

// La promesse est ATTENDUE : ignorée, un refus de navigation — garde de route,
// route disparue — ne laisserait aucune trace, et l'écran resterait muet.
async function openMerge(pair: DuplicatePair): Promise<void> {
  // Le SENS n'est pas décidé ici : l'écran de fusion propose, l'équipe tranche.
  // Les deux fiches partent donc comme « gauche » et « droite », pas comme
  // source et cible.
  await navigateTo(
    localePath({
      path: '/admin/organisations/fusion',
      query: {
        gauche: pair.left.organization_id,
        droite: pair.right.organization_id,
        paire: pair.id,
      },
    }),
  )
}
</script>

<template>
  <div class="mx-auto w-full max-w-6xl">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !canMerge"
      :required-scope="t('admin.organization.duplicates.forbidden.scope')"
      action-to="/admin/organisations"
      :action-label="t('admin.organization.duplicates.back')"
    />

    <template v-else>
      <header>
        <UiButton variant="link" icon="arrow-left" :to="localePath('/admin/organisations')">
          {{ t('admin.organization.duplicates.back') }}
        </UiButton>
        <h1 class="mt-2 text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.organization.duplicates.title') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.organization.duplicates.subtitle') }}
        </p>
      </header>

      <UiAlert
        v-if="notice"
        class="mt-6"
        intent="success"
        live
        dismissible
        :message="notice"
        @close="notice = null"
      />

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiLoadingState v-else-if="status === 'pending' && !queue" class="mt-8" />

      <template v-else>
        <UiEmptyState
          v-if="queue && queue.pending.length === 0 && queue.settled.length === 0"
          class="mt-8"
          icon="check-circle"
          :title="t('admin.organization.duplicates.empty.title')"
          :description="t('admin.organization.duplicates.empty.description')"
        />

        <template v-else>
          <section v-if="queue && queue.pending.length > 0" class="mt-8">
            <h2 class="text-lg font-semibold text-text">
              {{ t('admin.organization.duplicates.pending.title') }}
              <span class="ml-2 text-sm font-normal text-text-muted">
                {{ t('admin.organization.duplicates.pending.count', queue.pending.length) }}
              </span>
            </h2>

            <div class="mt-4 flex flex-col gap-4">
              <AdminOrganizationsDuplicatePair
                v-for="pair in queue.pending"
                :key="pair.id"
                :pair="pair"
                :timezone="timezone"
                :can-merge="canMerge"
                :busy="busyPairId === pair.id"
                @merge="openMerge"
                @distinct="askDistinct"
                @defer="
                  (target) =>
                    decide(target, 'deferred', null, t('admin.organization.duplicates.deferSuccess'))
                "
              />
            </div>
          </section>

          <!-- CE QUI EST SORTI DE LA FILE. Rangé, pas effacé : une décision se
               reprend, et une paire écartée par erreur serait autrement
               introuvable. -->
          <section v-if="queue && queue.settled.length > 0" class="mt-10">
            <h2 class="text-lg font-semibold text-text">
              {{ t('admin.organization.duplicates.settled.title') }}
              <span class="ml-2 text-sm font-normal text-text-muted">
                {{ t('admin.organization.duplicates.settled.count', queue.settled.length) }}
              </span>
            </h2>
            <p class="mt-1 text-sm text-text-muted">
              {{ t('admin.organization.duplicates.settled.hint') }}
            </p>

            <div class="mt-4 flex flex-col gap-4">
              <AdminOrganizationsDuplicatePair
                v-for="pair in queue.settled"
                :key="pair.id"
                :pair="pair"
                :timezone="timezone"
                :can-merge="canMerge"
                settled
                :busy="busyPairId === pair.id"
                @reopen="reopen"
              />
            </div>
          </section>
        </template>
      </template>

      <!-- « CE NE SONT PAS DES DOUBLONS » — la seule décision de cet écran qui
           retire quelque chose. Elle demande donc un dialogue, et offre un motif
           que la personne qui relira dans six mois pourra lire. -->
      <UiModal
        v-if="distinctTarget"
        :open="distinctTarget !== null"
        :title="t('admin.organization.duplicates.distinctDialog.title')"
        :description="
          t('admin.organization.duplicates.distinctDialog.description', {
            left: distinctTarget.left.legal_name,
            right: distinctTarget.right.legal_name,
          })
        "
        @update:open="(value: boolean) => { if (!value) distinctTarget = null }"
      >
        <UiTextarea
          v-model="distinctNote"
          :label="t('admin.organization.duplicates.distinctDialog.note')"
          :hint="t('admin.organization.duplicates.distinctDialog.noteHint')"
          :placeholder="t('admin.organization.duplicates.distinctDialog.notePlaceholder')"
          :rows="3"
          block
        />

        <template #footer>
          <UiButton variant="ghost" @click="distinctTarget = null">
            {{ t('common.actions.cancel') }}
          </UiButton>
          <UiButton
            :loading="busyPairId === distinctTarget.id"
            @click="
              decide(
                distinctTarget,
                'distinct',
                distinctNote.trim() || null,
                t('admin.organization.duplicates.distinctDialog.success'),
              )
            "
          >
            {{ t('admin.organization.duplicates.distinctDialog.confirm') }}
          </UiButton>
        </template>
      </UiModal>
    </template>
  </div>
</template>
