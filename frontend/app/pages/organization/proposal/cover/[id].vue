<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'
import type { ProposalFile } from '~/types/organization-workspace'
import type { EditableProposal } from '~/types/proposal-form'
import type { CoverDesignOptions, CoverStudioContext } from '~/types/programme/cover-design'
import type { AssetId } from '~/types/shared'
import type { AttachedImage } from '~/types/media'
import type { CoverSceneSource } from '~/composables/useCoverScene'
import type { CoverAccent, CoverDate, CoverFormat } from '~/utils/cover-studio'
import { findCoverTemplate } from '~/utils/cover-studio'

/**
 * L'ÉDITEUR DE L'IMAGE DE COUVERTURE — `/mon-organisation/dossiers/:id/couverture`.
 *
 * Direction « table de montage » (31/10) : outils à gauche (modèles, photo,
 * textes), l'image au centre avec ses trois sorties dessous, ce qui figure
 * dessus et sa couleur à droite. Tout se voit d'un coup d'œil et l'image se
 * redessine à chaque geste. « Enregistrer » dépose exactement ce qui est montré
 * comme couverture du dossier ; quitter avec des changements le demande.
 * `?nouveau=1` repart des choix par défaut.
 */

definePageMeta({
  layout: 'public',
  middleware: ['auth', 'requires-organization'],
  organizationReason: 'organization-space',
  actingForOrganization: true,
  fullBleed: true,
})

defineI18nRoute({
  paths: { fr: '/mon-organisation/dossiers/[id]/couverture', en: '/my-organization/submissions/[id]/cover' },
})

const { t } = useI18n()
const { tr } = useI18nText()
const { date, time, timeWithZone, dateRange, zoneLabel, intlLocale } = useDateTime()
const memberships = useMembershipStore()
const localePath = useLocalePath()
const route = useRoute()
const api = useApi()
const auth = useAuthStore()
const toast = useToast()

const proposalId = computed(() => String(route.params.id ?? ''))
const fileTo = computed(() => localePath(`/mon-organisation/dossiers/${proposalId.value}`))

interface StudioData {
  file: ProposalFile
  editable: EditableProposal
  studio: CoverStudioContext
}

const { data, status, error, refresh } = await useAsyncData<StudioData | null>(
  () => `cover-studio-${proposalId.value}`,
  async () => {
    const person = auth.person
    if (!person) return null
    const [file, editable, studio] = await Promise.all([
      api.workspace.proposalFile(proposalId.value, person.id),
      api.proposals.forEdit(proposalId.value),
      api.proposals.coverStudio(proposalId.value),
    ])
    return file && editable ? { file, editable, studio } : null
  },
  { lazy: true, watch: [proposalId] },
)

useHead(() => ({ title: t('organization.workspace.cover.title') }))

// ---------------------------------------------------------------------------
// Les choix
// ---------------------------------------------------------------------------

const template = ref<string>(COVER_TEMPLATES[0]?.code ?? 'texte-photo')
/** Code d'une couleur de la charte, ou couleur libre `#rrggbb`. */
const accent = ref<string>('bleu')
/** La dernière couleur libre, gardée pour y revenir après avoir essayé la charte. */
const customAccent = ref<string | null>(null)
const options = ref<CoverDesignOptions>({ ...DEFAULT_COVER_OPTIONS })

const isAccent = (value: string): value is CoverAccent => value in COVER_ACCENTS

const choices = () => JSON.stringify({ template: template.value, accent: accent.value, options: options.value })
const savedChoices = ref<string | null>(null)
const isDirty = computed(() => savedChoices.value !== null && savedChoices.value !== choices())
const savedOnce = ref(false)

// Les choix enregistrés reviennent une fois, à l'ouverture : une relecture après
// le dépôt d'une photo ne doit pas effacer ce qu'on est en train de composer.
watch(
  data,
  (current) => {
    if (!current || savedChoices.value !== null) return
    const design = current.studio.design
    if (design && route.query.nouveau !== '1') {
      template.value = findCoverTemplate(design.template_code).code
      if (isAccent(design.accent)) accent.value = design.accent
      else if (CUSTOM_ACCENT.test(design.accent)) accent.value = customAccent.value = design.accent
      options.value = { ...DEFAULT_COVER_OPTIONS, ...design.options }
    }
    savedChoices.value = choices()
  },
  { immediate: true },
)

const templateIndex = computed(() => COVER_TEMPLATES.findIndex((entry) => entry.code === template.value))
function stepTemplate(step: number): void {
  const count = COVER_TEMPLATES.length
  template.value = COVER_TEMPLATES[(templateIndex.value + step + count) % count]?.code ?? template.value
}

const usesPhoto = computed(() => findCoverTemplate(template.value).usesPhoto)
const editionIcon = computed(() => data.value?.studio.edition_logo ?? data.value?.studio.edition_thumbnail ?? null)
const hasLogo = computed(() => Boolean(data.value?.studio.organization_logo))

type Tool = 'templates' | 'photo' | 'texts'
const TOOLS: { key: Tool; icon: string }[] = [
  { key: 'templates', icon: 'grid' },
  { key: 'photo', icon: 'image' },
  { key: 'texts', icon: 'edit' },
]
const tool = ref<Tool>('templates')
const format = ref<CoverFormat>('cover')

const pickerOpen = ref(false)
function applyCustom(hex: string): void {
  customAccent.value = hex
  accent.value = hex
}

// ---------------------------------------------------------------------------
// La photo de fond : celle de l'édition d'office, celle de l'organisation sur demande
// ---------------------------------------------------------------------------

// `show_event_banner` dit laquelle sert ; la photo de l'organisation reste
// rattachée quand on revient à celle de l'édition, pour y retourner sans la redéposer.
const eventPhoto = computed(() => data.value?.studio.edition_cover ?? data.value?.studio.edition_banner ?? null)
// La photo déposée à l'instant compte avant que son traitement la rende servable.
const uploadedPhoto = ref<AttachedImage | null>(null)
const ownPhoto = computed(() => data.value?.studio.source_photo ?? uploadedPhoto.value)
const usingEventPhoto = computed(() => Boolean(eventPhoto.value) && (options.value.show_event_banner || !ownPhoto.value))
const backgroundPhoto = computed(() => (usingEventPhoto.value ? eventPhoto.value : ownPhoto.value))
const editionName = computed(() => {
  const edition = data.value?.file.tracking.edition
  return edition ? (edition.acronym ?? tr(edition.title)) : ''
})

const photoError = ref<string | null>(null)

async function attachSourcePhoto(assetId: AssetId | null): Promise<void> {
  if (!assetId) return
  photoError.value = null
  try {
    await api.media.attach({
      owner_schema: 'programme',
      owner_table: 'proposals',
      owner_id: proposalId.value,
      assignments: [{ role: 'cover_source', asset_id: assetId }],
    })
    options.value.show_event_banner = false
    await refresh()
  } catch (thrown) {
    photoError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

const { data: mediaRules } = useAsyncData(
  'cover-studio-media-rules',
  async () => {
    const [proposalRules, organizationRules] = await Promise.all([
      api.media.roles('programme', 'proposals'),
      api.media.roles('org', 'organizations'),
    ])
    return {
      photo: proposalRules.find((rule) => rule.role === 'cover_source') ?? null,
      logo: organizationRules.find((rule) => rule.role === 'logo') ?? null,
    }
  },
  { lazy: true, server: false },
)

// ---------------------------------------------------------------------------
// Le logo de l'organisation : posé d'ici, il sert à tous ses dossiers
// ---------------------------------------------------------------------------

const leadOrganizationId = computed(() => data.value?.file.tracking.proposal.organization_id ?? null)
const leadMembership = computed(() => memberships.submittable.find((entry) => entry.organization.id === leadOrganizationId.value))
// Seul un référent pose le logo (garde du module média) : les autres lisent qui le peut.
const canSetLogo = computed(() => leadMembership.value?.membership.role === 'manager')
const logoError = ref<string | null>(null)

async function attachLogo(assetId: AssetId | null): Promise<void> {
  if (!assetId || !leadOrganizationId.value) return
  logoError.value = null
  try {
    await api.media.attach({
      owner_schema: 'org',
      owner_table: 'organizations',
      owner_id: leadOrganizationId.value,
      assignments: [{ role: 'logo', asset_id: assetId }],
    })
    await refresh()
  } catch (thrown) {
    logoError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

// ---------------------------------------------------------------------------
// La scène
// ---------------------------------------------------------------------------

/** Jour, mois, heure… pris un à un dans le fuseau de l'édition, pour les modèles qui composent la date. */
function dateParts(at: string, timezone: string, withTime: boolean): CoverDate {
  const part = (options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(intlLocale.value, { timeZone: timezone, ...options }).format(new Date(at))
  return {
    day: part({ day: 'numeric' }),
    month: part({ month: 'long' }),
    monthShort: part({ month: 'short' }),
    weekday: part({ weekday: 'long' }),
    year: part({ year: 'numeric' }),
    time: withTime ? time(at, timezone) : null,
    zone: zoneLabel(timezone, data.value?.file.tracking.edition.city ?? undefined),
  }
}

// Le super-administrateur qui agit pour l'organisation n'en est pas membre : son nom se lit sur sa fiche.
const { data: actingOrganization } = useAsyncData(
  () => `cover-studio-acting-organization-${proposalId.value}`,
  async () => {
    const file = data.value?.file
    return file?.acting_for_organization ? api.organizations.byId(file.tracking.proposal.organization_id) : null
  },
  { server: false, lazy: true, default: () => null, watch: [() => data.value?.file.acting_for_organization] },
)

const organizationName = computed(
  () => leadMembership.value?.organization.legal_name ?? actingOrganization.value?.legal_name ?? '',
)

const source = computed<CoverSceneSource | null>(() => {
  const current = data.value
  if (!current) return null
  const { file, editable, studio } = current
  const edition = file.tracking.edition
  const session = [...file.tracking.sessions].sort((a, b) => a.session.starts_at.localeCompare(b.session.starts_at))[0]
  return {
    title: editable.draft.title,
    eyebrow: t('organization.workspace.cover.eyebrow', { edition: edition.acronym ?? tr(edition.title) }),
    dateLine: session
      ? `${date(session.session.starts_at, edition.timezone)} · ${timeWithZone(session.session.starts_at, edition.timezone, edition.city ?? undefined)}`
      : dateRange(edition.starts_at, edition.ends_at, edition.timezone),
    placeLine: [session?.room ? tr(session.room.name) : null, edition.city].filter(Boolean).join(', ') || null,
    date: dateParts(session?.session.starts_at ?? edition.starts_at, edition.timezone, Boolean(session)),
    room: session?.room ? tr(session.room.name) : null,
    city: edition.city ?? null,
    organizationName: organizationName.value || null,
    speakers: editable.draft.speakers.map((speaker) => ({
      name: speakerName(speaker),
      imageUrl: speaker.photo_image?.url ?? null,
    })),
    photoUrl: backgroundPhoto.value?.url ?? null,
    organizationLogoUrl: studio.organization_logo?.url ?? null,
    iconUrl: editionIcon.value?.url ?? null,
  }
})

const accentToken = computed(() => (isAccent(accent.value) ? COVER_ACCENTS[accent.value] : accent.value))
const scene = useCoverScene(source, accentToken)

const hints = computed(() => {
  const speakers = data.value?.editable.draft.speakers.length ?? 0
  return {
    show_date: source.value?.dateLine ?? '',
    show_speakers: speakers
      ? t('organization.workspace.cover.hints.speakers', { count: speakers }, speakers)
      : t('organization.workspace.cover.hints.noSpeakers'),
    show_partner_logos: t('organization.workspace.cover.hints.partners'),
    show_org_logo: t(hasLogo.value ? 'organization.workspace.cover.hints.orgLogoSet' : 'organization.workspace.cover.hints.orgLogoMissing'),
    ...(editionIcon.value ? { show_event_icon: editionName.value } : {}),
  }
})

const preview = ref<{ toJpeg: (maxBytes: number) => Promise<Blob | null> } | null>(null)

/** La version haute qualité : celle de la page de détail. Les listes et le partage ont leurs déclinaisons, faites par le serveur. */
const COVER_MAX_BYTES = 300 * 1024

// ---------------------------------------------------------------------------
// Enregistrer : l'image montrée devient la couverture du dossier
// ---------------------------------------------------------------------------

const saving = ref(false)
const saveError = ref<string | null>(null)

async function save(): Promise<boolean> {
  const current = data.value
  if (!current || !preview.value) return false
  saving.value = true
  saveError.value = null
  const saved = choices()
  try {
    const blob = await preview.value.toJpeg(COVER_MAX_BYTES)
    if (!blob) {
      saveError.value = t('organization.workspace.cover.renderFailed')
      return false
    }
    const asset = await api.media.upload({
      file: blob,
      filename: `${current.file.tracking.proposal.reference_code}-couverture.jpg`,
      mimeType: 'image/jpeg',
      altText: { fr: current.editable.draft.title },
      ownerSchema: 'programme',
      ownerTable: 'proposals',
      ownerId: proposalId.value,
      role: 'cover',
    })
    await api.media.attach({
      owner_schema: 'programme',
      owner_table: 'proposals',
      owner_id: proposalId.value,
      assignments: [{ role: 'cover', asset_id: asset.id }],
    })
    await api.proposals.saveCoverDesign(proposalId.value, {
      template_code: template.value,
      accent: accent.value,
      options: options.value,
    })
    savedChoices.value = saved
    savedOnce.value = true
    toast.success(t('organization.workspace.cover.savedToast'))
    return true
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, (key) => t(key))
    return false
  } finally {
    saving.value = false
  }
}

// ---------------------------------------------------------------------------
// Quitter avec des changements : on demande
// ---------------------------------------------------------------------------

const pendingLeave = ref<RouteLocationRaw | null>(null)
let leaving = false

onBeforeRouteLeave((to) => {
  if (leaving || !isDirty.value) return true
  pendingLeave.value = to.fullPath
  return false
})

async function leave(saveFirst: boolean): Promise<void> {
  const target = pendingLeave.value
  if (!target) return
  if (saveFirst && !(await save())) {
    pendingLeave.value = null
    return
  }
  leaving = true
  pendingLeave.value = null
  await navigateTo(target)
}

if (import.meta.client) {
  // Fermer l'onglet ou recharger : seul le navigateur peut poser la question, avec son propre texte.
  const warn = (event: BeforeUnloadEvent) => {
    if (!isDirty.value) return
    event.preventDefault()
    event.returnValue = ''
  }
  onMounted(() => window.addEventListener('beforeunload', warn))
  onBeforeUnmount(() => window.removeEventListener('beforeunload', warn))
}

// L'image tient en entier sous la barre avec ce qui l'entoure (≈ 29rem) : sa largeur suit la hauteur de l'écran.
const stageWidth = computed(() =>
  format.value === 'mail'
    ? '56rem'
    : 'min(48rem, max(26rem, calc((100dvh - var(--nav-height) - var(--topbar-height) - 29rem) * 16 / 9)))',
)

const statusKey = computed(() => (isDirty.value ? 'dirty' : savedOnce.value ? 'saved' : 'clean'))
</script>

<template>
  <div class="flex flex-col lg:h-[calc(100dvh-var(--nav-height)-var(--topbar-height))]">
    <div v-if="status === 'pending' || error || !data" class="mx-auto w-full max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <UiLoadingState v-if="status === 'pending'" variant="card" :lines="6" />
      <UiErrorState v-else-if="error" :retry-label="t('common.actions.retry')" @retry="refresh()" />
      <UiEmptyState
        v-else
        icon="document"
        :title="t('organization.workspace.proposal.notFound.title')"
        :description="t('organization.workspace.proposal.notFound.description')"
        :action-to="localePath('/mon-organisation')"
        :action-label="t('organization.workspace.proposal.notFound.action')"
      />
    </div>

    <template v-else>
      <header class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-border bg-surface-raised px-3 py-2 sm:px-4">
        <UiButton variant="ghost" icon="arrow-left" :to="fileTo">
          {{ t('organization.workspace.cover.backTo', { reference: data.file.tracking.proposal.reference_code }) }}
        </UiButton>
        <div class="hidden h-7 w-px bg-border sm:block" aria-hidden="true" />
        <div class="min-w-0 flex-1 basis-48">
          <h1 class="text-base leading-tight font-semibold">{{ t('organization.workspace.cover.title') }}</h1>
          <p class="truncate text-sm text-text-muted">{{ tr(data.file.tracking.proposal.title) }}</p>
        </div>
        <p role="status" class="flex items-center gap-2 text-sm text-text-secondary">
          <span class="size-2 rounded-full" :class="isDirty ? 'bg-warning-solid' : 'bg-success-solid'" aria-hidden="true" />
          {{ t(`organization.workspace.cover.status.${statusKey}`) }}
        </p>
        <UiButton variant="primary" icon="check" :loading="saving" :disabled="!scene" @click="save">
          {{ t('organization.workspace.cover.save') }}
        </UiButton>
      </header>

      <UiAlert v-if="saveError" class="mx-3 mt-3 sm:mx-4" intent="danger" :message="saveError" live />

      <div class="flex min-h-0 flex-1 flex-col lg:grid lg:grid-cols-[4.5rem_19rem_minmax(0,1fr)_18rem]">
        <!-- LES OUTILS -->
        <nav
          :aria-label="t('organization.workspace.cover.tools.label')"
          class="flex gap-1 border-y border-border bg-surface-raised p-1.5 max-lg:order-2 lg:flex-col lg:border-y-0 lg:border-r lg:p-2"
        >
          <button
            v-for="entry in TOOLS"
            :key="entry.key"
            type="button"
            :aria-pressed="tool === entry.key"
            class="flex min-h-16 flex-1 cursor-pointer flex-col items-center justify-center gap-1 rounded-lg text-xs font-semibold transition-colors duration-(--duration-fast) lg:flex-none"
            :class="tool === entry.key ? 'bg-accent-surface text-accent' : 'text-text-secondary hover:bg-surface-hover'"
            @click="tool = entry.key"
          >
            <UiIcon :name="entry.icon" size="1.375rem" />
            {{ t(`organization.workspace.cover.tools.${entry.key}`) }}
          </button>
        </nav>

        <!-- LE PANNEAU DE L'OUTIL -->
        <section class="bg-surface-raised px-5 py-5 max-lg:order-3 lg:overflow-y-auto lg:border-r lg:border-border">
          <template v-if="tool === 'templates'">
            <h2 class="text-lg font-bold">{{ t('organization.workspace.cover.tools.templates') }}</h2>
            <p class="mt-1 mb-5 text-sm text-text-muted">{{ t('organization.workspace.cover.templatesHint') }}</p>
            <CoverStudioTemplateList :scene="scene" :options="options" :selected="template" @select="template = $event" />
          </template>

          <template v-else-if="tool === 'photo'">
            <h2 class="text-lg font-bold">{{ t('organization.workspace.cover.photo.title') }}</h2>
            <p class="mt-1 mb-5 text-sm text-text-muted">
              {{ t(usesPhoto ? 'organization.workspace.cover.photo.hint' : 'organization.workspace.cover.photo.unused') }}
            </p>
            <div class="flex flex-col gap-3">
              <button
                v-if="eventPhoto"
                type="button"
                :aria-pressed="usingEventPhoto"
                class="flex cursor-pointer items-center gap-3 rounded-lg border-2 p-2 text-left"
                :class="usingEventPhoto ? 'border-accent bg-accent-surface' : 'border-border hover:border-border-strong'"
                @click="options.show_event_banner = true"
              >
                <img :src="eventPhoto.url" alt="" class="h-12 w-18 shrink-0 rounded object-cover" />
                <span class="flex flex-col">
                  <span class="font-semibold">{{ t('organization.workspace.cover.photo.eventChoice') }}</span>
                  <span class="text-xs text-text-secondary">{{ t('organization.workspace.cover.photo.eventChoiceDetail', { edition: editionName }) }}</span>
                </span>
              </button>
              <button
                v-if="ownPhoto"
                type="button"
                :aria-pressed="!usingEventPhoto"
                class="flex cursor-pointer items-center gap-3 rounded-lg border-2 p-2 text-left"
                :class="!usingEventPhoto ? 'border-accent bg-accent-surface' : 'border-border hover:border-border-strong'"
                @click="options.show_event_banner = false"
              >
                <img :src="ownPhoto.url" alt="" class="h-12 w-18 shrink-0 rounded object-cover" />
                <span class="font-semibold">{{ t('organization.workspace.cover.photo.ownChoice') }}</span>
              </button>
              <MediaImageField
                role="cover_source"
                :asset-id="null"
                :label="t('organization.workspace.cover.photo.field')"
                :alt-fallback="data.editable.draft.title"
                :button-label="t(ownPhoto ? 'organization.workspace.cover.photo.replaceOwn' : 'organization.workspace.cover.photo.addOwn')"
                button-icon="upload"
                :rule="mediaRules?.photo ?? null"
                :owner="{ schema: 'programme', table: 'proposals', id: proposalId }"
                @update:asset-id="attachSourcePhoto"
                @update:image="(image) => (uploadedPhoto = image)"
              />
            </div>
          </template>

          <template v-else>
            <h2 class="text-lg font-bold">{{ t('organization.workspace.cover.tools.texts') }}</h2>
            <p class="mt-1 mb-5 text-sm text-text-muted">{{ t('organization.workspace.cover.texts.hint') }}</p>
            <dl class="flex flex-col gap-4">
              <div>
                <dt class="text-xs text-text-muted">{{ t('organization.workspace.cover.texts.title') }}</dt>
                <dd class="mt-0.5 font-medium">{{ data.editable.draft.title }}</dd>
              </div>
              <div v-if="source?.dateLine">
                <dt class="text-xs text-text-muted">{{ t('organization.workspace.cover.texts.date') }}</dt>
                <dd class="mt-0.5 font-medium">{{ source.dateLine }}</dd>
              </div>
              <div v-if="source?.placeLine">
                <dt class="text-xs text-text-muted">{{ t('organization.workspace.cover.texts.place') }}</dt>
                <dd class="mt-0.5 font-medium">{{ source.placeLine }}</dd>
              </div>
              <div v-if="source?.speakers.length">
                <dt class="text-xs text-text-muted">{{ t('organization.workspace.cover.texts.speakers') }}</dt>
                <dd class="mt-0.5 font-medium">{{ source.speakers.map((speaker) => speaker.name).join(', ') }}</dd>
              </div>
            </dl>
            <UiButton class="mt-5" variant="secondary" icon="edit" :to="fileTo">{{ t('organization.workspace.cover.texts.edit') }}</UiButton>
          </template>
        </section>

        <!-- L'IMAGE ET SES SORTIES -->
        <section
          :aria-label="t('organization.workspace.cover.previewRegion')"
          class="flex min-w-0 flex-col bg-surface-sunken bg-[radial-gradient(var(--color-border)_1px,transparent_1px)] bg-size-[20px_20px] max-lg:order-1 lg:min-h-0"
        >
          <div class="flex flex-1 flex-col items-center px-3 py-5 sm:px-6 lg:min-h-0 lg:overflow-y-auto lg:px-10">
            <div class="my-auto flex w-full flex-col gap-3" :style="{ maxWidth: stageWidth }">
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="flex items-center gap-1 rounded-lg border border-border bg-surface-raised p-1 shadow-sm">
                  <UiButton variant="ghost" icon="chevron-left" icon-only :label="t('organization.workspace.cover.previousTemplate')" @click="stepTemplate(-1)" />
                  <span class="min-w-32 text-center font-semibold">{{ t(`organization.workspace.cover.templates.${template}`) }}</span>
                  <span class="pr-1 text-xs text-text-subtle">{{ templateIndex + 1 }} / {{ COVER_TEMPLATES.length }}</span>
                  <UiButton variant="ghost" icon="chevron-right" icon-only :label="t('organization.workspace.cover.nextTemplate')" @click="stepTemplate(1)" />
                </div>
                <span class="text-sm text-text-secondary">
                  {{ t(`organization.workspace.cover.formats.${format}`) }}, {{ t(`organization.workspace.cover.formats.${format}Size`) }}
                </span>
              </div>

              <div
                v-if="!hasLogo && options.show_org_logo"
                role="alert"
                class="flex flex-col gap-3 rounded-lg border border-danger-border bg-danger-surface p-3 pl-4 sm:flex-row sm:items-center sm:justify-between"
              >
                <p class="text-sm text-text-secondary">
                  <strong class="text-danger">{{ t('organization.workspace.proposal.gaps.logo.title') }}</strong>
                  ·
                  {{
                    canSetLogo
                      ? t('organization.workspace.cover.logoMissing')
                      : t('organization.workspace.proposal.gaps.logo.askManager', { organization: organizationName })
                  }}
                </p>
                <MediaImageField
                  v-if="canSetLogo && leadOrganizationId"
                  class="shrink-0"
                  role="logo"
                  :asset-id="null"
                  :label="t('organization.workspace.proposal.gaps.logo.field')"
                  :button-label="t('organization.workspace.proposal.gaps.logo.action')"
                  button-icon="upload"
                  button-variant="danger"
                  :rule="mediaRules?.logo ?? null"
                  :owner="{ schema: 'org', table: 'organizations', id: leadOrganizationId }"
                  @update:asset-id="attachLogo"
                />
              </div>
              <UiAlert v-if="logoError" intent="danger" :message="logoError" live />

              <div class="overflow-hidden rounded-md shadow-lg" :class="format === 'mail' ? 'relative aspect-[6/1]' : ''">
                <ClientOnly>
                  <CoverStudioCanvas
                    ref="preview"
                    :scene="scene"
                    :template="template"
                    :options="options"
                    :label="t('organization.workspace.cover.previewLabel', { title: data.editable.draft.title })"
                    :class="format === 'mail' ? 'absolute inset-x-0 top-1/2 -translate-y-1/2' : ''"
                  />
                  <template #fallback>
                    <div class="aspect-video bg-surface-sunken" />
                  </template>
                </ClientOnly>
                <div v-if="format === 'share'" class="bg-surface-raised px-4 py-3">
                  <p class="text-xs text-text-subtle uppercase">{{ t('organization.workspace.cover.share.site') }}</p>
                  <p class="mt-0.5 line-clamp-2 text-sm font-semibold text-text">{{ data.editable.draft.title }}</p>
                </div>
              </div>
              <p v-if="format === 'mail'" class="text-sm text-text-secondary">{{ t('organization.workspace.cover.formats.mailHint') }}</p>

              <div
                v-if="usesPhoto"
                class="flex flex-col gap-3 rounded-lg border border-border bg-surface-raised p-3 pl-4 sm:flex-row sm:items-center sm:justify-between"
              >
                <p class="text-sm text-text-secondary">
                  <template v-if="!backgroundPhoto">{{ t('organization.workspace.cover.photo.missing') }}</template>
                  <template v-else-if="usingEventPhoto">{{ t('organization.workspace.cover.photo.event', { edition: editionName }) }}</template>
                  <template v-else>{{ t('organization.workspace.cover.photo.own') }}</template>
                </p>
                <MediaImageField
                  class="shrink-0"
                  role="cover_source"
                  :asset-id="null"
                  :label="t('organization.workspace.cover.photo.field')"
                  :alt-fallback="data.editable.draft.title"
                  :button-label="t('organization.workspace.cover.photo.change')"
                  :rule="mediaRules?.photo ?? null"
                  :owner="{ schema: 'programme', table: 'proposals', id: proposalId }"
                  @update:asset-id="attachSourcePhoto"
                  @update:image="(image) => (uploadedPhoto = image)"
                />
              </div>
              <UiAlert v-if="photoError" intent="danger" :message="photoError" live />
            </div>
          </div>

          <div class="shrink-0 border-t border-border bg-surface-raised/80 px-3 py-3">
            <CoverStudioFormats :scene="scene" :template="template" :options="options" :selected="format" @select="format = $event" />
          </div>
        </section>

        <!-- CE QUI FIGURE SUR L'IMAGE, ET SA COULEUR -->
        <aside class="bg-surface-raised max-lg:order-4 max-lg:border-t max-lg:border-border lg:overflow-y-auto lg:border-l lg:border-border">
          <CoverStudioSettings
            v-model:options="options"
            v-model:accent="accent"
            :hints="hints"
            :custom-accent="customAccent"
            @open-picker="pickerOpen = true"
          />
        </aside>
      </div>

      <CoverStudioColorPicker v-model:open="pickerOpen" :initial="customAccent ?? '#1b2a6b'" @apply="applyCustom" />

      <UiModal
        :open="pendingLeave !== null"
        :title="t('organization.workspace.cover.leave.title')"
        :description="t('organization.workspace.cover.leave.description')"
        @update:open="(open) => !open && (pendingLeave = null)"
      >
        <template #footer>
          <UiButton variant="ghost" @click="leave(false)">{{ t('organization.workspace.cover.leave.discard') }}</UiButton>
          <UiButton variant="secondary" @click="pendingLeave = null">{{ t('organization.workspace.cover.leave.stay') }}</UiButton>
          <UiButton variant="primary" :loading="saving" @click="leave(true)">{{ t('organization.workspace.cover.leave.saveAndLeave') }}</UiButton>
        </template>
      </UiModal>
    </template>
  </div>
</template>
