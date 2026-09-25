<script setup lang="ts">
import type { EffectivePermission } from '~/types/identity'
import type { OfficialImportAdmin, UpdateOfficialImportPayload } from '~/types/negotiation-sessions'
import type { PublicEditionRow } from '~/types/views'
import type { SelectOption } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'IMPORT DES SESSIONS OFFICIELLES (FR-040). L'interrupteur agit sur-le-champ ;
 * le réglage s'enregistre d'un bouton. Portée globale, comme tout Guide Négo.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationImport' }],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()

useHead(() => ({ title: t('admin.negociations.import.title') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)
const canManage = computed(() => hasPermission(granted.value, 'negotiation.space.manage'))

const { data: editions, status: editionsStatus } = await useAsyncData<PublicEditionRow[]>(
  'admin-negotiation-cop-editions',
  () => api.events.publicList(),
  { default: () => [], lazy: true },
)
const choixEditions = computed(() => editionsDeCop(editions.value, locale.value))
const slug = computed(() =>
  typeof route.query.edition === 'string' && route.query.edition
    ? route.query.edition
    : (choixEditions.value.parDefaut ?? ''),
)

const { data: etat, status, error, refresh } = await useAsyncData<OfficialImportAdmin | null>(
  'admin-negotiation-import',
  () => (slug.value ? api.adminNegotiations.importOfficiel(slug.value) : Promise.resolve(null)),
  { lazy: true, watch: [slug] },
)

const isForbidden = computed(
  () => (!canManage.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)

function choisirLEdition(valeur: string): void {
  router.replace({ query: { ...route.query, edition: valeur } })
}

/** Le formulaire : une copie du réglage en base, reposée à chaque réponse. */
const brouillon = ref<UpdateOfficialImportPayload | null>(null)
const reglageDe = (e: OfficialImportAdmin): UpdateOfficialImportPayload => ({
  enabled: e.enabled,
  reader: e.reader,
  archive_name: e.archive_name ?? e.archives[0] ?? null,
  archive_first_day: e.archive_first_day,
  live_url: e.live_url,
  time_correction_minutes: e.time_correction_minutes,
  official_programme_url: e.official_programme_url,
  interval_seconds: e.interval_seconds,
  missed_threshold: e.missed_threshold,
})
watch(etat, (e) => (brouillon.value = e ? reglageDe(e) : null), { immediate: true })

const lecteurs = computed<SelectOption[]>(() =>
  (['archive', 'live'] as const).map((r) => ({
    value: r,
    label: t(`admin.negociations.import.settings.readers.${r}.label`),
    description: t(`admin.negociations.import.settings.readers.${r}.description`),
  })),
)
const jeux = computed<SelectOption[]>(() => (etat.value?.archives ?? []).map((j) => ({ value: j, label: j })))
const zone = computed(() => (etat.value ? timeZoneCityLabel(etat.value.edition.timezone as TimeZoneName) : ''))

const occupe = ref<'switch' | 'save' | 'read' | null>(null)
const erreur = ref<{ message: string; field: string | null } | null>(null)
const annonce = ref<string | null>(null)
const erreurDe = (champ: string) => (erreur.value?.field === champ ? erreur.value.message : undefined)

function retenirLErreur(e: unknown): void {
  const refus = normalizeApiError(e)
  erreur.value = {
    message: apiErrorMessage(e, t),
    field: refus instanceof ApiRequestError ? refus.field : null,
  }
}

async function ecrire(reglage: UpdateOfficialImportPayload, geste: 'switch' | 'save'): Promise<void> {
  if (!slug.value || occupe.value) return
  occupe.value = geste
  erreur.value = null
  annonce.value = null
  try {
    etat.value = await api.adminNegotiations.reglerLImport(slug.value, reglage)
    if (geste === 'save') annonce.value = t('admin.negociations.import.settings.saved')
  } catch (e) {
    retenirLErreur(e)
  } finally {
    occupe.value = null
  }
}

/** L'interrupteur repart du réglage EN BASE : un brouillon non enregistré ne s'envoie pas avec lui. */
function basculer(allume: boolean): void {
  if (etat.value) void ecrire({ ...reglageDe(etat.value), enabled: allume }, 'switch')
}

function enregistrer(): void {
  if (brouillon.value && etat.value) void ecrire({ ...brouillon.value, enabled: etat.value.enabled }, 'save')
}

let relecture: ReturnType<typeof setTimeout> | null = null
async function lireMaintenant(): Promise<void> {
  if (!slug.value || occupe.value) return
  occupe.value = 'read'
  erreur.value = null
  annonce.value = null
  try {
    await api.adminNegotiations.lireMaintenant(slug.value)
    annonce.value = t('admin.negociations.import.readNow.done')
    if (relecture) clearTimeout(relecture)
    relecture = setTimeout(() => void refresh(), 4000)
  } catch (e) {
    retenirLErreur(e)
  } finally {
    occupe.value = null
  }
}
onBeforeUnmount(() => relecture && clearTimeout(relecture))

const nombre = (valeur: string): number => (valeur.trim() === '' ? Number.NaN : Number(valeur))
</script>

<template>
  <div class="mx-auto w-full max-w-4xl">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.import.forbidden.scope')"
      :description="t('admin.negociations.import.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-4">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.negociations.import.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.import.subtitle') }}</p>
        </div>
        <UiSelect
          v-if="choixEditions.options.length > 1"
          class="w-full sm:w-56"
          :label="t('admin.negociations.import.edition')"
          :options="choixEditions.options"
          :model-value="slug"
          hide-optional
          @update:model-value="choisirLEdition"
        />
      </header>

      <UiEmptyState
        v-if="editionsStatus === 'success' && !choixEditions.options.length"
        class="mt-8"
        icon="calendar"
        :title="t('admin.negociations.import.noEdition.title')"
        :description="t('admin.negociations.import.noEdition.description')"
      />

      <UiErrorState v-else-if="error" class="mt-8" :retry-label="t('common.actions.retry')" @retry="refresh()" />

      <UiLoadingState v-else-if="!etat || !brouillon" class="mt-8" variant="card" />

      <template v-else>
        <UiAlert v-if="erreur && !erreur.field" class="mt-6" intent="danger" live :message="erreur.message" />
        <UiAlert v-else-if="annonce" class="mt-6" intent="success" live dismissible :message="annonce" />

        <section class="mt-8 flex flex-wrap items-center justify-between gap-4 rounded-lg border border-border p-4">
          <UiSwitch
            :model-value="etat.enabled"
            :label="t('admin.negociations.import.switch.label')"
            :hint="t('admin.negociations.import.switch.hint')"
            :loading="occupe === 'switch'"
            @update:model-value="basculer"
          />
          <div class="flex flex-col items-start gap-1">
            <UiButton variant="secondary" icon="refresh" :loading="occupe === 'read'" @click="lireMaintenant">
              {{ t('admin.negociations.import.readNow.action') }}
            </UiButton>
            <span class="text-xs text-text-muted">{{ t('admin.negociations.import.readNow.hint') }}</span>
          </div>
        </section>

        <AdminNegotiationImportStatus class="mt-8" :etat="etat" />

        <form class="mt-10 space-y-6" novalidate @submit.prevent="enregistrer">
          <h2 class="text-lg font-semibold">{{ t('admin.negociations.import.settings.title') }}</h2>

          <UiRadio
            name="import-reader"
            :label="t('admin.negociations.import.settings.reader')"
            :options="lecteurs"
            :model-value="brouillon.reader"
            :error="erreurDe('reader')"
            @update:model-value="(v: string) => brouillon && (brouillon.reader = v === 'live' ? 'live' : 'archive')"
          />

          <div v-if="brouillon.reader === 'archive'" class="grid gap-6 sm:grid-cols-2">
            <UiSelect
              v-model="brouillon.archive_name"
              :label="t('admin.negociations.import.settings.archive')"
              :options="jeux"
              :error="erreurDe('archive_name')"
              required
            />
            <UiDatePicker
              :model-value="brouillon.archive_first_day"
              :label="t('admin.negociations.import.settings.firstDay')"
              :hint="t('admin.negociations.import.settings.firstDayHint', { zone })"
              @update:model-value="(v: string) => brouillon && (brouillon.archive_first_day = v || null)"
            />
          </div>

          <UiInput
            v-else
            type="url"
            :model-value="brouillon.live_url"
            :label="t('admin.negociations.import.settings.liveUrl')"
            :hint="t('admin.negociations.import.settings.liveUrlHint')"
            :error="erreurDe('live_url')"
            required
            block
            @update:model-value="(v: string) => brouillon && (brouillon.live_url = v || null)"
          />

          <UiInput
            v-model="brouillon.official_programme_url"
            type="url"
            :label="t('admin.negociations.import.settings.programmeUrl')"
            :hint="t('admin.negociations.import.settings.programmeUrlHint')"
            :error="erreurDe('official_programme_url')"
            required
            block
          />

          <div class="grid gap-6 sm:grid-cols-3">
            <UiInput
              type="number"
              :min="60"
              :max="86400"
              step="60"
              :model-value="brouillon.interval_seconds"
              :label="t('admin.negociations.import.settings.interval')"
              :hint="t('admin.negociations.import.settings.intervalHint')"
              :error="erreurDe('interval_seconds')"
              required
              @update:model-value="(v: string) => brouillon && (brouillon.interval_seconds = nombre(v))"
            />
            <UiInput
              type="number"
              :min="1"
              :max="100"
              :model-value="brouillon.missed_threshold"
              :label="t('admin.negociations.import.settings.threshold')"
              :hint="t('admin.negociations.import.settings.thresholdHint')"
              :error="erreurDe('missed_threshold')"
              required
              @update:model-value="(v: string) => brouillon && (brouillon.missed_threshold = nombre(v))"
            />
            <UiInput
              type="number"
              step="15"
              :model-value="brouillon.time_correction_minutes"
              :label="t('admin.negociations.import.settings.correction')"
              :hint="t('admin.negociations.import.settings.correctionHint')"
              :error="erreurDe('time_correction_minutes')"
              required
              @update:model-value="(v: string) => brouillon && (brouillon.time_correction_minutes = nombre(v))"
            />
          </div>

          <UiButton type="submit" :loading="occupe === 'save'">
            {{ t('admin.negociations.import.settings.save') }}
          </UiButton>
        </form>
      </template>
    </template>
  </div>
</template>
