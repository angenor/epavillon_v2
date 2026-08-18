<script setup lang="ts">
import type {
  IncidentPayload,
  IncidentTargetOption,
  IncidentTargets,
  ManagedIncident,
} from '~/types/admin-incidents'
import type { IncidentScope, IncidentSeverity } from '~/types/live'
import type { TaxonomyTerm } from '~/types/reference'
import type { EventId, TimeZoneName } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * LE FORMULAIRE DE PUBLICATION.
 *
 * QUATRE DÉCISIONS, DANS L'ORDRE OÙ ON LES PREND EN SITUATION : qui voit le
 * message, de quoi il s'agit, ce qu'on écrit, quand il s'affiche. L'aperçu suit
 * la saisie sans qu'on ait à le demander — c'est la seule façon de juger un
 * aplat rouge avant qu'il s'affiche devant deux cents personnes.
 *
 * ── CE QUE LE FORMULAIRE TIENT PAR CONSTRUCTION ─────────────────────────────
 *
 * `ck_incidents_scope_target` exige exactement une cible par portée, et aucune
 * pour `global`. Changer de portée n'efface donc pas seulement l'affichage : le
 * brouillon lui-même ne garde qu'une cible. Sans cela, choisir « une journée »
 * puis se raviser pour « cet événement » enverrait deux cibles et la base
 * refuserait, sans que rien à l'écran ne l'explique.
 *
 * `ck_incidents_window` exige une fin postérieure au début. La fin est offerte
 * MAIS SIGNALÉE quand elle manque : c'est elle qui retire le bandeau sans que
 * personne ait à y penser, et son absence est exactement le défaut de la v1 —
 * des bandeaux restés en ligne des mois.
 *
 * ── LES DEUX LANGUES SONT EXIGÉES, ET C'EST UNE RÈGLE D'INTERFACE ───────────
 *
 * La base accepte un message en français seul (`message` est un `i18n_text` non
 * nul). Le prompt exige les deux, et il a raison : la moitié du public en ligne
 * lit l'anglais, et un bandeau qu'elle ne comprend pas n'informe personne.
 *
 * ── L'HEURE EST CELLE DE L'ÉDITION ──────────────────────────────────────────
 *
 * On saisit « 11:15 » en pensant à Belém, pas au fuseau de son propre
 * navigateur. La saisie est donc convertie une fois, par `instantFromWallClock`,
 * et le fuseau est rappelé sous les deux champs.
 */

interface Props {
  /** Message existant, en modification. Absent, le formulaire ouvre un brouillon. */
  incident?: ManagedIncident | null
  eventId: EventId
  targets: IncidentTargets
  kinds: TaxonomyTerm[]
  timezone: TimeZoneName
  zoneLabel?: string
  submitting?: boolean
  error?: string | null
  /** Portée et cible imposées par un raccourci (« Signaler un débordement »). */
  prefill?: Partial<IncidentPayload> | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  submit: [payload: IncidentPayload]
  cancel: []
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

const SCOPES: IncidentScope[] = ['global', 'event', 'event_day', 'session', 'organization']
const SEVERITIES: IncidentSeverity[] = ['info', 'warning', 'error', 'critical']

// ---------------------------------------------------------------------------
// L'état du formulaire
// ---------------------------------------------------------------------------

const scope = ref<IncidentScope>('event')
const targetId = ref<string>('')
const kindCode = ref<string>('technical_issue')
const severity = ref<IncidentSeverity>('warning')
const titleFr = ref('')
const titleEn = ref('')
const messageFr = ref('')
const messageEn = ref('')
const actionUrl = ref('')
const isDismissible = ref(true)
/** Immédiat ou programmé — la question que pose le prompt, dans ces mots. */
const startMode = ref<'now' | 'scheduled'>('now')
const startAt = ref('')
const hasEnd = ref(false)
const endAt = ref('')

/** Heure murale du fuseau de l'ÉDITION, au format qu'attend `UiDatePicker`. */
function wallClock(value: string | null): string {
  return value ? wallClockInZone(value, props.timezone).replace(' ', 'T') : ''
}

function loadFrom(incident: ManagedIncident): void {
  scope.value = incident.scope
  targetId.value = incident.scope === 'global' ? '' : (incident.target_id ?? '')
  kindCode.value = incident.kind_code
  severity.value = incident.severity
  titleFr.value = incident.title?.fr ?? ''
  titleEn.value = incident.title?.en ?? ''
  messageFr.value = incident.message.fr ?? ''
  messageEn.value = incident.message.en ?? ''
  actionUrl.value = incident.action_url ?? ''
  isDismissible.value = incident.is_dismissible
  startMode.value = 'scheduled'
  startAt.value = wallClock(incident.display_from)
  hasEnd.value = incident.display_until !== null
  endAt.value = wallClock(incident.display_until)
}

function applyPrefill(prefill: Partial<IncidentPayload>): void {
  if (prefill.scope) scope.value = prefill.scope
  if (prefill.session_id) targetId.value = prefill.session_id
  if (prefill.event_day_id) targetId.value = prefill.event_day_id
  if (prefill.organization_id) targetId.value = prefill.organization_id
  if (prefill.incident_kind_code) kindCode.value = prefill.incident_kind_code
  if (prefill.severity) severity.value = prefill.severity
  if (prefill.title?.fr) titleFr.value = prefill.title.fr
  if (prefill.title?.en) titleEn.value = prefill.title.en
  if (prefill.message?.fr) messageFr.value = prefill.message.fr
  if (prefill.message?.en) messageEn.value = prefill.message.en
  if (prefill.display_until) {
    hasEnd.value = true
    endAt.value = wallClock(prefill.display_until)
  }
}

watchEffect(() => {
  if (props.incident) loadFrom(props.incident)
  else if (props.prefill) applyPrefill(props.prefill)
})

// ---------------------------------------------------------------------------
// Les choix offerts
// ---------------------------------------------------------------------------

const scopeOptions = computed<SelectOption[]>(() =>
  SCOPES.map((value) => ({
    value,
    label: t(`admin.incident.form.scope.option.${value}`),
    description: t(`admin.incident.form.scope.help.${value}`),
  })),
)

/** Les cibles de la portée choisie — et rien d'autre : règle métier n° 8. */
const targetChoices = computed<IncidentTargetOption[]>(() => {
  switch (scope.value) {
    case 'event_day':
      return props.targets.days
    case 'session':
      return props.targets.sessions
    case 'organization':
      return props.targets.organizations
    default:
      return []
  }
})

/**
 * Le créneau d'une séance est FORMATÉ ICI, dans le fuseau de l'édition. Une
 * liste déroulante qui affiche « 2027-11-13T09:30:00-03:00 » ne se lit pas, et
 * c'est le genre de fuite qu'une donnée brute passée en libellé produit.
 */
const targetOptions = computed<SelectOption[]>(() =>
  targetChoices.value.map((option) => ({
    value: option.id,
    label: option.label,
    description: option.starts_at
      ? dateTime(option.starts_at, props.timezone)
      : (option.hint ?? undefined),
  })),
)

const kindOptions = computed<SelectOption[]>(() =>
  props.kinds.map((kind) => ({ value: kind.code, label: tr(kind.label) })),
)

/**
 * CHANGER DE PORTÉE OUBLIE LA CIBLE — mais seulement quand c'est l'utilisateur
 * qui change de portée.
 *
 * `ck_incidents_scope_target` n'accepte qu'une cible : passer de « une journée »
 * à « une activité » en gardant l'ancienne ferait refuser l'enregistrement par
 * la base, sans que rien à l'écran ne l'explique. La remise à zéro est donc
 * faite ICI, dans le gestionnaire du choix, et NON dans un `watch` sur `scope` :
 * un observateur effacerait aussi la cible qu'un pré-remplissage vient de poser,
 * et le raccourci « Signaler un débordement » arriverait sur un formulaire dont
 * l'activité s'est effacée toute seule.
 */
function chooseScope(next: string): void {
  if (next === scope.value) return
  scope.value = next as IncidentScope
  targetId.value = ''
}

// ---------------------------------------------------------------------------
// Ce que le formulaire enverra
// ---------------------------------------------------------------------------

const payload = computed<IncidentPayload>(() => {
  const start =
    startMode.value === 'now'
      ? new Date().toISOString()
      : (instantFromWallClock(startAt.value, props.timezone) ?? new Date().toISOString())

  return {
    scope: scope.value,
    event_id: scope.value === 'event' ? props.eventId : null,
    event_day_id: scope.value === 'event_day' ? targetId.value || null : null,
    session_id: scope.value === 'session' ? targetId.value || null : null,
    organization_id: scope.value === 'organization' ? targetId.value || null : null,
    incident_kind_code: kindCode.value,
    severity: severity.value,
    title: trimmedI18n({ fr: titleFr.value, en: titleEn.value }),
    message: { fr: messageFr.value.trim(), en: messageEn.value.trim() },
    action_url: actionUrl.value.trim() || null,
    is_dismissible: isDismissible.value,
    display_from: start,
    display_until: hasEnd.value
      ? instantFromWallClock(endAt.value, props.timezone)
      : null,
    publish: false,
  }
})

const issues = computed(() => validateIncident(payload.value))
const isValid = computed(() => issues.value.length === 0)

/** Le libellé du bouton de publication suit ce qui va réellement se passer. */
const publishLabel = computed(() =>
  props.incident
    ? t('admin.incident.form.actions.update')
    : startMode.value === 'scheduled'
      ? t('admin.incident.form.actions.publishScheduled')
      : t('admin.incident.form.actions.publish'),
)

function submit(publish: boolean): void {
  emit('submit', { ...payload.value, publish })
}
</script>

<template>
  <form class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_28rem]" @submit.prevent="submit(true)">
    <div class="space-y-6">
      <!-- 1. QUI VOIT CE MESSAGE ------------------------------------------ -->
      <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
        <h2 class="font-display text-base">{{ t('admin.incident.form.sections.scope') }}</h2>

        <UiRadio
          :model-value="scope"
          class="mt-3"
          :options="scopeOptions"
          :label="t('admin.incident.form.scope.label')"
          :hint="t('admin.incident.form.scope.hint')"
          :disabled="submitting"
          @update:model-value="chooseScope"
        />

        <UiFormField
          v-if="scope !== 'global' && scope !== 'event'"
          class="mt-4"
          :label="t(`admin.incident.form.scope.target.${scope}`)"
          :error="issues.includes('missing_target') ? t('admin.incident.form.error.missing_target') : undefined"
          required
        >
          <UiSelect
            v-model="targetId"
            :options="targetOptions"
            :placeholder="t('admin.incident.form.scope.target.placeholder')"
            :disabled="submitting"
          />
        </UiFormField>
      </section>

      <!-- 2. DE QUOI IL S'AGIT --------------------------------------------- -->
      <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
        <h2 class="font-display text-base">{{ t('admin.incident.form.sections.nature') }}</h2>

        <div class="mt-3 grid gap-4 md:grid-cols-2">
          <UiFormField
            :label="t('admin.incident.form.kind.label')"
            :hint="t('admin.incident.form.kind.hint')"
            required
          >
            <UiSelect v-model="kindCode" :options="kindOptions" :disabled="submitting" />
          </UiFormField>

          <fieldset>
            <legend class="text-sm font-medium">{{ t('admin.incident.form.severity.label') }}</legend>
            <div class="mt-2 flex flex-wrap gap-2">
              <UiButton
                v-for="value in SEVERITIES"
                :key="value"
                type="button"
                :variant="severity === value ? 'secondary' : 'ghost'"
                size="sm"
                :aria-pressed="severity === value"
                :disabled="submitting"
                @click="severity = value"
              >
                {{ t(`admin.incident.form.severity.option.${value}`) }}
              </UiButton>
            </div>
            <p class="mt-2 max-w-(--measure) text-sm text-text-muted">
              {{ t(`admin.incident.form.severity.help.${severity}`) }}
            </p>
          </fieldset>
        </div>
      </section>

      <!-- 3. CE QUE LE PUBLIC LIT ------------------------------------------ -->
      <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
        <h2 class="font-display text-base">{{ t('admin.incident.form.sections.message') }}</h2>

        <div class="mt-3 grid gap-4 md:grid-cols-2">
          <UiInput
            v-model="titleFr"
            :label="t('admin.incident.form.field.titleFr')"
            :hint="t('admin.incident.form.field.titleHint')"
            :maxlength="120"
            :disabled="submitting"
          />
          <UiInput
            v-model="titleEn"
            :label="t('admin.incident.form.field.titleEn')"
            :maxlength="120"
            :disabled="submitting"
          />

          <UiTextarea
            v-model="messageFr"
            :label="t('admin.incident.form.field.messageFr')"
            :hint="t('admin.incident.form.field.messageHint')"
            :rows="4"
            :maxlength="600"
            required
            :disabled="submitting"
          />
          <UiTextarea
            v-model="messageEn"
            :label="t('admin.incident.form.field.messageEn')"
            :rows="4"
            :maxlength="600"
            required
            :error="issues.includes('missing_message') ? t('admin.incident.form.error.missing_message') : undefined"
            :disabled="submitting"
          />
        </div>

        <div class="mt-4 grid gap-4 md:grid-cols-2">
          <UiInput
            v-model="actionUrl"
            type="url"
            :label="t('admin.incident.form.field.actionUrl')"
            :hint="t('admin.incident.form.field.actionUrlHint')"
            :disabled="submitting"
          />
          <UiSwitch
            v-model="isDismissible"
            class="self-end pb-2"
            :label="t('admin.incident.form.field.dismissible')"
            :hint="t('admin.incident.form.field.dismissibleHint')"
            :disabled="submitting"
          />
        </div>
      </section>

      <!-- 4. QUAND IL S'AFFICHE -------------------------------------------- -->
      <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
        <h2 class="font-display text-base">{{ t('admin.incident.form.sections.window') }}</h2>

        <div class="mt-3 grid gap-4 md:grid-cols-2">
          <div>
            <fieldset>
              <legend class="text-sm font-medium">{{ t('admin.incident.form.window.start') }}</legend>
              <div class="mt-2 flex flex-wrap gap-2">
                <UiButton
                  type="button"
                  :variant="startMode === 'now' ? 'secondary' : 'ghost'"
                  size="sm"
                  :aria-pressed="startMode === 'now'"
                  :disabled="submitting"
                  @click="startMode = 'now'"
                >
                  {{ t('admin.incident.form.window.startNow') }}
                </UiButton>
                <UiButton
                  type="button"
                  :variant="startMode === 'scheduled' ? 'secondary' : 'ghost'"
                  size="sm"
                  :aria-pressed="startMode === 'scheduled'"
                  :disabled="submitting"
                  @click="startMode = 'scheduled'"
                >
                  {{ t('admin.incident.form.window.startScheduled') }}
                </UiButton>
              </div>
            </fieldset>

            <UiDatePicker
              v-if="startMode === 'scheduled'"
              v-model="startAt"
              class="mt-3"
              with-time
              :label="t('admin.incident.form.window.start')"
              :timezone-label="zoneLabel"
              :disabled="submitting"
            />
          </div>

          <div>
            <UiSwitch
              :model-value="hasEnd"
              :label="t('admin.incident.form.window.end')"
              :hint="t('admin.incident.form.window.endHint')"
              :disabled="submitting"
              @update:model-value="hasEnd = $event"
            />

            <UiDatePicker
              v-if="hasEnd"
              v-model="endAt"
              class="mt-3"
              with-time
              :label="t('admin.incident.form.window.end')"
              :timezone-label="zoneLabel"
              :error="issues.includes('invalid_window') ? t('admin.incident.form.error.invalid_window') : undefined"
              :disabled="submitting"
            />
            <!-- SANS FIN, QUELQU'UN DEVRA Y PENSER. C'est exactement ce que la
                 v1 oubliait de faire. -->
            <UiAlert
              v-else
              class="mt-3"
              intent="warning"
              compact
              :message="t('admin.incident.form.window.endOpenHint')"
            />
          </div>
        </div>

        <p class="mt-3 text-sm text-text-subtle">
          {{ t('admin.incident.form.window.zone', { zone: zoneLabel ?? timezone }) }}
        </p>
      </section>

      <UiAlert v-if="error" intent="danger" live :message="error" />

      <div class="flex flex-wrap items-center gap-3">
        <UiButton type="submit" :loading="submitting" :disabled="!isValid" icon="broadcast">
          {{ publishLabel }}
        </UiButton>
        <UiButton
          type="button"
          variant="secondary"
          :disabled="submitting || !isValid"
          @click="submit(false)"
        >
          {{ t('admin.incident.form.actions.saveDraft') }}
        </UiButton>
        <UiButton type="button" variant="ghost" :disabled="submitting" @click="emit('cancel')">
          {{ t('admin.incident.form.actions.cancel') }}
        </UiButton>
      </div>
    </div>

    <!-- L'APERÇU SUIT LA SAISIE, et il colle en haut sur écran large : on écrit
         en regardant ce que le public verra. -->
    <div class="xl:sticky xl:top-24 xl:self-start">
      <AdminIncidentsPreview :payload="payload" :timezone="timezone" :zone-label="zoneLabel" />
    </div>
  </form>
</template>
