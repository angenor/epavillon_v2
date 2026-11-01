<script setup lang="ts">
import type { PublicSessionSpeaker } from '~/types/programme/session'

/** Les intervenants de l'activité, en cartes à la couleur du jour : ce que la base publie, sans rien de plus. */

interface Props {
  speakers: PublicSessionSpeaker[]
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
      role: t(`activity.people.role.${speaker.role}`),
      bio: speaker.bio ? tr(speaker.bio) : '',
      avatar: speaker.avatar,
      initials: initialsOf(speaker.display_name),
    })),
)
</script>

<template>
  <section v-if="speakers.length" aria-labelledby="intervenants-titre">
    <h2 id="intervenants-titre" class="heading-condensed text-[clamp(1.8rem,3.4vw,2.3rem)] text-heading">
      {{ t('activity.people.speakers') }}
    </h2>
    <ul class="mt-4.5 grid grid-cols-1 gap-3.5 sm:grid-cols-2">
      <li
        v-for="speaker in speakers"
        :key="speaker.id"
        class="flex min-w-0 flex-col rounded-lg border border-border-subtle bg-surface-raised p-3.5"
      >
        <div class="flex items-center gap-3.5">
          <UiImage
            v-if="speaker.avatar"
            :image="speaker.avatar"
            ratio="1 / 1"
            rounded="rounded-full"
            class="size-14 shrink-0 rounded-full border-2 border-(--color-day-line)"
            sizes="56px"
          />
          <span
            v-else
            class="flex size-14 shrink-0 items-center justify-center rounded-full border-2 border-(--color-day-line) bg-(--color-day-surface) font-condensed text-[1.2rem] font-bold text-(--color-day)"
            aria-hidden="true"
          >
            {{ speaker.initials }}
          </span>
          <div class="min-w-0">
            <p class="font-condensed text-[1.2rem] leading-[1.1] font-bold text-heading">{{ speaker.name }}</p>
            <p v-if="speaker.position" class="mt-0.5 text-[0.84rem] leading-snug text-text-secondary">{{ speaker.position }}</p>
            <p class="mt-1 text-[0.72rem] font-bold tracking-[0.05em] text-(--color-day) uppercase">{{ speaker.role }}</p>
          </div>
        </div>
        <details v-if="speaker.bio" class="group mt-1 text-sm">
          <summary class="inline-flex min-h-11 cursor-pointer list-none items-center gap-1 font-bold text-accent [&::-webkit-details-marker]:hidden">
            {{ t('activity.people.bio') }}
            <UiIcon name="chevron-down" size="1rem" class="transition-transform group-open:rotate-180" />
          </summary>
          <p class="pb-1 leading-relaxed whitespace-pre-line text-text-secondary">{{ speaker.bio }}</p>
        </details>
      </li>
    </ul>
  </section>
</template>
