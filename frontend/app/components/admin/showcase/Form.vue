<script setup lang="ts">
import type {
  ShowcaseFormField,
  ShowcaseFormScreen,
  ShowcaseFormValues,
  ShowcaseSessionOption,
  ShowcaseValidationError,
} from '~/types/admin-showcase'
import type { HighlightStatus } from '~/types/content'
import type { EventId, I18nText } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * LE CORPS DU FORMULAIRE DE LA VITRINE — commun à la création et à la modification.
 *
 * UN SEUL COMPOSANT POUR LES DEUX PAGES. `nouveau.vue` et `[id].vue` ne diffèrent
 * que par ce qu'elles chargent et par le libellé du bouton : dupliquer six
 * sections de saisie pour cela, c'est garantir que la deuxième divergera.
 *
 * ── L'APERÇU EST VIVANT, ET C'EST CE QUI REND L'ÉCRAN UTILISABLE ────────────
 *
 * `ShowcaseFormScreen.preview` est l'état ENREGISTRÉ ; il est recomposé à chaque
 * frappe depuis les valeurs en cours et les référentiels
 * (`showcasePreviewOf()`). Un aperçu qui n'obéirait qu'à l'enregistrement
 * obligerait à publier pour voir — exactement ce que la v1 faisait faire.
 *
 * ── LA VALIDATION DEVANCE LA BASE, ELLE NE LA REMPLACE PAS ─────────────────
 *
 * Quatre contraintes de `115_content.sql` produisent des erreurs PostgreSQL
 * illisibles : fenêtre inversée, organisation désignée ET nommée, libellé de
 * lien sans lien, français manquant. Le formulaire les rejoue AVANT l'envoi pour
 * rendre un message français exploitable, et l'API refuse encore avec les mêmes
 * codes sur les mêmes champs — `ShowcaseWriteResult.errors`.
 *
 * ── DEUX EXCLUSIVITÉS QUI SE MONTRENT PLUTÔT QU'ELLES NE SE DEVINENT ───────
 *
 * La PERSONNE du répertoire prime sur le nom libre (`COALESCE` de la vue), et
 * l'organisation se DÉSIGNE ou se NOMME, jamais les deux
 * (`ck_highlights_organization_shape`, règle métier n° 1). Dans les deux cas le
 * champ perdant est neutralisé et le dit : laisser saisir un texte qui ne
 * sortira jamais est la meilleure façon de faire recréer le doublon « IFDD » que
 * la v2 corrige.
 *
 * ── LE FUSEAU DE LA FENÊTRE DE DIFFUSION EST UTC, ET C'EST UN CHOIX ────────
 *
 * Une diapositive n'appartient pas à un lieu : elle s'affiche sur l'accueil de
 * la plateforme. L'afficher dans le fuseau de l'édition ferait lire deux dates
 * différentes à deux administrateurs regardant la même ligne. Le fuseau est
 * écrit à côté du champ, comme toute date du projet.
 */

interface Props {
  screen: ShowcaseFormScreen
  /**
   * Les séances offertes au rattachement. Portées par la PAGE et non par
   * `screen` : changer d'édition doit rafraîchir la liste sans recharger
   * l'écran, sous peine de perdre la saisie en cours.
   */
  sessions: ShowcaseSessionOption[]
  /** Libellé du bouton d'envoi — « Créer » ou « Enregistrer ». */
  submitLabel: string
  submitting?: boolean
  /** Les refus rendus par l'API, posés sur leurs champs. */
  serverErrors?: ShowcaseValidationError[]
  /** Une erreur de réseau, distincte d'un refus de validation. */
  formError?: string | null
  /** Les séances de l'édition choisie se chargent. */
  sessionsLoading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  serverErrors: () => [],
  formError: null,
})

const emit = defineEmits<{
  submit: [values: ShowcaseFormValues]
  cancel: []
  /** L'édition a changé : à la page d'aller chercher ses séances. */
  eventChange: [eventId: EventId | null]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeWithZone } = useDateTime()

/** Le fuseau de la fenêtre de diffusion — voir l'en-tête. */
const WINDOW_TZ = 'UTC'

const STATUSES: HighlightStatus[] = ['draft', 'published', 'archived']

// ---------------------------------------------------------------------------
// L'état de saisie
// ---------------------------------------------------------------------------

/** Copie de travail : le tableau de thématiques est cloné, sans quoi une
 *  annulation laisserait la sélection modifiée dans la réponse d'API. */
function cloneValues(values: ShowcaseFormValues): ShowcaseFormValues {
  return { ...values, theme_codes: [...values.theme_codes] }
}

const values = ref<ShowcaseFormValues>(cloneValues(props.screen.values))

/** Les erreurs ne s'affichent qu'après une première tentative — voir plus bas.
 *  Déclaré ICI parce que `resetFrom()` le remet à zéro, et que le `watch`
 *  immédiat qui l'appelle s'exécute pendant la mise en place du composant. */
const attempted = ref(false)

/** Heures murales UTC des deux bornes, pour les contrôles natifs. */
const wall = ref({ starts_at: '', ends_at: '' })

function toWall(instant: string | null): string {
  return instant ? wallClockInZone(instant, WINDOW_TZ).replace(' ', 'T') : ''
}

function resetFrom(source: ShowcaseFormValues): void {
  values.value = cloneValues(source)
  wall.value = { starts_at: toWall(source.starts_at), ends_at: toWall(source.ends_at) }
  attempted.value = false
}

watch(() => props.screen.values, resetFrom, { immediate: true })

function setWall(key: 'starts_at' | 'ends_at', next: string): void {
  wall.value[key] = next
  values.value[key] = next ? instantFromWallClock(next, WINDOW_TZ) : null
}

// ---------------------------------------------------------------------------
// La validation
// ---------------------------------------------------------------------------

const localErrors = computed(() =>
  validateShowcaseForm(values.value, {
    isGlobalScope: props.screen.is_global_scope,
    sessions: props.sessions,
  }),
)

/**
 * Ce qui s'affiche : les refus de l'API toujours, les nôtres après la première
 * tentative. Signaler « le français est obligatoire » sur un formulaire vierge
 * qu'on vient d'ouvrir n'aide personne.
 */
const shownErrors = computed<ShowcaseValidationError[]>(() => {
  const seen = new Set<ShowcaseFormField>()
  const merged: ShowcaseValidationError[] = []
  for (const error of [...props.serverErrors, ...(attempted.value ? localErrors.value : [])]) {
    if (seen.has(error.field)) continue
    seen.add(error.field)
    merged.push(error)
  }
  return merged
})

function errorOf(field: ShowcaseFormField): string | undefined {
  const code = showcaseErrorOf(shownErrors.value, field)
  return code ? t(`admin.showcase.form.error.${code}`) : undefined
}

const root = ref<HTMLElement | null>(null)

/**
 * Le focus va au PREMIER champ en défaut.
 *
 * Par `data-field` et non par identifiant : les champs multilingues sont rendus
 * par un composant qui compose lui-même les siens, et tenir une table de
 * correspondance identifiant/colonne serait une deuxième source de vérité.
 */
function focusFirstError(): void {
  const first = localErrors.value[0]
  if (!first || !root.value) return
  const holder = root.value.querySelector(`[data-field="${first.field}"]`)
  if (!holder) return
  holder.scrollIntoView({ block: 'center', behavior: 'smooth' })
  // Le CONTRÔLE d'abord, l'onglet de langue seulement à défaut : un champ
  // multilingue commence par ses deux onglets, et y poser le focus laisserait
  // l'éditeur devant un bouton « Français » sans comprendre ce qu'on lui demande.
  const control =
    holder.querySelector<HTMLElement>('input, textarea, select') ??
    holder.querySelector<HTMLElement>('button')
  control?.focus({ preventScroll: true })
}

function onSubmit(): void {
  attempted.value = true
  if (localErrors.value.length > 0) {
    nextTick(focusFirstError)
    return
  }
  emit('submit', {
    ...values.value,
    // Les chaînes vidées redeviennent `null` : `''` en base serait servi comme
    // une valeur, et un nom d'auteur vide s'afficherait sous la citation.
    author_name: trimmedOrNull(values.value.author_name),
    organization_label: trimmedOrNull(values.value.organization_label),
    link_url: trimmedOrNull(values.value.link_url),
    quote: emptyToNull(values.value.quote),
    body: emptyToNull(values.value.body),
    author_title: emptyToNull(values.value.author_title),
    link_label: emptyToNull(values.value.link_label),
    theme_codes: [...values.value.theme_codes],
  })
}

// ---------------------------------------------------------------------------
// Les listes de choix
// ---------------------------------------------------------------------------

const natureOptions = computed<SelectOption[]>(() =>
  props.screen.natures.map((nature) => ({ value: nature.code, label: tr(nature.label) })),
)

const statusOptions = computed<SelectOption[]>(() =>
  STATUSES.map((status) => ({
    value: status,
    label: t(`admin.showcase.form.status.${status}`),
    description: t(`admin.showcase.form.statusHint.${status}`),
  })),
)

/**
 * Les éditions du PÉRIMÈTRE, plus le contenu de plateforme quand la portée est
 * globale. Un compte détaché n'y voit pas l'option : elle lui serait refusée par
 * l'API, et l'offrir pour la refuser ensuite est une impasse.
 */
const eventOptions = computed<SelectOption[]>(() => {
  const options: SelectOption[] = props.screen.events.map((event) => ({
    value: event.id,
    label: tr(event.title),
    description: event.acronym ?? undefined,
  }))
  if (props.screen.is_global_scope) {
    options.unshift({
      value: '',
      label: t('admin.showcase.form.link.platform'),
      description: t('admin.showcase.form.link.platformHint'),
    })
  }
  return options
})

const sessionOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.showcase.form.link.noSession') },
  ...props.sessions.map((session) => ({
    value: session.id,
    label: tr(session.title),
    // Toute date affichée porte son fuseau — celui de la SÉANCE, pas celui du
    // navigateur : « 14:30 » sans mention ferait choisir la mauvaise séance.
    description: `${date(session.starts_at, session.timezone)} · ${timeWithZone(session.starts_at, session.timezone)}`,
  })),
])

const personOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.showcase.form.attribution.noPerson') },
  ...props.screen.people.map((person) => ({ value: person.id, label: person.display_name })),
])

const organizationOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.showcase.form.attribution.noOrganization') },
  ...props.screen.organizations.map((organization) => ({
    value: organization.id,
    label: organization.legal_name,
    description: organization.acronym ?? undefined,
  })),
])

const countryOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.showcase.form.attribution.noCountry') },
  ...props.screen.countries.map((country) => ({ value: country.id, label: tr(country.name) })),
])

// ---------------------------------------------------------------------------
// Les deux exclusivités
// ---------------------------------------------------------------------------

/** Une personne du répertoire est choisie : le nom libre ne sortira pas. */
const personWins = computed(() => values.value.person_id !== null)
/** Une organisation est désignée : le libellé libre est refusé par la base. */
const organizationPicked = computed(() => values.value.organization_id !== null)

function setEvent(next: string): void {
  const eventId = next || null
  values.value.event_id = eventId
  // Une séance appartient à SON édition : la garder après un changement
  // d'édition produirait la contradiction que le trigger refuse.
  values.value.session_id = null
  emit('eventChange', eventId)
}

function setSession(next: string): void {
  values.value.session_id = next || null
  // La séance IMPOSE son édition (`tg_highlights_normalize`) : on l'aligne
  // plutôt que d'attendre le refus.
  const session = props.sessions.find((entry) => entry.id === next)
  if (session) values.value.event_id = session.event_id
}

// ---------------------------------------------------------------------------
// L'aperçu et l'état de diffusion
// ---------------------------------------------------------------------------

const preview = computed(() =>
  showcasePreviewOf(values.value, props.screen.preview, {
    natures: props.screen.natures,
    events: props.screen.events,
    sessions: props.sessions,
    organizations: props.screen.organizations,
    people: props.screen.people,
    countries: props.screen.countries,
    themes: props.screen.available_themes,
    media: props.screen.media,
  }),
)

/**
 * Ce que la vitrine ferait de ces valeurs si on les enregistrait maintenant.
 * Recalculé à la seconde de l'ouverture, pas en continu : un état de diffusion
 * qui basculerait pendant la saisie ferait clignoter le formulaire pour rien.
 */
const now = ref(Date.now())
onMounted(() => (now.value = Date.now()))
const broadcastState = computed(() => showcaseBroadcastStateOf(values.value, now.value))
</script>

<template>
  <form ref="root" novalidate class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_24rem]" @submit.prevent="onSubmit">
    <div class="min-w-0 space-y-6">
      <UiAlert v-if="props.formError" intent="danger" live :message="props.formError" />

      <UiAlert
        v-else-if="shownErrors.length"
        intent="danger"
        live
        :title="t('admin.showcase.form.errorSummary.title')"
        :message="t('admin.showcase.form.errorSummary.description', { count: shownErrors.length })"
      />

      <!-- ================= NATURE ET EMPLACEMENT ================= -->
      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.submitting">
        <legend class="px-2 font-semibold">{{ t('admin.showcase.form.sections.nature') }}</legend>

        <div class="mt-3 grid gap-4 sm:grid-cols-2">
          <div data-field="nature_code">
            <UiSelect
              :model-value="values.nature_code"
              :options="natureOptions"
              :label="t('admin.showcase.form.nature.label')"
              :hint="t('admin.showcase.form.nature.hint')"
              :error="errorOf('nature_code')"
              required
              @update:model-value="(next: string) => (values.nature_code = next)"
            />
          </div>

          <!-- L'EMPLACEMENT NE SE CHOISIT PLUS (24/08) : il n'y en a qu'un, le
               bandeau d'ouverture. Le panneau latéral de l'accueil s'alimente
               seul — événements à venir, puis frise des activités retenues —
               et `home_aside` a quitté le modèle. Un menu déroulant à une
               seule entrée ne pose pas de question, il en fait perdre le sens. -->
        </div>

        <AdminShowcaseThemePicker
          v-if="props.screen.available_themes.length"
          v-model="values.theme_codes"
          class="mt-5"
          :themes="props.screen.available_themes"
          :label="t('admin.showcase.form.themes.label')"
          :hint="t('admin.showcase.form.themes.hint')"
        />
      </fieldset>

      <!-- ================= TEXTES ================= -->
      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.submitting">
        <legend class="px-2 font-semibold">{{ t('admin.showcase.form.sections.texts') }}</legend>

        <div class="mt-3 space-y-5">
          <div data-field="title">
            <!-- `title` est NOT NULL en base : le champ multilingue peut rendre
                 `null` quand on vide le français, on le ramène à un texte vide
                 plutôt que de casser le contrat de la colonne. La validation, elle,
                 refusera de l'enregistrer ainsi. -->
            <AdminEventsI18nField
              :model-value="values.title"
              @update:model-value="(next: I18nText | null) => (values.title = next ?? { fr: '' })"
              :label="t('admin.showcase.form.text.title')"
              :hint="t('admin.showcase.form.text.titleHint')"
              :error="errorOf('title')"
              required
            />
          </div>

          <div data-field="quote">
            <AdminEventsI18nField
              v-model="values.quote"
              :label="t('admin.showcase.form.text.quote')"
              :hint="t('admin.showcase.form.text.quoteHint')"
              :error="errorOf('quote')"
              multiline
              :rows="3"
            />
          </div>

          <div data-field="body">
            <AdminEventsI18nField
              v-model="values.body"
              :label="t('admin.showcase.form.text.body')"
              :hint="t('admin.showcase.form.text.bodyHint')"
              :error="errorOf('body')"
              multiline
              :rows="5"
            />
          </div>
        </div>
      </fieldset>

      <!-- ================= ATTRIBUTION ================= -->
      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.submitting">
        <legend class="px-2 font-semibold">{{ t('admin.showcase.form.sections.attribution') }}</legend>

        <div class="mt-3 grid gap-4 sm:grid-cols-2">
          <UiSelect
            :model-value="values.person_id ?? ''"
            :options="personOptions"
            :label="t('admin.showcase.form.attribution.person')"
            :hint="t('admin.showcase.form.attribution.personHint')"
            @update:model-value="(next: string) => (values.person_id = next || null)"
          />

          <!-- La personne du répertoire PRIME (`COALESCE` de la vue publique) :
               le nom libre est neutralisé plutôt que laissé saisissable pour
               rien. -->
          <div data-field="author_name">
            <UiInput
              :model-value="values.author_name ?? ''"
              :label="t('admin.showcase.form.attribution.authorName')"
              :hint="personWins ? t('admin.showcase.form.attribution.authorNameOverridden') : undefined"
              :disabled="personWins"
              @update:model-value="(next: string) => (values.author_name = next || null)"
            />
          </div>
        </div>

        <div class="mt-4" data-field="author_title">
          <AdminEventsI18nField
            v-model="values.author_title"
            :label="t('admin.showcase.form.attribution.authorTitle')"
            :hint="t('admin.showcase.form.attribution.authorTitleHint')"
            :error="errorOf('author_title')"
          />
        </div>

        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <div data-field="organization_id">
            <UiSelect
              :model-value="values.organization_id ?? ''"
              :options="organizationOptions"
              :label="t('admin.showcase.form.attribution.organization')"
              :hint="t('admin.showcase.form.attribution.organizationHint')"
              :error="errorOf('organization_id')"
              @update:model-value="(next: string) => (values.organization_id = next || null)"
            />
          </div>

          <!-- RÈGLE MÉTIER N° 1 : une organisation se DÉSIGNE ou se NOMME.
               Retaper « IFDD » à côté d'une fiche existante recrée le doublon
               que la v2 corrige — `ck_highlights_organization_shape` le refuse. -->
          <div data-field="organization_label">
            <UiInput
              :model-value="values.organization_label ?? ''"
              :label="t('admin.showcase.form.attribution.organizationLabel')"
              :hint="
                organizationPicked
                  ? t('admin.showcase.form.attribution.organizationLabelBlocked')
                  : t('admin.showcase.form.attribution.organizationLabelHint')
              "
              :error="errorOf('organization_label')"
              :disabled="organizationPicked"
              @update:model-value="(next: string) => (values.organization_label = next || null)"
            />
          </div>
        </div>

        <div class="mt-4 sm:max-w-sm">
          <UiSelect
            :model-value="values.country_id ?? ''"
            :options="countryOptions"
            :label="t('admin.showcase.form.attribution.country')"
            :hint="t('admin.showcase.form.attribution.countryHint')"
            @update:model-value="(next: string) => (values.country_id = next || null)"
          />
        </div>
      </fieldset>

      <!-- ================= MÉDIA ================= -->
      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.submitting">
        <legend class="px-2 font-semibold">{{ t('admin.showcase.form.sections.media') }}</legend>
        <p class="mt-2 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.showcase.form.media.intro') }}
        </p>

        <AdminShowcaseMediaPanel class="mt-4" :media="props.screen.media" />

        <div class="mt-4 sm:max-w-xs" data-field="background_color_hex">
          <UiInput
            :model-value="values.background_color_hex ?? ''"
            :label="t('admin.showcase.form.media.color')"
            :hint="t('admin.showcase.form.media.colorHint')"
            :error="errorOf('background_color_hex')"
            placeholder="#RRGGBB"
            :maxlength="7"
            @update:model-value="(next: string) => (values.background_color_hex = next || null)"
          />
        </div>
      </fieldset>

      <!-- ================= RATTACHEMENT ================= -->
      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.submitting">
        <legend class="px-2 font-semibold">{{ t('admin.showcase.form.sections.link') }}</legend>

        <div class="mt-3 grid gap-4 sm:grid-cols-2">
          <div data-field="event_id">
            <UiSelect
              :model-value="values.event_id ?? ''"
              :options="eventOptions"
              :label="t('admin.showcase.form.link.event')"
              :hint="t('admin.showcase.form.link.eventHint')"
              :error="errorOf('event_id')"
              @update:model-value="setEvent"
            />
          </div>

          <div data-field="session_id">
            <UiSelect
              :model-value="values.session_id ?? ''"
              :options="sessionOptions"
              :label="t('admin.showcase.form.link.session')"
              :hint="
                props.sessionsLoading
                  ? t('admin.showcase.form.link.sessionLoading')
                  : t('admin.showcase.form.link.sessionHint')
              "
              :error="errorOf('session_id')"
              :disabled="values.event_id === null || props.sessionsLoading"
              @update:model-value="setSession"
            />
          </div>
        </div>

        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <div data-field="link_url">
            <UiInput
              :model-value="values.link_url ?? ''"
              type="url"
              :label="t('admin.showcase.form.link.url')"
              :hint="t('admin.showcase.form.link.urlHint')"
              :error="errorOf('link_url')"
              placeholder="https://"
              @update:model-value="(next: string) => (values.link_url = next || null)"
            />
          </div>

          <div data-field="link_label">
            <AdminEventsI18nField
              v-model="values.link_label"
              :label="t('admin.showcase.form.link.label')"
              :hint="t('admin.showcase.form.link.labelHint')"
              :error="errorOf('link_label')"
            />
          </div>
        </div>
      </fieldset>

      <!-- ================= DIFFUSION ================= -->
      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.submitting">
        <legend class="px-2 font-semibold">{{ t('admin.showcase.form.sections.broadcast') }}</legend>

        <div class="mt-3 grid gap-4 sm:grid-cols-2">
          <UiSelect
            :model-value="values.status"
            :options="statusOptions"
            :label="t('admin.showcase.form.broadcast.status')"
            @update:model-value="(next: string) => (values.status = next as HighlightStatus)"
          />

          <div data-field="sort_order">
            <UiInput
              :model-value="values.sort_order"
              type="number"
              :min="0"
              :label="t('admin.showcase.form.broadcast.order')"
              :hint="t('admin.showcase.form.broadcast.orderHint')"
              @update:model-value="(next: string) => (values.sort_order = Number(next) || 0)"
            />
          </div>
        </div>

        <div class="mt-4 grid gap-4 sm:grid-cols-2">
          <div data-field="starts_at">
            <!-- L'aide GÉNÉRIQUE du sélecteur de date parle du « fuseau de
                 l'événement » : ici il n'y en a pas, la fenêtre vaut pour toute
                 la plateforme. On la remplace plutôt que de laisser un texte
                 juste ailleurs et faux ici. -->
            <UiDatePicker
              :model-value="wall.starts_at"
              with-time
              :label="t('admin.showcase.form.broadcast.startsAt')"
              :hint="t('admin.showcase.form.broadcast.windowHint')"
              :timezone-label="t('admin.showcase.form.broadcast.timezone')"
              @update:model-value="(next: string) => setWall('starts_at', next)"
            />
          </div>

          <div data-field="ends_at">
            <UiDatePicker
              :model-value="wall.ends_at"
              with-time
              :label="t('admin.showcase.form.broadcast.endsAt')"
              :hint="t('admin.showcase.form.broadcast.windowHint')"
              :timezone-label="t('admin.showcase.form.broadcast.timezone')"
              :min="wall.starts_at || undefined"
              :error="errorOf('ends_at')"
              @update:model-value="(next: string) => setWall('ends_at', next)"
            />
          </div>
        </div>

        <!-- LE STATUT NE SUFFIT PAS À DIRE CE QUI EST À L'ÉCRAN : « publiée »
             plus une fenêtre passée, c'est un contenu invisible. On le dit ici
             plutôt que de laisser l'éditeur le découvrir sur l'accueil. -->
        <div class="mt-4 flex flex-wrap items-center gap-3 rounded-md border border-border bg-surface-sunken px-3 py-2.5">
          <AdminShowcaseStateBadge
            :state="broadcastState"
            :label="t(`admin.showcase.form.state.${broadcastState}`)"
            size="sm"
          />
          <p class="min-w-0 text-sm text-text-secondary">
            {{ t(`admin.showcase.form.stateHint.${broadcastState}`) }}
          </p>
        </div>
      </fieldset>

      <div class="flex flex-wrap items-center gap-3">
        <UiButton type="submit" :loading="props.submitting">{{ props.submitLabel }}</UiButton>
        <UiButton variant="ghost" :disabled="props.submitting" @click="emit('cancel')">
          {{ t('common.actions.cancel') }}
        </UiButton>
      </div>
    </div>

    <!-- L'APERÇU. Collant sur écran large : on écrit en le regardant, et il n'y
         a pas d'intérêt à devoir remonter pour le retrouver. -->
    <aside class="min-w-0 xl:sticky xl:top-24 xl:self-start">
      <AdminShowcasePreview :row="preview" />
    </aside>
  </form>
</template>
