<script setup lang="ts">
import type {
  EditionErrorCode,
  EditionFormError,
  EditionFormOptions,
  EditionFormPayload,
} from '~/types/admin-events'
import type { AttachedImage, EditionImageRole } from '~/types/media'
import type { ParticipationMode } from '~/types/event/edition'
import type { SelectOption } from '~/types/ui'

/**
 * CRÉATION ET ÉDITION D'UNE ÉDITION — un seul formulaire pour les deux.
 *
 * Les deux écrans saisissent exactement les mêmes dix-huit champs et se heurtent
 * aux mêmes contraintes. Deux composants auraient divergé dès le premier champ
 * ajouté, et le message de `ux_events_series_edition` n'aurait été écrit que d'un
 * côté.
 *
 * ── LE FUSEAU EST SAISI AVANT LES DATES, ET CE N'EST PAS UN DÉTAIL DE MISE EN PAGE
 *
 * Les dates sont des heures MURALES : « le 9 novembre à 9 h » vaut À BELÉM. Le
 * contrôle natif `datetime-local` ne connaît pas les fuseaux, la conversion en
 * instant appartient donc à cet écran — et elle a besoin du fuseau. Le placer
 * après les dates ferait ressaisir celles-ci à chaque changement. Changer le
 * fuseau CONSERVE l'heure murale : on corrige un fuseau parce qu'on s'est trompé
 * de fuseau, pas pour décaler la COP de trois heures.
 *
 * ── CE QUI SE MONTRE AVANT D'ÊTRE REFUSÉ ────────────────────────────────────
 *
 * Le pays et la ville deviennent obligatoires dès que le mode n'est pas « en
 * ligne » (`ck_events_physical_location`). L'astérisque apparaît donc à la volée,
 * sans attendre l'envoi : la contrainte est connue, la dire après coup serait un
 * choix.
 *
 * ── LES VISUELS ─────────────────────────────────────────────────────────────
 *
 * `event.events` NE PORTE PAS ses images : le rattachement média est polymorphe
 * (`media.attachments`). Depuis le 19/08 l'édition en porte TROIS — 32:9, 16:9,
 * 1:1 — et leur dessin appartient à `AdminEventsImagesPanel`, non à ce
 * formulaire déjà long. Ce fichier ne fait que leur passer les valeurs.
 */

interface Props {
  /** Valeurs de départ. Absentes à la création. */
  initial?: EditionFormPayload | null
  options: EditionFormOptions
  /** Les trois déclinaisons déjà rattachées, pour l'aperçu de chaque emplacement. */
  images?: Record<EditionImageRole, AttachedImage | null> | null
  errors: EditionFormError[]
  busy?: boolean
  /** Vrai en création : le statut est alors figé à « brouillon ». */
  isCreation?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ submit: [payload: EditionFormPayload]; cancel: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()

const MODES: ParticipationMode[] = ['in_person', 'hybrid', 'online']

/** Valeurs de départ d'une création : une édition part toujours en brouillon. */
function blank(): EditionFormPayload {
  const year = new Date().getUTCFullYear() + 1
  return {
    id: null,
    series_id: null,
    edition_label: null,
    edition_year: year,
    title: { fr: '' },
    acronym: null,
    slug: '',
    description: { fr: '' },
    status: 'draft',
    participation_mode: 'hybrid',
    timezone: 'Europe/Paris',
    starts_at: '',
    ends_at: '',
    country_id: null,
    city: null,
    address: null,
    latitude: null,
    longitude: null,
    has_pavilion: false,
    highlights: null,
    images: { banner: null, cover: null, thumbnail: null },
  }
}

const form = ref<EditionFormPayload>(props.initial ? { ...props.initial } : blank())

watch(
  () => props.initial,
  (next) => {
    if (next) form.value = { ...next }
  },
)

// ---------------------------------------------------------------------------
// Le slug, proposé pendant la frappe
// ---------------------------------------------------------------------------

/**
 * L'IDENTIFIANT D'ADRESSE SUIT LE LIBELLÉ, JUSQU'À CE QU'ON Y TOUCHE.
 *
 * Il se compose par `slugify()`, qui rejoue `platform.slugify()` à la lettre : le
 * front et la base tombent d'accord sur la forme, la base restant seule juge de
 * l'unicité (`ux_events_slug`).
 *
 * LA SOURCE EST LE LIBELLÉ D'ÉDITION, PAS LE TITRE. « COP31 » donne
 * `cop31` ; le titre complet donnerait
 * `cop31-conference-des-nations-unies-sur-les-changements-climatiques`, qu'aucune
 * adresse publique ne veut porter. À défaut de libellé, on retombe sur le titre —
 * un rendez-vous ponctuel n'en a pas.
 *
 * DEUX GARDE-FOUS, et chacun répare une faute qu'on ne voit qu'à l'usage :
 *
 *  · DÈS QUE LA PERSONNE ÉDITE LE SLUG, il cesse de suivre. Sans cela, corriger
 *    l'adresse puis rectifier une faute de frappe dans le libellé écraserait la
 *    correction, sous les doigts.
 *  · EN MODIFICATION, IL NE SUIT JAMAIS. Le slug d'une édition publiée est une
 *    adresse que des courriels, des programmes et des moteurs de recherche
 *    portent déjà : la renommer parce qu'on corrige un accent dans le libellé
 *    romprait tous ces liens en silence.
 */
const slugTouched = ref(!props.isCreation)

/** Ce dont le slug se compose : le libellé s'il existe, le titre sinon. */
const slugSource = computed(() => form.value.edition_label?.trim() || form.value.title.fr || '')

watch(slugSource, (source) => {
  if (slugTouched.value) return
  form.value.slug = slugify(source)
})

function setSlug(next: string): void {
  // La saisie manuelle est respectée telle quelle — on ne « corrige » pas sous les
  // doigts. La forme attendue est rappelée par l'aide du champ, et la base
  // tranchera.
  slugTouched.value = true
  form.value.slug = next
}

/** Reprendre la proposition automatique après l'avoir écartée. */
function resetSlug(): void {
  slugTouched.value = false
  form.value.slug = slugify(slugSource.value)
}

const slugIsAuto = computed(() => !slugTouched.value && form.value.slug.length > 0)

// ---------------------------------------------------------------------------
// Dates : heures murales dans le fuseau de l'édition
// ---------------------------------------------------------------------------

/**
 * Les deux bornes en heure murale, tenues à part de l'instant.
 *
 * On ne dérive pas la valeur du champ depuis `form.starts_at` à chaque frappe :
 * une saisie partielle (« 2027-11-0 ») ne donne aucun instant, et le champ se
 * viderait sous les doigts. On tient donc l'heure murale comme état de saisie, et
 * l'on ne compose l'instant que lorsqu'elle est complète.
 */
const wall = ref({
  starts_at: props.initial?.starts_at
    ? wallClockInZone(props.initial.starts_at, props.initial.timezone).replace(' ', 'T')
    : '',
  ends_at: props.initial?.ends_at
    ? wallClockInZone(props.initial.ends_at, props.initial.timezone).replace(' ', 'T')
    : '',
})

watch(
  () => props.initial,
  (next) => {
    if (!next) return
    wall.value = {
      starts_at: next.starts_at ? wallClockInZone(next.starts_at, next.timezone).replace(' ', 'T') : '',
      ends_at: next.ends_at ? wallClockInZone(next.ends_at, next.timezone).replace(' ', 'T') : '',
    }
  },
)

function setWall(key: 'starts_at' | 'ends_at', value: string): void {
  wall.value[key] = value
  form.value[key] = instantFromWallClock(value, form.value.timezone) ?? ''
}

/**
 * Changer de fuseau CONSERVE l'heure murale et recompose l'instant. « 9 h » reste
 * « 9 h » : on corrige un fuseau mal choisi, on ne déplace pas la conférence.
 */
watch(
  () => form.value.timezone,
  (zone) => {
    for (const key of ['starts_at', 'ends_at'] as const) {
      if (wall.value[key]) form.value[key] = instantFromWallClock(wall.value[key], zone) ?? ''
    }
  },
)

const zoneLabel = computed(() => {
  const option = props.options.timezones.find((entry) => entry.value === form.value.timezone)
  return t('common.datetime.zoneOf', {
    zone: option?.city ?? timeZoneCityLabel(form.value.timezone),
  })
})

// ---------------------------------------------------------------------------
// Options des sélecteurs
// ---------------------------------------------------------------------------

const seriesOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.event.form.fields.seriesNone') },
  ...props.options.series
    .filter((entry) => entry.is_active || entry.id === form.value.series_id)
    .map((entry) => ({
      value: entry.id,
      label: tr(entry.name),
      description: t('admin.event.list.seriesKind.' + entry.kind),
    })),
])

const countryOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.event.form.fields.countryNone') },
  ...props.options.countries.map((entry) => ({
    value: entry.id,
    label: tr(entry.name),
    description: entry.iso2,
  })),
])

const timezoneOptions = computed<SelectOption[]>(() =>
  props.options.timezones.map((entry) => ({
    value: entry.value,
    label: entry.city,
    // Le décalage lève l'ambiguïté entre deux fuseaux voisins : Belém et São
    // Paulo ne portent pas le même, et le nom de ville seul ne le dit pas.
    description: `${entry.value} · ${entry.offset_label}`,
  })),
)

const statusOptions = computed<SelectOption[]>(() =>
  props.options.statuses.map((status) => ({
    value: status,
    label: t('admin.event.list.status.' + status),
  })),
)

const modeOptions = computed<SelectOption[]>(() =>
  MODES.map((mode) => ({ value: mode, label: t('admin.event.form.mode.' + mode) })),
)

// ---------------------------------------------------------------------------
// Erreurs
// ---------------------------------------------------------------------------

/** L'erreur d'un champ, s'il en porte une. */
function errorOf(field: keyof EditionFormPayload): string | undefined {
  const found = props.errors.find((entry) => entry.field === field)
  return found ? t('admin.event.form.errors.' + found.code) : undefined
}

/** Les erreurs qui ne visent aucun champ : elles s'affichent en tête. */
const globalErrors = computed(() =>
  props.errors
    .filter((entry) => entry.field === null)
    .map((entry) => t('admin.event.form.errors.' + (entry.code as EditionErrorCode))),
)

/** Le lieu est exigé dès que l'édition n'est pas entièrement en ligne. */
const locationRequired = computed(() => form.value.participation_mode !== 'online')

function submit(): void {
  emit('submit', { ...form.value })
}
</script>

<template>
  <form class="space-y-8" novalidate @submit.prevent="submit">
    <UiAlert
      v-if="props.errors.length > 0"
      intent="danger"
      live
      :title="t('admin.event.form.errors.title')"
    >
      <ul v-if="globalErrors.length > 0" class="mt-1 space-y-0.5 text-sm">
        <li v-for="message in globalErrors" :key="message">{{ message }}</li>
      </ul>
    </UiAlert>

    <!-- IDENTITÉ ------------------------------------------------------------->
    <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
      <legend class="px-2 font-display text-lg font-semibold">
        {{ t('admin.event.form.sections.identity.legend') }}
      </legend>
      <p class="mb-5 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.event.form.sections.identity.hint') }}
      </p>

      <div class="grid gap-5 lg:grid-cols-3">
        <div class="lg:col-span-2">
          <UiSelect
            :model-value="form.series_id ?? ''"
            :label="t('admin.event.form.fields.series')"
            :hint="t('admin.event.form.fields.seriesHint')"
            :options="seriesOptions"
            hide-optional
            @update:model-value="(next: string) => (form.series_id = next || null)"
          />
        </div>

        <UiInput
          :model-value="form.edition_year"
          type="number"
          :min="2000"
          :max="2100"
          :label="t('admin.event.form.fields.editionYear')"
          :error="errorOf('edition_year')"
          required
          @update:model-value="(next: string) => (form.edition_year = Number(next))"
        />

        <UiInput
          :model-value="form.edition_label ?? ''"
          :label="t('admin.event.form.fields.editionLabel')"
          :hint="t('admin.event.form.fields.editionLabelHint')"
          :error="errorOf('edition_label')"
          :maxlength="40"
          @update:model-value="(next: string) => (form.edition_label = next || null)"
        />

        <UiInput
          :model-value="form.acronym ?? ''"
          :label="t('admin.event.form.fields.acronym')"
          :hint="t('admin.event.form.fields.acronymHint')"
          :maxlength="16"
          @update:model-value="(next: string) => (form.acronym = next || null)"
        />

        <UiInput
          :model-value="form.slug"
          :label="t('admin.event.form.fields.slug')"
          :hint="slugIsAuto
            ? t('admin.event.form.fields.slugAuto')
            : t('admin.event.form.fields.slugHint')"
          :error="errorOf('slug')"
          required
          :maxlength="120"
          @update:model-value="setSlug"
        >
          <!-- Reprendre la proposition n'apparaît QUE si on l'a écartée, et
               seulement en création : en modification, le slug est une adresse
               publique qu'on ne recompose pas d'un bouton. -->
          <template v-if="props.isCreation && slugTouched" #suffix>
            <UiButton
              variant="ghost"
              size="sm"
              icon="refresh"
              icon-only
              :label="t('admin.event.form.fields.slugReset')"
              @click="resetSlug"
            />
          </template>
        </UiInput>

        <div class="lg:col-span-3">
          <AdminEventsI18nField
            v-model="form.title"
            :label="t('admin.event.form.fields.title')"
            :error="errorOf('title')"
            required
            :maxlength="240"
          />
        </div>

        <!-- Le statut est figé à la création : une édition naît en brouillon, et
             l'annoncer se fait en connaissance de ce qu'elle contient. -->
        <UiSelect
          v-if="!props.isCreation"
          :model-value="form.status"
          :label="t('admin.event.form.fields.status')"
          :options="statusOptions"
          hide-optional
          @update:model-value="(next: string) => (form.status = next as typeof form.status)"
        />
      </div>
    </fieldset>

    <!-- PRÉSENTATION -------------------------------------------------------->
    <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
      <legend class="px-2 font-display text-lg font-semibold">
        {{ t('admin.event.form.sections.content.legend') }}
      </legend>
      <p class="mb-5 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.event.form.sections.content.hint') }}
      </p>

      <div class="space-y-5">
        <!-- LA DESCRIPTION EST LUE PAR LE PUBLIC sur la page de l'édition : elle a
             droit au même éditeur que la présentation détaillée d'une proposition
             (A4). Structure seulement — ni police, ni couleur, ni taille. -->
        <AdminEventsI18nField
          v-model="form.description"
          :label="t('admin.event.form.fields.description')"
          :error="errorOf('description')"
          required
          rich
          :rows="8"
        />
        <AdminEventsI18nField
          v-model="form.highlights"
          :label="t('admin.event.form.fields.highlights')"
          :hint="t('admin.event.form.fields.highlightsHint')"
          multiline
          :rows="4"
        />
      </div>
    </fieldset>

    <!-- PÉRIODE ET FUSEAU --------------------------------------------------->
    <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
      <legend class="px-2 font-display text-lg font-semibold">
        {{ t('admin.event.form.sections.period.legend') }}
      </legend>
      <p class="mb-5 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.event.form.sections.period.hint') }}
      </p>

      <div class="grid gap-5 lg:grid-cols-3">
        <!-- LE FUSEAU D'ABORD : les dates saisies ensuite s'y rapportent. -->
        <UiSelect
          :model-value="form.timezone"
          :label="t('admin.event.form.fields.timezone')"
          :hint="t('admin.event.form.fields.timezoneHint')"
          :options="timezoneOptions"
          :error="errorOf('timezone')"
          required
          hide-optional
          @update:model-value="(next: string) => (form.timezone = next)"
        />

        <UiDatePicker
          :model-value="wall.starts_at"
          with-time
          :label="t('admin.event.form.fields.startsAt')"
          :timezone-label="zoneLabel"
          :error="errorOf('starts_at')"
          required
          @update:model-value="(next: string) => setWall('starts_at', next)"
        />

        <UiDatePicker
          :model-value="wall.ends_at"
          with-time
          :label="t('admin.event.form.fields.endsAt')"
          :timezone-label="zoneLabel"
          :min="wall.starts_at || undefined"
          :error="errorOf('ends_at')"
          required
          @update:model-value="(next: string) => setWall('ends_at', next)"
        />
      </div>
    </fieldset>

    <!-- LIEU ----------------------------------------------------------------->
    <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
      <legend class="px-2 font-display text-lg font-semibold">
        {{ t('admin.event.form.sections.location.legend') }}
      </legend>
      <p class="mb-5 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.event.form.sections.location.hint') }}
      </p>

      <div class="grid gap-5 lg:grid-cols-3">
        <UiSelect
          :model-value="form.participation_mode"
          :label="t('admin.event.form.fields.participationMode')"
          :options="modeOptions"
          required
          hide-optional
          @update:model-value="
            (next: string) => (form.participation_mode = next as ParticipationMode)
          "
        />

        <UiSelect
          :model-value="form.country_id ?? ''"
          :label="t('admin.event.form.fields.country')"
          :options="countryOptions"
          :error="errorOf('country_id')"
          :required="locationRequired"
          hide-optional
          @update:model-value="(next: string) => (form.country_id = next || null)"
        />

        <UiInput
          :model-value="form.city ?? ''"
          :label="t('admin.event.form.fields.city')"
          :error="errorOf('city')"
          :required="locationRequired"
          :maxlength="120"
          @update:model-value="(next: string) => (form.city = next || null)"
        />

        <div class="lg:col-span-3">
          <UiInput
            :model-value="form.address ?? ''"
            :label="t('admin.event.form.fields.address')"
            :maxlength="240"
            @update:model-value="(next: string) => (form.address = next || null)"
          />
        </div>

        <!-- COORDONNÉES DU LIEU, facultatives et INDÉPENDANTES de l'adresse.
             Une adresse de parc des expositions ne se géocode pas toujours : elle
             place un marqueur à deux kilomètres du pavillon. L'équipe relève donc
             le point sur place. `ck_events_coordinates` les veut tous deux ou
             aucun — l'erreur se pose sur celui qui manque. -->
        <UiInput
          :model-value="form.latitude ?? ''"
          type="number"
          step="0.000001"
          :min="-90"
          :max="90"
          :label="t('admin.event.form.fields.latitude')"
          :hint="t('admin.event.form.fields.coordinatesHint')"
          :error="errorOf('latitude')"
          @update:model-value="(next: string) => (form.latitude = next === '' ? null : Number(next))"
        />
        <UiInput
          :model-value="form.longitude ?? ''"
          type="number"
          step="0.000001"
          :min="-180"
          :max="180"
          :label="t('admin.event.form.fields.longitude')"
          :error="errorOf('longitude')"
          @update:model-value="(next: string) => (form.longitude = next === '' ? null : Number(next))"
        />

        <div class="flex items-end">
          <UiButton
            v-if="form.latitude !== null && form.longitude !== null"
            variant="link"
            size="sm"
            icon="globe"
            :href="`https://www.openstreetmap.org/?mlat=${form.latitude}&mlon=${form.longitude}#map=17/${form.latitude}/${form.longitude}`"
          >
            {{ t('admin.event.form.fields.coordinatesCheck') }}
          </UiButton>
        </div>
      </div>
    </fieldset>

    <!-- PAVILLON ------------------------------------------------------------->
    <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
      <legend class="px-2 font-display text-lg font-semibold">
        {{ t('admin.event.form.sections.pavilion.legend') }}
      </legend>
      <p class="mb-5 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.event.form.sections.pavilion.hint') }}
      </p>

      <UiSwitch
        :model-value="form.has_pavilion"
        :label="t('admin.event.form.fields.hasPavilion')"
        @update:model-value="(next: boolean) => (form.has_pavilion = next)"
      />
    </fieldset>

    <!-- VISUELS -------------------------------------------------------------->
    <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
      <legend class="px-2 font-display text-lg font-semibold">
        {{ t('admin.event.form.sections.visuals.legend') }}
      </legend>
      <p class="mb-5 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.event.form.sections.visuals.hint') }}
      </p>

      <AdminEventsImagesPanel
        :images="props.images ?? { banner: null, cover: null, thumbnail: null }"
        :model-value="form.images"
        @update:model-value="(next) => (form.images = next)"
      />

      <p class="mt-4 text-sm text-text-subtle">{{ t('admin.event.form.uploadNotice') }}</p>
    </fieldset>

    <div class="flex flex-wrap items-center gap-3">
      <UiButton type="submit" :loading="props.busy">
        {{ t('admin.event.form.actions.save') }}
      </UiButton>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('cancel')">
        {{ t('admin.event.form.actions.cancel') }}
      </UiButton>
    </div>
  </form>
</template>
