<script setup lang="ts">
import type { PublicSessionSpeaker } from '~/types/programme/session'

/** Les intervenants de l'activité, en cartes centrées — ce que la base publie, sans rien de plus. */

interface Props {
  speakers: PublicSessionSpeaker[]
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

function firstWord(text: string): { lead: string; rest: string } {
  const index = text.indexOf(' ')
  return index > 0 ? { lead: text.slice(0, index), rest: text.slice(index) } : { lead: text, rest: '' }
}

const title = computed(() => firstWord(t('activity.people.speakers')))

const ROLE_ORDER = ['moderator', 'keynote', 'facilitator', 'speaker', 'panelist', 'interpreter']

const speakers = computed(() =>
  [...props.speakers]
    .sort((a, b) => ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role) || a.sort_order - b.sort_order)
    .map((speaker) => ({
      id: speaker.id,
      name: speaker.display_name,
      jobTitle: speaker.job_title_snapshot ?? '',
      organization: speaker.organization_snapshot ?? '',
      bio: speaker.bio ? tr(speaker.bio) : '',
      avatar: speaker.avatar,
      initials: initialsOf(speaker.display_name),
    })),
)
</script>

<template>
  <section v-if="speakers.length" aria-labelledby="intervenants-titre" class="font-sans">
    <h2 id="intervenants-titre" class="border-b border-text pb-3 font-sans text-[26px] leading-tight font-light text-text">
      <b class="font-bold">{{ title.lead }}</b>{{ title.rest }}
    </h2>
    <ul class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 sm:grid-cols-3">
      <li v-for="speaker in speakers" :key="speaker.id" class="flex min-w-0 flex-col items-center text-center">
        <UiImage
          v-if="speaker.avatar"
          :image="speaker.avatar"
          ratio="1 / 1"
          rounded="rounded-full"
          class="size-24 sm:size-28"
          sizes="112px"
        />
        <span
          v-else
          class="flex size-24 items-center justify-center rounded-full bg-surface-inverse text-3xl font-light text-text-on-inverse sm:size-28"
          aria-hidden="true"
        >
          {{ speaker.initials }}
        </span>
        <p class="mt-4 text-lg leading-snug font-bold text-balance text-text sm:text-xl">{{ speaker.name }}</p>
        <p v-if="speaker.jobTitle" class="mt-1 text-[15px] text-text-muted italic">{{ speaker.jobTitle }}</p>
        <p v-if="speaker.organization" class="mt-0.5 text-[15px] text-text-muted">{{ speaker.organization }}</p>
        <details v-if="speaker.bio" class="group mt-1 text-sm">
          <summary class="inline-flex min-h-11 cursor-pointer list-none items-center gap-1 font-bold text-accent [&::-webkit-details-marker]:hidden">
            {{ t('activity.people.bio') }}
            <UiIcon name="chevron-down" size="1rem" class="transition-transform group-open:rotate-180" />
          </summary>
          <p class="pb-1 text-left leading-relaxed whitespace-pre-line text-text-muted">{{ speaker.bio }}</p>
        </details>
      </li>
    </ul>
  </section>
</template>
