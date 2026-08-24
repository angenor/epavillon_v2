<script setup lang="ts">
import type {
  ShowcaseListRow,
  ShowcaseListScreen,
  ShowcaseWriteResult,
} from '~/types/admin-showcase'
import type { HighlightPlacement, HighlightStatus } from '~/types/content'
import type { EffectivePermission } from '~/types/identity'

/**
 * LA VITRINE — `/admin/vitrine`.
 *
 * CE QU'ON VIENT FAIRE ICI, ET DANS QUEL ORDRE. Décider ce que le public voit en
 * arrivant sur la plateforme, et surtout DANS QUEL ORDRE il le voit. La v1 n'en
 * était pas capable — son carrousel suivait `created_at DESC`, et cinq widgets
 * d'annonce étaient écrits en dur dans des composants Vue. Cet écran remplace
 * les deux.
 *
 * ── LE PÉRIMÈTRE D'ADMINISTRATION, PRIS PAR LES DEUX BOUTS ─────────────────
 *
 * Règle métier n° 8, ADR-14. La LISTE est filtrée par l'API
 * (`api.adminShowcase.list(scope)`) : une édition hors périmètre n'y entre pas,
 * même par une URL forgée — et c'est bien l'API qui le garantit, pas cet écran.
 * Les ACTIONS, elles, se décident ligne par ligne : `content.highlight.manage`
 * s'accorde sur une ÉDITION, et une diapositive sans édition parle au nom de la
 * plateforme entière — elle exige la portée globale. D'où deux questions
 * distinctes, et deux fonctions distinctes :
 *   · `hasPermissionOnAnyScope` ouvre l'écran — droit d'entrer ;
 *   · `hasPermission(…, row.event_id)` ouvre une ligne — étendue de ce qu'on peut.
 * Les confondre donnerait soit un écran interdit à tort à une coordonnatrice
 * détachée, soit des boutons offerts sur des contenus qu'elle ne peut pas
 * toucher.
 *
 * ── L'ORDRE EST LA FONCTION PRINCIPALE ─────────────────────────────────────
 *
 * Les boutons monter/descendre vivent dans le tableau ; l'écran, lui, applique
 * la réponse. Un déplacement touche TOUJOURS DEUX LIGNES : `placement_rows`
 * rend l'emplacement entier renuméroté, et ne rafraîchir que la ligne cliquée
 * laisserait sa voisine afficher un rang faux. Le déplacement est aussi
 * ANNONCÉ (`role="status"`) : à la voix, deux lignes qui échangent leur place ne
 * produisent aucun son.
 *
 * ── QUATRE ÉTATS ───────────────────────────────────────────────────────────
 *
 * Chargement (lignes squelettes), vide (aucune diapositive — c'est l'état d'un
 * pavillon qui n'a pas encore ouvert), erreur avec reprise, accès refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.showcase' }],
})

defineI18nRoute({ paths: { fr: '/admin/vitrine', en: '/admin/showcase' } })

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.showcase.list.title') }))

await adminScope.ensureLoaded()

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<ShowcaseListScreen | null>(
  'admin-showcase',
  () => api.adminShowcase.list(adminScope.scope),
  { watch: [() => adminScope.scope], lazy: true },
)

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-showcase-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** Le droit d'ENTRER : la vitrine n'appartient à aucune édition en particulier. */
const canEnter = computed(() => hasPermissionOnAnyScope(granted.value, 'content.highlight.manage'))

/** Le droit d'AGIR sur une ligne — sur son édition, ou globalement si elle n'en a pas. */
const canManageRow = (row: ShowcaseListRow): boolean =>
  hasPermission(granted.value, 'content.highlight.manage', row.event_id)

const isSettling = computed(
  () => adminScope.isLoading || status.value === 'pending' || permissionStatus.value === 'pending',
)

// ---------------------------------------------------------------------------
// Les lignes — tenues localement, pour que les écritures s'appliquent sans
// recharger tout l'écran (une réponse d'écriture porte déjà les lignes à jour).
// ---------------------------------------------------------------------------

const rows = ref<ShowcaseListRow[]>([])
watch(screen, (next) => (rows.value = next ? [...next.rows] : []), { immediate: true })

/**
 * UN SEUL EMPLACEMENT DEPUIS LE 24/08 — le bandeau d'ouverture.
 *
 * Cet écran portait deux onglets, `bandeau` et `panneau`, et l'onglet vivait
 * dans l'URL (`?emplacement=`). Le panneau latéral de l'accueil ne se compose
 * plus : il affiche les événements à venir et la frise des activités retenues,
 * sans rien d'éditorial. `home_aside` a donc quitté le modèle, et les onglets
 * avec — un onglet unique n'est pas un choix, c'est un cadre vide.
 *
 * Jamais retriées : `sort_order` est l'ordre de défilement du bandeau public.
 */
const visibleRows = computed(() => rows.value)

// ---------------------------------------------------------------------------
// Les écritures
// ---------------------------------------------------------------------------

const busyId = ref<string | null>(null)
const writeError = ref<string | null>(null)
/** Ce que les lecteurs d'écran entendent après un déplacement — voir l'en-tête. */
const announcement = ref('')

/**
 * Applique la réponse d'une écriture.
 *
 * `placement_rows` d'abord : c'est l'emplacement ENTIER, renuméroté, et c'est la
 * seule forme juste après un déplacement ou une création. `row` ensuite, pour un
 * simple changement de statut. Le rechargement complet reste le dernier recours,
 * et non le premier réflexe : il ferait clignoter la liste à chaque publication.
 */
function applyResult(result: ShowcaseWriteResult, target: HighlightPlacement): boolean {
  if (!result.ok) {
    writeError.value = t('admin.showcase.list.error.refused')
    return false
  }

  if (result.placement_rows) {
    rows.value = [
      ...rows.value.filter((row) => row.placement !== target),
      ...result.placement_rows,
    ]
    return true
  }

  const updated = result.row
  if (updated) {
    rows.value = rows.value.map((row) => (row.id === updated.id ? updated : row))
    return true
  }

  void refresh()
  return true
}

/** Toute écriture partage la même enveloppe : verrou de ligne, erreur, reprise. */
async function write(
  row: ShowcaseListRow,
  action: () => Promise<ShowcaseWriteResult>,
): Promise<ShowcaseWriteResult | null> {
  busyId.value = row.id
  writeError.value = null
  try {
    const result = await action()
    applyResult(result, row.placement)
    return result
  } catch {
    // Un refus de périmètre est une ERREUR levée, pas une réponse vide : on le
    // dit, et on ne laisse surtout pas croire que l'action a été prise.
    writeError.value = t('admin.showcase.list.error.forbidden')
    return null
  } finally {
    busyId.value = null
  }
}

async function move(row: ShowcaseListRow, direction: 'up' | 'down'): Promise<void> {
  const result = await write(row, () =>
    api.adminShowcase.move({ id: row.id, direction }, adminScope.scope),
  )
  if (!result?.ok) return

  const bucket = rows.value.filter((entry) => entry.placement === row.placement)
  const position = bucket.findIndex((entry) => entry.id === row.id)
  if (position < 0) return
  announcement.value = t('admin.showcase.list.order.announce', {
    title: tr(row.title).trim() || t('admin.showcase.list.row.untitled'),
    position: position + 1,
    total: bucket.length,
  })
}

async function setStatus(row: ShowcaseListRow, next: HighlightStatus): Promise<void> {
  await write(row, () => api.adminShowcase.setStatus({ id: row.id, status: next }, adminScope.scope))
}

async function duplicate(row: ShowcaseListRow): Promise<void> {
  await write(row, () => api.adminShowcase.duplicate(row.id, adminScope.scope))
}

function open(row: ShowcaseListRow): void {
  void navigateTo(localePath(`/admin/vitrine/${row.id}`))
}

const newSlideTo = computed(() => localePath('/admin/vitrine/nouveau'))
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <!-- ACCÈS REFUSÉ, EN TOUT PREMIER. Aucune édition administrée, aucune
         permission, ou une API qui refuse : trois raisons, un seul écran, et
         jamais un tableau vide — « il n'y a rien ici » et « ceci ne vous regarde
         pas » ne se disent pas de la même façon. -->
    <UiForbiddenState
      v-if="!isSettling && (!adminScope.canAdminister || !canEnter || screen === null)"
      :required-scope="t('admin.showcase.list.forbidden.scope')"
      :action-to="localePath('/admin')"
      :action-label="t('admin.showcase.list.forbidden.action')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.showcase.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.showcase.list.subtitle') }}
          </p>
        </div>

        <UiButton icon="plus" :to="newSlideTo">{{ t('admin.showcase.list.new') }}</UiButton>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <template v-else>
        <UiAlert v-if="writeError" class="mt-6" intent="danger" live :message="writeError" />

        <!-- Un compte détaché ne voit pas les contenus de plateforme et ne peut
             pas en créer : le dire vaut mieux que de laisser chercher pourquoi
             l'option « toute la plateforme » manque au formulaire. -->
        <UiAlert
          v-else-if="screen && !screen.is_global_scope"
          class="mt-6"
          intent="info"
          compact
          :message="t('admin.showcase.list.scopedNotice')"
        />

        <div class="mt-6">
          <p class="max-w-(--measure) text-sm text-text-muted">
            {{ t('admin.showcase.list.placementHint.home_hero') }}
          </p>

          <UiEmptyState
            v-if="!isSettling && rows.length === 0"
            class="mt-6"
            icon="grid"
            :title="t('admin.showcase.list.empty.title')"
            :description="t('admin.showcase.list.empty.description')"
            :action-label="t('admin.showcase.list.new')"
            :action-to="newSlideTo"
          />

          <AdminShowcaseTable
            v-else
            class="mt-4"
            :rows="visibleRows"
            :caption="t('admin.showcase.list.caption')"
            :can-manage="canManageRow"
            :loading="isSettling"
            :busy-id="busyId"
            @move="move"
            @edit="open"
            @duplicate="duplicate"
            @status="setStatus"
          >
            <template #empty>
              <UiEmptyState
                icon="grid"
                compact
                :title="t('admin.showcase.list.emptyPlacement.title')"
                :description="t('admin.showcase.list.emptyPlacement.home_hero')"
                :action-label="t('admin.showcase.list.new')"
                :action-to="newSlideTo"
              />
            </template>
          </AdminShowcaseTable>
        </div>

        <!-- Le déplacement se voit ; il doit aussi s'entendre. -->
        <p role="status" aria-live="polite" class="sr-only">{{ announcement }}</p>
      </template>
    </template>
  </div>
</template>
