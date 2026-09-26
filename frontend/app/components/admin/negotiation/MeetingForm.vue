<script setup lang="ts">
import type {
  AdminFrancophoneMeeting,
  FrancophoneMeetingFormat,
  FrancophoneMeetingInput,
} from '~/types/negotiation-meetings'
import type { TaxonomyTerm } from '~/types/reference'
import type { I18nText, TimeZoneName, Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * UNE RÉUNION DE LA FRANCOPHONIE, saisie ou reprise. Les heures se saisissent
 * dans le fuseau de l'édition. Ce qui ne s'applique pas se masque sans
 * s'effacer : revenir sur un choix retrouve la saisie ; seul l'envoi l'omet.
 */

export interface MeetingFormFailure {
  message: string
  /** Le champ que l'API nomme, s'il en nomme un. */
  field: string | null
}

const props = withDefaults(
  defineProps<{
    meeting?: AdminFrancophoneMeeting | null
    timezone: TimeZoneName
    /** « heure d'Antalya ». */
    zoneLabel: string
    submitting?: boolean
    failure?: MeetingFormFailure | null
    readonly?: boolean
    /** Incrémenté par l'écran après un enregistrement : le formulaire se recale. */
    revision?: number
  }>(),
  { meeting: null, submitting: false, failure: null, readonly: false, revision: 0 },
)
const emit = defineEmits<{ submit: [input: FrancophoneMeetingInput] }>()

const { t, locale } = useI18n()
const api = useApi()

const { data: vocabulaire } = await useAsyncData<TaxonomyTerm[]>(
  'reference-terms-francophone_meeting_type',
  () => api.reference.terms('francophone_meeting_type'),
  { default: () => [], lazy: true },
)

interface Etat {
  type: string
  title: I18nText | null
  description: I18nText | null
  startAt: string
  endAt: string
  format: FrancophoneMeetingFormat
  venue: string
  externalUrl: string
  requiresRegistration: boolean
  capacity: string
  waitlistEnabled: boolean
  opensAt: string
  closesAt: string
  access: 'open' | 'limited'
  audience: I18nText | null
  isIfdd: boolean
  organizerOrgId: Uuid | null
}

const murale = (instant: string | null): string =>
  instant ? wallClockInZone(instant, props.timezone).replace(' ', 'T') : ''

function depuis(m: AdminFrancophoneMeeting | null): Etat {
  return {
    type: m?.type?.code ?? '',
    title: m?.title ?? null,
    description: m?.description ?? null,
    startAt: murale(m?.start_at ?? null),
    endAt: murale(m?.end_at ?? null),
    format: m?.format ?? 'onsite',
    venue: m?.venue ?? '',
    externalUrl: m?.external_url ?? '',
    requiresRegistration: m?.requires_registration ?? true,
    capacity: m?.capacity != null ? String(m.capacity) : '',
    waitlistEnabled: m?.waitlist_enabled ?? true,
    opensAt: murale(m?.registration_opens_at ?? null),
    closesAt: murale(m?.registration_closes_at ?? null),
    access: m && !m.open_access ? 'limited' : 'open',
    audience: m?.access_audience ?? null,
    isIfdd: m?.is_ifdd_organized ?? true,
    organizerOrgId: m?.organizer_org_id ?? null,
  }
}

const etat = ref<Etat>(depuis(props.meeting))
watch([() => props.meeting?.id, () => props.revision], () => (etat.value = depuis(props.meeting)))

const natures = computed<SelectOption[]>(() => {
  const options = vocabulaire.value
    .filter((terme) => terme.is_active)
    .map((terme) => ({ value: terme.code, label: resolveI18nText(terme.label, locale.value) }))
  const actuelle = props.meeting?.type
  if (actuelle && !options.some((o) => o.value === actuelle.code)) {
    options.push({ value: actuelle.code, label: resolveI18nText(actuelle.label, locale.value) })
  }
  return options
})

const formats = computed<SelectOption[]>(() =>
  (['onsite', 'online', 'hybrid'] as const).map((f) => ({
    value: f,
    label: t(`admin.negociations.reunions.format.${f}`),
  })),
)
const acces = computed<SelectOption[]>(() =>
  (['open', 'limited'] as const).map((a) => ({
    value: a,
    label: t(`admin.negociations.reunions.form.access.${a}.label`),
    description: t(`admin.negociations.reunions.form.access.${a}.hint`),
  })),
)

function choisirLeFormat(valeur: string): void {
  if (valeur === 'onsite' || valeur === 'online' || valeur === 'hybrid') etat.value.format = valeur
}

const avecLieu = computed(() => etat.value.format !== 'online')
const avecLien = computed(() => etat.value.format !== 'onsite')
const limite = computed(() => etat.value.access === 'limited')

const instant = (murale: string): string | null => instantFromWallClock(murale, props.timezone)

/** Les refus que l'écran voit avant l'API ; l'API reste juge. */
const problemes = computed<Record<string, string>>(() => {
  const e = etat.value
  const p: Record<string, string> = {}
  if (!e.title?.fr?.trim()) p.title = t('admin.negociations.reunions.form.errors.title')
  if (!instant(e.startAt)) p.start_at = t('admin.negociations.reunions.form.errors.start')
  if (!instant(e.endAt)) p.end_at = t('admin.negociations.reunions.form.errors.end')
  else if (instant(e.startAt) && Date.parse(instant(e.endAt)!) <= Date.parse(instant(e.startAt)!)) {
    p.end_at = t('admin.negociations.reunions.form.errors.period')
  }
  if (e.requiresRegistration && e.capacity.trim() && !(Number.isInteger(Number(e.capacity)) && Number(e.capacity) > 0)) {
    p.capacity = t('admin.negociations.reunions.form.errors.capacity')
  }
  const ouvre = e.requiresRegistration ? instant(e.opensAt) : null
  const ferme = e.requiresRegistration ? instant(e.closesAt) : null
  if (ouvre && ferme && Date.parse(ferme) <= Date.parse(ouvre)) {
    p.registration_closes_at = t('admin.negociations.reunions.form.errors.window')
  }
  if (limite.value && !e.audience?.fr?.trim()) p.access_audience = t('admin.negociations.reunions.form.errors.audience')
  if (!e.isIfdd && !e.organizerOrgId) p.organizer_org_id = t('admin.negociations.reunions.form.errors.organizer')
  return p
})

const tente = ref(false)

const CHAMPS_DU_FORMULAIRE = new Set([
  'type', 'title', 'description', 'access_audience', 'start_at', 'end_at', 'venue', 'external_url',
  'capacity', 'registration_opens_at', 'registration_closes_at', 'organizer_org_id',
])

const erreurDe = (champ: string): string | undefined =>
  (tente.value ? problemes.value[champ] : undefined) ??
  (props.failure?.field === champ ? props.failure.message : undefined)

/** Un refus qui ne nomme aucun champ d'ici se lit en tête des boutons. */
const echecGeneral = computed(() =>
  props.failure && !CHAMPS_DU_FORMULAIRE.has(props.failure.field ?? '') ? props.failure.message : null,
)

const saisie = computed<FrancophoneMeetingInput>(() => {
  const e = etat.value
  const texte = (v: string): string | null => v.trim() || null
  return {
    type: e.type || null,
    title: e.title ?? { fr: '' },
    description: e.description,
    start_at: instant(e.startAt) ?? '',
    end_at: instant(e.endAt) ?? '',
    format: e.format,
    venue: avecLieu.value ? texte(e.venue) : null,
    external_url: avecLien.value ? texte(e.externalUrl) : null,
    capacity: e.requiresRegistration && e.capacity.trim() ? Number(e.capacity) : null,
    waitlist_enabled: e.waitlistEnabled,
    requires_registration: e.requiresRegistration,
    registration_opens_at: e.requiresRegistration ? instant(e.opensAt) : null,
    registration_closes_at: e.requiresRegistration ? instant(e.closesAt) : null,
    open_access: !limite.value,
    access_audience: limite.value ? e.audience : null,
    is_ifdd_organized: e.isIfdd,
    organizer_org_id: e.isIfdd ? null : e.organizerOrgId,
  }
})

function envoyer(): void {
  tente.value = true
  if (props.readonly || props.submitting || Object.keys(problemes.value).length) return
  emit('submit', saisie.value)
}
</script>

<template>
  <form class="space-y-6" novalidate @submit.prevent="envoyer">
    <fieldset :disabled="props.readonly" class="space-y-6">
      <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
        <h2 class="text-lg font-semibold">{{ t('admin.negociations.reunions.form.sections.what') }}</h2>
        <div class="mt-4 space-y-5">
          <UiSelect
            v-model="etat.type"
            :label="t('admin.negociations.reunions.form.type.label')"
            :hint="t('admin.negociations.reunions.form.type.hint')"
            :placeholder="t('admin.negociations.reunions.form.type.placeholder')"
            :options="natures"
            :error="erreurDe('type')"
            :readonly="props.readonly"
            block
          />
          <AdminEventsI18nField
            v-model="etat.title"
            :label="t('admin.negociations.reunions.form.title')"
            :error="erreurDe('title')"
            :disabled="props.readonly"
            :maxlength="200"
            required
          />
          <AdminEventsI18nField
            v-model="etat.description"
            :label="t('admin.negociations.reunions.form.description')"
            :error="erreurDe('description')"
            :disabled="props.readonly"
            multiline
            :maxlength="2000"
          />
        </div>
      </section>

      <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
        <h2 class="text-lg font-semibold">{{ t('admin.negociations.reunions.form.sections.when') }}</h2>
        <div class="mt-4 grid gap-5 md:grid-cols-2">
          <UiDatePicker
            v-model="etat.startAt"
            with-time
            :label="t('admin.negociations.reunions.form.start')"
            :timezone-label="props.zoneLabel"
            :error="erreurDe('start_at')"
            :readonly="props.readonly"
            required
          />
          <UiDatePicker
            v-model="etat.endAt"
            with-time
            :min="etat.startAt || undefined"
            :label="t('admin.negociations.reunions.form.end')"
            :timezone-label="props.zoneLabel"
            :error="erreurDe('end_at')"
            :readonly="props.readonly"
            required
          />
        </div>
        <UiRadio
          :model-value="etat.format"
          class="mt-5"
          inline
          :label="t('admin.negociations.reunions.form.format')"
          :options="formats"
          :readonly="props.readonly"
          required
          @update:model-value="choisirLeFormat"
        />
        <div class="mt-3 grid gap-5 md:grid-cols-2">
          <UiInput
            v-if="avecLieu"
            v-model="etat.venue"
            :label="t('admin.negociations.reunions.form.venue.label')"
            :hint="t('admin.negociations.reunions.form.venue.hint')"
            :error="erreurDe('venue')"
            :readonly="props.readonly"
            :maxlength="200"
            block
          />
          <UiInput
            v-if="avecLien"
            v-model="etat.externalUrl"
            type="url"
            :label="t('admin.negociations.reunions.form.link.label')"
            :hint="t('admin.negociations.reunions.form.link.hint')"
            :error="erreurDe('external_url')"
            :readonly="props.readonly"
            block
          />
        </div>
      </section>

      <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
        <h2 class="text-lg font-semibold">{{ t('admin.negociations.reunions.form.sections.registration') }}</h2>
        <UiSwitch
          v-model="etat.requiresRegistration"
          class="mt-3"
          :label="t('admin.negociations.reunions.form.registration.label')"
          :hint="t('admin.negociations.reunions.form.registration.hint')"
          :disabled="props.readonly"
        />
        <template v-if="etat.requiresRegistration">
          <div class="mt-4 grid gap-5 md:grid-cols-2">
            <UiInput
              v-model="etat.capacity"
              type="number"
              inputmode="numeric"
              :min="1"
              step="1"
              :label="t('admin.negociations.reunions.form.capacity.label')"
              :hint="t('admin.negociations.reunions.form.capacity.hint')"
              :error="erreurDe('capacity')"
              :readonly="props.readonly"
            />
            <UiSwitch
              v-model="etat.waitlistEnabled"
              class="self-end"
              :label="t('admin.negociations.reunions.form.waitlist.label')"
              :hint="t('admin.negociations.reunions.form.waitlist.hint')"
              :disabled="props.readonly"
            />
            <UiDatePicker
              v-model="etat.opensAt"
              with-time
              :label="t('admin.negociations.reunions.form.window.opens')"
              :hint="t('admin.negociations.reunions.form.window.opensHint', { zone: props.zoneLabel })"
              :timezone-label="props.zoneLabel"
              :error="erreurDe('registration_opens_at')"
              :readonly="props.readonly"
            />
            <UiDatePicker
              v-model="etat.closesAt"
              with-time
              :label="t('admin.negociations.reunions.form.window.closes')"
              :hint="t('admin.negociations.reunions.form.window.closesHint', { zone: props.zoneLabel })"
              :timezone-label="props.zoneLabel"
              :error="erreurDe('registration_closes_at')"
              :readonly="props.readonly"
            />
          </div>
        </template>
      </section>

      <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
        <h2 class="text-lg font-semibold">{{ t('admin.negociations.reunions.form.sections.who') }}</h2>
        <UiRadio
          :model-value="etat.access"
          class="mt-3"
          :label="t('admin.negociations.reunions.form.access.label')"
          :options="acces"
          :readonly="props.readonly"
          required
          @update:model-value="(v: string) => (etat.access = v === 'limited' ? 'limited' : 'open')"
        />
        <AdminEventsI18nField
          v-if="limite"
          v-model="etat.audience"
          class="mt-3"
          :label="t('admin.negociations.reunions.form.audience.label')"
          :hint="t('admin.negociations.reunions.form.audience.hint')"
          :error="erreurDe('access_audience')"
          :disabled="props.readonly"
          :maxlength="200"
          required
        />
        <UiSwitch
          v-model="etat.isIfdd"
          class="mt-5"
          :label="t('admin.negociations.reunions.form.ifdd.label')"
          :hint="t('admin.negociations.reunions.form.ifdd.hint')"
          :disabled="props.readonly"
        />
        <AdminNegotiationMeetingOrganizerPicker
          v-if="!etat.isIfdd"
          v-model="etat.organizerOrgId"
          class="mt-3"
          :current-name="props.meeting && !props.meeting.is_ifdd_organized ? props.meeting.organizer : null"
          :error="erreurDe('organizer_org_id')"
          :disabled="props.readonly"
        />
      </section>
    </fieldset>

    <UiAlert v-if="echecGeneral" intent="danger" live :message="echecGeneral" />
    <UiAlert
      v-else-if="tente && Object.keys(problemes).length"
      intent="warning"
      compact
      live
      :message="t('admin.negociations.reunions.form.errors.summary')"
    />

    <div v-if="!props.readonly" class="flex flex-wrap items-center gap-3">
      <UiButton type="submit" :loading="props.submitting">
        {{ props.meeting ? t('admin.negociations.reunions.form.save') : t('admin.negociations.reunions.form.create') }}
      </UiButton>
      <slot name="actions" />
    </div>
  </form>
</template>
