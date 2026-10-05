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
 * LE BOUTON NE PORTE PAS LE NOM DE LA PERSONNE (05/10) : `title` (« Mon espace »)
 * et `subtitle` (le sigle de l'organisation) disent où l'on va sans afficher qui
 * l'on est sur un écran partagé. Le nom n'apparaît qu'à l'ouverture. `badge`
 * compte ce qui attend la personne ; le slot `lead` en montre la première ligne.
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
  /** `inverse` : déclencheur posé sur un aplat institutionnel ; le panneau, lui, ne change pas. */
  tone?: 'default' | 'inverse'
  /** Texte du bouton ; absent, le bouton n'est que la bulle. */
  title?: string
  subtitle?: string
  /** Nombre de choses qui attendent la personne ; 0 ou absent, pas de pastille. */
  badge?: number
  /** Ce que compte la pastille, annoncé aux lecteurs d'écran. */
  badgeLabel?: string
  /** Intitulé de la liste des entrées. */
  itemsLabel?: string
}

const props = withDefaults(defineProps<Props>(), { tone: 'default' })
const emit = defineEmits<{ 'sign-out': []; open: [] }>()

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

const tones = computed(() =>
  props.tone === 'inverse'
    ? {
        trigger: props.title
          ? 'border-border-on-inverse bg-surface-inverse-raised hover:border-accent-on-inverse'
          : 'hover:border-border-on-inverse focus-visible:border-border-on-inverse',
        open: 'border-border-on-inverse bg-surface-inverse-raised',
        bubble: 'border-accent-on-inverse bg-surface-inverse-raised text-text-on-inverse',
        badge: 'border-surface-inverse-raised bg-warning-on-inverse text-text-on-warning-inverse',
        title: 'text-text-on-inverse',
        chevron: 'text-text-on-inverse-muted',
      }
    : {
        trigger: props.title
          ? 'border-border bg-surface-raised hover:border-border-strong'
          : 'hover:border-border focus-visible:border-border',
        open: 'border-border bg-surface-hover',
        bubble: 'border-accent-border bg-accent-surface text-accent',
        badge: 'border-surface-raised bg-warning-surface text-warning',
        title: 'text-text',
        chevron: 'text-text-muted',
      },
)

const isCurrent = (to: string): boolean => route.path === localePath(to)

function cancelClose(): void {
  if (closeTimer !== null) {
    clearTimeout(closeTimer)
    closeTimer = null
  }
}

function open(): void {
  cancelClose()
  if (!isOpen.value) emit('open')
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
      class="flex min-h-(--target-min) cursor-pointer items-center gap-2 rounded-full border px-1 transition-colors duration-(--duration-fast)"
      :class="[props.title ? 'pe-3' : 'border-transparent', tones.trigger, isOpen ? tones.open : '']"
      :aria-expanded="isOpen"
      aria-haspopup="menu"
      aria-controls="ui-user-menu"
      @click="onTriggerClick()"
      @keydown.down.prevent="openWithFocus()"
    >
      <span class="sr-only">{{ props.label }}<template v-if="props.badge && props.badgeLabel"> — {{ props.badgeLabel }}</template></span>
      <!-- La bulle porte l'accent en fond très pâle, pas en aplat saturé : dans
           une barre où tout le reste est du texte, un disque plein tirerait
           l'œil plus que la navigation elle-même. -->
      <span aria-hidden="true" class="relative shrink-0">
        <span class="grid size-9 place-items-center rounded-full border text-sm font-semibold" :class="tones.bubble">
          {{ initials }}
        </span>
        <span
          v-if="props.badge"
          class="absolute -end-1.5 -top-1 grid h-4.5 min-w-4.5 place-items-center rounded-full border-2 px-1 text-[11px] leading-none font-bold tabular-nums"
          :class="tones.badge"
        >
          {{ props.badge > 9 ? '9+' : props.badge }}
        </span>
      </span>
      <span v-if="props.title" aria-hidden="true" class="flex max-w-40 min-w-0 flex-col text-start leading-tight" :class="tones.title">
        <span class="truncate text-sm font-semibold">{{ props.title }}</span>
        <span v-if="props.subtitle" class="truncate text-xs" :class="tones.chevron">{{ props.subtitle }}</span>
      </span>
      <UiIcon
        name="chevron-down"
        size="1rem"
        :stroke-width="1.8"
        class="transition-transform duration-(--duration-fast)"
        :class="[tones.chevron, isOpen ? 'rotate-180' : '']"
      />
    </button>

    <div
      v-if="isOpen"
      id="ui-user-menu"
      ref="panel"
      role="menu"
      :aria-label="props.label"
      class="absolute end-0 top-full z-40 mt-1.5 w-96 overflow-hidden rounded-[10px] border border-border bg-surface-overlay shadow-lg"
    >
      <!-- Qui est connecté, en tête. La bulle ne porte que deux lettres : sans
           ce rappel, deux personnes aux mêmes initiales ne se distinguent pas. -->
      <div class="flex items-center gap-3 border-b border-border-subtle px-4.5 py-4">
        <span aria-hidden="true" class="grid size-11 shrink-0 place-items-center rounded-full border border-accent-border bg-accent-surface font-bold text-accent">
          {{ initials }}
        </span>
        <div class="min-w-0">
          <p class="truncate text-[15px] font-bold text-text">{{ props.name }}</p>
          <p v-if="props.email" class="truncate text-[13px] text-text-muted">{{ props.email }}</p>
        </div>
      </div>

      <slot name="lead" />

      <div class="py-1.5">
        <p
          v-if="props.itemsLabel"
          class="px-4.5 pt-1.5 pb-1 text-[11.5px] font-bold text-text-muted uppercase"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ props.itemsLabel }}
        </p>

        <NuxtLink
          v-for="item in props.items"
          :key="item.to"
          :to="localePath(item.to)"
          role="menuitem"
          class="flex min-h-(--target-min) items-center gap-2.5 px-4.5 text-sm font-semibold no-underline transition-colors duration-(--duration-fast) hover:bg-surface-hover"
          :class="isCurrent(item.to) ? 'text-accent' : 'text-text'"
          :aria-current="isCurrent(item.to) ? 'page' : undefined"
          @click="close()"
        >
          <UiIcon v-if="item.icon" :name="item.icon" size="1.1rem" :stroke-width="1.7" class="shrink-0 text-text-muted" />
          <span class="min-w-0 flex-1 truncate">{{ t(item.labelKey) }}</span>
          <span v-if="item.meta !== undefined || item.count !== undefined" class="max-w-36 shrink-0 truncate text-[13px] font-normal text-text-muted tabular-nums">
            {{ item.meta ?? item.count }}
          </span>
        </NuxtLink>
      </div>

      <!-- La déconnexion est séparée : c'est la seule entrée qui ne mène nulle
           part et qu'on ne veut pas atteindre par erreur en descendant la liste. -->
      <div class="border-t border-border-subtle py-1">
        <button
          type="button"
          role="menuitem"
          class="flex min-h-(--target-min) w-full cursor-pointer items-center gap-2.5 px-4.5 text-start text-sm text-text-secondary transition-colors duration-(--duration-fast) hover:bg-surface-hover hover:text-text"
          @click="onSignOut()"
        >
          <UiIcon name="log-out" size="1.1rem" :stroke-width="1.7" class="text-text-muted" />
          {{ props.signOutLabel }}
        </button>
      </div>
    </div>
  </div>
</template>
