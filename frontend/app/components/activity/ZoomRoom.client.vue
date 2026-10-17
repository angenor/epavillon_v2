<script setup lang="ts">
import type { MeetingAccess, PublicSessionMeeting } from '~/types/live'
import type { PublicScheduleRow } from '~/types/views'
import type { MeetingRoomStep } from '~/composables/useMeetingRoom'
import type { AttachedImage } from '~/types/media'

/**
 * La salle Zoom d'une activité, sur l'aplat sombre du direct.
 *
 * `stage` : dans la page de l'activité, l'invitation seule ; « Rejoindre » mène
 * à la salle pleine page, qui entre aussitôt (arbitré le 17/10 : une réunion
 * ouverte dans la page puis agrandie ressortait et rentrait de Zoom).
 * `page` : la salle pleine page (vue client du SDK, celle du logiciel), qui prend
 * tout l'écran et renvoie à `leaveUrl` en sortant.
 */

interface Props {
  session: PublicScheduleRow
  meeting: PublicSessionMeeting
  mode: 'stage' | 'page'
  /** Salle pleine page, entrée directe comprise, pour `stage`. */
  roomTo?: string
  /** Adresse absolue de l'activité, où la vue client renvoie en sortant. */
  leaveUrl?: string
  zoneLabel?: string
  /** Arrivée depuis « Rejoindre » : on entre sans attendre de second clic. */
  autoEnter?: boolean
  /** Couverture de l'activité, gardée sous l'invitation. */
  cover?: AttachedImage | null
}

const props = defineProps<Props>()
/** Vrai de l'entrée à la sortie : le parent ne démonte pas la salle pendant ce temps. */
const emit = defineEmits<{ 'update:inMeeting': [value: boolean] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const route = useRoute()
const localePath = useLocalePath()
const { phase, step, error, errorStatus, needsRegistration, waitlisted, joinUrl, prefetch, join, onRegistered } = useMeetingRoom(
  () => props.session,
)

const dialogOpen = ref(false)
/** Messages du SDK qui disent que l'image ou le son n'ont pas pu se connecter, dans sa langue. */
const failureTexts = ref<string[]>([])

const loginTo = computed(() => `${localePath('auth-login')}?redirect=${encodeURIComponent(route.fullPath)}`)
const stepLabel = computed(() => t(`zoom-room.step.${step.value === 'joined' || !step.value ? 'joining' : step.value}`))

// Le SDK s'annonce lourd : on le charge pendant que la personne lit, pour que le clic garde son geste (le son en dépend).
watch(
  phase,
  (current) => {
    if (current !== 'ready' || props.mode !== 'page') return
    prefetch()
    void loadZoomClientView().catch(() => undefined)
  },
  { immediate: true },
)
watch(
  () => phase.value === 'loading' || phase.value === 'joined',
  (active) => emit('update:inMeeting', active),
)

async function startClientView(access: MeetingAccess, setStep: (next: MeetingRoomStep) => void): Promise<void> {
  // Le SDK pose son fond noir plein écran dès son chargement : il ne doit jamais cacher un message d'échec.
  const hideRoot = () => {
    const root = document.getElementById('zmmtg-root')
    if (root) root.style.display = 'none'
  }
  try {
    const zoomMtg = await loadZoomClientView()
    zoomMtg.preLoadWasm()
    zoomMtg.prepareWebSDK()
    const language = zoomLanguage(locale.value)
    await zoomMtg.i18n.load(language)
    failureTexts.value = zoomMediaFailureTexts(zoomMtg, language)
    const root = document.getElementById('zmmtg-root')
    if (root) root.style.display = 'block'
    await new Promise((resolve, reject) =>
      zoomMtg.init({
        leaveUrl: props.leaveUrl ?? window.location.href,
        patchJsMedia: true,
        leaveOnPageUnload: true,
        disablePreview: Boolean(props.autoEnter),
        success: resolve,
        error: reject,
      }),
    )
    setStep('joining')
    await new Promise((resolve, reject) =>
      zoomMtg.join({ ...zoomJoinFields(access), passWord: access.password, success: resolve, error: reject }),
    )
  } catch (failure) {
    hideRoot()
    throw failure
  }
}

function enter(): void {
  void join(startClientView)
}

let autoEntered = false
watch(
  phase,
  (current) => {
    if (current !== 'ready' || props.mode !== 'page' || !props.autoEnter || autoEntered) return
    autoEntered = true
    enter()
  },
  { immediate: true },
)

onBeforeUnmount(() => emit('update:inMeeting', false))
</script>

<template>
  <div class="relative size-full bg-surface-inverse text-text-on-inverse">
    <div
      class="relative flex size-full flex-col bg-surface-inverse px-5 sm:px-8"
      :class="[
        props.cover ? 'justify-end pt-24 pb-6 text-left sm:pb-8' : 'justify-center py-8 text-center',
        props.mode === 'stage' ? 'min-h-80 lg:min-h-0' : '',
      ]"
      :aria-busy="phase === 'checking' || phase === 'loading' ? 'true' : undefined"
    >
      <!-- La salle est joignable des semaines à l'avance : l'invitation se pose sur l'image de l'activité au lieu de l'effacer. -->
      <template v-if="props.cover">
        <UiImage
          :image="props.cover"
          ratio="auto"
          loading="eager"
          class="absolute inset-0"
          frame-class="size-full"
          sizes="(min-width: 1024px) 50rem, 100vw"
        />
        <span class="scrim-fade-bottom pointer-events-none absolute inset-0" aria-hidden="true" />
      </template>
      <div class="relative flex flex-col gap-4" :class="props.cover ? 'items-start' : 'items-center'">
        <span
          v-if="!props.cover"
          class="inline-flex size-14 items-center justify-center rounded-full bg-live text-live-contrast sm:size-16"
          aria-hidden="true"
        >
          <UiIcon name="video" size="1.75rem" />
        </span>
        <p
          class="flex items-center gap-2 text-xs font-bold uppercase"
          :class="props.cover ? 'text-text-on-inverse' : 'text-text-on-inverse-muted'"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          <span v-if="props.cover" class="inline-flex size-7 items-center justify-center rounded-full bg-live text-live-contrast" aria-hidden="true">
            <UiIcon name="video" size="0.875rem" />
          </span>
          {{ t('zoom-room.label') }}
        </p>

        <p v-if="phase === 'checking'" class="flex items-center gap-2 text-sm" role="status">
          <UiSpinner size="1rem" />{{ t('zoom-room.checking') }}
        </p>

        <template v-else-if="phase === 'login'">
          <p class="max-w-md text-sm">{{ t('zoom-room.login.message') }}</p>
          <UiButton :to="loginTo" size="lg">{{ t('zoom-room.login.action') }}</UiButton>
        </template>

        <template v-else-if="phase === 'register'">
          <p class="max-w-md text-sm">{{ t('zoom-room.register.message') }}</p>
          <UiButton size="lg" @click="dialogOpen = true">{{ t('zoom-room.register.action') }}</UiButton>
        </template>

        <template v-else-if="phase === 'ready'">
          <p class="max-w-md text-sm">{{ t('zoom-room.ready') }}</p>
          <!-- Lien simple, pas le routeur : passer d'une mise en page à l'autre remonte la salle, et Zoom ne démarre qu'une fois par document. -->
          <a
            v-if="props.mode === 'stage' && props.roomTo"
            :href="props.roomTo"
            class="inline-flex min-h-12 items-center gap-2.5 rounded-md bg-live px-6 text-base font-bold text-live-contrast"
          >
            <UiIcon name="video" size="1.125rem" />
            {{ t('zoom-room.join') }}
          </a>
          <button
            v-else
            type="button"
            class="inline-flex min-h-12 cursor-pointer items-center gap-2.5 rounded-md bg-live px-6 text-base font-bold text-live-contrast"
            @click="enter"
          >
            <UiIcon name="video" size="1.125rem" />
            {{ t('zoom-room.join') }}
          </button>
        </template>

        <p v-else-if="phase === 'loading'" class="flex items-center gap-2 text-sm" role="status">
          <UiSpinner size="1rem" />{{ stepLabel }}
        </p>

        <p v-else-if="phase === 'joined'" class="text-sm" role="status">{{ t('zoom-room.joined') }}</p>

        <template v-else-if="phase === 'error'">
          <p class="max-w-md text-sm" role="alert">{{ error }}</p>
          <div class="flex flex-wrap justify-center gap-2">
            <UiButton v-if="errorStatus === 401" :to="loginTo">{{ t('zoom-room.login.action') }}</UiButton>
            <UiButton v-else-if="errorStatus === 403 && needsRegistration" @click="dialogOpen = true">{{ t('zoom-room.register.action') }}</UiButton>
            <UiButton variant="inverse" icon="refresh" @click="enter">{{ t('zoom-room.retry') }}</UiButton>
          </div>
        </template>

        <p v-if="waitlisted && (phase === 'ready' || phase === 'error')" class="max-w-md text-[0.8125rem] text-text-on-inverse-muted">
          {{ t('zoom-room.waitlisted') }}
        </p>

        <UiButton v-if="joinUrl && phase !== 'checking'" variant="inverse" icon="external-link" :href="joinUrl">
          {{ t('zoom-room.fallback') }}
        </UiButton>
      </div>
    </div>

    <ActivityZoomRoomAlerts
      v-if="props.mode === 'page' && phase === 'joined'"
      :failure-texts="failureTexts"
      :join-url="joinUrl"
      :session-id="props.session.id"
    />

    <ActivityRegistrationDialog
      v-model:open="dialogOpen"
      :session-id="props.session.id"
      :session-title="tr(props.session.title)"
      :timezone="props.session.timezone"
      :timezone-label="props.zoneLabel"
      @done="onRegistered"
    />
  </div>
</template>
