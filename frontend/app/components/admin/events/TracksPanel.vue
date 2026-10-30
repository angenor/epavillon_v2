<script setup lang="ts">
import type { EditionDetail, EditionTrack, EditionTrackPayload } from '~/types/admin-events'

/**
 * ONGLET « JOURNÉES SPÉCIALES ».
 *
 * ── CE QUE CET ÉCRAN NE FAIT PAS, ET LE DIT ─────────────────────────────────
 *
 * IL NE COMPOSE PAS LE FIL. Le rattachement d'une activité à une journée spéciale
 * est une décision ÉDITORIALE prise au planificateur (A9), dans
 * `programme.session_tracks` : l'IFDD choisit les activités qui en font partie
 * parmi celles qui ont été retenues. Toutes les activités du 12 novembre ne font
 * pas partie de la Journée finance durable. Le décompte est donc affiché en
 * lecture seule, avec un lien vers le planificateur — sans quoi l'écran laisserait
 * chercher longtemps un bouton qui n'existe pas.
 *
 * LE CONTENU DE LA PAGE PUBLIQUE (30/10) a sa propre page, `…/journees/{fil}` :
 * lieu, médias, encart, partenaires, parties du déroulé, moments sans activité.
 * Trop dense pour un tiroir, et chaque bloc y a sa route.
 *
 * LA PÉRIODE ANNONCÉE N'IMPOSE RIEN. `starts_on` et `ends_on` sont ce qu'on dit au
 * public ; une activité hors de cette plage reste rattachable. Trois raisons
 * inscrites au modèle : un fil n'occupe pas forcément le jour entier, un jour peut
 * en porter deux, un fil peut déborder sur deux jours. Le libellé du champ le dit.
 *
 * LA PAGE PUBLIQUE EST UN INTERRUPTEUR ICI ET UNE DATE EN BASE
 * (`published_at`) : refermer puis rouvrir ne réécrit pas la date de première
 * ouverture. Le formulaire n'a pas à saisir un horodatage pour dire « oui ».
 *
 * LES THÉMATIQUES PARTENT AVEC LE FIL, ET ELLES REMPLACENT LA LISTE. Les
 * omettre de l'enregistrement les efface toutes — c'est la contrepartie de
 * pouvoir en retirer une. Elles viennent de `reference.taxonomy_terms` et
 * arrivent par la composition (`available_themes`) : ni libellé ni couleur
 * n'est écrit ici.
 */

interface Props {
  detail: EditionDetail
  canManage: boolean
  busy?: boolean
  /** Erreur de la dernière écriture, déjà traduite. */
  error?: string | null
  plannerTo: string
  /** Adresse de la page d'un fil au back-office : son contenu public s'y compose. */
  pageTo: (trackId: string) => string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  save: [payload: EditionTrackPayload]
  remove: [trackId: string]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const timezone = computed(() => props.detail.edition.timezone)

// ---------------------------------------------------------------------------
// Formulaire
// ---------------------------------------------------------------------------

const draft = ref<EditionTrackPayload | null>(null)
const isCreation = computed(() => draft.value?.id === null)

function blank(): EditionTrackPayload {
  return {
    id: null,
    event_id: props.detail.edition.id,
    code: '',
    slug: '',
    kind: 'special_day',
    title: { fr: '' },
    subtitle: null,
    description: null,
    // Par défaut, la période de l'édition : c'est le cadre dans lequel un fil se
    // situe, et personne n'annonce une journée spéciale hors de sa COP.
    starts_on: props.detail.period.first_day,
    ends_on: props.detail.period.first_day,
    color_hex: null,
    curated_by: null,
    is_published: false,
    sort_order: (props.detail.tracks.length + 1) * 10,
    theme_codes: [],
  }
}

function openCreate(): void {
  draft.value = blank()
}

function openEdit(track: EditionTrack): void {
  draft.value = {
    id: track.id,
    event_id: props.detail.edition.id,
    code: track.code,
    slug: track.slug,
    kind: track.kind,
    title: track.title,
    subtitle: track.subtitle,
    description: track.description,
    starts_on: track.starts_on,
    ends_on: track.ends_on,
    color_hex: track.color_hex,
    curated_by: track.curated_by,
    is_published: track.published_at !== null,
    sort_order: track.sort_order,
    theme_codes: track.themes.map((theme) => theme.code),
  }
}

/** Fermer le formulaire une fois l'écriture acceptée — pas avant. */
watch(
  () => props.detail.tracks,
  () => {
    if (!props.error) draft.value = null
  },
)

// ---------------------------------------------------------------------------
// Suppression
// ---------------------------------------------------------------------------

const doomed = ref<EditionTrack | null>(null)

function confirmRemove(): void {
  if (doomed.value) emit('remove', doomed.value.id)
  doomed.value = null
}

function periodLabel(track: EditionTrack): string {
  if (!track.starts_on) return t('admin.event.tabs.tracksTab.periodNone')
  const start = date(`${track.starts_on}T12:00:00Z`, timezone.value)
  if (!track.ends_on || track.ends_on === track.starts_on) {
    return t('admin.event.tabs.tracksTab.periodIndicative', { range: start })
  }
  return t('admin.event.tabs.tracksTab.periodIndicative', {
    range: t('common.datetime.dateRange', {
      start,
      end: date(`${track.ends_on}T12:00:00Z`, timezone.value),
    }),
  })
}
</script>

<template>
  <section>
    <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div class="min-w-0">
        <h2 class="font-display text-xl font-semibold">
          {{ t('admin.event.tabs.tracksTab.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.event.tabs.tracksTab.intro') }}
        </p>
      </div>

      <UiButton v-if="props.canManage" icon="plus" :disabled="props.busy" @click="openCreate">
        {{ t('admin.event.tabs.tracksTab.add') }}
      </UiButton>
    </header>

    <UiEmptyState
      v-if="props.detail.tracks.length === 0"
      class="mt-5"
      icon="grid"
      :title="t('admin.event.tabs.tracksTab.empty.title')"
      :description="t('admin.event.tabs.tracksTab.empty.description')"
    />

    <ul v-else class="mt-5 grid gap-4 lg:grid-cols-2">
      <li
        v-for="track in props.detail.tracks"
        :key="track.id"
        class="rounded-lg border border-border bg-surface-raised p-4"
      >
        <div class="flex items-start gap-3">
          <!-- La couleur du fil, en filet vertical : elle identifie le fil sur la
               page publique, elle doit être reconnaissable ici. -->
          <span
            class="mt-0.5 h-10 w-1 shrink-0 rounded-full"
            :style="{ backgroundColor: track.color_hex ?? 'var(--color-border)' }"
            aria-hidden="true"
          />

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <h3 class="min-w-0 truncate font-semibold text-text">{{ tr(track.title) }}</h3>
              <UiBadge
                intent="neutral"
                size="sm"
                :label="t('admin.event.tabs.tracksTab.kind.' + track.kind)"
              />
              <!-- Vert pour ce qui est confirmé : la page publique est ouverte.
                   Gris pour ce qui est clos, ici « pas encore ouvert ». -->
              <UiBadge
                v-if="track.published_at"
                intent="success"
                size="sm"
                :label="t('admin.event.tabs.tracksTab.published', {
                  date: date(track.published_at, timezone),
                })"
              />
              <UiBadge
                v-else
                intent="neutral"
                size="sm"
                :label="t('admin.event.tabs.tracksTab.unpublished')"
              />
            </div>

            <p v-if="track.subtitle" class="mt-1 text-sm text-text-secondary">
              {{ tr(track.subtitle) }}
            </p>

            <dl class="mt-2 space-y-1 text-sm text-text-muted">
              <div>
                <dt class="sr-only">{{ t('admin.event.tabs.tracksTab.form.startsOn') }}</dt>
                <dd>{{ periodLabel(track) }}</dd>
              </div>
              <div>
                <dt class="sr-only">{{ t('admin.event.tabs.tracksTab.form.curatedBy') }}</dt>
                <dd>
                  {{
                    track.curator_name
                      ? t('admin.event.tabs.tracksTab.curator', { name: track.curator_name })
                      : t('admin.event.tabs.tracksTab.curatorNone')
                  }}
                </dd>
              </div>
            </dl>

            <UiThemeTagList
              v-if="track.themes.length > 0"
              class="mt-2"
              :themes="track.themes"
            />

            <!-- LA COMPOSITION EST AILLEURS, ET C'EST DIT ICI. -->
            <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-border pt-3">
              <p class="text-sm font-medium text-text">
                {{ t('admin.event.tabs.tracksTab.composition', track.session_count) }}
              </p>
              <span class="text-xs text-text-subtle">
                {{ t('admin.event.tabs.tracksTab.compositionHint') }}
              </span>
              <UiButton
                variant="link"
                size="sm"
                class="ml-auto"
                :to="props.plannerTo"
              >
                {{ t('admin.event.tabs.tracksTab.openPlanner') }}
              </UiButton>
            </div>

            <UiButton
              class="mt-3"
              variant="secondary"
              size="sm"
              icon="document"
              :to="props.pageTo(track.id)"
            >
              {{ t('admin.event.tabs.tracksTab.editPage') }}
            </UiButton>
          </div>

          <div v-if="props.canManage" class="flex shrink-0 flex-col gap-1">
            <UiButton
              variant="ghost"
              size="sm"
              icon="edit"
              icon-only
              :label="t('common.actions.edit')"
              @click="openEdit(track)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="trash"
              icon-only
              :label="t('admin.event.tabs.tracksTab.remove.action')"
              @click="doomed = track"
            />
          </div>
        </div>
      </li>
    </ul>

    <!-- FORMULAIRE ------------------------------------------------------------>
    <UiDrawer
      :open="draft !== null"
      :title="
        isCreation
          ? t('admin.event.tabs.tracksTab.form.createTitle')
          : t('admin.event.tabs.tracksTab.form.editTitle')
      "
      width="34rem"
      @update:open="(next: boolean) => { if (!next) draft = null }"
    >
      <AdminEventsTrackForm
        v-if="draft"
        :initial="draft"
        :detail="props.detail"
        :busy="props.busy"
        :error="props.error"
        @submit="(payload: EditionTrackPayload) => emit('save', payload)"
        @cancel="draft = null"
      />
    </UiDrawer>

    <!-- SUPPRESSION : la seule de cet onglet qui perd du travail éditorial ----->
    <UiModal
      :open="doomed !== null"
      :title="t('admin.event.tabs.tracksTab.remove.title')"
      :description="t('admin.event.tabs.tracksTab.remove.description')"
      @update:open="(next: boolean) => { if (!next) doomed = null }"
    >
      <UiAlert
        v-if="doomed && doomed.session_count > 0"
        intent="warning"
        :message="t('admin.event.tabs.tracksTab.remove.withSessions', doomed.session_count)"
      />

      <template #footer>
        <UiButton variant="ghost" @click="doomed = null">
          {{ t('admin.event.tabs.confirm.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="props.busy" @click="confirmRemove">
          {{ t('admin.event.tabs.tracksTab.remove.action') }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>
