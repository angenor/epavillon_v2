<script setup lang="ts">
import type { NavSection } from '~/types/navigation'

/**
 * Navigation latérale du back-office — sections, entrées, compteurs.
 *
 * SECTIONS TITRÉES plutôt qu'une liste plate : « Programmation », « Référentiels »,
 * « Exploitation ». Douze entrées d'affilée ne se parcourent pas ; trois groupes
 * de quatre, si. Chaque titre est un vrai `<h2>` — c'est ce qui permet de
 * naviguer par en-têtes avec un lecteur d'écran.
 *
 * COLONNE FIXE DE 260 px au-delà de 1024 px : la largeur vient du guide — elle
 * tient « Espace négociateurs » et son compteur sur une seule ligne, ce qui est
 * le libellé le plus long de la section, sans voler de place au tableau qui suit.
 * La colonne est une PILE : marque en haut, entrées au milieu — seule zone qui
 * défile —, compte et retour au site en bas. Une liste qui s'allonge ne pousse
 * donc jamais le pied hors de l'écran.
 *
 * EN DESSOUS, C'EST UN TIROIR QUI COUVRE LA PAGE, et ce n'est pas un raffinement.
 * Le panneau était rendu dans le flux, avant l'en-tête : le bouton qui l'ouvrait
 * se trouvait donc SOUS lui, et l'ouvrir poussait tout le contenu vers le bas
 * sans que rien n'apparaisse à l'endroit regardé. Le tiroir glisse au-dessus, sur
 * un voile, et se referme au voile, à Échap, ou en arrivant sur la page demandée
 * (le layout remet `open` à faux au changement de route). Fermé, il est
 * `invisible` : ses liens sortent alors de l'ordre de tabulation, ce qu'une
 * simple translation hors cadre ne fait pas.
 *
 * ICÔNE ET COMPTEUR FACULTATIFS. L'icône aide au repérage d'une liste de douze
 * entrées parcourue chaque jour ; le compteur poussé à droite dit ce qui attend
 * (« Dossiers déposés · 24 ») sans obliger à ouvrir l'écran. Ni l'un ni l'autre
 * n'est requis : une entrée sans icône ne décale pas son libellé, une section
 * entière peut s'en passer.
 *
 * L'ENTRÉE COURANTE PORTE UN LISERÉ, en plus de son aplat teinté. Sur une colonne
 * de douze lignes lue en diagonale, un fond très clair ne se repère pas : le
 * trait vertical accroche l'œil au bord de la colonne, là où il descend.
 *
 * LES SÉPARATIONS SONT EN `--color-border`, JAMAIS EN `-subtle` : en thème sombre,
 * le trait à peine visible et le fond des panneaux valent tous deux `gris-800`.
 * Les trois traits de la colonne y étaient donc rigoureusement invisibles, et le
 * regroupement des entrées avec eux.
 *
 * REPLIÉE (`collapsed`), LA COLONNE DEVIENT UN RAIL D'ICÔNES de 68 px, au-delà de
 * 1024 px seulement : le tiroir garde ses libellés. Toutes les variantes portent
 * donc le préfixe `lg:`, y compris dans les emplacements, qui reçoivent
 * `collapsed` pour s'y plier. Les libellés restent dans le DOM (`lg:sr-only`) :
 * lecteurs d'écran et navigation par titres n'y perdent rien.
 */

interface Props {
  sections: NavSection[]
  /** Nom de la navigation, annoncé par les lecteurs d'écran. */
  label: string
  /** Tiroir déployé sur écran étroit — contrôlé par le layout. */
  open?: boolean
  /** Rail d'icônes sur grand écran ; sans effet sur le tiroir. */
  collapsed?: boolean
  /** Montre le bouton qui replie et déplie la colonne sur grand écran. */
  collapsible?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ close: []; toggleCollapsed: [] }>()

const { t } = useI18n()
const localePath = useLocalePath()
const route = useRoute()

const isCurrent = (to: string): boolean => route.path === localePath(to)

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && props.open) emit('close')
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <Transition
    enter-active-class="transition-opacity duration-(--duration-base)"
    leave-active-class="transition-opacity duration-(--duration-base)"
    enter-from-class="opacity-0"
    leave-to-class="opacity-0"
  >
    <div
      v-if="props.open"
      class="fixed inset-0 z-40 bg-scrim/50 lg:hidden"
      aria-hidden="true"
      @click="emit('close')"
    />
  </Transition>

  <aside
    id="ui-sidenav"
    class="fixed start-0 top-0 z-50 flex h-dvh w-[min(19rem,86vw)] flex-col overflow-x-hidden bg-surface-raised shadow-lg transition-[transform,visibility] duration-(--duration-base) ease-out lg:visible lg:sticky lg:z-0 lg:h-screen lg:shrink-0 lg:translate-x-0 lg:border-e lg:border-border lg:shadow-none lg:transition-[width]"
    :class="[
      props.open ? 'visible translate-x-0' : 'invisible -translate-x-full',
      props.collapsed ? 'lg:w-17' : 'lg:w-65',
    ]"
  >
    <div
      v-if="$slots.brand"
      class="flex shrink-0 items-start gap-2 border-b border-border px-5 py-4"
      :class="props.collapsed ? 'lg:flex-col lg:items-center lg:px-3' : ''"
    >
      <div class="min-w-0 flex-1">
        <slot name="brand" :collapsed="props.collapsed" />
      </div>

      <button
        v-if="props.collapsible"
        type="button"
        class="hidden min-h-(--target-min) min-w-(--target-min) shrink-0 cursor-pointer items-center justify-center rounded-md text-text-subtle transition-colors hover:bg-surface-hover hover:text-text lg:inline-flex"
        :class="props.collapsed ? '' : '-me-2'"
        :aria-expanded="!props.collapsed"
        aria-controls="ui-sidenav"
        :aria-label="props.collapsed ? t('nav.admin.sidebar.expand') : t('nav.admin.sidebar.collapse')"
        :title="props.collapsed ? t('nav.admin.sidebar.expand') : t('nav.admin.sidebar.collapse')"
        @click="emit('toggleCollapsed')"
      >
        <UiIcon name="sidebar" size="1.15rem" :stroke-width="1.8" />
      </button>

      <button
        type="button"
        class="-mt-1 -me-2 shrink-0 cursor-pointer rounded-md p-2 text-text-subtle transition-colors hover:bg-surface-hover hover:text-text lg:hidden"
        @click="emit('close')"
      >
        <span class="sr-only">{{ t('common.a11y.closeMenu') }}</span>
        <UiIcon name="close" size="1.15rem" :stroke-width="1.8" />
      </button>
    </div>

    <nav class="min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-3 py-4" :aria-label="props.label">
      <div
        v-for="(section, index) in props.sections"
        :key="section.labelKey"
        :class="index > 0 ? ['mt-5 border-t border-border pt-4', props.collapsed ? 'lg:mt-3 lg:pt-3' : ''] : ''"
      >
        <h2
          class="px-3 pb-2 text-xs font-semibold tracking-caps text-text-subtle uppercase"
          :class="props.collapsed ? 'lg:sr-only' : ''"
        >
          {{ t(section.labelKey) }}
        </h2>
        <ul class="space-y-0.5">
          <li v-for="item in section.items" :key="item.to">
            <NuxtLink
              :to="localePath(item.to)"
              class="group relative flex min-h-(--target-min) items-center gap-3 rounded-md py-2 pe-2 ps-3 text-sm no-underline transition-colors duration-(--duration-fast)"
              :class="[
                isCurrent(item.to)
                  ? 'bg-accent-surface font-semibold text-accent'
                  : 'text-text-secondary hover:bg-surface-hover hover:text-text',
                props.collapsed ? 'lg:justify-center lg:px-0' : '',
              ]"
              :title="props.collapsed ? t(item.labelKey) : undefined"
              :aria-current="isCurrent(item.to) ? 'page' : undefined"
            >
              <span
                v-if="isCurrent(item.to)"
                class="absolute inset-y-1.5 start-0 w-[3px] rounded-e-full bg-accent-solid"
                aria-hidden="true"
              />
              <UiIcon
                v-if="item.icon"
                :name="item.icon"
                size="1.05rem"
                class="shrink-0 transition-colors duration-(--duration-fast)"
                :class="isCurrent(item.to) ? 'text-accent' : 'text-text-subtle group-hover:text-text'"
              />
              <span
                class="min-w-0 flex-1 truncate"
                :class="props.collapsed ? 'lg:sr-only' : ''"
              >{{ t(item.labelKey) }}</span>
              <!-- Compteur poussé à la marge intérieure droite, jamais collé au
                   libellé : c'est ce qui le rend comparable d'une entrée à
                   l'autre en descendant la colonne. Même pastille que les
                   onglets — 22 px, aplat accent. -->
              <UiCounter
                v-if="item.count !== undefined"
                :value="item.count"
                tone="accent"
                class="shrink-0"
                :class="props.collapsed ? 'lg:hidden' : ''"
              />
            </NuxtLink>
          </li>
        </ul>
      </div>
    </nav>

    <div v-if="$slots.footer" class="shrink-0 border-t border-border p-3">
      <slot name="footer" :collapsed="props.collapsed" />
    </div>
  </aside>
</template>
