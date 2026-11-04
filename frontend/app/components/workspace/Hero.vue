<script setup lang="ts">
import type { AttachedImage } from '~/types/media'
import type { Organization } from '~/types/org'
import type { SelectOption, TabItem } from '~/types/ui'
import type { WorkspaceEditionTab } from '~/utils/workspace-editions'

/**
 * Le bandeau de l'espace organisation (04/11, direction condensée) : photo de
 * l'édition ouverte voilée, logo de l'organisation en tuile blanche, et les
 * onglets d'édition posés sur la photo. Changer d'édition change la photo.
 * Sans organisation (chargement, erreur, refus), le titre de la page seul.
 */

interface Props {
  organization: Organization | null
  logo: string | null
  image: AttachedImage | null
  loading?: boolean
  tabs: WorkspaceEditionTab[]
  activeSlug: string | null
  organizationOptions: SelectOption[]
  organizationId: string | null
  /** Le volet que commandent les onglets, rendu sous le bandeau. */
  panelId: string
}

const props = withDefaults(defineProps<Props>(), { loading: false })
const emit = defineEmits<{ selectOrganization: [id: string]; selectTab: [slug: string] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const seen = useWorkspaceSeen()

const tabItems = computed<TabItem[]>(() =>
  props.tabs.map((tab) => ({ value: tab.edition.slug, label: editionName(tab) })),
)

const tabOf = (slug: string) => props.tabs.find((tab) => tab.edition.slug === slug)

function editionName(tab: WorkspaceEditionTab): string {
  return tab.edition.acronym ?? tr(tab.edition.title)
}

function editionState(tab: WorkspaceEditionTab): 'over' | 'ongoing' | 'upcoming' {
  if (tab.over) return 'over'
  return Date.parse(tab.edition.starts_at) <= Date.now() ? 'ongoing' : 'upcoming'
}

function editionMeta(tab: WorkspaceEditionTab): string {
  const parts = [
    t(`organization.workspace.editions.state.${editionState(tab)}`),
    t('organization.workspace.proposals.count', tab.proposals.length).toLowerCase(),
  ]
  if (tab.over && tab.missingReports > 0) {
    parts.push(t('organization.workspace.editions.missingReports', tab.missingReports).toLowerCase())
  } else if (!tab.over && tab.actions.length > 0) {
    parts.push(t('organization.workspace.editions.todo', tab.actions.length))
  }
  const unread = tab.proposals.reduce((sum, tracking) => sum + tracking.unread_comments, 0)
  if (unread > 0) parts.push(t('organization.workspace.editions.unread', unread))
  return parts.join(', ')
}
</script>

<template>
  <!-- Bas réduit sur une colonne : rien n'y remonte (les cartes ne débordent qu'à partir de `lg`). -->
  <UiMediaBanner class="max-lg:pb-9!" :image="props.image">
    <div class="flex flex-wrap items-end justify-between gap-x-8 gap-y-5">
      <div class="flex min-w-0 items-center gap-3.5 sm:gap-5">
        <span
          v-if="props.organization || props.loading"
          class="flex h-16 min-w-16 shrink-0 items-center justify-center rounded-lg p-1.5 shadow-glass sm:h-22 sm:min-w-22"
          :class="props.organization ? 'bg-surface-inverse-selected' : 'animate-pulse bg-surface-inverse-raised motion-reduce:animate-none'"
          aria-hidden="true"
        >
          <img v-if="props.logo" :src="props.logo" alt="" class="max-h-12 max-w-28 object-contain sm:max-h-17.5 sm:max-w-42">
          <span v-else-if="props.organization" class="px-2 font-condensed text-lg font-bold tracking-[0.02em] text-text-on-inverse-selected">
            {{ props.organization.acronym ?? initialsOf(props.organization.legal_name) }}
          </span>
        </span>
        <div class="min-w-0">
          <p class="font-condensed text-[0.95rem] font-semibold tracking-[0.05em] text-text-on-inverse-muted uppercase">
            {{ props.organization ? t('organization.workspace.title') : t('organization.workspace.kicker') }}
          </p>
          <h1
            class="mt-1 max-w-[26ch] font-condensed text-[clamp(1.85rem,4vw,3.1rem)] leading-[1.02] font-semibold tracking-[0.005em] text-balance text-text-on-inverse"
          >
            {{ props.organization?.legal_name ?? t('organization.workspace.title') }}
          </h1>
        </div>
      </div>

      <!-- Le sélecteur n'apparaît qu'à qui a plusieurs organisations : une
           liste déroulante à une entrée n'est pas un choix, c'est un obstacle. -->
      <div v-if="props.organization && props.organizationOptions.length > 1" class="w-75 max-w-full">
        <p class="mb-1 text-[0.8rem] text-text-on-inverse-muted" aria-hidden="true">
          {{ t('organization.workspace.organizationPicker.label') }}
        </p>
        <UiSelect
          id="workspace-organization"
          :model-value="props.organizationId"
          :options="props.organizationOptions"
          :label="t('organization.workspace.organizationPicker.label')"
          hide-label
          hide-optional
          @update:model-value="(id) => id && emit('selectOrganization', id)"
        />
      </div>
    </div>

    <UiTabs
      v-if="props.organization && props.tabs.length > 0 && props.activeSlug"
      class="mt-7"
      tone="banner"
      :items="tabItems"
      :model-value="props.activeSlug"
      :label="t('organization.workspace.editions.label')"
      :panel-id="() => props.panelId"
      @update:model-value="(slug) => emit('selectTab', slug)"
    >
      <template #tab="{ item }">
        <template v-for="tab in [tabOf(item.value)]" :key="item.value">
          <template v-if="tab">
            <!-- Décoratives : leur texte alternatif précéderait le nom de l'édition dans l'onglet. -->
            <span
              v-if="tab.logo"
              aria-hidden="true"
              class="grid size-12 shrink-0 place-items-center overflow-hidden rounded-md bg-surface-inverse-selected p-1"
            >
              <img :src="tab.logo.sources.thumb?.url ?? tab.logo.url" alt="" class="max-h-full max-w-full object-contain">
            </span>
            <UiImage
              v-else-if="tab.thumbnail"
              :image="tab.thumbnail"
              ratio="1 / 1"
              sizes="48px"
              rounded="rounded-md"
              class="w-12 shrink-0"
              aria-hidden="true"
            />
            <span
              v-else
              aria-hidden="true"
              class="grid size-12 shrink-0 place-items-center rounded-md bg-surface-inverse-raised font-condensed text-sm font-bold text-text-on-inverse"
            >
              {{ initialsOf(editionName(tab)) }}
            </span>
            <span class="min-w-0">
              <span class="flex flex-wrap items-center gap-x-2.5 gap-y-1 font-condensed text-[1.6rem] leading-none font-bold tracking-[0.02em] uppercase">
                {{ editionName(tab) }}
                <small v-if="tab.edition.city" class="font-sans text-[0.85rem] font-semibold tracking-normal normal-case opacity-80">
                  {{ tab.edition.city }}
                </small>
                <UiCounter v-if="seen.tabUnseen(tab) > 0" :value="seen.tabUnseen(tab)" tone="accent" />
              </span>
              <span class="mt-1 block text-[0.8rem] opacity-85">{{ editionMeta(tab) }}</span>
            </span>
          </template>
        </template>
      </template>
    </UiTabs>
  </UiMediaBanner>
</template>
