<script setup lang="ts">
import type { PublicSessionSpeaker } from '~/types/programme/session'

/**
 * Les intervenants de l'activité, en cartes verticales à la couleur du jour : ce que la base publie, sans rien de plus.
 * Fonction et organisation s'arrêtent sur des points de suspension ; la carte ouvre la fiche entière (jamais le courriel, qui n'est pas public).
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
    <h2 id="intervenants-titre" class="heading-condensed text-[clamp(1.8rem,3.4vw,2.3rem)] text-heading">
      {{ t('activity.people.speakers') }}
    </h2>
    <ul class="mt-4.5 grid grid-cols-2 gap-2.5 sm:grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] sm:gap-3.5">
      <li v-for="speaker in speakers" :key="speaker.id" class="min-w-0">
        <button
          type="button"
          class="flex size-full cursor-pointer flex-col items-center rounded-lg border border-border-subtle bg-surface-raised px-2.5 pt-4 pb-3.5 text-center transition-colors duration-(--duration-fast) hover:border-(--color-day-line) sm:px-3.5 sm:pt-5 sm:pb-4.5"
          :aria-label="t('activity.people.open', { name: speaker.name })"
          @click="opened = speaker"
        >
          <UiImage
            v-if="speaker.avatar"
            :image="speaker.avatar"
            ratio="1 / 1"
            rounded="rounded-full"
            class="size-19 shrink-0 rounded-full border-3 border-(--color-day-line) sm:size-24"
            sizes="96px"
          />
          <span
            v-else
            class="flex size-19 shrink-0 items-center justify-center rounded-full border-3 border-(--color-day-line) bg-(--color-day-surface) font-condensed text-[1.5rem] font-bold text-(--color-day) sm:size-24 sm:text-[1.9rem]"
            aria-hidden="true"
          >
            {{ speaker.initials }}
          </span>
          <span class="mt-3.5 font-condensed text-[1.15rem] leading-[1.05] font-bold text-heading sm:text-[1.3rem]">{{ speaker.name }}</span>
          <span class="mt-1.5 text-[0.72rem] font-bold tracking-[0.05em] text-(--color-day) uppercase">{{ speaker.role }}</span>
          <span v-if="speaker.jobTitle" class="mt-2.5 line-clamp-3 text-[0.86rem] leading-snug text-text-secondary">{{ speaker.jobTitle }}</span>
          <span v-if="speaker.organization" class="mt-1 line-clamp-2 text-[0.82rem] leading-snug font-semibold text-text-muted">{{ speaker.organization }}</span>
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
