<script setup lang="ts">
import type { NavItem } from '~/types/navigation'

/**
 * Barre de navigation principale — l'en-tête du site public.
 *
 * ELLE NE DÉCIDE DE RIEN. Les entrées lui sont données par le layout : c'est lui
 * qui sait quelles pages existent, et lui qui consultera `platform.feature_flags`
 * pour masquer les modules fermés. Une barre qui connaîtrait ses propres liens
 * obligerait à la modifier à chaque écran ajouté.
 *
 * PAGE COURANTE : marquée par `aria-current="page"` ET par un traitement visuel.
 * L'un sans l'autre laisse la moitié des visiteurs sans repère.
 *
 * CE TRAITEMENT EST UN FILET DE 3 px, PAS UN APLAT — règle du guide. Un aplat
 * derrière l'entrée courante la ferait passer pour un bouton, dans une barre où
 * tout le reste est un lien ; et il entre en concurrence avec le survol, qui,
 * lui, teinte bien le fond. La graisse seule ne suffit pas : elle se compare mal
 * de loin, et disparaît pour qui lit avec une police de substitution.
 *
 * MENU MOBILE : replié sous 1024 px, ouvert par un bouton qui déclare ce qu'il
 * contrôle (`aria-controls`, `aria-expanded`). Le menu se referme à chaque
 * changement de route — c'est au layout de le dire, il connaît la route.
 *
 * STICKY : la barre reste en tête au défilement. C'est une navigation de site
 * public, où l'on saute souvent d'une section à l'autre.
 */

interface Props {
  items: NavItem[]
  /** Nom de la navigation, annoncé par les lecteurs d'écran. */
  label: string
  /** Menu mobile ouvert — contrôlé par le layout. */
  open?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean] }>()

const { t } = useI18n()
const localePath = useLocalePath()
const route = useRoute()

const isCurrent = (to: string): boolean => route.path === localePath(to)
</script>

<template>
  <!-- `min-h-(--nav-height)` : la hauteur de cette barre est une donnée de mise
       en page, lue par le bandeau d'accueil pour occuper exactement un écran.
       Sans elle, elle variait de 69 à 67,8 px selon la largeur, au gré du
       rendu du logo. Le menu mobile déplié la fait grandir, et c'est voulu. -->
  <header class="sticky top-0 z-30 min-h-(--nav-height) border-b border-border bg-surface-raised">
    <div class="mx-auto flex w-full max-w-[1280px] items-center gap-4 px-4 py-3 sm:px-6">
      <slot name="brand" />

      <nav class="ml-auto hidden items-center gap-1 lg:flex" :aria-label="props.label">
        <NuxtLink
          v-for="item in props.items"
          :key="item.to"
          :to="localePath(item.to)"
          class="inline-flex min-h-(--target-min) items-center rounded-md px-3 text-sm font-semibold no-underline transition-colors duration-(--duration-fast)"
          :class="
            isCurrent(item.to)
              ? 'rounded-b-none text-accent shadow-[inset_0_-3px_0_var(--color-accent)]'
              : 'text-text-secondary hover:bg-surface-hover hover:text-text'
          "
          :aria-current="isCurrent(item.to) ? 'page' : undefined"
        >
          {{ t(item.labelKey) }}
        </NuxtLink>
      </nav>

      <div class="ml-auto flex items-center gap-2 lg:ml-0">
        <slot name="actions" />

        <button
          type="button"
          class="rounded-md border border-border p-2 text-text transition-colors hover:bg-surface-hover lg:hidden"
          :aria-expanded="props.open"
          aria-controls="ui-navbar-mobile"
          @click="emit('update:open', !props.open)"
        >
          <span class="sr-only">
            {{ props.open ? t('common.a11y.closeMenu') : t('common.a11y.openMenu') }}
          </span>
          <UiIcon :name="props.open ? 'close' : 'menu'" size="1.25rem" :stroke-width="1.8" />
        </button>
      </div>
    </div>

    <nav
      v-if="props.open"
      id="ui-navbar-mobile"
      class="border-t border-border-subtle bg-surface-raised px-4 py-2 lg:hidden"
      :aria-label="props.label"
    >
      <!-- Menu empilé : le même filet de 3 px, mais posé au bord d'attaque de
           l'entrée. Sous une liste verticale, un filet inférieur se lirait comme
           un séparateur entre deux entrées, pas comme un repère de position. -->
      <NuxtLink
        v-for="item in props.items"
        :key="item.to"
        :to="localePath(item.to)"
        class="flex min-h-(--target-min) items-center rounded-md px-3 text-sm font-semibold no-underline transition-colors duration-(--duration-fast)"
        :class="
          isCurrent(item.to)
            ? 'text-accent shadow-[inset_3px_0_0_var(--color-accent)]'
            : 'text-text-secondary hover:bg-surface-hover hover:text-text'
        "
        :aria-current="isCurrent(item.to) ? 'page' : undefined"
      >
        {{ t(item.labelKey) }}
      </NuxtLink>

      <div v-if="$slots['mobile-footer']" class="mt-2 flex items-center gap-2 border-t border-border-subtle pt-2">
        <slot name="mobile-footer" />
      </div>
    </nav>
  </header>
</template>
