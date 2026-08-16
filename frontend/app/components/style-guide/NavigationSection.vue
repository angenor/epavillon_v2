<script setup lang="ts">
import type { StepItem, TabItem } from '~/types/ui'
import type { BreadcrumbItem } from '~/types/navigation'

/**
 * Section « Navigation et surcouches » — onglets, frise d'étapes, fil d'Ariane,
 * menu d'actions, modale, tiroir.
 *
 * LES DEUX BARRES DE NAVIGATION (`UiNavBar`, `UiSideNav`) NE SONT PAS RENDUES
 * ICI : elles sont sous les yeux, en haut et à gauche de cette page même. Les
 * dupliquer dans un cadre de démonstration donnerait deux en-têtes empilés et
 * deux `<header>` dans le document, ce qui n'aide personne. Le guide dit où les
 * regarder.
 */

const { t } = useI18n()

const activeTab = ref('proposal')
const tabs = computed<TabItem[]>(() => [
  { value: 'proposal', label: t('style-guide.navigation.tabs.proposal') },
  { value: 'reviews', label: t('style-guide.navigation.tabs.reviews'), count: 4 },
  { value: 'exchanges', label: t('style-guide.navigation.tabs.exchanges'), count: 12 },
  { value: 'history', label: t('style-guide.navigation.tabs.history') },
  { value: 'sessions', label: t('style-guide.navigation.tabs.sessions'), disabled: true },
])

const currentStep = ref('speakers')
const steps = computed<StepItem[]>(() => [
  { value: 'organization', label: t('style-guide.navigation.steps.organization') },
  { value: 'activity', label: t('style-guide.navigation.steps.activity') },
  { value: 'themes', label: t('style-guide.navigation.steps.themes'), state: 'error' },
  { value: 'speakers', label: t('style-guide.navigation.steps.speakers') },
  { value: 'schedule', label: t('style-guide.navigation.steps.schedule') },
  { value: 'documents', label: t('style-guide.navigation.steps.documents'), state: 'skipped' },
  { value: 'review', label: t('style-guide.navigation.steps.review') },
])

const breadcrumb = computed<BreadcrumbItem[]>(() => [
  { labelKey: 'nav.admin.proposals', to: '/admin/propositions' },
  { label: 'COP31 — Belém 2027', to: '/admin/propositions' },
  // Libellé venu de la BASE, déjà résolu : un fil d'Ariane mêle les deux sortes.
  { label: "Financer l'adaptation côtière en Afrique de l'Ouest" },
])

const isModalOpen = ref(false)
const isDrawerOpen = ref(false)
const lastAction = ref<string | null>(null)

const menuItems = computed(() => [
  { value: 'edit', label: t('common.actions.edit'), icon: 'edit' },
  { value: 'duplicate', label: t('common.actions.duplicate'), icon: 'copy' },
  { value: 'export', label: t('common.actions.export'), icon: 'download', separatorBefore: true },
  { value: 'delete', label: t('common.actions.delete'), icon: 'trash', destructive: true, separatorBefore: true },
  { value: 'locked', label: t('style-guide.navigation.menu.locked'), icon: 'lock', disabled: true },
])
</script>

<template>
  <StyleGuideSection
    id="navigation"
    :title="t('style-guide.navigation.title')"
    :description="t('style-guide.navigation.description')"
  >
    <!-- ONGLETS -->
    <StyleGuideDemo
      :title="t('style-guide.navigation.tabs.title')"
      :note="t('style-guide.navigation.tabs.note')"
    >
      <UiTabs v-model="activeTab" :items="tabs" :label="t('data.tabs.label')" />
      <p class="mt-4 text-sm text-text-muted">
        {{ t('style-guide.navigation.tabs.active', { tab: tabs.find((tab) => tab.value === activeTab)?.label }) }}
      </p>
    </StyleGuideDemo>

    <!-- FRISE D'ÉTAPES -->
    <StyleGuideDemo
      :title="t('style-guide.navigation.steps.title')"
      :note="t('style-guide.navigation.steps.note')"
    >
      <UiStepper
        v-model="currentStep"
        :steps="steps"
        :label="t('style-guide.navigation.steps.label')"
      />
    </StyleGuideDemo>

    <!-- FIL D'ARIANE -->
    <StyleGuideDemo
      :title="t('style-guide.navigation.breadcrumb.title')"
      :note="t('style-guide.navigation.breadcrumb.note')"
    >
      <UiBreadcrumb :items="breadcrumb" :root="{ label: t('nav.admin.title'), to: '/admin' }" />
    </StyleGuideDemo>

    <!-- SURCOUCHES -->
    <StyleGuideDemo
      :title="t('style-guide.navigation.overlays.title')"
      :note="t('style-guide.navigation.overlays.note')"
    >
      <div class="flex flex-wrap items-center gap-3">
        <UiButton variant="secondary" icon="ban" @click="isModalOpen = true">
          {{ t('style-guide.navigation.overlays.openModal') }}
        </UiButton>
        <UiButton variant="secondary" icon="document" @click="isDrawerOpen = true">
          {{ t('style-guide.navigation.overlays.openDrawer') }}
        </UiButton>
        <UiContextMenu
          :items="menuItems"
          :label="t('style-guide.navigation.menu.label')"
          @select="(value) => (lastAction = value)"
        />
        <span v-if="lastAction" class="text-sm text-text-muted">
          {{ t('style-guide.navigation.menu.selected', { action: lastAction }) }}
        </span>
      </div>

      <UiModal
        v-model:open="isModalOpen"
        :title="t('style-guide.navigation.overlays.modalTitle')"
        :description="t('style-guide.navigation.overlays.modalDescription')"
      >
        <p class="text-sm text-text-muted">{{ t('style-guide.navigation.overlays.modalBody') }}</p>
        <UiAlert intent="warning" compact class="mt-4">
          {{ t('style-guide.navigation.overlays.modalWarning') }}
        </UiAlert>

        <template #footer>
          <UiButton variant="ghost" @click="isModalOpen = false">{{ t('common.actions.cancel') }}</UiButton>
          <UiButton variant="danger" icon="ban" @click="isModalOpen = false">
            {{ t('style-guide.navigation.overlays.modalConfirm') }}
          </UiButton>
        </template>
      </UiModal>

      <UiDrawer
        v-model:open="isDrawerOpen"
        :title="t('style-guide.navigation.overlays.drawerTitle')"
        :description="t('style-guide.navigation.overlays.drawerDescription')"
      >
        <dl class="space-y-4 text-sm">
          <div>
            <dt class="font-medium text-text">{{ t('style-guide.navigation.overlays.drawerField1') }}</dt>
            <dd class="mt-0.5 text-text-muted">Réseau ouest-africain pour l'action climatique</dd>
          </div>
          <div>
            <dt class="font-medium text-text">{{ t('style-guide.navigation.overlays.drawerField2') }}</dt>
            <dd class="mt-0.5 text-text-muted">COP31-00147</dd>
          </div>
          <div>
            <dt class="font-medium text-text">{{ t('style-guide.navigation.overlays.drawerField3') }}</dt>
            <dd class="mt-0.5">
              <UiBadge intent="info" size="sm">{{ t('style-guide.business.status.under_review') }}</UiBadge>
            </dd>
          </div>
        </dl>

        <template #footer>
          <UiButton variant="ghost" @click="isDrawerOpen = false">{{ t('common.actions.close') }}</UiButton>
          <UiButton icon="arrow-right">{{ t('style-guide.navigation.overlays.drawerAction') }}</UiButton>
        </template>
      </UiDrawer>
    </StyleGuideDemo>

    <!-- BARRES DE NAVIGATION -->
    <StyleGuideDemo
      :title="t('style-guide.navigation.bars.title')"
      :note="t('style-guide.navigation.bars.note')"
    >
      <ul class="space-y-2 text-sm text-text-muted">
        <li class="flex gap-2">
          <UiIcon name="arrow-up-right" size="1rem" class="mt-0.5 shrink-0 text-text-subtle" />
          {{ t('style-guide.navigation.bars.navbar') }}
        </li>
        <li class="flex gap-2">
          <UiIcon name="arrow-left" size="1rem" class="mt-0.5 shrink-0 text-text-subtle" />
          {{ t('style-guide.navigation.bars.sidenav') }}
        </li>
      </ul>
    </StyleGuideDemo>
  </StyleGuideSection>
</template>
