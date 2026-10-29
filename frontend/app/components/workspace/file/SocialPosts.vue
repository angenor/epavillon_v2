<script setup lang="ts">
import type { SocialPost } from '~/types/programme/day-plan'
import type { Uuid } from '~/types/shared'

/**
 * LES PUBLICATIONS SUR LES RÉSEAUX (29/10) : l'organisation colle le lien de
 * chaque publication, l'IFDD la relaie. Le réseau se reconnaît à l'adresse,
 * côté API : aucune liste déroulante à remplir.
 */

interface Props {
  posts: SocialPost[]
  canEdit: boolean
  busy: boolean
  error: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ add: [url: string]; remove: [postId: Uuid] }>()

const { t } = useI18n()

const url = ref('')

function add(): void {
  const value = url.value.trim()
  if (!value) return
  emit('add', value)
}

// Le lien accepté apparaît dans la liste : le champ se vide pour le suivant.
watch(
  () => props.posts.length,
  () => {
    if (!props.error) url.value = ''
  },
)
</script>

<template>
  <section
    id="publications"
    aria-labelledby="publications-title"
    class="scroll-mt-24 rounded-lg border border-border bg-surface-raised p-5 sm:p-6"
  >
    <h2 id="publications-title" class="text-xl font-semibold text-heading">
      {{ t('organization.workspace.proposal.social.title') }}
    </h2>
    <p class="mt-1 text-sm text-text-muted">{{ t('organization.workspace.proposal.social.description') }}</p>

    <ul v-if="props.posts.length > 0" class="mt-4 flex flex-col gap-2">
      <li v-for="post in props.posts" :key="post.id" class="flex items-center gap-3 rounded-md border border-border px-3 py-2">
        <UiBadge :label="t(`organization.workspace.proposal.social.platforms.${post.platform}`)" class="shrink-0" />
        <a :href="post.url" target="_blank" rel="noopener" class="min-w-0 flex-1 truncate text-sm">{{ post.url }}</a>
        <UiButton
          v-if="props.canEdit"
          variant="ghost"
          icon="trash"
          :disabled="props.busy"
          :aria-label="t('organization.workspace.proposal.social.removeLabel', { url: post.url })"
          @click="emit('remove', post.id)"
        >
          {{ t('organization.workspace.proposal.social.remove') }}
        </UiButton>
      </li>
    </ul>

    <form v-if="props.canEdit" class="mt-4 flex flex-col gap-3 sm:flex-row sm:items-end" @submit.prevent="add">
      <UiInput
        id="social-post-url"
        v-model="url"
        class="flex-1"
        type="url"
        :label="t('organization.workspace.proposal.social.field')"
        :placeholder="t('organization.workspace.proposal.social.placeholder')"
      />
      <UiButton type="submit" variant="secondary" icon="plus" :loading="props.busy">
        {{ t('organization.workspace.proposal.social.add') }}
      </UiButton>
    </form>
    <UiAlert v-if="props.error" class="mt-3" intent="danger" :message="props.error" live />
  </section>
</template>
