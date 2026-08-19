<script setup lang="ts">
import type { ShowcaseListRow } from '~/types/admin-showcase'
import type { HighlightStatus } from '~/types/content'
import type { MenuItem, TableColumn } from '~/types/ui'

/**
 * LE TABLEAU DE LA VITRINE — et, avant tout, SON ORDRE.
 *
 * L'ORDRE EST LA FONCTION PRINCIPALE DE CET ÉCRAN. Son absence était le défaut
 * n° 6 de la v1 : le carrousel suivait `created_at DESC`, et l'IFDD ne décidait
 * pas ce qui passe en premier. La première colonne n'est donc pas une
 * décoration — c'est ce pour quoi on ouvre cette page.
 *
 * BOUTONS MONTER / DESCENDRE, PAS UN GLISSER-DÉPOSER SEUL. Deux cibles de 44 px,
 * dans le parcours de tabulation, avec un nom accessible qui NOMME la
 * diapositive (« Monter : Témoignage de Mariam Diallo ») et non « Monter ». Un
 * réordonnancement qui n'existe qu'à la souris exclut la moitié des façons de
 * travailler, et se perd au premier écran tactile étroit.
 *
 * LE FOCUS SURVIT AU DÉPLACEMENT, et ce n'est pas un raffinement : sans lui,
 * remonter une diapositive de quatre rangs au clavier demande de retrouver le
 * bouton après chaque pression. La ligne garde son élément (le tableau est
 * clé par identifiant) ; il ne reste qu'à basculer sur le bouton opposé quand
 * celui qu'on vient d'actionner arrive en butée et se désactive.
 *
 * LE TABLEAU NE TRIE RIEN. `sort_order` est l'ordre de défilement du bandeau
 * public : le voir autrement ici que sur l'accueil rendrait les deux boutons
 * incompréhensibles. Aucune colonne n'est donc `sortable`.
 *
 * LES EXTRÉMITÉS SONT DONNÉES, PAS RECALCULÉES. `is_first` / `is_last` viennent
 * de l'API, où l'ordre est connu pour l'emplacement ENTIER.
 *
 * LA PERMISSION SE TESTE LIGNE PAR LIGNE, et c'est la règle métier n° 8 prise
 * par le bon bout : `content.highlight.manage` s'accorde sur une ÉDITION, et une
 * diapositive de plateforme (`event_id` nul) demande la portée globale. D'où une
 * FONCTION en propriété plutôt qu'un booléen — un seul drapeau pour tout le
 * tableau offrirait des actions qu'un compte détaché n'a pas.
 *
 * LA VIGNETTE SUIT LE MÊME REPLI QUE LE RAIL PUBLIC : `thumbnail`, à défaut
 * `background_image`, à défaut l'aplat `background_color_hex`. Une case vide
 * dans la liste alors que le bandeau affiche une couleur ferait croire à un
 * média manquant.
 */

interface Props {
  rows: ShowcaseListRow[]
  /** Nom du tableau, annoncé par les lecteurs d'écran. */
  caption: string
  /** `content.highlight.manage`, SUR L'ÉDITION de la ligne — voir l'en-tête. */
  canManage: (row: ShowcaseListRow) => boolean
  loading?: boolean
  /** Ligne dont l'écriture est en cours : ses actions sont neutralisées. */
  busyId?: string | null
}

const props = defineProps<Props>()

const emit = defineEmits<{
  move: [row: ShowcaseListRow, direction: 'up' | 'down']
  edit: [row: ShowcaseListRow]
  duplicate: [row: ShowcaseListRow]
  status: [row: ShowcaseListRow, status: HighlightStatus]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

/**
 * Le fuseau d'affichage de la fenêtre de diffusion.
 *
 * UTC, et c'est un choix : une diapositive n'appartient pas à un lieu. Une
 * fenêtre qui s'ouvre le 1er septembre le fait pour toute la plateforme, et
 * l'afficher dans le fuseau de l'édition ferait lire deux dates différentes à
 * deux administrateurs regardant la même ligne.
 */
const WINDOW_TZ = 'UTC'

const columns = computed<TableColumn[]>(() => [
  { key: 'order', label: t('admin.showcase.list.columns.order'), width: '8rem' },
  { key: 'slide', label: t('admin.showcase.list.columns.slide') },
  { key: 'attribution', label: t('admin.showcase.list.columns.attribution'), hideBelow: 'lg' },
  { key: 'event', label: t('admin.showcase.list.columns.event'), hideBelow: 'xl' },
  { key: 'state', label: t('admin.showcase.list.columns.state'), width: '9rem' },
  { key: 'window', label: t('admin.showcase.list.columns.window'), hideBelow: 'xl', width: '11rem' },
  { key: 'actions', label: t('admin.showcase.list.columns.actions'), align: 'end', width: '4rem' },
])

const titleOf = (row: ShowcaseListRow): string =>
  tr(row.title).trim() || t('admin.showcase.list.row.untitled')

/** La vignette du rail, ou son repli — voir l'en-tête. */
const previewImage = (row: ShowcaseListRow) => row.thumbnail ?? row.background_image

/** L'attribution telle qu'elle sortira : la personne prime sur le nom libre. */
function attributionOf(row: ShowcaseListRow): string[] {
  const parts: string[] = []
  if (row.author_name) parts.push(row.author_name)
  if (row.organization_name) {
    parts.push(
      row.organization_acronym
        ? `${row.organization_name} (${row.organization_acronym})`
        : row.organization_name,
    )
  }
  return parts
}

/** Les bornes de la fenêtre, chacune facultative. `null` des deux côtés = sans limite. */
function windowOf(row: ShowcaseListRow): string {
  if (row.starts_at === null && row.ends_at === null) {
    return t('admin.showcase.list.row.window.always')
  }
  if (row.starts_at !== null && row.ends_at !== null) {
    return t('admin.showcase.list.row.window.between', {
      from: date(row.starts_at, WINDOW_TZ),
      to: date(row.ends_at, WINDOW_TZ),
    })
  }
  if (row.starts_at !== null) {
    return t('admin.showcase.list.row.window.from', { from: date(row.starts_at, WINDOW_TZ) })
  }
  return t('admin.showcase.list.row.window.until', { to: date(row.ends_at, WINDOW_TZ) })
}

const isBusy = (row: ShowcaseListRow): boolean => props.busyId === row.id

// ---------------------------------------------------------------------------
// L'ordre — les deux boutons et la survie du focus
// ---------------------------------------------------------------------------

const buttons = new Map<string, HTMLButtonElement>()
const buttonKey = (id: string, direction: 'up' | 'down'): string => `${id}:${direction}`

function setButton(id: string, direction: 'up' | 'down', element: Element | null): void {
  const key = buttonKey(id, direction)
  if (element instanceof HTMLButtonElement) buttons.set(key, element)
  else buttons.delete(key)
}

/** La ligne qu'on vient de déplacer, et dans quel sens : le focus la suit. */
const pendingFocus = ref<{ id: string; direction: 'up' | 'down' } | null>(null)

function move(row: ShowcaseListRow, direction: 'up' | 'down'): void {
  pendingFocus.value = { id: row.id, direction }
  emit('move', row, direction)
}

/**
 * Après le déplacement, le focus revient sur le bouton actionné — ou sur son
 * opposé si la ligne est arrivée en butée et que celui-ci vient de se
 * désactiver. Un focus perdu sur `<body>` renvoie au début de la page, et le
 * réordonnancement au clavier devient impraticable au deuxième cran.
 */
watch(
  () => props.rows,
  async () => {
    const target = pendingFocus.value
    if (!target) return
    pendingFocus.value = null
    await nextTick()
    const row = props.rows.find((entry) => entry.id === target.id)
    if (!row) return
    const stillEnabled = target.direction === 'up' ? !row.is_first : !row.is_last
    const direction = stillEnabled ? target.direction : target.direction === 'up' ? 'down' : 'up'
    buttons.get(buttonKey(row.id, direction))?.focus()
  },
)

/**
 * Le menu d'actions d'une ligne.
 *
 * AUCUNE SUPPRESSION, et ce n'est pas un oubli : `content.highlight_status` vaut
 * `draft | published | archived`, le modèle n'offre pas d'effacement. Retirer de
 * la vitrine, c'est archiver — le témoignage de la COP30 se remet en avant à la
 * COP31, et l'historique de ce qui a été montré ne disparaît pas.
 */
function actionsOf(row: ShowcaseListRow): MenuItem[] {
  const items: MenuItem[] = [
    { value: 'edit', label: t('admin.showcase.list.actions.edit'), icon: 'edit' },
    { value: 'duplicate', label: t('admin.showcase.list.actions.duplicate'), icon: 'copy' },
  ]

  const diffusion: MenuItem[] = []
  if (row.status !== 'published') {
    diffusion.push({
      value: 'published',
      label: t('admin.showcase.list.actions.publish'),
      icon: 'eye',
    })
  }
  if (row.status === 'published') {
    diffusion.push({
      value: 'draft',
      label: t('admin.showcase.list.actions.unpublish'),
      icon: 'eye-off',
    })
  }
  if (row.status === 'archived') {
    diffusion.push({
      value: 'draft',
      label: t('admin.showcase.list.actions.restore'),
      icon: 'refresh',
    })
  } else {
    diffusion.push({
      value: 'archived',
      label: t('admin.showcase.list.actions.archive'),
      icon: 'ban',
    })
  }

  const first = diffusion[0]
  if (first) {
    first.separatorBefore = true
    first.groupLabel = t('admin.showcase.list.actions.group')
  }

  return [...items, ...diffusion]
}

function onSelect(row: ShowcaseListRow, value: string): void {
  if (value === 'edit') return emit('edit', row)
  if (value === 'duplicate') return emit('duplicate', row)
  emit('status', row, value as HighlightStatus)
}

/**
 * Deux boutons de 44 px, désactivés aux extrémités et pendant l'écriture.
 * `min-w` autant que `min-h` : une cible haute mais étroite reste manquée.
 */
const ORDER_BUTTON =
  'inline-flex min-h-(--target-min) min-w-(--target-min) cursor-pointer items-center ' +
  'justify-center rounded-md border border-border text-text-muted transition-colors ' +
  'duration-(--duration-fast) hover:bg-surface-hover hover:text-text ' +
  'disabled:cursor-not-allowed disabled:opacity-[.45] disabled:hover:bg-transparent'
</script>

<template>
  <UiTable
    :columns="columns"
    :rows="props.rows"
    row-key="id"
    :caption="props.caption"
    :loading="props.loading"
    :hoverable="false"
    dense
  >
    <template #cell-order="{ row }">
      <div class="flex items-center gap-1">
        <!-- Le rang est ANNONCÉ, pas seulement dessiné : c'est l'information que
             les deux boutons modifient, et elle doit rester lisible à la voix. -->
        <span class="w-5 shrink-0 text-center font-mono text-xs text-text-subtle">
          {{ props.rows.indexOf(row) + 1 }}
        </span>

        <button
          :ref="(element) => setButton(row.id, 'up', element as Element | null)"
          type="button"
          :class="ORDER_BUTTON"
          :disabled="!props.canManage(row) || row.is_first || isBusy(row)"
          @click="move(row, 'up')"
        >
          <span class="sr-only">
            {{ t('admin.showcase.list.order.up', { title: titleOf(row) }) }}
          </span>
          <UiIcon name="chevron-up" size="1.1rem" aria-hidden="true" />
        </button>

        <button
          :ref="(element) => setButton(row.id, 'down', element as Element | null)"
          type="button"
          :class="ORDER_BUTTON"
          :disabled="!props.canManage(row) || row.is_last || isBusy(row)"
          @click="move(row, 'down')"
        >
          <span class="sr-only">
            {{ t('admin.showcase.list.order.down', { title: titleOf(row) }) }}
          </span>
          <UiIcon name="chevron-down" size="1.1rem" aria-hidden="true" />
        </button>
      </div>
    </template>

    <template #cell-slide="{ row }">
      <div class="flex min-w-0 items-start gap-3">
        <!-- Repli identique à celui du rail public : vignette, image de fond,
             aplat. Le cadre garde la même taille dans les trois cas. -->
        <UiImage
          v-if="previewImage(row)"
          :image="previewImage(row)"
          ratio="16 / 9"
          rounded="rounded-md"
          frame-class="w-20 shrink-0"
        />
        <span
          v-else
          class="block h-[2.8125rem] w-20 shrink-0 rounded-md border border-border"
          :style="row.background_color_hex ? { backgroundColor: row.background_color_hex } : undefined"
          :class="row.background_color_hex ? '' : 'bg-surface-sunken'"
          aria-hidden="true"
        />

        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <!-- REPLI SUR UN LIBELLÉ D'INTERFACE, JAMAIS SUR LE CODE. Le terme
                 peut avoir été désactivé au back-office — `v_showcase` joint sur
                 `is_active`, et `nature_label` sort alors nul. Afficher
                 « negotiator_voice » donnerait un identifiant technique à lire
                 dans un tableau, ce que la règle d'usage n° 5 interdit. Ici le
                 repli EST une chaîne d'interface : il ne nomme pas la nature,
                 il dit qu'elle n'existe plus. -->
            <UiBadge size="sm" :dot-color="row.nature_color">
              {{ tr(row.nature_label) || t('admin.showcase.list.row.natureWithdrawn') }}
            </UiBadge>
            <!-- La vidéo change ce que le public voit : elle se dit dans la
                 liste, sans quoi seule l'ouverture du formulaire l'apprend. -->
            <UiBadge v-if="row.has_video" size="sm" intent="info" icon="video">
              {{ t('admin.showcase.list.row.hasVideo') }}
            </UiBadge>
          </div>

          <p class="mt-1 font-semibold text-text">
            <button
              type="button"
              class="cursor-pointer text-left hover:text-accent hover:underline"
              @click="emit('edit', row)"
            >
              {{ titleOf(row) }}
            </button>
          </p>

          <p v-if="row.session_title" class="mt-0.5 truncate text-xs text-text-subtle">
            {{ t('admin.showcase.list.row.session', { session: tr(row.session_title) }) }}
          </p>
        </div>
      </div>
    </template>

    <template #cell-attribution="{ row }">
      <div v-if="attributionOf(row).length" class="text-text-secondary">
        <span v-for="(part, index) in attributionOf(row)" :key="index" class="block truncate">
          {{ part }}
        </span>
        <span v-if="row.country_name" class="block truncate text-xs text-text-subtle">
          {{ tr(row.country_name) }}
        </span>
      </div>
      <span v-else class="text-text-subtle">{{ t('common.labels.none') }}</span>
    </template>

    <template #cell-event="{ row }">
      <!-- `event_id` nul n'est pas « pas d'édition » : c'est un contenu de
           PLATEFORME, visible sur l'accueil quelle que soit la COP en cours, et
           réservé à la portée globale. Le dire en toutes lettres. -->
      <span
        v-if="row.event_id === null"
        class="inline-flex items-center gap-1.5 text-text-secondary"
      >
        <UiIcon name="globe" size="0.95rem" aria-hidden="true" />
        {{ t('admin.showcase.list.row.platform') }}
      </span>
      <span v-else class="truncate text-text-secondary">{{ tr(row.event_title) }}</span>
    </template>

    <template #cell-state="{ row }">
      <AdminShowcaseStateBadge
        :state="row.broadcast_state"
        :label="t(`admin.showcase.list.state.${row.broadcast_state}`)"
        size="sm"
      />
    </template>

    <template #cell-window="{ row }">
      <span class="text-xs text-text-secondary">{{ windowOf(row) }}</span>
    </template>

    <template #cell-actions="{ row }">
      <div class="flex justify-end">
        <UiContextMenu
          :items="actionsOf(row)"
          :label="t('admin.showcase.list.actions.menu', { title: titleOf(row) })"
          :disabled="!props.canManage(row) || isBusy(row)"
          @select="(value) => onSelect(row, value)"
        />
      </div>
    </template>

    <template #empty>
      <slot name="empty" />
    </template>
  </UiTable>
</template>
