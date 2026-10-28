<script setup lang="ts">
import type { AttachedImage } from '~/types/media'

/**
 * La galerie d'une activité (D7) : des vignettes, et la photo en grand dans une
 * boîte de dialogue native (`UiModal` : focus piégé, Échap ferme). Les flèches
 * du clavier passent d'une photo à l'autre. L'appelant ne la monte que s'il y a
 * au moins une photo ; l'API ne sert que les photos prêtes et publiques.
 */

const props = defineProps<{ photos: AttachedImage[] }>()

const { t } = useI18n()
const { tr } = useI18nText()

const openIndex = ref<number | null>(null)
const current = computed(() => (openIndex.value === null ? null : (props.photos[openIndex.value] ?? null)))
const caption = computed(() => (current.value?.caption ? tr(current.value.caption) : ''))
const total = computed(() => props.photos.length)

function open(index: number): void {
  openIndex.value = index
}

function close(): void {
  openIndex.value = null
}

function step(offset: -1 | 1): void {
  if (openIndex.value === null || total.value === 0) return
  openIndex.value = (openIndex.value + offset + total.value) % total.value
}

function onKeydown(event: KeyboardEvent): void {
  if (openIndex.value === null) return
  if (event.key === 'ArrowLeft') {
    event.preventDefault()
    step(-1)
  } else if (event.key === 'ArrowRight') {
    event.preventDefault()
    step(1)
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <section aria-labelledby="galerie-titre">
    <h2 id="galerie-titre" class="border-b border-text pb-3 font-sans text-[26px] font-light text-text">
      <b class="font-bold">{{ t('activity.gallery.title') }}</b>
    </h2>

    <ul class="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-3">
      <li v-for="(photo, index) in props.photos" :key="photo.asset_id">
        <button
          type="button"
          class="block w-full cursor-pointer rounded-md focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
          :aria-label="t('activity.gallery.open', { index: index + 1, total })"
          @click="open(index)"
        >
          <UiImage :image="photo" ratio="4 / 3" rounded="rounded-md" sizes="(min-width: 1024px) 15rem, (min-width: 640px) 30vw, 45vw" />
        </button>
      </li>
    </ul>

    <UiModal
      :open="current !== null"
      size="xl"
      :title="t('activity.gallery.dialog', { index: (openIndex ?? 0) + 1, total })"
      @update:open="(value: boolean) => !value && close()"
    >
      <figure v-if="current" class="m-0">
        <UiImage
          :image="current"
          ratio="auto"
          fit="contain"
          loading="eager"
          frame-class="h-[min(62vh,40rem)]"
          rounded="rounded-md"
          sizes="(min-width: 640px) 56rem, 100vw"
        />
        <figcaption v-if="caption || current.credit" class="mt-3 text-sm text-text-secondary">
          {{ caption }}
          <span v-if="current.credit" class="italic text-text-muted">{{ current.credit }}</span>
        </figcaption>
      </figure>
      <p class="mt-3 text-xs text-text-muted">{{ t('activity.gallery.keyboard') }}</p>

      <template v-if="total > 1" #footer>
        <UiButton variant="secondary" icon="chevron-left" @click="step(-1)">
          {{ t('activity.gallery.previous') }}
        </UiButton>
        <UiButton variant="secondary" icon-trailing="chevron-right" @click="step(1)">
          {{ t('activity.gallery.next') }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>
