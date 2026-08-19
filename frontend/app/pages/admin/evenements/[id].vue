<script setup lang="ts">
import type {
  CallFormError,
  CommitteePayload,
  DayGenerationPlan,
  EditionCallPayload,
  EditionChannelPayload,
  EditionCriterion,
  EditionDayPayload,
  EditionDetail,
  EditionFormError,
  EditionFormOptions,
  EditionFormPayload,
  EditionRoomPayload,
  EditionTabKey,
  EditionTabResult,
  EditionTrackPayload,
  EditionVenuePayload,
} from '~/types/admin-events'
import type { EffectivePermission } from '~/types/identity'
import type { TabItem } from '~/types/ui'
import { ForbiddenError } from '~/composables/useApi'

/**
 * FICHE D'UNE ÉDITION — `/admin/evenements/[id]`.
 *
 * SEPT PANNEAUX : la fiche de l'édition, puis les six onglets du prompt — journées
 * du calendrier, journées spéciales, lieux et salles, canal de diffusion, appel à
 * propositions, comité de sélection.
 *
 * ── UNE SEULE COMPOSITION POUR LES SIX ONGLETS ──────────────────────────────
 *
 * `api.adminEvents.detail()` rend les six d'un coup, et chaque écriture rend la
 * composition ENTIÈRE. Le coût est assumé, et il achète une propriété qui vaut
 * cher : les décomptes des cinq autres onglets restent justes. Ajouter une salle
 * change le nombre d'activités plaçables ; retirer un jour détache des séances ;
 * désactiver un canal touche la règle du direct unique. Rendre seulement l'objet
 * modifié laisserait cinq onglets afficher des chiffres faux jusqu'au prochain
 * rechargement.
 *
 * ── L'ONGLET VIT DANS L'URL ─────────────────────────────────────────────────
 *
 * `?onglet=appel`. On se transmet le lien de l'appel d'une COP entre deux
 * personnes de l'équipe, et un rechargement ne doit pas ramener au premier onglet.
 *
 * ── RÈGLE MÉTIER N° 8 ───────────────────────────────────────────────────────
 *
 * `api.adminEvents.detail()` LÈVE pour une édition hors périmètre, y compris quand
 * l'URL est forgée à la main — l'écran rend alors « accès refusé », pas une fiche
 * vide. Une édition inexistante, elle, rend `null` : « cela n'existe pas » et « vous
 * n'y avez pas accès » n'appellent pas le même écran, et les confondre renseignerait
 * sur l'existence de ce qu'on n'a pas le droit de voir.
 *
 * ── DEUX PERMISSIONS, PAS UNE ───────────────────────────────────────────────
 *
 * `event.event.manage` pour l'édition, ses journées, ses fils, ses lieux et ses
 * canaux ; `event.call.manage` pour l'appel et son comité. Ce sont deux permissions
 * distinctes du catalogue, et les tester séparément permet à un chargé de
 * programmation de composer les journées spéciales sans toucher à la grille
 * d'évaluation. L'affichage suit ; le refus appartient à l'API.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.events', to: '/admin/evenements' }],
})

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

const eventId = computed(() => String(route.params.id))

await adminScope.ensureLoaded()

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------

const forbidden = ref(false)

const {
  data: detail,
  status,
  error,
  refresh,
} = await useAsyncData<EditionDetail | null>(
  () => `admin-edition-${eventId.value}`,
  async () => {
    forbidden.value = false
    try {
      return await api.adminEvents.detail(eventId.value, adminScope.scope)
    } catch (thrown) {
      // Hors périmètre : ce n'est pas une panne, c'est un refus. On l'attrape ici
      // pour rendre l'écran « accès refusé » plutôt que l'écran d'erreur.
      if (thrown instanceof ForbiddenError) {
        forbidden.value = true
        return null
      }
      throw thrown
    }
  },
  { watch: [eventId], lazy: true },
)

const { data: options } = await useAsyncData<EditionFormOptions | null>(
  'admin-edition-form-options',
  () => api.adminEvents.formOptions(),
  { lazy: true },
)

const { data: defaultCriteria } = await useAsyncData<EditionCriterion[]>(
  'admin-call-default-criteria',
  () => api.adminEvents.defaultCriteria(),
  { default: () => [], lazy: true },
)

const { data: dayPlan, refresh: refreshPlan } = await useAsyncData<DayGenerationPlan | null>(
  () => `admin-edition-day-plan-${eventId.value}`,
  async () => {
    try {
      return await api.adminEvents.dayPlan(eventId.value, adminScope.scope)
    } catch {
      return null
    }
  },
  { watch: [eventId], lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-edition-detail-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canManageEvent = computed(() =>
  hasPermission(granted.value, 'event.event.manage', eventId.value),
)
const canManageCall = computed(() =>
  hasPermission(granted.value, 'event.call.manage', eventId.value),
)

useHead(() => ({
  title: detail.value ? tr(detail.value.edition.title) : t('admin.event.list.title'),
}))

// ---------------------------------------------------------------------------
// Onglets — portés par l'URL
// ---------------------------------------------------------------------------

/** Les clés d'URL sont en FRANÇAIS : ce sont des adresses qu'on se transmet. */
const TAB_PARAM: Record<string, EditionTabKey> = {
  journees: 'days',
  'journees-speciales': 'tracks',
  lieux: 'venues',
  diffusion: 'channels',
  appel: 'call',
  comite: 'committee',
}
const PARAM_BY_TAB = Object.fromEntries(
  Object.entries(TAB_PARAM).map(([param, key]) => [key, param]),
) as Record<EditionTabKey, string>

const activeTab = computed<EditionTabKey>(
  () => TAB_PARAM[String(route.query.onglet ?? '')] ?? 'days',
)

function setTab(value: string): void {
  const next = { ...route.query }
  const param = PARAM_BY_TAB[value as EditionTabKey]
  if (!param || param === 'journees') delete next.onglet
  else next.onglet = param
  router.replace({ query: next })
}

/**
 * Les compteurs viennent des données, jamais d'une valeur écrite en dur.
 *
 * L'appel n'a PAS de compteur : il y en a zéro ou un, et « Appel 1 » ne renseigne
 * personne. C'est la seule cardinalité 0..1 de cet écran, et l'onglet la rend en
 * n'affichant rien.
 */
const tabs = computed<TabItem[]>(() => [
  {
    value: 'days',
    label: t('admin.event.tabs.days'),
    count: detail.value?.days.length,
  },
  {
    value: 'tracks',
    label: t('admin.event.tabs.tracks'),
    count: detail.value?.tracks.length,
  },
  {
    value: 'venues',
    label: t('admin.event.tabs.venues'),
    count: detail.value?.venues.reduce((sum, venue) => sum + venue.rooms.length, 0),
  },
  {
    value: 'channels',
    label: t('admin.event.tabs.channels'),
    count: detail.value?.channels.filter((channel) => channel.event_id !== null).length,
  },
  { value: 'call', label: t('admin.event.tabs.call') },
  {
    value: 'committee',
    label: t('admin.event.tabs.committee'),
    count: detail.value?.committee.length,
    // Sans appel, il n'y a pas de comité à composer : l'onglet reste visible pour
    // qu'on sache qu'il existe, mais il n'invite pas.
    disabled: Boolean(detail.value) && detail.value?.call === null,
  },
])

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

const busy = ref(false)
const tabError = ref<string | null>(null)
const tabNotice = ref<string | null>(null)
const detachedCount = ref(0)

/** Traduit un code d'erreur d'onglet dans le vocabulaire de l'onglet concerné. */
function tabErrorMessage(section: string, code: string | null): string | null {
  if (!code) return null
  const key = `admin.event.tabs.${section}.errors.${code}`
  const message = t(key)
  return message === key ? code : message
}

async function runTabWrite(
  section: string,
  action: () => Promise<EditionTabResult>,
): Promise<void> {
  busy.value = true
  tabError.value = null
  tabNotice.value = null
  detachedCount.value = 0
  try {
    const result = await action()
    if (!result.ok) {
      tabError.value = tabErrorMessage(section, result.error_code)
      return
    }
    // `deactivated` n'est pas un refus : le canal a servi, il est resté, inactif.
    if (result.error_code === 'deactivated') {
      tabNotice.value = t('admin.event.tabs.channelsTab.deactivated')
    }
    detachedCount.value = result.sessions_detached
    if (result.detail) detail.value = result.detail
    await refreshPlan()
  } finally {
    busy.value = false
  }
}

// -- Fiche de l'édition ----------------------------------------------------

const editingEdition = ref(false)
const editionErrors = ref<EditionFormError[]>([])

/** La fiche, recomposée en charge utile de formulaire. */
const editionPayload = computed<EditionFormPayload | null>(() => {
  const edition = detail.value?.edition
  if (!edition) return null
  return {
    id: edition.id,
    series_id: edition.series_id,
    edition_label: edition.edition_label,
    edition_year: edition.edition_year,
    title: edition.title,
    acronym: edition.acronym,
    slug: edition.slug,
    // La description et le message d'accueil sont portés par le DÉTAIL, pas par la
    // ligne de liste : un tableau de huit colonnes n'a pas à charger deux
    // paragraphes par édition, et le formulaire n'a pas à les demander à part.
    description: detail.value?.description ?? { fr: '' },
    status: edition.status,
    participation_mode: edition.participation_mode,
    timezone: edition.timezone,
    starts_at: edition.starts_at,
    ends_at: edition.ends_at,
    country_id: edition.country_id,
    city: edition.city,
    address: edition.address,
    latitude: edition.latitude,
    longitude: edition.longitude,
    has_pavilion: edition.has_pavilion,
    highlights: detail.value?.highlights ?? null,
    images: {
      banner: detail.value?.images.banner?.asset_id ?? null,
      cover: detail.value?.images.cover?.asset_id ?? null,
      thumbnail: detail.value?.images.thumbnail?.asset_id ?? null,
    },
  }
})

async function saveEdition(payload: EditionFormPayload): Promise<void> {
  busy.value = true
  editionErrors.value = []
  try {
    const result = await api.adminEvents.save(payload, auth.person?.id ?? null, adminScope.scope)
    if (!result.ok) {
      editionErrors.value = result.errors
      return
    }
    editingEdition.value = false
    // Le titre et le fuseau de l'édition alimentent le sélecteur de la tête de
    // page : le laisser en arrière serait afficher deux vérités sur le même écran.
    await adminScope.reload()
    await refresh()
    await refreshPlan()
    if (result.days_created > 0) {
      tabNotice.value = t('admin.event.form.saved.daysAdded', result.days_created)
    }
  } finally {
    busy.value = false
  }
}

// -- Onglets ---------------------------------------------------------------

const callErrors = ref<CallFormError[]>([])
const scoresAffected = ref(false)
const committeeNotice = ref<string | null>(null)
const removedWithAssignments = ref<string[]>([])

async function saveCall(payload: EditionCallPayload): Promise<void> {
  busy.value = true
  callErrors.value = []
  scoresAffected.value = false
  try {
    const result = await api.adminEvents.saveCall(
      payload,
      auth.person?.id ?? null,
      adminScope.scope,
    )
    if (!result.ok) {
      callErrors.value = result.errors
      return
    }
    scoresAffected.value = result.scores_affected
    await refresh()
  } finally {
    busy.value = false
  }
}

async function saveCommittee(payload: CommitteePayload): Promise<void> {
  busy.value = true
  committeeNotice.value = null
  removedWithAssignments.value = []
  try {
    const result = await api.adminEvents.saveCommittee(eventId.value, payload, adminScope.scope)
    if (!result.ok) return
    committeeNotice.value = t('admin.event.tabs.committeeTab.saved')
    removedWithAssignments.value = result.removed_with_assignments.map(
      (entry) => `${entry.full_name} (${entry.assigned_count})`,
    )
    await refresh()
  } finally {
    busy.value = false
  }
}

/** Nombre de jours annoncé par la création, transmis par l'URL. */
const createdDays = computed(() => Number(route.query.jours ?? 0))

const plannerTo = computed(() => localePath('/admin/programmation'))

/**
 * LE VISUEL DU BANDEAU — le 32:9 d'abord, le 16:9 à défaut.
 *
 * Le bandeau occupe toute la largeur d'un écran de travail : c'est la forme du
 * rôle `banner`. Quand il manque, `cover` recadré vaut mieux qu'un en-tête nu —
 * mais `thumbnail` n'entre PAS dans ce repli : un carré étiré sur 1 500 px ne
 * montre plus rien, et l'en-tête sobre est alors préférable.
 */
const heroImage = computed(() => detail.value?.images.banner ?? detail.value?.images.cover ?? null)

const periodLabel = computed(() => {
  const edition = detail.value?.edition
  if (!edition) return ''
  return t('common.datetime.dateRange', {
    start: date(edition.starts_at, edition.timezone),
    end: date(edition.ends_at, edition.timezone),
  })
})
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <!-- ACCÈS REFUSÉ — hors périmètre d'administration. Distinct d'une édition
         inexistante : l'un dit « vous n'avez pas ce droit », l'autre « cela
         n'existe pas ». Les confondre renseignerait sur ce qu'on ne doit pas voir. -->
    <UiForbiddenState
      v-if="forbidden || (!adminScope.isLoading && !adminScope.canAdminister)"
      :required-scope="t('admin.event.list.forbidden.scope')"
      action-to="/admin/evenements"
      :action-label="t('admin.event.form.actions.backToList')"
    />

    <UiErrorState
      v-else-if="error"
      :retry-label="t('common.actions.retry')"
      @retry="refresh()"
    />

    <UiEmptyState
      v-else-if="!detail && status !== 'pending'"
      icon="calendar"
      :title="t('common.states.notFound.title')"
      :description="t('common.states.notFound.description')"
      :action-label="t('admin.event.form.actions.backToList')"
      action-to="/admin/evenements"
    />

    <UiLoadingState v-else-if="!detail" :label="t('common.states.loading.label')" />

    <template v-else>
      <!-- EN-TÊTE DE L'ÉDITION, EN DEUX RENDUS ---------------------------------

           AVEC VISUEL, un bandeau plein cadre : dans un back-office où l'on
           passe d'une COP à un cycle de webinaires vingt fois par jour, une
           image reconnue d'un coup d'œil vaut mieux qu'un titre lu.

           SANS VISUEL, l'en-tête sobre — et surtout PAS le même bandeau sur un
           aplat. Le voile et le fondu ne se posent que sur un média : sur une
           surface de page ils ne séparent rien et ne font que de l'ornement.

           LA HAUTEUR EST BORNÉE PAR LE CONTENU, jamais par le ratio de l'image.
           Un 32:9 pleine largeur mesurerait 420 px sur un grand écran et
           pousserait les onglets hors de vue : c'est une fiche de travail, le
           bandeau la coiffe et ne la remplace pas. `min-h` par palier, et
           `object-cover` recadre ce qui dépasse. -->
      <header
        v-if="heroImage"
        class="relative overflow-hidden rounded-xl bg-surface-inverse text-text-on-inverse"
      >
        <UiImage
          :image="heroImage"
          ratio="auto"
          frame-class="size-full"
          class="absolute inset-0"
          loading="eager"
          sizes="100vw"
        />
        <!-- Le voile en deux temps : l'aplat pour les pastilles et le fil, qui
             se posent haut ; le fondu pour le bloc de titre, en bas. -->
        <div class="pointer-events-none absolute inset-0 bg-scrim/35" aria-hidden="true" />
        <div
          class="scrim-fade-bottom pointer-events-none absolute inset-x-0 bottom-0 h-4/5"
          aria-hidden="true"
        />

        <!-- LE BOUTON EST DANS LE COIN, PAS DANS LE FLUX. En élément flex, il
             passait sous le titre dès que celui-ci tenait deux lignes — c'est-à-dire
             presque toujours, un intitulé de COP faisant soixante caractères. -->
        <div class="relative flex min-h-52 flex-col justify-end p-5 sm:min-h-60 sm:p-7 lg:min-h-64">
          <div class="min-w-0 pe-0 sm:pe-44">
            <p class="flex flex-wrap items-center gap-x-2 text-sm text-text-on-inverse">
              <span v-if="detail.edition.series_name">{{ tr(detail.edition.series_name) }}</span>
              <span v-if="detail.edition.edition_label" class="font-mono">
                {{ detail.edition.edition_label }}
              </span>
              <span>{{ detail.edition.edition_year }}</span>
            </p>

            <h1 class="mt-1 text-3xl leading-tight font-semibold text-balance text-text-on-inverse">
              {{ tr(detail.edition.title) }}
            </h1>

            <p
              class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-text-on-inverse-muted"
            >
              <span>{{ periodLabel }}</span>
              <span>
                {{ t('common.datetime.zoneOf', {
                  zone: detail.edition.city ?? detail.edition.timezone,
                }) }}
              </span>
              <span v-if="detail.edition.city">
                {{ detail.edition.city
                }}<template v-if="detail.edition.country_name">,
                  {{ tr(detail.edition.country_name) }}</template>
              </span>
              <span v-else>{{ t('admin.event.list.cell.online') }}</span>
            </p>

            <!-- LES PASTILLES GARDENT LEUR DESSIN CLAIR, et c'est délibéré :
                 leur fond est OPAQUE, donc lisible sur n'importe quelle
                 photographie, et leur code de couleur est le même partout dans
                 le back-office. Un jeu de pastilles propre à cet écran serait
                 un second vocabulaire d'état à retenir. -->
            <div class="mt-3 flex flex-wrap items-center gap-2">
              <UiBadge
                :intent="detail.edition.status === 'ongoing' ? 'warning' : 'info'"
                :label="t('admin.event.list.status.' + detail.edition.status)"
              />
              <UiBadge
                :intent="detail.edition.has_pavilion ? 'info' : 'neutral'"
                :label="t(detail.edition.has_pavilion
                  ? 'admin.event.list.cell.pavilion'
                  : 'admin.event.list.cell.noPavilion')"
              />
              <UiBadge
                v-if="detail.edition.programme_published_at"
                intent="success"
                :label="t('admin.event.list.programme.published', {
                  date: dateTime(detail.edition.programme_published_at, detail.edition.timezone),
                })"
              />
              <UiBadge v-else intent="neutral" :label="t('admin.event.list.programme.unpublished')" />
            </div>
          </div>

          <!-- LE VERRE, ICI, EST DANS SON SEUL CAS AUTORISÉ : une commande posée
               sur un média. Un bouton d'interface ordinaire, dessiné pour une
               surface claire, deviendrait illisible dès que la photographie
               change — et elle change à chaque édition. -->
          <button
            v-if="canManageEvent && !editingEdition"
            type="button"
            class="mt-4 inline-flex min-h-(--target-min) cursor-pointer items-center gap-2 self-start rounded-md border border-glass-border bg-glass px-4 text-sm font-semibold text-text-on-inverse shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover sm:absolute sm:end-7 sm:top-7 sm:mt-0"
            @click="editingEdition = true"
          >
            <UiIcon name="edit" size="1rem" />
            {{ t('admin.event.form.editTitle') }}
          </button>
        </div>
      </header>

      <!-- SANS VISUEL : l'en-tête sobre, sur la surface de page. -->
      <header v-else class="flex flex-wrap items-start justify-between gap-x-6 gap-y-4">
        <div class="min-w-0">
          <p class="flex flex-wrap items-center gap-x-2 text-sm text-text-muted">
            <span v-if="detail.edition.series_name">{{ tr(detail.edition.series_name) }}</span>
            <span v-if="detail.edition.edition_label" class="font-mono">
              {{ detail.edition.edition_label }}
            </span>
            <span>{{ detail.edition.edition_year }}</span>
          </p>

          <h1 class="mt-1 text-3xl leading-tight font-semibold text-balance">
            {{ tr(detail.edition.title) }}
          </h1>

          <p class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-text-secondary">
            <span>{{ periodLabel }}</span>
            <span class="text-text-subtle">
              {{ t('common.datetime.zoneOf', {
                zone: detail.edition.city ?? detail.edition.timezone,
              }) }}
            </span>
            <span v-if="detail.edition.city">
              {{ detail.edition.city
              }}<template v-if="detail.edition.country_name">,
                {{ tr(detail.edition.country_name) }}</template>
            </span>
            <span v-else>{{ t('admin.event.list.cell.online') }}</span>
          </p>

          <div class="mt-3 flex flex-wrap items-center gap-2">
            <UiBadge
              :intent="detail.edition.status === 'ongoing' ? 'warning' : 'info'"
              :label="t('admin.event.list.status.' + detail.edition.status)"
            />
            <UiBadge
              :intent="detail.edition.has_pavilion ? 'info' : 'neutral'"
              :label="t(detail.edition.has_pavilion
                ? 'admin.event.list.cell.pavilion'
                : 'admin.event.list.cell.noPavilion')"
            />
            <UiBadge
              v-if="detail.edition.programme_published_at"
              intent="success"
              :label="t('admin.event.list.programme.published', {
                date: dateTime(detail.edition.programme_published_at, detail.edition.timezone),
              })"
            />
            <UiBadge
              v-else
              intent="neutral"
              :label="t('admin.event.list.programme.unpublished')"
            />
          </div>
        </div>

        <UiButton
          v-if="canManageEvent && !editingEdition"
          variant="secondary"
          icon="edit"
          @click="editingEdition = true"
        >
          {{ t('admin.event.form.editTitle') }}
        </UiButton>
      </header>

      <!-- Ce que la création vient de faire au calendrier, annoncé en arrivant. -->
      <UiAlert
        v-if="createdDays > 0"
        class="mt-6"
        intent="success"
        live
        dismissible
        :message="t('admin.event.form.saved.created', { days: createdDays })"
        @dismiss="router.replace({ query: { ...route.query, jours: undefined } })"
      />

      <UiAlert
        v-if="tabNotice"
        class="mt-6"
        intent="success"
        live
        dismissible
        :message="tabNotice"
        @dismiss="tabNotice = null"
      />

      <!-- LES CONSÉQUENCES `ON DELETE SET NULL`, DITES À VOIX HAUTE. Silencieuses
           en base ; l'écran les chiffre. -->
      <UiAlert
        v-if="detachedCount > 0"
        class="mt-4"
        intent="warning"
        live
        dismissible
        :message="t('admin.event.tabs.detachedNotice', detachedCount)"
        @dismiss="detachedCount = 0"
      />

      <!-- FICHE DE L'ÉDITION, EN MODIFICATION ---------------------------------->
      <section v-if="editingEdition" class="mt-8">
        <h2 class="sr-only">{{ t('admin.event.form.editTitle') }}</h2>
        <AdminEventsEditionForm
          v-if="options && editionPayload"
          :initial="editionPayload"
          :options="options"
          :images="detail.images"
          :errors="editionErrors"
          :busy="busy"
          @submit="saveEdition"
          @cancel="editingEdition = false"
        />
        <UiLoadingState v-else :label="t('common.states.loading.label')" />
      </section>

      <!-- LES SIX ONGLETS ------------------------------------------------------->
      <template v-else>
        <UiTabs
          class="mt-8"
          :items="tabs"
          :model-value="activeTab"
          :label="t('admin.event.tabs.label')"
          @update:model-value="setTab"
        />

        <div class="mt-6">
          <AdminEventsDaysPanel
            v-if="activeTab === 'days'"
            :detail="detail"
            :plan="dayPlan ?? null"
            :can-manage="canManageEvent"
            :busy="busy"
            @generate="(remove) => runTabWrite('daysTab', () =>
              api.adminEvents.generateDays(eventId, remove, adminScope.scope))"
            @save-day="(payload: EditionDayPayload) => runTabWrite('daysTab', () =>
              api.adminEvents.saveDay(eventId, payload, adminScope.scope))"
          />

          <AdminEventsTracksPanel
            v-else-if="activeTab === 'tracks'"
            :detail="detail"
            :can-manage="canManageEvent"
            :busy="busy"
            :error="tabError"
            :planner-to="plannerTo"
            @save="(payload: EditionTrackPayload) => runTabWrite('tracksTab', () =>
              api.adminEvents.saveTrack(payload, adminScope.scope))"
            @remove="(trackId: string) => runTabWrite('tracksTab', () =>
              api.adminEvents.removeTrack(eventId, trackId, adminScope.scope))"
          />

          <AdminEventsVenuesPanel
            v-else-if="activeTab === 'venues'"
            :detail="detail"
            :can-manage="canManageEvent"
            :busy="busy"
            :error="tabError"
            @save-venue="(payload: EditionVenuePayload) => runTabWrite('venuesTab', () =>
              api.adminEvents.saveVenue(payload, adminScope.scope))"
            @remove-venue="(venueId: string) => runTabWrite('venuesTab', () =>
              api.adminEvents.removeVenue(eventId, venueId, adminScope.scope))"
            @save-room="(payload: EditionRoomPayload) => runTabWrite('venuesTab', () =>
              api.adminEvents.saveRoom(eventId, payload, adminScope.scope))"
            @remove-room="(roomId: string) => runTabWrite('venuesTab', () =>
              api.adminEvents.removeRoom(eventId, roomId, adminScope.scope))"
          />

          <AdminEventsChannelsPanel
            v-else-if="activeTab === 'channels'"
            :detail="detail"
            :can-manage="canManageEvent"
            :busy="busy"
            :error="tabError"
            :notice="tabNotice"
            @save="(payload: EditionChannelPayload) => runTabWrite('channelsTab', () =>
              api.adminEvents.saveChannel(payload, adminScope.scope))"
            @remove="(channelId: string) => runTabWrite('channelsTab', () =>
              api.adminEvents.removeChannel(eventId, channelId, adminScope.scope))"
          />

          <AdminEventsCallPanel
            v-else-if="activeTab === 'call'"
            :detail="detail"
            :can-manage="canManageCall"
            :busy="busy"
            :errors="callErrors"
            :default-criteria="defaultCriteria"
            :scores-affected="scoresAffected"
            @save="saveCall"
          />

          <AdminEventsCommitteePanel
            v-else
            :detail="detail"
            :can-manage="canManageCall"
            :busy="busy"
            :notice="committeeNotice"
            :removed-with-assignments="removedWithAssignments"
            @save="saveCommittee"
          />
        </div>
      </template>
    </template>
  </div>
</template>
