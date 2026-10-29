<script setup lang="ts">
import type { ProposalFile } from '~/types/organization-workspace'
import type { EditableProposal } from '~/types/proposal-form'
import type { CoverDesignOptions, CoverStudioContext } from '~/types/programme/cover-design'
import type { AssetId } from '~/types/shared'
import type { CoverSceneSource } from '~/composables/useCoverScene'
import type { CoverAccent } from '~/utils/cover-studio'
import { findCoverTemplate } from '~/utils/cover-studio'

/**
 * L'ATELIER DE L'IMAGE DE COUVERTURE — `/mon-organisation/dossiers/:id/couverture`.
 *
 * Un modèle, ce qui figure dessus, une couleur de la charte : l'aperçu se
 * redessine à chaque choix, et « Enregistrer » dépose exactement ce qui est
 * montré comme couverture du dossier. Elle sert aussi d'aperçu quand on partage
 * le lien. Les choix sont gardés pour que « Modifier » rouvre l'atelier là où on
 * l'a laissé ; `?nouveau=1` repart du catalogue.
 */

definePageMeta({
  layout: 'public',
  middleware: ['auth', 'requires-organization'],
  organizationReason: 'organization-space',
})

defineI18nRoute({
  paths: { fr: '/mon-organisation/dossiers/[id]/couverture', en: '/my-organization/submissions/[id]/cover' },
})

const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeWithZone, dateRange } = useDateTime()
const localePath = useLocalePath()
const route = useRoute()
const api = useApi()
const auth = useAuthStore()

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

const templates = COVER_TEMPLATES
const accents = Object.entries(COVER_ACCENTS) as [CoverAccent, string][]

const template = ref<string>(templates[0]?.code ?? 'texte-photo')
const accent = ref<CoverAccent>('bleu')
const options = ref<CoverDesignOptions>({ ...DEFAULT_COVER_OPTIONS })

const isAccent = (value: string): value is CoverAccent => value in COVER_ACCENTS

// Les choix enregistrés reviennent une fois, à l'ouverture : une relecture après
// le dépôt d'une photo ne doit pas effacer ce qu'on est en train de composer.
let restored = false
watch(
  data,
  (current) => {
    if (!current || restored) return
    restored = true
    const design = current.studio.design
    if (!design || route.query.nouveau === '1') return
    template.value = findCoverTemplate(design.template_code).code
    if (isAccent(design.accent)) accent.value = design.accent
    options.value = { ...DEFAULT_COVER_OPTIONS, ...design.options }
  },
  { immediate: true },
)

const usesPhoto = computed(() => findCoverTemplate(template.value).usesPhoto)
const hasBanner = computed(() => Boolean(data.value?.studio.edition_banner))
const hasIcon = computed(() => Boolean(data.value?.studio.edition_thumbnail))
const hasLogo = computed(() => Boolean(data.value?.studio.organization_logo))

type OptionKey = keyof CoverDesignOptions

const toggles = computed<OptionKey[]>(() => [
  'show_date',
  'show_speakers',
  'show_logos',
  ...(hasBanner.value ? (['show_event_banner'] as const) : []),
  ...(hasIcon.value ? (['show_event_icon'] as const) : []),
])

// ---------------------------------------------------------------------------
// La scène
// ---------------------------------------------------------------------------

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
    speakers: editable.draft.speakers.map((speaker) => ({
      name: speakerName(speaker),
      imageUrl: speaker.photo_image?.url ?? null,
    })),
    photoUrl: studio.source_photo?.url ?? null,
    organizationLogoUrl: studio.organization_logo?.url ?? null,
    bannerUrl: studio.edition_banner?.url ?? null,
    iconUrl: studio.edition_thumbnail?.url ?? null,
  }
})

const accentToken = computed(() => COVER_ACCENTS[accent.value])
const scene = useCoverScene(source, accentToken)

const preview = ref<{ toBlob: () => Promise<Blob | null> } | null>(null)

// ---------------------------------------------------------------------------
// La photo de départ, gardée à part de l'image fabriquée
// ---------------------------------------------------------------------------

const changingPhoto = ref(false)
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
    changingPhoto.value = false
    await refresh()
  } catch (thrown) {
    photoError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

const { data: photoRule } = useAsyncData(
  'cover-studio-photo-rule',
  async () => (await api.media.roles('programme', 'proposals')).find((rule) => rule.role === 'cover_source') ?? null,
  { lazy: true, server: false },
)

// ---------------------------------------------------------------------------
// Enregistrer : l'image montrée devient la couverture du dossier
// ---------------------------------------------------------------------------

const saving = ref(false)
const saveError = ref<string | null>(null)

async function save(): Promise<void> {
  const current = data.value
  if (!current || !preview.value) return
  saving.value = true
  saveError.value = null
  try {
    const blob = await preview.value.toBlob()
    if (!blob) {
      saveError.value = t('organization.workspace.cover.renderFailed')
      return
    }
    const asset = await api.media.upload({
      file: blob,
      filename: `${current.file.tracking.proposal.reference_code}-couverture.png`,
      mimeType: 'image/png',
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
    await navigateTo(`${fileTo.value}#couverture`)
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, (key) => t(key))
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
    <UiBreadcrumb
      :items="[
        { label: t('organization.workspace.proposal.breadcrumb'), to: localePath('/mon-organisation') },
        { label: data?.file.tracking.proposal.reference_code ?? '', to: fileTo },
        { label: t('organization.workspace.cover.title') },
      ]"
    />

    <UiLoadingState v-if="status === 'pending'" class="mt-10" variant="card" :lines="6" />

    <UiErrorState v-else-if="error" class="mt-10" :retry-label="t('common.actions.retry')" @retry="refresh()" />

    <UiEmptyState
      v-else-if="!data"
      class="mt-10"
      icon="document"
      :title="t('organization.workspace.proposal.notFound.title')"
      :description="t('organization.workspace.proposal.notFound.description')"
      :action-to="localePath('/mon-organisation')"
      :action-label="t('organization.workspace.proposal.notFound.action')"
    />

    <template v-else>
      <header class="mt-6 flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div class="min-w-0">
          <p class="text-sm text-text-subtle">{{ t('organization.workspace.cover.title') }}</p>
          <h1 class="mt-1 text-2xl leading-tight font-semibold text-balance">{{ tr(data.file.tracking.proposal.title) }}</h1>
        </div>
        <div class="flex shrink-0 flex-wrap gap-3">
          <UiButton variant="secondary" :to="fileTo">{{ t('organization.workspace.cover.back') }}</UiButton>
          <UiButton variant="primary" icon="check" :loading="saving" :disabled="!scene" @click="save">
            {{ t('organization.workspace.cover.save') }}
          </UiButton>
        </div>
      </header>

      <UiAlert v-if="saveError" class="mt-6" intent="danger" :message="saveError" live />

      <div class="mt-8 grid gap-8 lg:grid-cols-[22rem_minmax(0,1fr)] lg:items-start">
        <!-- LES CHOIX -->
        <div class="flex flex-col gap-8 rounded-lg border border-border bg-surface-raised p-5">
          <fieldset>
            <legend class="font-semibold text-heading">{{ t('organization.workspace.cover.steps.template') }}</legend>
            <div class="mt-3 grid grid-cols-2 gap-3">
              <button
                v-for="entry in templates"
                :key="entry.code"
                type="button"
                :aria-pressed="template === entry.code"
                class="flex cursor-pointer flex-col gap-1.5 rounded-lg border-2 p-1.5 text-left transition-colors duration-(--duration-fast)"
                :class="template === entry.code ? 'border-accent' : 'border-border-subtle hover:border-border-strong'"
                @click="template = entry.code"
              >
                <CoverStudioCanvas
                  :scene="scene"
                  :template="entry.code"
                  :options="options"
                  :label="t(`organization.workspace.cover.templates.${entry.code}`)"
                  class="rounded"
                />
                <span class="px-0.5 text-sm font-semibold text-text">{{ t(`organization.workspace.cover.templates.${entry.code}`) }}</span>
              </button>
            </div>
            <p class="mt-2 text-sm text-text-muted">{{ t('organization.workspace.cover.catalogHint') }}</p>
          </fieldset>

          <fieldset>
            <legend class="font-semibold text-heading">{{ t('organization.workspace.cover.steps.content') }}</legend>
            <p class="mt-1 text-sm text-text-muted">{{ t('organization.workspace.cover.contentHint') }}</p>
            <div class="mt-3 flex flex-col gap-2">
              <UiCheckbox
                v-for="key in toggles"
                :id="`cover-${key}`"
                :key="key"
                v-model="options[key]"
                :label="t(`organization.workspace.cover.options.${key}`, { edition: data.file.tracking.edition.acronym ?? '' })"
              />
            </div>
            <template v-if="usesPhoto">
              <p class="mt-4 text-sm text-text-secondary">
                {{ data.studio.source_photo ? t('organization.workspace.cover.photoSet') : t('organization.workspace.cover.photoMissing') }}
              </p>
              <UiButton v-if="!changingPhoto" class="mt-2" variant="secondary" icon="image" @click="changingPhoto = true">
                {{ data.studio.source_photo ? t('organization.workspace.cover.changePhoto') : t('organization.workspace.cover.addPhoto') }}
              </UiButton>
              <MediaImageField
                v-else
                class="mt-2"
                role="cover_source"
                :asset-id="null"
                :label="t('organization.workspace.cover.photoField')"
                :rule="photoRule ?? null"
                :owner="{ schema: 'programme', table: 'proposals', id: proposalId }"
                @update:asset-id="attachSourcePhoto"
              />
              <UiAlert v-if="photoError" class="mt-2" intent="danger" :message="photoError" live />
            </template>
          </fieldset>

          <fieldset>
            <legend class="font-semibold text-heading">{{ t('organization.workspace.cover.steps.color') }}</legend>
            <div class="mt-3 flex flex-wrap gap-3">
              <button
                v-for="[code, token] in accents"
                :key="code"
                type="button"
                :aria-pressed="accent === code"
                :aria-label="t(`organization.workspace.cover.accents.${code}`)"
                class="size-11 cursor-pointer rounded-full border-4 border-surface-raised ring-2 transition-shadow duration-(--duration-fast)"
                :class="accent === code ? 'ring-text' : 'ring-transparent hover:ring-border-strong'"
                :style="{ background: `var(${token})` }"
                @click="accent = code"
              />
            </div>
          </fieldset>
        </div>

        <!-- L'APERÇU -->
        <div class="flex min-w-0 flex-col gap-5">
          <div class="overflow-hidden rounded-lg border border-border shadow-sm">
            <ClientOnly>
              <CoverStudioCanvas
                ref="preview"
                :scene="scene"
                :template="template"
                :options="options"
                :label="t('organization.workspace.cover.previewLabel', { title: data.editable.draft.title })"
              />
              <template #fallback>
                <div class="aspect-video bg-surface-sunken" />
              </template>
            </ClientOnly>
          </div>

          <div
            v-if="!hasLogo && options.show_logos"
            role="alert"
            class="flex flex-col gap-2 rounded-lg border-2 border-danger bg-danger-surface p-4 sm:flex-row sm:items-center sm:justify-between"
          >
            <p class="text-sm text-text-secondary">
              <strong class="text-danger">{{ t('organization.workspace.proposal.gaps.logo.title') }}</strong>
              · {{ t('organization.workspace.cover.logoMissing') }}
            </p>
            <UiButton variant="danger" class="shrink-0" :to="`${fileTo}#logo`">
              {{ t('organization.workspace.proposal.gaps.logo.action') }}
            </UiButton>
          </div>

          <section aria-labelledby="share-preview-title" class="rounded-lg border border-border bg-surface-raised p-5">
            <h2 id="share-preview-title" class="font-semibold text-heading">{{ t('organization.workspace.cover.share.title') }}</h2>
            <p class="mt-1 text-sm text-text-muted">{{ t('organization.workspace.cover.share.description') }}</p>
            <div class="mt-4 max-w-sm overflow-hidden rounded-lg border border-border">
              <ClientOnly>
                <CoverStudioCanvas
                  :scene="scene"
                  :template="template"
                  :options="options"
                  :label="t('organization.workspace.cover.share.title')"
                />
              </ClientOnly>
              <div class="bg-surface-sunken px-3 py-2">
                <p class="text-xs text-text-subtle uppercase">{{ t('organization.workspace.cover.share.site') }}</p>
                <p class="mt-0.5 line-clamp-2 text-sm font-semibold text-text">{{ data.editable.draft.title }}</p>
              </div>
            </div>
          </section>
        </div>
      </div>
    </template>
  </div>
</template>
