<script setup lang="ts">
import type { NavItem } from '~/types/navigation'

/**
 * Menu du compte — la bulle d'initiales de la barre de navigation, et ce qu'elle
 * déroule : les destinations personnelles, puis la déconnexion.
 *
 * IL NE DÉCIDE DE RIEN, comme `UiNavBar` : les entrées lui sont données par le
 * layout, qui seul sait où mène « Mon organisation » selon le rattachement.
 *
 * PAS DE PHOTO, ET CE N'EST PAS UN OUBLI : voir `initialsOf()`, que le pied de
 * la navigation du back-office partage avec ce menu.
 *
 * ── OUVERTURE AU SURVOL, MAIS PAS SEULEMENT ─────────────────────────────────
 *
 * Le survol est le geste attendu à la souris, et c'est ce qui est demandé. Il ne
 * peut pas être le SEUL : il n'existe ni au doigt, ni au clavier. Le
 * déclencheur est donc un vrai bouton qui répond aussi au clic et au focus.
 *
 * La fermeture au survol est RETARDÉE (`CLOSE_DELAY`) : entre le bouton et le
 * panneau, le curseur traverse quelques pixels, et une fermeture immédiate rend
 * le menu inatteignable. Un pointeur grossier (tactile) n'ouvre jamais au
 * survol — sinon le premier appui ouvre et le second referme sans rien faire.
 *
 * ── PANNEAU FERMÉ = PANNEAU ABSENT DU DOM ───────────────────────────────────
 *
 * Ses entrées ne sont ni focalisables ni lisibles tant qu'il est replié, sans
 * jouer sur `tabindex`. Échap referme et REND LE FOCUS au déclencheur.
 *
 * CE MENU N'EXISTE QU'À PARTIR DE `sm`. Sous cette largeur, la barre replie tout
 * dans son menu mobile : un panneau flottant y serait plus étroit que l'écran
 * qui le porte.
 */

interface Props {
  /** Nom affiché — `person.display_name`, colonne générée. */
  name: string
  /** Adresse principale, affichée en tête du panneau. */
  email?: string
  /** Destinations personnelles — « Mon organisation », et ce qui viendra. */
  items: NavItem[]
  /** Nom accessible du déclencheur, annoncé par les lecteurs d'écran. */
  label: string
  /** Libellé de la déconnexion — l'action qui ferme le panneau. */
  signOutLabel: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'sign-out': [] }>()

const { t } = useI18n()
const localePath = useLocalePath()
const route = useRoute()

/** Marge de traversée entre le bouton et le panneau, en millisecondes. */
const CLOSE_DELAY = 160

const isOpen = ref(false)
const root = ref<HTMLElement | null>(null)
const trigger = ref<HTMLButtonElement | null>(null)
const panel = ref<HTMLElement | null>(null)
let closeTimer: ReturnType<typeof setTimeout> | null = null

const initials = computed(() => initialsOf(props.name))

const isCurrent = (to: string): boolean => route.path === localePath(to)

function cancelClose(): void {
  if (closeTimer !== null) {
    clearTimeout(closeTimer)
    closeTimer = null
  }
}

function open(): void {
  cancelClose()
  isOpen.value = true
}

function close(): void {
  cancelClose()
  isOpen.value = false
}

/** Le survol n'ouvre que là où il existe vraiment. */
function openOnHover(): void {
  if (window.matchMedia('(hover: hover)').matches) open()
}

function closeOnHover(): void {
  cancelClose()
  closeTimer = setTimeout(close, CLOSE_DELAY)
}

function closeAndRefocus(): void {
  close()
  trigger.value?.focus()
}

/** Le focus sort de l'ensemble bouton + panneau : le menu n'a plus de raison d'être. */
function onFocusOut(event: FocusEvent): void {
  const next = event.relatedTarget
  if (next instanceof Node && root.value?.contains(next)) return
  close()
}

function onPointerDownOutside(event: PointerEvent): void {
  if (!isOpen.value) return
  if (event.target instanceof Node && root.value?.contains(event.target)) return
  close()
}

/** Ouverture au clavier : le focus descend dans le panneau, sinon il reste bloqué. */
async function openWithFocus(): Promise<void> {
  open()
  await nextTick()
  panel.value?.querySelector<HTMLElement>('a, button')?.focus()
}

function onTriggerClick(): void {
  if (isOpen.value) close()
  else void openWithFocus()
}

function onSignOut(): void {
  close()
  emit('sign-out')
}

// Une navigation referme le panneau : la destination a changé sous lui.
watch(() => route.fullPath, close)

onMounted(() => document.addEventListener('pointerdown', onPointerDownOutside))
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onPointerDownOutside)
  cancelClose()
})
</script>

<template>
  <div
    ref="root"
    class="relative hidden sm:block"
    @mouseenter="openOnHover()"
    @mouseleave="closeOnHover()"
    @focusout="onFocusOut"
    @keydown.esc.prevent="closeAndRefocus()"
  >
    <button
      ref="trigger"
      type="button"
      class="flex min-h-(--target-min) cursor-pointer items-center gap-1.5 rounded-full border border-transparent px-1 transition-colors duration-(--duration-fast) hover:border-border focus-visible:border-border"
      :class="isOpen ? 'border-border bg-surface-hover' : ''"
      :aria-expanded="isOpen"
      aria-haspopup="menu"
      aria-controls="ui-user-menu"
      @click="onTriggerClick()"
      @keydown.down.prevent="openWithFocus()"
    >
      <span class="sr-only">{{ props.label }}</span>
      <!-- La bulle porte l'accent en fond très pâle, pas en aplat saturé : dans
           une barre où tout le reste est du texte, un disque plein tirerait
           l'œil plus que la navigation elle-même. -->
      <span
        aria-hidden="true"
        class="grid size-9 shrink-0 place-items-center rounded-full border border-accent-border bg-accent-surface text-sm font-semibold text-accent"
      >
        {{ initials }}
      </span>
      <UiIcon
        name="chevron-down"
        size="1rem"
        :stroke-width="1.8"
        class="text-text-muted transition-transform duration-(--duration-fast)"
        :class="isOpen ? 'rotate-180' : ''"
      />
    </button>

    <div
      v-if="isOpen"
      id="ui-user-menu"
      ref="panel"
      role="menu"
      :aria-label="props.label"
      class="absolute end-0 top-full z-40 mt-1 w-64 rounded-lg border border-border bg-surface-overlay py-1 shadow-md"
    >
      <!-- Qui est connecté, en tête. La bulle ne porte que deux lettres : sans
           ce rappel, deux personnes aux mêmes initiales ne se distinguent pas. -->
      <div class="border-b border-border-subtle px-3 pb-2 pt-1.5">
        <p class="truncate text-sm font-semibold text-text">{{ props.name }}</p>
        <p v-if="props.email" class="truncate text-xs text-text-secondary">{{ props.email }}</p>
      </div>

      <NuxtLink
        v-for="item in props.items"
        :key="item.to"
        :to="localePath(item.to)"
        role="menuitem"
        class="flex min-h-(--target-min) items-center gap-2 px-3 text-sm no-underline transition-colors duration-(--duration-fast)"
        :class="
          isCurrent(item.to)
            ? 'font-semibold text-accent'
            : 'text-text-secondary hover:bg-surface-hover hover:text-text'
        "
        :aria-current="isCurrent(item.to) ? 'page' : undefined"
        @click="close()"
      >
        <UiIcon v-if="item.icon" :name="item.icon" size="1.1rem" :stroke-width="1.7" />
        {{ t(item.labelKey) }}
      </NuxtLink>

      <!-- La déconnexion est séparée : c'est la seule entrée qui ne mène nulle
           part et qu'on ne veut pas atteindre par erreur en descendant la liste. -->
      <div class="mt-1 border-t border-border-subtle pt-1">
        <button
          type="button"
          role="menuitem"
          class="flex min-h-(--target-min) w-full cursor-pointer items-center gap-2 px-3 text-start text-sm text-text-secondary transition-colors duration-(--duration-fast) hover:bg-surface-hover hover:text-text"
          @click="onSignOut()"
        >
          <UiIcon name="arrow-right" size="1.1rem" :stroke-width="1.7" />
          {{ props.signOutLabel }}
        </button>
      </div>
    </div>
  </div>
</template>
