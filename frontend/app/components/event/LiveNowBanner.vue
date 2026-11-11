<script setup lang="ts">
/**
 * LE DIRECT EN COURS, SOUS LA BARRE DE TOUTES LES PAGES PUBLIQUES (10/11) :
 * maquette `docs/maquettes/direct-en-cours/`. Pas d'heure, qui se lirait comme
 * une annonce : « en cours depuis 12 min ». La couverture (à défaut la photo
 * de l'édition) sous un voile, un bouton de lecture rouge ; sur téléphone, ni
 * l'une ni l'autre, la place manque. Refermé, il laisse `EventLiveNowPill`.
 *
 * La durée part de l'heure prévue : la vue publique ne porte pas l'instant réel
 * du démarrage, et l'écart se compte en minutes.
 */

const { t } = useI18n()
const { tr } = useI18nText()
const { live, to, bannerShown, dismiss } = useLiveNow()
const k = (key: string, params: Record<string, unknown> = {}) => t(`live-now.${key}`, params)

const now = useLiveClock()

const session = computed(() => live.value?.session ?? null)
const image = computed(() => session.value?.cover ?? editionPhoto(live.value?.edition))
const organization = computed(() => session.value?.organization_name ?? session.value?.organization_acronym ?? '')
const shortOrganization = computed(() => session.value?.organization_acronym ?? organization.value)

const since = computed(() => {
  const start = session.value ? Date.parse(session.value.starts_at) : now.value
  const minutes = Math.floor((now.value - start) / 60_000)
  if (minutes < 1) return k('justStarted')
  if (minutes < 60) return k('since', { minutes })
  const rest = minutes % 60
  return rest
    ? k('sinceHours', { hours: Math.floor(minutes / 60), minutes: String(rest).padStart(2, '0') })
    : k('sinceHoursExact', { hours: minutes / 60 })
})
</script>

<template>
  <Transition name="live-now">
    <div v-if="bannerShown && session" role="status" :aria-label="k('label')" class="border-b border-border-subtle bg-surface-raised shadow-md">
      <div class="relative mx-auto flex w-full max-w-[1280px] flex-wrap items-center gap-x-4 gap-y-3 px-4 py-3 sm:flex-nowrap sm:px-6 sm:py-2.5">
        <NuxtLink
          :to="to"
          tabindex="-1"
          aria-hidden="true"
          class="group relative hidden aspect-video w-32 shrink-0 overflow-hidden rounded-md bg-surface-inverse sm:block"
        >
          <UiImage v-if="image" :image="image" ratio="auto" frame-class="size-full" class="absolute inset-0" sizes="128px" />
          <span class="absolute inset-0 bg-scrim/40" />
          <span
            class="absolute top-1/2 left-1/2 grid size-9 -translate-1/2 place-items-center rounded-full bg-live text-live-contrast ring-3 ring-text-on-inverse/85 transition-transform group-hover:scale-110"
          >
            <svg viewBox="0 0 12 14" class="ms-0.5 h-3.5 w-3" fill="currentColor"><path d="M0 0l12 7-12 7z" /></svg>
          </span>
        </NuxtLink>

        <div class="min-w-0 flex-1 pe-10 sm:pe-0">
          <p class="flex flex-wrap items-center gap-x-2 font-condensed text-[0.95rem] leading-none font-bold tracking-[0.08em] text-live uppercase">
            <span class="live-pulse size-2 shrink-0 rounded-full bg-live" aria-hidden="true" />
            {{ k('tag') }}
            <span class="font-semibold tracking-[0.04em] text-text-secondary">{{ since }}</span>
          </p>
          <p class="mt-1.5 truncate font-condensed text-xl leading-tight font-semibold text-heading max-sm:line-clamp-2 max-sm:text-lg max-sm:whitespace-normal">
            {{ tr(session.title) }}
          </p>
          <p v-if="organization" class="mt-1.5 flex min-w-0 items-center gap-2 text-[0.82rem] font-bold text-text-secondary">
            <OrganizationLogoMark :name="organization" :acronym="session.organization_acronym" :logo="session.organization_logo ?? null" size="sm" />
            <span class="min-w-0 truncate max-sm:hidden">{{ organization }}</span>
            <span class="min-w-0 truncate sm:hidden">{{ shortOrganization }}</span>
          </p>
        </div>

        <UiButton :to="to" :label="k('join')" class="shrink-0 max-sm:w-full" />

        <button
          type="button"
          class="grid size-(--target-min) shrink-0 cursor-pointer place-items-center rounded-md text-text-muted transition-colors hover:bg-surface-hover hover:text-text max-sm:absolute max-sm:top-1 max-sm:end-1"
          :aria-label="k('dismiss')"
          @click="dismiss()"
        >
          <UiIcon name="close" size="1.125rem" />
        </button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.live-now-enter-active {
  transition:
    opacity 420ms cubic-bezier(0.2, 0.8, 0.3, 1),
    transform 420ms cubic-bezier(0.2, 0.8, 0.3, 1);
}

.live-now-enter-from {
  opacity: 0;
  transform: translateY(-14px);
}

@media (prefers-reduced-motion: reduce) {
  .live-now-enter-active {
    transition: none;
  }
}
</style>
