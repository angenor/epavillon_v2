<script setup lang="ts">
/**
 * NOUVELLE VERSION EN LIGNE (maquette `docs/maquettes/nouvelle-version/`).
 *
 * Deux cas, choisis à la mise en ligne. Ordinaire : une carte en haut de l'écran, sous la barre (02/11)
 * propose de recharger ; « Plus tard » la range, et la page suivante se charge
 * dans la nouvelle version (plugin `app-version`). Imposé : un voile et un
 * décompte, sans moyen de refuser, pour une correction urgente.
 *
 * Un seul pour toute l'application, posé par les gabarits comme le bandeau
 * d'API injoignable. Cyan de l'information, jaune de l'attention : ni l'un ni
 * l'autre ne reprend le bleu nuit des barres.
 */
const { stale, forced, dismissed, reload } = useAppVersion()
const { t } = useI18n()

const DELAY_S = 10
const left = ref(DELAY_S)
let timer: number | undefined

watch(
  forced,
  (isForced) => {
    if (!isForced || import.meta.server) return
    left.value = DELAY_S
    timer = window.setInterval(() => {
      left.value -= 1
      if (left.value <= 0) {
        window.clearInterval(timer)
        reload()
      }
    }, 1000)
  },
  { immediate: true },
)

onBeforeUnmount(() => window.clearInterval(timer))

const progress = computed(() => `${((DELAY_S - left.value) / DELAY_S) * 100}%`)
</script>

<template>
  <ClientOnly>
    <div
      v-if="forced"
      class="fixed inset-0 z-[100] grid place-items-center bg-[color-mix(in_srgb,var(--ifdd-noir)_55%,transparent)] p-4 backdrop-blur-[3px]"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="nouvelle-version-titre"
      aria-describedby="nouvelle-version-texte"
    >
      <div class="w-full max-w-[460px] overflow-hidden rounded-lg bg-surface-raised pb-5.5 text-center text-text shadow-lg">
        <p class="mb-4.5 flex items-center justify-center gap-2.5 bg-(--ifdd-jaune) px-4 py-3 font-bold text-(--ifdd-noir)">
          <UiIcon name="warning" size="1.25rem" :stroke-width="2.2" aria-hidden="true" />
          {{ t('new-version-notice.forced.kicker') }}
        </p>
        <h2 id="nouvelle-version-titre" class="mx-5.5 mb-2 text-xl font-bold text-heading">
          {{ t('new-version-notice.forced.title') }}
        </h2>
        <p id="nouvelle-version-texte" class="mx-5.5 mb-3.5 text-sm leading-relaxed text-text-secondary">
          {{ t('new-version-notice.forced.description') }}
        </p>
        <p class="text-[2.6rem] leading-none font-bold text-heading tabular-nums" aria-live="polite">
          {{ t('new-version-notice.forced.seconds', { count: left }) }}
        </p>
        <div class="mx-5.5 mt-3 mb-4.5 h-1.5 overflow-hidden rounded-full bg-surface-sunken" aria-hidden="true">
          <i class="block h-full bg-(--ifdd-jaune) transition-[width] duration-1000 ease-linear" :style="{ width: progress }" />
        </div>
        <div class="mx-5.5">
          <UiButton block @click="reload">{{ t('new-version-notice.forced.now') }}</UiButton>
        </div>
      </div>
    </div>

    <div
      v-else-if="stale && !dismissed"
      class="fixed inset-x-3 top-[calc(var(--nav-height)+0.75rem)] z-[90] mx-auto flex max-w-[720px] flex-wrap items-center gap-3.5 rounded-lg border border-l-[5px] border-border border-l-(--ifdd-cyan) bg-surface-raised py-3 pr-3 pl-4 text-text shadow-lg sm:flex-nowrap"
      role="status"
    >
      <UiIcon name="refresh" size="1.375rem" :stroke-width="2" class="shrink-0 text-accent" aria-hidden="true" />
      <p class="min-w-0 flex-1 text-sm leading-snug text-text-secondary">
        <b class="block text-base text-heading">{{ t('new-version-notice.title') }}</b>
        {{ t('new-version-notice.description') }}
      </p>
      <div class="flex w-full shrink-0 gap-2 sm:w-auto">
        <UiButton variant="secondary" class="flex-1 sm:flex-none" @click="dismissed = true">
          {{ t('new-version-notice.later') }}
        </UiButton>
        <UiButton class="flex-1 sm:flex-none" @click="reload">{{ t('new-version-notice.reload') }}</UiButton>
      </div>
    </div>
  </ClientOnly>
</template>
