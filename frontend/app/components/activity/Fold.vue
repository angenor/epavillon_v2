<script setup lang="ts">
/**
 * Un texte long replié : environ six lignes, le bas fondu dans la page, et
 * « Lire la suite » qui déplie tout (maquette `a-propos-replie.html`, 02/11).
 * Un texte qui tient s'affiche en entier, sans bouton : on le mesure une fois
 * monté, le rendu serveur le montre replié.
 */

const { t } = useI18n()

const body = useTemplateRef<HTMLElement>('body')
const open = ref(false)
const overflows = ref(true)
const id = useId()

function measure(): void {
  const element = body.value
  if (!element || open.value) return
  overflows.value = element.scrollHeight > element.clientHeight + 8
}

onMounted(() => {
  measure()
  window.addEventListener('resize', measure)
})
onBeforeUnmount(() => window.removeEventListener('resize', measure))
</script>

<template>
  <div class="fold" :class="{ 'is-closed': !open && overflows }">
    <div :id="id" ref="body" class="fold-body" :class="{ 'is-clamped': !open }">
      <slot />
    </div>
    <button
      v-if="overflows"
      type="button"
      class="mt-1 inline-flex min-h-(--target-min) cursor-pointer items-center gap-2 px-0.5 font-condensed text-[1.05rem] font-bold tracking-[0.04em] text-text-link uppercase underline-offset-4 hover:underline"
      :aria-expanded="open"
      :aria-controls="id"
      @click="open = !open"
    >
      {{ t(open ? 'activity.fold.less' : 'activity.fold.more') }}
      <UiIcon name="chevron-down" size="1.1rem" :stroke-width="2.4" class="transition-transform duration-200" :class="{ 'rotate-180': open }" />
    </button>
  </div>
</template>

<style scoped>
.fold {
  position: relative;
}

.fold-body.is-clamped {
  max-height: 11.5em;
  overflow: hidden;
}

.fold.is-closed::after {
  content: "";
  position: absolute;
  inset-inline: 0;
  bottom: var(--target-min);
  height: 5.5em;
  /* Une carte posée sur la page redonne sa propre surface : `[--fold-fade:…]`. */
  background: linear-gradient(to bottom, transparent, var(--fold-fade, var(--color-surface)));
  pointer-events: none;
}
</style>
