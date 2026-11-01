<script setup lang="ts">
/**
 * « Rejoindre le direct » : l'aplat rouge du direct (règle des couleurs d'état),
 * qui mène au lecteur de la page de l'activité (`#direct`). `UiButton` n'a pas
 * de variante « direct » : `danger` dit détruire ou refuser.
 */

interface Props {
  to: string
  label: string
  block?: boolean
}

const props = withDefaults(defineProps<Props>(), { block: false })
</script>

<template>
  <NuxtLink
    :to="props.to"
    class="relative z-10 inline-flex min-h-11 cursor-pointer items-center justify-center gap-2 rounded-md bg-live px-4 font-condensed text-[1.05rem] leading-none font-semibold tracking-[0.05em] whitespace-nowrap text-live-contrast uppercase no-underline transition hover:text-live-contrast hover:shadow-md motion-safe:hover:-translate-y-px"
    :class="props.block ? 'w-full' : ''"
  >
    <UiIcon name="broadcast" size="1.1em" />
    {{ props.label }}
  </NuxtLink>
</template>
