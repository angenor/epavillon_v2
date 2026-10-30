<script setup lang="ts">
import type { EditionDetail, EditionTabResult, EditionTrackPayload } from '~/types/admin-events'
import type {
  TrackCompositionEntry,
  TrackMomentPayload,
  TrackPageSettings,
  TrackPartPayload,
} from '~/types/event/track-page'
import type { TrackSessionPlacementPayload } from '~/types/programme/track-page'
import type { EffectivePermission } from '~/types/identity'
import type { AttachableRoleRule, AttachedMedia } from '~/types/media'
import type { TaxonomyTerm } from '~/types/reference'
import type { TrackPageDraft } from '~/utils/track-page'

/**
 * LA PAGE PUBLIQUE D'UN FIL, AU BACK-OFFICE : `/admin/evenements/{édition}/journees/{fil}`.
 *
 * Une page et non le tiroir de l'onglet « Journées spéciales » : huit blocs,
 * des listes ordonnées, trois médias, deux dialogues, et quatre routes
 * distinctes. Le tiroir garde la fiche du fil ; il s'ouvre aussi d'ici.
 *
 * Deux rythmes d'écriture. Le CONTENU (lieu, accroche, publics, médias, encart,
 * partenaires) est un brouillon enregistré d'un geste : médias d'abord, puis
 * `PUT /admin/tracks/{id}/page`. Le DÉROULÉ (parties, moments, placement des
 * activités) part à chaque geste, chaque route rendant les réglages entiers.
 *
 * Règle n° 8 : l'édition est vérifiée contre le périmètre avant tout appel, et
 * un fil d'une autre édition que celle de l'adresse est introuvable.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.events', to: '/admin/evenements' },
    { labelKey: 'admin.event.track.breadcrumb' },
  ],
})

const { t } = useI18n()
const { tr } = useI18nText()
const { date, zoneOf } = useDateTime()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const localePath = useLocalePath()

const eventId = computed(() => String(route.params.id))
const trackId = computed(() => String(route.params.trackId))

await adminScope.ensureLoaded()

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------

const forbidden = ref(false)

interface Screen {
  detail: EditionDetail
  settings: TrackPageSettings
}

const { data: screen, status, error, refresh } = await useAsyncData<Screen | null>(
  () => `admin-track-page-${trackId.value}`,
  async () => {
    forbidden.value = false
    try {
      const [detail, settings] = await Promise.all([
        api.adminEvents.detail(eventId.value, adminScope.scope),
        api.adminTrackPage.read(eventId.value, trackId.value, adminScope.scope),
      ])
      if (!detail || settings.event_id !== eventId.value) return null
      return { detail, settings }
    } catch (thrown) {
      if (thrown instanceof ForbiddenError) {
        forbidden.value = true
        return null
      }
      if (thrown instanceof ApiRequestError && thrown.status === 404) return null
      throw thrown
    }
  },
  { watch: [trackId], lazy: true },
)

const owner = computed(() => trackMediaOwner(trackId.value))

const { data: media, refresh: refreshMedia } = await useAsyncData<AttachedMedia[]>(
  () => `admin-track-media-${trackId.value}`,
  () => api.media.attached(owner.value),
  { default: () => [], lazy: true, server: false },
)

const { data: mediaRules } = await useAsyncData<AttachableRoleRule[]>(
  'media-roles-event-programme-tracks',
  () => api.media.roles('event', 'programme_tracks'),
  { default: () => [], lazy: true },
)

const { data: partnerRoles } = await useAsyncData<TaxonomyTerm[]>(
  'track-partner-roles',
  () => api.adminTrackPage.partnerRoles(),
  { default: () => [], lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-track-page-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canManage = computed(() => hasPermission(granted.value, 'event.event.manage', eventId.value))
const canPlace = computed(() => hasPermission(granted.value, 'programme.session.schedule', eventId.value))

const detail = computed(() => screen.value?.detail ?? null)
const settings = computed(() => screen.value?.settings ?? null)
const track = computed(() => detail.value?.tracks.find((entry) => entry.id === trackId.value) ?? null)
const edition = computed(() => detail.value?.edition ?? null)
const timezone = computed(() => edition.value?.timezone ?? 'UTC')
const zoneLabel = computed(() => (edition.value ? zoneOf(edition.value.city ?? edition.value.timezone) : ''))

useHead(() => ({ title: track.value ? tr(track.value.title) : t('admin.event.track.breadcrumb') }))

const editionTo = computed(() => localePath(`/admin/evenements/${eventId.value}?onglet=journees-speciales`))
const venuesTo = computed(() => localePath(`/admin/evenements/${eventId.value}?onglet=lieux`))
const plannerTo = computed(() => localePath('/admin/programmation'))
// Page publique `pages/track/[edition]/[track].vue` (chemin français, localisé par i18n).
const publicTo = computed(() =>
  edition.value && track.value?.published_at
    ? localePath(`/programmations/${edition.value.slug}/journees/${track.value.slug}`)
    : null,
)

// ---------------------------------------------------------------------------
// Le contenu : un brouillon, un enregistrement
// ---------------------------------------------------------------------------

const draft = ref<TrackPageDraft | null>(null)
const baseline = ref({ page: '', media: '' })

function resetDraft(): void {
  if (!settings.value) return
  draft.value = draftFromSettings(settings.value, media.value)
  baseline.value = { page: pageSignature(draft.value), media: mediaSignature(draft.value) }
}

const pageChanged = computed(() => Boolean(draft.value) && pageSignature(draft.value!) !== baseline.value.page)
const mediaChanged = computed(() => Boolean(draft.value) && mediaSignature(draft.value!) !== baseline.value.media)
const isDirty = computed(() => pageChanged.value || mediaChanged.value)

// Une écriture du déroulé rend aussi les réglages : elle n'efface pas une saisie
// en cours du contenu.
watch([settings, media], () => {
  if (!isDirty.value) resetDraft()
}, { immediate: true })

const saving = ref(false)
const saveError = ref<string | null>(null)
const saved = ref(false)

async function handleFailure(thrown: unknown, target: Ref<string | null>): Promise<void> {
  if (isSessionLost(thrown)) {
    await navigateTo({ path: localePath('auth-login'), query: { redirect: route.fullPath } })
    return
  }
  if (thrown instanceof ForbiddenError) {
    forbidden.value = true
    return
  }
  target.value = apiErrorMessage(thrown, t)
}

async function saveContent(): Promise<void> {
  if (!draft.value || !screen.value) return
  saving.value = true
  saveError.value = null
  saved.value = false
  try {
    // Les médias d'abord : un fichier refusé laisse la page intacte.
    if (mediaChanged.value) {
      await api.media.attach({
        owner_schema: owner.value.schema,
        owner_table: owner.value.table,
        owner_id: trackId.value,
        assignments: mediaAssignments(draft.value),
      })
    }
    const next = pageChanged.value
      ? await api.adminTrackPage.save(eventId.value, trackId.value, payloadFromDraft(draft.value), adminScope.scope)
      : screen.value.settings
    screen.value = { ...screen.value, settings: next }
    await refreshMedia()
    resetDraft()
    saved.value = true
  } catch (thrown) {
    await handleFailure(thrown, saveError)
  } finally {
    saving.value = false
  }
}

onBeforeRouteLeave(() => {
  if (!isDirty.value) return true
  return window.confirm(t('admin.event.track.leaveConfirm'))
})

// ---------------------------------------------------------------------------
// La fiche du fil : le formulaire de l'onglet, dans le même tiroir
// ---------------------------------------------------------------------------

const editingTrack = ref(false)
const trackBusy = ref(false)
const trackError = ref<string | null>(null)

const trackPayload = computed<EditionTrackPayload | null>(() => {
  const current = track.value
  if (!current) return null
  return {
    id: current.id,
    event_id: eventId.value,
    code: current.code,
    slug: current.slug,
    kind: current.kind,
    title: current.title,
    subtitle: current.subtitle,
    description: current.description,
    starts_on: current.starts_on,
    ends_on: current.ends_on,
    color_hex: current.color_hex,
    curated_by: current.curated_by,
    is_published: current.published_at !== null,
    sort_order: current.sort_order,
    theme_codes: current.themes.map((theme) => theme.code),
  }
})

function trackErrorMessage(result: EditionTabResult): string {
  const key = `admin.event.tabs.tracksTab.errors.${result.error_code}`
  const message = t(key)
  return message === key ? (result.error_code ?? '') : message
}

async function saveTrack(payload: EditionTrackPayload): Promise<void> {
  if (!screen.value) return
  trackBusy.value = true
  trackError.value = null
  try {
    const result = await api.adminEvents.saveTrack(payload, adminScope.scope)
    if (!result.ok) {
      trackError.value = trackErrorMessage(result)
      return
    }
    if (result.detail) screen.value = { ...screen.value, detail: result.detail }
    editingTrack.value = false
  } catch (thrown) {
    await handleFailure(thrown, trackError)
  } finally {
    trackBusy.value = false
  }
}

// ---------------------------------------------------------------------------
// Le déroulé : chaque geste part aussitôt
// ---------------------------------------------------------------------------

const agendaBusy = ref(false)
const agendaError = ref<string | null>(null)

async function runAgendaWrite(action: () => Promise<TrackPageSettings>): Promise<void> {
  if (!screen.value) return
  agendaBusy.value = true
  agendaError.value = null
  try {
    const next = await action()
    screen.value = { ...screen.value, settings: next }
  } catch (thrown) {
    await handleFailure(thrown, agendaError)
    if (thrown instanceof ApiRequestError && thrown.status === 404) await refresh()
  } finally {
    agendaBusy.value = false
  }
}

const scope = () => adminScope.scope

function savePart(partId: string | null, payload: TrackPartPayload): Promise<void> {
  return runAgendaWrite(() =>
    partId
      ? api.adminTrackPage.updatePart(eventId.value, trackId.value, partId, payload, scope())
      : api.adminTrackPage.createPart(eventId.value, trackId.value, payload, scope()),
  )
}

/** Échange les rangs de deux parties voisines ; deux rangs égaux sont d'abord distingués. */
function movePart(index: number, delta: -1 | 1): Promise<void> {
  const parts = [...(settings.value?.parts ?? [])].sort((a, b) => a.sort_order - b.sort_order)
  const a = parts[index]
  const b = parts[index + delta]
  if (!a || !b) return Promise.resolve()
  const distinct = a.sort_order !== b.sort_order
  const orderA = distinct ? b.sort_order : (index + delta + 1) * 10
  const orderB = distinct ? a.sort_order : (index + 1) * 10
  const body = (part: typeof a, sort_order: number): TrackPartPayload => ({
    label: part.label,
    title: part.title,
    summary: part.summary,
    sort_order,
  })
  return runAgendaWrite(async () => {
    await api.adminTrackPage.updatePart(eventId.value, trackId.value, a.id, body(a, orderA), scope())
    return api.adminTrackPage.updatePart(eventId.value, trackId.value, b.id, body(b, orderB), scope())
  })
}

function saveMoment(momentId: string | null, payload: TrackMomentPayload): Promise<void> {
  return runAgendaWrite(() =>
    momentId
      ? api.adminTrackPage.updateMoment(eventId.value, trackId.value, momentId, payload, scope())
      : api.adminTrackPage.createMoment(eventId.value, trackId.value, payload, scope()),
  )
}

async function place(entry: TrackCompositionEntry, payload: TrackSessionPlacementPayload): Promise<void> {
  if (!screen.value) return
  agendaBusy.value = true
  agendaError.value = null
  try {
    const placed = await api.adminTrackPage.placeSession(eventId.value, trackId.value, entry.session_id, payload, scope())
    const current = screen.value.settings
    screen.value = {
      ...screen.value,
      settings: {
        ...current,
        sessions: current.sessions.map((s) =>
          s.session_id === placed.session_id
            ? { ...s, part_id: placed.part_id, is_highlight: placed.is_highlight, sort_order: placed.sort_order }
            : s,
        ),
      },
    }
  } catch (thrown) {
    await handleFailure(thrown, agendaError)
  } finally {
    agendaBusy.value = false
  }
}

// ---------------------------------------------------------------------------
// Sommaire
// ---------------------------------------------------------------------------

const SECTIONS = ['place', 'media', 'feature', 'partners', 'agenda'] as const

const periodLabel = computed(() => {
  const current = track.value
  if (!current?.starts_on) return t('admin.event.tabs.tracksTab.periodNone')
  const start = date(`${current.starts_on}T12:00:00Z`, timezone.value)
  if (!current.ends_on || current.ends_on === current.starts_on) return start
  return t('common.datetime.dateRange', { start, end: date(`${current.ends_on}T12:00:00Z`, timezone.value) })
})

const defaultDay = computed(() => track.value?.starts_on ?? detail.value?.period.first_day ?? '')
</script>

<template>
  <div class="mx-auto w-full max-w-6xl">
    <UiForbiddenState
      v-if="forbidden || (!adminScope.isLoading && !adminScope.canAdminister)"
      :required-scope="t('admin.event.list.forbidden.scope')"
      action-to="/admin/evenements"
      :action-label="t('admin.event.form.actions.backToList')"
    />

    <UiErrorState v-else-if="error" :retry-label="t('common.actions.retry')" @retry="refresh()" />

    <UiEmptyState
      v-else-if="(!screen || !track) && status !== 'pending'"
      icon="calendar"
      :title="t('common.states.notFound.title')"
      :description="t('common.states.notFound.description')"
      :action-label="t('admin.event.track.backToEdition')"
      :action-to="editionTo"
    />

    <div v-else-if="!screen || !track || !draft" class="space-y-6">
      <UiSkeletonLoader class="h-24" />
      <UiSkeletonLoader class="h-64" />
      <UiSkeletonLoader class="h-64" />
    </div>

    <template v-else>
      <!-- EN-TÊTE -->
      <header class="flex flex-wrap items-start justify-between gap-x-6 gap-y-4 border-b border-border pb-6">
        <div class="min-w-0">
          <UiButton variant="link" size="sm" icon="arrow-left" :to="editionTo">
            {{ edition ? tr(edition.title) : t('admin.event.track.backToEdition') }}
          </UiButton>
          <div class="mt-2 flex items-start gap-3">
            <span
              class="mt-1.5 h-8 w-1 shrink-0 rounded-full"
              :style="{ backgroundColor: track.color_hex ?? 'var(--color-border)' }"
              aria-hidden="true"
            />
            <div class="min-w-0">
              <h1 class="text-3xl leading-tight font-semibold text-balance">{{ tr(track.title) }}</h1>
              <p v-if="track.subtitle" class="mt-1 text-text-secondary">{{ tr(track.subtitle) }}</p>
            </div>
          </div>
          <div class="mt-3 flex flex-wrap items-center gap-2 text-sm text-text-muted">
            <UiBadge intent="neutral" :label="t('admin.event.tabs.tracksTab.kind.' + track.kind)" />
            <UiBadge
              v-if="track.published_at"
              intent="success"
              :label="t('admin.event.tabs.tracksTab.published', { date: date(track.published_at, timezone) })"
            />
            <UiBadge v-else intent="neutral" :label="t('admin.event.tabs.tracksTab.unpublished')" />
            <span>{{ periodLabel }}</span>
            <span>{{ t('admin.event.tabs.tracksTab.composition', settings?.sessions.length ?? 0) }}</span>
          </div>
        </div>

        <div class="flex flex-wrap gap-2">
          <UiButton v-if="publicTo" variant="secondary" icon-trailing="arrow-up-right" :to="publicTo">
            {{ t('admin.event.track.viewPublic') }}
          </UiButton>
          <UiButton v-if="canManage" variant="secondary" icon="edit" @click="editingTrack = true">
            {{ t('admin.event.track.editTrack') }}
          </UiButton>
        </div>
      </header>

      <UiAlert
        v-if="!track.published_at"
        class="mt-6"
        intent="info"
        :message="t('admin.event.track.unpublishedNotice')"
      />

      <div class="mt-8 grid gap-10 lg:grid-cols-[12rem_minmax(0,1fr)]">
        <!-- Sommaire, grand écran seulement : sur mobile, la page se lit d'un trait. -->
        <nav class="hidden lg:block" :aria-label="t('admin.event.track.toc')">
          <ol class="sticky top-24 space-y-1 border-s border-border text-sm">
            <li v-for="section in SECTIONS" :key="section">
              <a
                :href="`#track-${section}`"
                class="-ms-px block border-s-2 border-transparent py-1.5 ps-3 text-text-muted hover:border-accent hover:text-text"
              >{{ t(`admin.event.track.sections.${section}.title`) }}</a>
            </li>
          </ol>
        </nav>

        <div class="min-w-0 space-y-12">
          <section
            v-for="section in SECTIONS"
            :id="`track-${section}`"
            :key="section"
            class="scroll-mt-24"
            :aria-labelledby="`track-${section}-heading`"
          >
            <h2 :id="`track-${section}-heading`" class="text-xl font-semibold">
              {{ t(`admin.event.track.sections.${section}.title`) }}
            </h2>
            <p class="mt-1 mb-5 max-w-(--measure) text-sm text-text-muted">
              {{ t(`admin.event.track.sections.${section}.intro`) }}
            </p>

            <AdminTracksPresentationSection
              v-if="section === 'place'"
              v-model="draft"
              :venues="screen.detail.venues"
              :venues-to="venuesTo"
              :disabled="!canManage || saving"
            />
            <AdminTracksMediaSection
              v-else-if="section === 'media'"
              v-model="draft"
              :rules="mediaRules"
              :track-id="trackId"
              :disabled="!canManage || saving"
            />
            <AdminTracksFeatureSection
              v-else-if="section === 'feature'"
              v-model="draft"
              :rules="mediaRules"
              :track-id="trackId"
              :disabled="!canManage || saving"
            />
            <AdminTracksPartnersSection
              v-else-if="section === 'partners'"
              v-model="draft"
              :roles="partnerRoles"
              :disabled="!canManage || saving"
            />
            <template v-else-if="settings">
              <UiAlert
                v-if="agendaError"
                class="mb-4"
                intent="danger"
                live
                dismissible
                :message="agendaError"
                @dismiss="agendaError = null"
              />
              <AdminTracksAgendaSection
                :settings="settings"
                :timezone="timezone"
                :zone-label="zoneLabel"
                :default-day="defaultDay"
                :planner-to="plannerTo"
                :can-manage="canManage"
                :can-place="canPlace"
                :busy="agendaBusy"
                :error="agendaError"
                @save-part="savePart"
                @remove-part="(partId: string) => runAgendaWrite(() => api.adminTrackPage.removePart(eventId, trackId, partId, scope()))"
                @move-part="movePart"
                @save-moment="saveMoment"
                @remove-moment="(momentId: string) => runAgendaWrite(() => api.adminTrackPage.removeMoment(eventId, trackId, momentId, scope()))"
                @place="place"
              />
            </template>
          </section>
        </div>
      </div>

      <!-- L'ENREGISTREMENT DU CONTENU, collé au bas de la page : il couvre quatre blocs. -->
      <div
        v-if="canManage && (isDirty || saveError || saved)"
        class="sticky bottom-0 z-30 mt-10 border-t border-border bg-surface-raised py-3"
      >
        <div class="flex flex-wrap items-center justify-end gap-3">
          <UiAlert v-if="saveError" class="w-full" intent="danger" live compact :message="saveError" />
          <p class="me-auto text-sm" :class="isDirty ? 'text-warning' : 'text-success'" aria-live="polite">
            {{ isDirty ? t('admin.event.track.unsaved') : t('admin.event.track.saved') }}
          </p>
          <UiButton v-if="isDirty" variant="ghost" :disabled="saving" @click="resetDraft">
            {{ t('admin.event.track.discard') }}
          </UiButton>
          <UiButton v-if="isDirty" :loading="saving" @click="saveContent">
            {{ t('admin.event.track.save') }}
          </UiButton>
          <UiButton v-else variant="ghost" icon="close" icon-only :label="t('common.actions.close')" @click="saved = false" />
        </div>
      </div>

      <UiDrawer
        :open="editingTrack"
        :title="t('admin.event.tabs.tracksTab.form.editTitle')"
        width="34rem"
        @update:open="(next: boolean) => { if (!next) editingTrack = false }"
      >
        <AdminEventsTrackForm
          v-if="trackPayload"
          :initial="trackPayload"
          :detail="screen.detail"
          :busy="trackBusy"
          :error="trackError"
          @submit="saveTrack"
          @cancel="editingTrack = false"
        />
      </UiDrawer>
    </template>
  </div>
</template>
