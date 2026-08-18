<script setup lang="ts">
import type {
  DayGenerationPlan,
  EditionDay,
  EditionDayPayload,
  EditionDetail,
} from '~/types/admin-events'
import type { I18nText } from '~/types/shared'
import type { TableColumn } from '~/types/ui'

/**
 * ONGLET « JOURNÉES DU CALENDRIER ».
 *
 * LE CALENDRIER SE GÉNÈRE, IL NE SE SAISIT PAS. Une ligne par date de la période,
 * dérivée des dates de l'édition. Mais la génération est un GESTE EXPLICITE et non
 * un effet de bord : `event.event_days` ne porte aucun trigger de dérivation, et
 * retirer un jour détache les séances qu'il portait
 * (`xmod_fk_sessions_event_day ON DELETE SET NULL`). On annonce donc le plan —
 * combien de jours à créer, lesquels sortent de la période, combien de séances ils
 * portent — avant d'agir.
 *
 * LES JOURS HORS PÉRIODE NE SONT PAS SUPPRIMÉS D'OFFICE. Une édition garde
 * parfois une soirée d'ouverture la veille de son premier jour officiel : le
 * retrait est un choix de l'équipe, marqué d'une case à cocher, jamais une
 * conséquence silencieuse d'un changement de dates.
 *
 * CE QUI EST ÉDITORIAL SURVIT À LA RÉGÉNÉRATION : titre, page dédiée, description,
 * mise en avant, couleur. Le générateur n'écrit que la date et le rang — inventer
 * « Jour 3 » produirait un titre que personne n'a écrit et qui s'afficherait tel
 * quel sur la page publique.
 */

interface Props {
  detail: EditionDetail
  plan: DayGenerationPlan | null
  canManage: boolean
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  generate: [removeOutsidePeriod: boolean]
  saveDay: [payload: EditionDayPayload]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const timezone = computed(() => props.detail.edition.timezone)

const columns = computed<TableColumn[]>(() => [
  { key: 'day_date', label: t('admin.event.tabs.daysTab.columns.date'), width: '14rem' },
  { key: 'title', label: t('admin.event.tabs.daysTab.columns.title') },
  { key: 'slug', label: t('admin.event.tabs.daysTab.columns.slug'), hideBelow: 'lg' },
  {
    key: 'session_count',
    label: t('admin.event.tabs.daysTab.columns.sessions'),
    numeric: true,
    align: 'end',
    width: '8rem',
  },
  { key: 'actions', label: t('admin.event.tabs.daysTab.columns.actions'), width: '7rem' },
])

/** Une date civile s'affiche sans fuseau : c'est un jour, pas un instant. */
function dayLabel(day: EditionDay): string {
  return date(`${day.day_date}T12:00:00Z`, timezone.value)
}

// ---------------------------------------------------------------------------
// Génération
// ---------------------------------------------------------------------------

const planOpen = ref(false)
const removeOutside = ref(false)

function confirmGenerate(): void {
  emit('generate', removeOutside.value)
  planOpen.value = false
  removeOutside.value = false
}

/** Rien à faire : ni jour à créer, ni jour hors période. Le bouton reste, inerte. */
const nothingToDo = computed(
  () => (props.plan?.to_create.length ?? 0) === 0 && (props.plan?.to_review.length ?? 0) === 0,
)

/**
 * UNE PÉRIODE ANORMALEMENT LONGUE.
 *
 * `event.event_days` est le calendrier d'une conférence : une COP en compte
 * douze. Un cycle de webinaires étalé sur l'année en produirait trois cents, ce
 * qui n'a aucun sens — chaque ligne serait un jour sans activité. Le modèle
 * n'interdit rien, et il a raison de ne rien interdire : la période est légitime,
 * c'est la génération jour par jour qui ne l'est pas. On avertit donc avant de
 * confirmer, plutôt que d'écrire trois cents lignes sans un mot.
 *
 * Le seuil est celui d'une conférence longue, segment de haut niveau compris.
 */
const LONG_PERIOD_DAYS = 40
const isUnusuallyLong = computed(() => (props.plan?.to_create.length ?? 0) > LONG_PERIOD_DAYS)

// ---------------------------------------------------------------------------
// Édition d'une journée
// ---------------------------------------------------------------------------

const editing = ref<EditionDay | null>(null)
const draft = ref<EditionDayPayload | null>(null)

function openDay(day: EditionDay): void {
  editing.value = day
  draft.value = {
    id: day.id,
    title: day.title,
    slug: day.slug,
    description: day.description,
    is_featured: day.is_featured,
    color_hex: day.color_hex,
  }
}

function submitDay(): void {
  if (draft.value) emit('saveDay', { ...draft.value })
  editing.value = null
  draft.value = null
}

/** La palette du guide de style : on choisit une couleur, on ne la tape pas. */
const PALETTE = ['#0B6C9E', '#1F5F8B', '#8C6D1F', '#2E6B45', '#7A3E6B', '#8B3A2E']

function setColor(value: string | null): void {
  if (draft.value) draft.value.color_hex = value
}

function setTitle(value: I18nText | null): void {
  if (draft.value) draft.value.title = value
}

function setDescription(value: I18nText | null): void {
  if (draft.value) draft.value.description = value
}
</script>

<template>
  <section>
    <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div class="min-w-0">
        <h2 class="font-display text-xl font-semibold">
          {{ t('admin.event.tabs.daysTab.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.event.tabs.daysTab.intro') }}
        </p>
      </div>

      <UiButton
        v-if="props.canManage"
        variant="secondary"
        icon="calendar"
        :disabled="props.busy || nothingToDo"
        @click="planOpen = true"
      >
        {{ t('admin.event.tabs.daysTab.generate') }}
      </UiButton>
    </header>

    <UiTable
      class="mt-5"
      :columns="columns"
      :rows="props.detail.days"
      row-key="id"
      row-label-key="day_date"
      :caption="t('admin.event.tabs.daysTab.title')"
      visually-hidden-caption
      :loading="props.busy"
    >
      <template #cell-day_date="{ row }">
        <div class="flex items-center gap-2">
          <!-- La couleur du jour, quand elle existe : un carré, pas un fond de
               ligne. Le fond colorierait une ligne entière pour une donnée
               d'habillage. -->
          <span
            v-if="row.color_hex"
            class="size-3 shrink-0 rounded-sm border border-border"
            :style="{ backgroundColor: row.color_hex }"
            aria-hidden="true"
          />
          <div class="min-w-0">
            <p class="text-sm font-medium text-text">{{ dayLabel(row) }}</p>
            <div class="mt-0.5 flex flex-wrap gap-1">
              <UiBadge
                v-if="row.is_featured"
                intent="info"
                size="sm"
                :label="t('admin.event.tabs.daysTab.featuredBadge')"
              />
              <!-- Hors période : jaune, ce qui demande attention. Ni une réussite,
                   ni une erreur — un jour qu'on garde ou qu'on retire. -->
              <UiBadge
                v-if="row.is_outside_period"
                intent="warning"
                size="sm"
                :label="t('admin.event.tabs.daysTab.outsideBadge')"
              />
            </div>
          </div>
        </div>
      </template>

      <template #cell-title="{ row }">
        <span v-if="row.title" class="text-sm text-text">{{ tr(row.title) }}</span>
        <span v-else class="text-sm text-text-subtle">
          {{ t('admin.event.tabs.daysTab.noTitle') }}
        </span>
      </template>

      <template #cell-slug="{ row }">
        <code v-if="row.slug" class="font-mono text-xs text-text-muted">{{ row.slug }}</code>
        <span v-else class="text-xs text-text-subtle">
          {{ t('admin.event.tabs.daysTab.noSlug') }}
        </span>
      </template>

      <template #cell-session_count="{ row }">
        <span class="font-mono text-sm tabular-nums">{{ row.session_count }}</span>
      </template>

      <template #cell-actions="{ row }">
        <UiButton
          v-if="props.canManage"
          variant="ghost"
          size="sm"
          icon="edit"
          icon-only
          :label="t('common.actions.edit')"
          @click="openDay(row)"
        />
      </template>

      <template #empty>
        <UiEmptyState
          icon="calendar"
          :title="t('admin.event.tabs.daysTab.empty.title')"
          :description="t('admin.event.tabs.daysTab.empty.description')"
        />
      </template>
    </UiTable>

    <!-- LE PLAN DE GÉNÉRATION, ANNONCÉ AVANT D'AGIR ---------------------------->
    <UiModal
      v-model:open="planOpen"
      :title="t('admin.event.tabs.daysTab.plan.title')"
      size="lg"
    >
      <div v-if="props.plan" class="space-y-4">
        <ul class="space-y-1 text-sm">
          <li>{{ t('admin.event.tabs.daysTab.plan.create', props.plan.to_create.length) }}</li>
          <li class="text-text-muted">
            {{ t('admin.event.tabs.daysTab.plan.unchanged', props.plan.unchanged) }}
          </li>
        </ul>

        <UiAlert
          v-if="isUnusuallyLong"
          intent="warning"
          :title="t('admin.event.tabs.daysTab.plan.longPeriod.title', props.plan.to_create.length)"
          :message="t('admin.event.tabs.daysTab.plan.longPeriod.description')"
        />

        <div v-if="props.plan.to_review.length > 0" class="rounded-md border border-warning-border bg-warning-surface p-4">
          <p class="text-sm font-semibold text-warning">
            {{ t('admin.event.tabs.daysTab.plan.outside', props.plan.to_review.length) }}
          </p>
          <ul class="mt-2 space-y-0.5 text-sm text-text-secondary">
            <li v-for="stale in props.plan.to_review" :key="stale.id">
              {{ t('admin.event.tabs.daysTab.plan.outsideDetail', {
                date: date(`${stale.day_date}T12:00:00Z`, timezone),
                count: stale.session_count,
              }, stale.session_count) }}
            </li>
          </ul>

          <UiCheckbox
            :model-value="removeOutside"
            class="mt-3"
            :label="t('admin.event.tabs.daysTab.plan.removeOutside')"
            :hint="t('admin.event.tabs.daysTab.plan.removeWarning')"
            @update:model-value="(next: boolean) => (removeOutside = next)"
          />
        </div>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="planOpen = false">
          {{ t('admin.event.tabs.confirm.cancel') }}
        </UiButton>
        <UiButton :loading="props.busy" @click="confirmGenerate">
          {{ t('admin.event.tabs.confirm.confirm') }}
        </UiButton>
      </template>
    </UiModal>

    <!-- CONTENU ÉDITORIAL D'UNE JOURNÉE --------------------------------------->
    <UiModal
      :open="editing !== null"
      :title="editing ? t('admin.event.tabs.daysTab.edit.title', { date: dayLabel(editing) }) : ''"
      size="lg"
      @update:open="(next: boolean) => { if (!next) { editing = null; draft = null } }"
    >
      <div v-if="draft" class="space-y-5">
        <AdminEventsI18nField
          :model-value="draft.title"
          :label="t('admin.event.tabs.daysTab.edit.titleField')"
          @update:model-value="setTitle"
        />

        <UiInput
          :model-value="draft.slug ?? ''"
          :label="t('admin.event.tabs.daysTab.edit.slugField')"
          :hint="t('admin.event.tabs.daysTab.edit.slugHint', {
            event: props.detail.edition.slug,
            slug: draft.slug || 'journee',
          })"
          :maxlength="120"
          @update:model-value="(next: string) => (draft!.slug = next || null)"
        />

        <AdminEventsI18nField
          :model-value="draft.description"
          :label="t('admin.event.tabs.daysTab.edit.descriptionField')"
          multiline
          :rows="3"
          @update:model-value="setDescription"
        />

        <UiSwitch
          :model-value="draft.is_featured"
          :label="t('admin.event.tabs.daysTab.edit.featured')"
          @update:model-value="(next: boolean) => (draft!.is_featured = next)"
        />

        <fieldset>
          <legend class="mb-2 text-sm font-bold text-text">
            {{ t('admin.event.tabs.daysTab.edit.color') }}
          </legend>
          <div class="flex flex-wrap items-center gap-2">
            <button
              v-for="color in PALETTE"
              :key="color"
              type="button"
              class="size-(--target-compact) cursor-pointer rounded-md border-2 transition-transform hover:scale-105"
              :class="draft.color_hex === color ? 'border-text' : 'border-border'"
              :style="{ backgroundColor: color }"
              :aria-pressed="draft.color_hex === color"
              :aria-label="color"
              @click="setColor(color)"
            />
            <UiButton variant="ghost" size="sm" @click="setColor(null)">
              {{ t('admin.event.tabs.daysTab.edit.colorNone') }}
            </UiButton>
          </div>
        </fieldset>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="editing = null; draft = null">
          {{ t('admin.event.tabs.confirm.cancel') }}
        </UiButton>
        <UiButton :loading="props.busy" @click="submitDay">
          {{ t('common.actions.save') }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>
