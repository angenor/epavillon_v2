<script setup lang="ts">
import type { PublicSessionSpeaker } from '~/types/programme/session'

/**
 * Les intervenants de l'activité, assis derrière leur chevalet (maquette `intervenants-directions`, 02/11) : ce que la base publie, sans rien de plus.
 * Le carton porte nom, organisation et rôle, la modération en carton plein à la couleur du jour ; la fonction, souvent longue, est dans la fiche
 * qu'ouvre le clic (jamais le courriel, qui n'est pas public).
 */

interface Props {
  speakers: PublicSessionSpeaker[]
  /** Dans la carte d'un dossier de séance commune : une liste serrée, sans titre de section. */
  compact?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

const ROLE_ORDER = ['moderator', 'keynote', 'facilitator', 'speaker', 'panelist', 'interpreter']

const speakers = computed(() =>
  [...props.speakers]
    .sort((a, b) => ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role) || a.sort_order - b.sort_order)
    .map((speaker) => ({
      id: speaker.id,
      name: speaker.display_name,
      position: [speaker.job_title_snapshot, speaker.organization_snapshot].filter(Boolean).join(', '),
      jobTitle: speaker.job_title_snapshot ?? '',
      organization: speaker.organization_snapshot ?? '',
      role: t(`activity.people.role.${speaker.role}`),
      isLead: speaker.role === 'moderator',
      bio: speaker.bio ? tr(speaker.bio) : '',
      avatar: speaker.avatar,
      initials: initialsOf(speaker.display_name),
    })),
)

type SpeakerCard = (typeof speakers.value)[number]
const opened = ref<SpeakerCard | null>(null)
const isOpen = computed({
  get: () => opened.value !== null,
  set: (value: boolean) => {
    if (!value) opened.value = null
  },
})

/**
 * UNE SEULE RANGÉE (10/11) : un panel nombreux passait sur trois ou quatre
 * rangées. Quand elle déborde, elle défile seule d'un siège toutes les
 * 3,5 s, revient au début en fin de rangée, et s'arrête tant qu'on la survole,
 * la parcourt au clavier, la touche (8 s) ou lit une fiche. Rien ne bouge avec
 * « moins d'animations » : les flèches restent.
 */
const AUTOPLAY_MS = 3500
const TOUCH_PAUSE_MS = 8000
const track = ref<HTMLElement | null>(null)
const overflowing = ref(false)
const hovered = ref(false)
const focused = ref(false)
let pausedUntil = 0
let reducedMotion = false
let timer: ReturnType<typeof setInterval> | undefined
let observer: ResizeObserver | undefined

function measure(): void {
  const el = track.value
  overflowing.value = !!el && el.scrollWidth > el.clientWidth + 2
}

function step(direction: 1 | -1): void {
  const el = track.value
  if (!el) return
  const behavior: ScrollBehavior = reducedMotion ? 'auto' : 'smooth'
  const seat = el.querySelector('li')?.getBoundingClientRect().width ?? 176
  if (direction === 1 && el.scrollLeft + el.clientWidth >= el.scrollWidth - 4) el.scrollTo({ left: 0, behavior })
  else if (direction === -1 && el.scrollLeft <= 4) el.scrollTo({ left: el.scrollWidth, behavior })
  else el.scrollBy({ left: direction * seat, behavior })
}

function pauseAfterTouch(): void {
  pausedUntil = Date.now() + TOUCH_PAUSE_MS
}

function move(direction: 1 | -1): void {
  pauseAfterTouch()
  step(direction)
}

onMounted(() => {
  reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  observer = new ResizeObserver(measure)
  if (track.value) observer.observe(track.value)
  measure()
  if (reducedMotion) return
  timer = setInterval(() => {
    const idle = !hovered.value && !focused.value && !isOpen.value && Date.now() > pausedUntil
    if (overflowing.value && idle && document.visibilityState === 'visible') step(1)
  }, AUTOPLAY_MS)
})
onBeforeUnmount(() => {
  clearInterval(timer)
  observer?.disconnect()
})
</script>

<template>
  <ul v-if="props.compact && speakers.length" class="grid gap-2 border-t border-border-subtle pt-3" :aria-label="t('activity.people.speakers')">
    <li v-for="speaker in speakers" :key="speaker.id" class="flex min-w-0 items-center gap-2.5">
      <UiImage
        v-if="speaker.avatar"
        :image="speaker.avatar"
        ratio="1 / 1"
        rounded="rounded-full"
        class="size-9 shrink-0 rounded-full border-2 border-(--color-day-line)"
        sizes="36px"
      />
      <span
        v-else
        class="flex size-9 shrink-0 items-center justify-center rounded-full border-2 border-(--color-day-line) bg-(--color-day-surface) font-condensed text-[0.85rem] font-bold text-(--color-day)"
        aria-hidden="true"
      >
        {{ speaker.initials }}
      </span>
      <div class="min-w-0 flex-1">
        <p class="font-condensed text-[1.05rem] leading-[1.1] font-bold text-heading">{{ speaker.name }}</p>
        <p v-if="speaker.position" class="text-[0.8rem] leading-snug text-text-secondary">{{ speaker.position }}</p>
      </div>
      <p class="shrink-0 text-[0.68rem] font-bold tracking-[0.06em] text-(--color-day) uppercase">{{ speaker.role }}</p>
    </li>
  </ul>

  <section v-else-if="!props.compact && speakers.length" aria-labelledby="intervenants-titre">
    <div class="flex items-end justify-between gap-4">
      <h2 id="intervenants-titre" class="heading-condensed text-[clamp(1.8rem,3.4vw,2.3rem)] text-heading">
        {{ t('activity.people.speakers') }}
      </h2>
      <div v-if="overflowing" class="flex shrink-0 gap-2">
        <UiButton variant="secondary" icon="arrow-left" icon-only :label="t('activity.people.previous')" @click="move(-1)" />
        <UiButton variant="secondary" icon="arrow-right" icon-only :label="t('activity.people.next')" @click="move(1)" />
      </div>
    </div>
    <ul
      ref="track"
      class="people-track mt-5 flex snap-x snap-mandatory overflow-x-auto overscroll-x-contain pb-2"
      @mouseenter="hovered = true"
      @mouseleave="hovered = false"
      @focusin="focused = true"
      @focusout="focused = false"
      @pointerdown="pauseAfterTouch()"
      @wheel.passive="pauseAfterTouch()"
    >
      <li v-for="speaker in speakers" :key="speaker.id" class="flex w-40 shrink-0 snap-start sm:w-44">
        <button
          type="button"
          class="seat flex flex-1 cursor-pointer flex-col items-center justify-end pb-3.5"
          :class="{ 'seat-lead': speaker.isLead }"
          :aria-label="t('activity.people.open', { name: speaker.name })"
          @click="opened = speaker"
        >
          <UiImage
            v-if="speaker.avatar"
            :image="speaker.avatar"
            ratio="1 / 1"
            rounded="rounded-full"
            class="seat-face size-22 sm:size-26"
            sizes="104px"
          />
          <span
            v-else
            class="seat-face seat-initials flex size-22 items-center justify-center font-condensed text-[1.7rem] font-bold text-(--color-day) sm:size-26 sm:text-[2rem]"
            aria-hidden="true"
          >
            {{ speaker.initials }}
          </span>
          <!-- Hauteur fixe : un nom ou une organisation à rallonge ne grandit plus son carton ; le texte entier est dans la fiche. -->
          <span class="tent flex h-32 w-[calc(100%-1.125rem)] flex-col items-center overflow-hidden px-3 pt-4 pb-3 text-center">
            <span class="tent-name line-clamp-2 font-condensed text-[1.12rem] leading-[1.05] font-bold text-heading uppercase sm:text-[1.32rem]">{{ speaker.name }}</span>
            <span v-if="speaker.organization" class="tent-org mt-1.5 line-clamp-2 text-[0.78rem] leading-snug font-semibold text-text-secondary">{{ speaker.organization }}</span>
            <span class="tent-role mt-auto pt-1.5 text-[0.7rem] font-bold tracking-[0.06em] text-(--color-day) uppercase">{{ speaker.role }}</span>
          </span>
        </button>
      </li>
    </ul>

    <UiModal v-model:open="isOpen" :title="opened?.name ?? ''" size="md">
      <div v-if="opened" class="grid gap-4">
        <div class="flex items-center gap-4">
          <UiImage
            v-if="opened.avatar"
            :image="opened.avatar"
            ratio="1 / 1"
            rounded="rounded-full"
            class="size-24 shrink-0 rounded-full border-3 border-(--color-day-line)"
            sizes="96px"
          />
          <span
            v-else
            class="flex size-24 shrink-0 items-center justify-center rounded-full border-3 border-(--color-day-line) bg-(--color-day-surface) font-condensed text-[1.9rem] font-bold text-(--color-day)"
            aria-hidden="true"
          >
            {{ opened.initials }}
          </span>
          <p class="text-[0.75rem] font-bold tracking-[0.05em] text-(--color-day) uppercase">{{ opened.role }}</p>
        </div>
        <dl class="grid gap-4">
          <div v-if="opened.jobTitle">
            <dt class="font-condensed text-[0.95rem] font-semibold tracking-[0.05em] text-text-muted uppercase">{{ t('activity.people.jobTitle') }}</dt>
            <dd class="mt-1 leading-relaxed text-text">{{ opened.jobTitle }}</dd>
          </div>
          <div v-if="opened.organization">
            <dt class="font-condensed text-[0.95rem] font-semibold tracking-[0.05em] text-text-muted uppercase">{{ t('activity.people.organization') }}</dt>
            <dd class="mt-1 leading-relaxed text-text">{{ opened.organization }}</dd>
          </div>
          <div v-if="opened.bio">
            <dt class="font-condensed text-[0.95rem] font-semibold tracking-[0.05em] text-text-muted uppercase">{{ t('activity.people.bio') }}</dt>
            <dd class="mt-1 leading-relaxed whitespace-pre-line text-text">{{ opened.bio }}</dd>
          </div>
        </dl>
      </div>
    </UiModal>
  </section>
</template>

<style scoped>
.people-track {
  scrollbar-width: thin;
}
/* Le plateau de la table, continu d'un siège à l'autre : les sièges d'un rang s'y alignent, quelle que soit la hauteur du carton. */
.seat {
  position: relative;
}
.seat::after {
  content: "";
  position: absolute;
  inset: auto 0 0;
  height: 0.875rem;
  border-top: 2px solid var(--color-border-strong);
  background: var(--color-surface-sunken);
}
/* Assis derrière son carton : le bas du portrait passe sous le chevalet. */
.seat-face {
  flex-shrink: 0;
  margin-bottom: -1.375rem;
  border-radius: 9999px;
  overflow: hidden;
  box-shadow: 0 0 0 4px var(--color-surface);
}
.seat-initials {
  background: color-mix(in srgb, var(--color-day-line) 22%, var(--color-surface-raised));
}
.tent {
  position: relative;
  z-index: 1;
  background: var(--color-surface-raised);
  clip-path: polygon(5% 0, 95% 0, 100% 100%, 0 100%);
  box-shadow: inset 0 5px 0 var(--color-day-line), inset 0 0 0 1px var(--color-border-subtle);
  transition: transform var(--duration-fast) ease-out;
}
.seat-lead .tent {
  background: var(--color-day-solid);
  box-shadow: none;
}
.seat-lead .tent-name,
.seat-lead .tent-org,
.seat-lead .tent-role {
  color: var(--color-day-on-solid);
}
@media (prefers-reduced-motion: no-preference) {
  .seat:hover .tent,
  .seat:focus-visible .tent {
    transform: translateY(-4px);
  }
}
</style>
