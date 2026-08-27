<script setup lang="ts">
/**
 * ÉDITEUR DE RECADRAGE — on choisit ce que l'image montre, avant de la déposer.
 *
 * ── POURQUOI RECADRER ICI PLUTÔT QUE ROGNER AILLEURS ────────────────────────
 *
 * `media.attachable_roles.expected_aspect_ratio` exige une forme, et le trigger
 * de `media.attachments` refuse ce qui s'en écarte de plus de 2 %. Sans cet
 * écran, la forme s'apprenait PAR LE REFUS — après que le fichier a traversé le
 * réseau, et sans qu'on sache quoi recouper. La poignée qu'on tire est donc
 * VERROUILLÉE sur le rapport attendu : ce qui sort d'ici est recevable par
 * construction, et le refus de la base redevient ce qu'il doit être — un
 * filet, pas un mode d'emploi.
 *
 * ── ET SURTOUT PAS UN ROGNAGE AUTOMATIQUE ───────────────────────────────────
 *
 * Un 32:9 découpé au centre d'une photographie de conférence décapite les
 * intervenants ; un carré tiré du même fichier ne garde qu'une épaule. C'est la
 * personne qui sait où est le sujet. L'écran lui donne la grille des tiers et se
 * tait.
 *
 * ── TROIS DÉCLINAISONS ? NON. LE WORKER S'EN CHARGE ─────────────────────────
 *
 * Un seul fichier part. Les tailles `lg`, `md` et `thumb` sont produites par le
 * module Média (`domain/variants.rs`) et servies par `sources`. En fabriquer ici
 * serait écrire une seconde fois un invariant que la plateforme porte déjà — et
 * trois objets déposés pour un seul rôle, dont deux orphelins le jour même.
 *
 * ── LE POIDS EST MESURÉ, JAMAIS ESTIMÉ ──────────────────────────────────────
 *
 * Le fichier annoncé sous l'aperçu EST celui qui partira : chaque réglage
 * réencode l'image et garde le résultat. Une estimation calculée autrement
 * finirait par diverger du fichier réel, et le plafond du rôle serait franchi au
 * dépôt alors que l'écran annonçait le contraire.
 *
 * ── LA COULEUR VIENT DES JETONS, MÊME DANS UN CANEVAS ───────────────────────
 *
 * Un canevas ne connaît pas les variables CSS : elles sont lues une fois au
 * montage sur un élément de la page. Écrire `#00A1E4` ici rouvrirait la porte
 * que `design-tokens.css` a fermée, et le thème sombre ne suivrait pas.
 */

const props = withDefaults(
  defineProps<{
    /** Le fichier choisi sur le disque. Ce n'est PAS ce qui partira. */
    file: File
    title: string
    description?: string
    /**
     * Largeur ÷ hauteur imposée à la sélection. `null` : recadrage libre.
     * Vient de `media.attachable_roles`, jamais d'une constante d'écran.
     */
    aspectRatio?: number | null
    /**
     * La même forme, ÉCRITE POUR L'ŒIL — « 32:9 ». « 3,56 » est le quotient que
     * la base compare ; il ne dit rien à qui cadre une photographie.
     */
    ratioLabel?: string
    /** Plafond du rôle, en octets. Le bouton refuse au-delà. */
    maxByteSize?: number | null
    /** L'appelant travaille — dépôt en cours. */
    busy?: boolean
    /** Le bouton d'envoi reste fermé tant que l'appelant n'a pas ce qu'il faut. */
    applyDisabled?: boolean
    /**
     * POURQUOI le bouton est fermé, dit à côté de lui.
     *
     * Un bouton grisé sans raison visible est une impasse : le champ qui manque
     * peut être hors de vue — la colonne de réglages dépasse la hauteur de la
     * boîte de dialogue —, et rien ne laisse deviner ce qu'on attend.
     */
    applyHint?: string
    applyLabel?: string
  }>(),
  { aspectRatio: null, maxByteSize: null },
)

const emit = defineEmits<{
  apply: [result: { blob: Blob; width: number; height: number; mimeType: string; filename: string }]
  cancel: []
}>()

const { t, locale } = useI18n()

type Format = 'jpeg' | 'webp' | 'png'
const FORMATS: Format[] = ['jpeg', 'webp', 'png']

/** Le plus petit côté de sélection, en pixels d'écran. Sous cela, on ne vise plus. */
const MIN_DISPLAY_SIDE = 40
/** Rayon de préhension d'une poignée. Généreux : c'est aussi une cible tactile. */
const GRAB_RADIUS = 16
const HANDLE_RADIUS = 7
/** Largeur de sortie proposée par défaut. Au-delà, le poids ne paie plus. */
const DEFAULT_OUTPUT_WIDTH = 2000
const MIN_OUTPUT_WIDTH = 320

// ---------------------------------------------------------------------------
// État
// ---------------------------------------------------------------------------

const frame = ref<HTMLElement | null>(null)
const canvas = ref<HTMLCanvasElement | null>(null)

const image = ref<HTMLImageElement | null>(null)
const loadFailed = ref(false)
/**
 * L'ENCODAGE A ÉCHOUÉ — et il faut le DIRE.
 *
 * Trois chemins y mènent, tous silencieux jusqu'ici : un contexte de canevas
 * refusé, un `toBlob` qui rend `null` (format non gravé dans le navigateur), et
 * un `drawImage` qui lève sur une sélection dégénérée. Dans les trois cas le
 * fichier n'existait pas, le bouton restait fermé, et **rien à l'écran ne
 * l'expliquait** — l'impasse exacte que ce composant doit éviter.
 */
const encodeFailed = ref(false)

/** La sélection, EN PIXELS DE L'IMAGE et non de l'écran. */
const crop = reactive({ x: 0, y: 0, w: 0, h: 0 })

/** Taille du cadre de dessin, en pixels CSS. Tenue par l'observateur de taille. */
const view = reactive({ w: 0, h: 0 })

const format = ref<Format>('jpeg')
const quality = ref(82)
const outputWidth = ref(DEFAULT_OUTPUT_WIDTH)

const output = ref<{ blob: Blob; width: number; height: number } | null>(null)
const outputUrl = ref<string>('')
const encoding = ref(false)

const cursor = ref('default')

const sourceUrl = ref('')

// ---------------------------------------------------------------------------
// Chargement du fichier
// ---------------------------------------------------------------------------

onMounted(() => {
  sourceUrl.value = URL.createObjectURL(props.file)
  const element = new Image()
  element.onload = () => {
    // SANS DIMENSIONS, IL N'Y A RIEN À RECADRER. Un SVG qui ne porte qu'un
    // `viewBox` se charge sans erreur et mesure zéro : la sélection serait vide
    // et `drawImage` lèverait, loin d'ici et sans rien dire.
    if (element.naturalWidth === 0 || element.naturalHeight === 0) {
      loadFailed.value = true
      return
    }
    image.value = element
    initCrop()
    outputWidth.value = Math.min(DEFAULT_OUTPUT_WIDTH, Math.round(crop.w))
    draw()
    scheduleEncode()
  }
  element.onerror = () => (loadFailed.value = true)
  element.src = sourceUrl.value
})

onBeforeUnmount(() => {
  if (sourceUrl.value) URL.revokeObjectURL(sourceUrl.value)
  if (outputUrl.value) URL.revokeObjectURL(outputUrl.value)
  if (encodeTimer) clearTimeout(encodeTimer)
})

/**
 * L'OBSERVATEUR DE TAILLE PLUTÔT QU'UNE MESURE AU MONTAGE.
 *
 * Le cadre vit dans une boîte de dialogue : au montage du composant, elle vient
 * tout juste de s'ouvrir et sa largeur peut encore valoir zéro. Une mesure unique
 * dessinerait alors dans le vide, sans rien signaler. L'observateur redessine
 * aussi quand on tourne le téléphone, ce qu'aucune mesure au montage ne fait.
 */
let observer: ResizeObserver | null = null
onMounted(() => {
  if (!frame.value) return
  observer = new ResizeObserver((entries) => {
    const box = entries[0]?.contentRect
    if (!box || box.width === 0) return
    view.w = box.width
    view.h = box.height
    draw()
  })
  observer.observe(frame.value)
})
onBeforeUnmount(() => observer?.disconnect())

// ---------------------------------------------------------------------------
// Géométrie : image ↔ écran
// ---------------------------------------------------------------------------

/** Facteur d'affichage : l'image entière tient dans le cadre, sans être agrandie. */
const scale = computed(() => {
  const source = image.value
  if (!source || view.w === 0 || view.h === 0) return 1
  return Math.min(view.w / source.width, view.h / source.height)
})

const offset = computed(() => {
  const source = image.value
  if (!source) return { x: 0, y: 0 }
  return {
    x: (view.w - source.width * scale.value) / 2,
    y: (view.h - source.height * scale.value) / 2,
  }
})

const toScreen = (x: number, y: number) => ({
  x: offset.value.x + x * scale.value,
  y: offset.value.y + y * scale.value,
})

const toImage = (x: number, y: number) => ({
  x: (x - offset.value.x) / scale.value,
  y: (y - offset.value.y) / scale.value,
})

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max)

/** La plus grande sélection au bon rapport, centrée. C'est le point de départ. */
function initCrop(): void {
  const source = image.value
  if (!source) return
  const ratio = props.aspectRatio
  if (!ratio) {
    Object.assign(crop, { x: 0, y: 0, w: source.width, h: source.height })
    return
  }
  const width = Math.min(source.width, source.height * ratio)
  const height = width / ratio
  Object.assign(crop, {
    x: (source.width - width) / 2,
    y: (source.height - height) / 2,
    w: width,
    h: height,
  })
}

// ---------------------------------------------------------------------------
// Dessin
// ---------------------------------------------------------------------------

interface Palette {
  ground: string
  accent: string
  handle: string
  scrim: string
}

let palette: Palette | null = null

/** Les jetons, lus UNE FOIS : `getComputedStyle` à chaque image saccaderait le glissé. */
function readPalette(): Palette {
  const style = getComputedStyle(document.documentElement)
  const read = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback
  return {
    ground: read('--color-surface-sunken', '#e9edf1'),
    accent: read('--color-accent-solid', '#0081b5'),
    handle: read('--color-accent-contrast', '#ffffff'),
    scrim: read('--color-scrim', '#000000'),
  }
}

function draw(): void {
  const element = canvas.value
  const source = image.value
  if (!element || !source || view.w === 0) return

  palette ??= readPalette()
  const ratio = window.devicePixelRatio || 1
  element.width = Math.round(view.w * ratio)
  element.height = Math.round(view.h * ratio)

  const ctx = element.getContext('2d')
  if (!ctx) return
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0)

  const dw = source.width * scale.value
  const dh = source.height * scale.value
  const { x: ox, y: oy } = offset.value

  ctx.fillStyle = palette.ground
  ctx.fillRect(0, 0, view.w, view.h)
  ctx.drawImage(source, ox, oy, dw, dh)

  // LE VOILE PORTE LE CONTRASTE, pas le flou : ce qui est hors sélection doit se
  // deviner — on recadre en regardant ce qu'on laisse.
  ctx.globalAlpha = 0.55
  ctx.fillStyle = palette.scrim
  ctx.fillRect(0, 0, view.w, view.h)
  ctx.globalAlpha = 1

  const a = toScreen(crop.x, crop.y)
  const b = toScreen(crop.x + crop.w, crop.y + crop.h)
  const cw = b.x - a.x
  const ch = b.y - a.y

  ctx.save()
  ctx.beginPath()
  ctx.rect(a.x, a.y, cw, ch)
  ctx.clip()
  ctx.drawImage(source, ox, oy, dw, dh)
  ctx.restore()

  // La grille des tiers — la seule aide au cadrage que l'écran se permette.
  ctx.strokeStyle = palette.handle
  ctx.globalAlpha = 0.4
  ctx.lineWidth = 1
  for (let i = 1; i < 3; i += 1) {
    ctx.beginPath()
    ctx.moveTo(a.x + (cw * i) / 3, a.y)
    ctx.lineTo(a.x + (cw * i) / 3, b.y)
    ctx.moveTo(a.x, a.y + (ch * i) / 3)
    ctx.lineTo(b.x, a.y + (ch * i) / 3)
    ctx.stroke()
  }
  ctx.globalAlpha = 1

  ctx.strokeStyle = palette.accent
  ctx.lineWidth = 2
  ctx.strokeRect(a.x, a.y, cw, ch)

  ctx.fillStyle = palette.handle
  for (const point of handlePoints()) {
    ctx.beginPath()
    ctx.arc(point.x, point.y, HANDLE_RADIUS, 0, Math.PI * 2)
    ctx.fill()
    ctx.stroke()
  }
}

type Handle = 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'w' | 'e'

const HANDLES: Handle[] = ['nw', 'ne', 'sw', 'se', 'n', 's', 'w', 'e']

const CURSORS: Record<Handle, string> = {
  nw: 'nwse-resize',
  se: 'nwse-resize',
  ne: 'nesw-resize',
  sw: 'nesw-resize',
  n: 'ns-resize',
  s: 'ns-resize',
  w: 'ew-resize',
  e: 'ew-resize',
}

/** La position d'une poignée, en pixels d'image. */
function handleAt(handle: Handle): { x: number; y: number } {
  const midX = crop.x + crop.w / 2
  const midY = crop.y + crop.h / 2
  const x = handle.includes('w') ? crop.x : handle.includes('e') ? crop.x + crop.w : midX
  const y = handle.includes('n') ? crop.y : handle.includes('s') ? crop.y + crop.h : midY
  return { x, y }
}

function handlePoints(): { x: number; y: number }[] {
  return HANDLES.map((handle) => {
    const point = handleAt(handle)
    return toScreen(point.x, point.y)
  })
}

watch(() => [crop.x, crop.y, crop.w, crop.h, props.aspectRatio], draw)

// ---------------------------------------------------------------------------
// Manipulation
// ---------------------------------------------------------------------------

type Mode = { kind: 'move' } | { kind: 'resize'; handle: Handle } | null

let mode: Mode = null
let startPointer = { x: 0, y: 0 }
let startCrop = { x: 0, y: 0, w: 0, h: 0 }

/** Que vise-t-on à cet endroit ? Les poignées d'abord, le centre ensuite. */
function targetAt(x: number, y: number): Handle | 'move' | null {
  for (const handle of HANDLES) {
    const point = handleAt(handle)
    const screen = toScreen(point.x, point.y)
    if (Math.hypot(x - screen.x, y - screen.y) <= GRAB_RADIUS) return handle
  }
  const a = toScreen(crop.x, crop.y)
  const b = toScreen(crop.x + crop.w, crop.y + crop.h)
  if (x >= a.x && x <= b.x && y >= a.y && y <= b.y) return 'move'
  return null
}

function pointerPosition(event: PointerEvent): { x: number; y: number } {
  const element = canvas.value
  if (!element) return { x: 0, y: 0 }
  const box = element.getBoundingClientRect()
  return { x: event.clientX - box.left, y: event.clientY - box.top }
}

function onPointerDown(event: PointerEvent): void {
  if (!image.value) return
  const position = pointerPosition(event)
  const target = targetAt(position.x, position.y)
  if (!target) return
  mode = target === 'move' ? { kind: 'move' } : { kind: 'resize', handle: target }
  startPointer = position
  startCrop = { ...crop }
  ;(event.target as HTMLElement).setPointerCapture(event.pointerId)
  event.preventDefault()
}

function onPointerMove(event: PointerEvent): void {
  const source = image.value
  if (!source) return
  const position = pointerPosition(event)

  if (!mode) {
    const target = targetAt(position.x, position.y)
    cursor.value = target === 'move' ? 'move' : target ? CURSORS[target] : 'default'
    return
  }

  const dx = (position.x - startPointer.x) / scale.value
  const dy = (position.y - startPointer.y) / scale.value

  if (mode.kind === 'move') {
    crop.x = clamp(startCrop.x + dx, 0, source.width - startCrop.w)
    crop.y = clamp(startCrop.y + dy, 0, source.height - startCrop.h)
    return
  }

  resize(mode.handle, toImage(position.x, position.y))
}

function onPointerUp(event: PointerEvent): void {
  if (!mode) return
  mode = null
  ;(event.target as HTMLElement).releasePointerCapture?.(event.pointerId)
  scheduleEncode()
}

/**
 * Le redimensionnement, en trois temps : le bord tiré, le rapport imposé, puis
 * le retour dans l'image. L'ordre compte — imposer le rapport après le
 * recadrage dans les bords le romprait, et c'est lui que la base vérifie.
 */
function resize(handle: Handle, pointer: { x: number; y: number }): void {
  const source = image.value
  if (!source) return

  const min = MIN_DISPLAY_SIDE / scale.value
  let left = startCrop.x
  let top = startCrop.y
  let right = startCrop.x + startCrop.w
  let bottom = startCrop.y + startCrop.h

  if (handle.includes('w')) left = clamp(pointer.x, 0, right - min)
  if (handle.includes('e')) right = clamp(pointer.x, left + min, source.width)
  if (handle.includes('n')) top = clamp(pointer.y, 0, bottom - min)
  if (handle.includes('s')) bottom = clamp(pointer.y, top + min, source.height)

  const ratio = props.aspectRatio
  if (ratio) {
    let width = right - left
    let height = bottom - top
    if (handle === 'n' || handle === 's') {
      width = height * ratio
      const centerX = (left + right) / 2
      left = centerX - width / 2
      right = centerX + width / 2
    } else if (handle === 'e' || handle === 'w') {
      height = width / ratio
      const centerY = (top + bottom) / 2
      top = centerY - height / 2
      bottom = centerY + height / 2
    } else {
      if (width / height > ratio) width = height * ratio
      else height = width / ratio
      if (handle.includes('w')) left = right - width
      else right = left + width
      if (handle.includes('n')) top = bottom - height
      else bottom = top + height
    }

    // Le rapport tenu, la sélection peut déborder : on la RÉTRÉCIT autour de son
    // ancre plutôt que de la couper — couper romprait le rapport.
    const availableW = handle.includes('w') ? right : handle.includes('e') ? source.width - left : source.width
    const availableH = handle.includes('n') ? bottom : handle.includes('s') ? source.height - top : source.height
    const shrink = Math.min(1, availableW / (right - left), availableH / (bottom - top))
    if (shrink < 1) {
      const width2 = (right - left) * shrink
      const height2 = (bottom - top) * shrink
      if (handle.includes('w')) left = right - width2
      else if (handle.includes('e')) right = left + width2
      else {
        const centerX = (left + right) / 2
        left = centerX - width2 / 2
        right = centerX + width2 / 2
      }
      if (handle.includes('n')) top = bottom - height2
      else if (handle.includes('s')) bottom = top + height2
      else {
        const centerY = (top + bottom) / 2
        top = centerY - height2 / 2
        bottom = centerY + height2 / 2
      }
    }
  }

  const width = right - left
  const height = bottom - top
  crop.x = clamp(left, 0, Math.max(0, source.width - width))
  crop.y = clamp(top, 0, Math.max(0, source.height - height))
  crop.w = Math.min(width, source.width)
  crop.h = Math.min(height, source.height)
}

/**
 * LE CLAVIER DÉPLACE ET REDIMENSIONNE AUSSI.
 *
 * Un recadrage qui n'existe qu'à la souris n'existe pas pour tout le monde : les
 * flèches déplacent la sélection, `Maj` la redimensionne par le coin bas-droit.
 */
function onKeyDown(event: KeyboardEvent): void {
  const source = image.value
  if (!source) return
  const step = (event.altKey ? 1 : 10) / scale.value
  const moves: Record<string, [number, number]> = {
    ArrowLeft: [-step, 0],
    ArrowRight: [step, 0],
    ArrowUp: [0, -step],
    ArrowDown: [0, step],
  }
  const move = moves[event.key]
  if (!move) return
  event.preventDefault()

  if (event.shiftKey) {
    startCrop = { ...crop }
    resize('se', { x: crop.x + crop.w + move[0], y: crop.y + crop.h + move[1] })
  } else {
    crop.x = clamp(crop.x + move[0], 0, source.width - crop.w)
    crop.y = clamp(crop.y + move[1], 0, source.height - crop.h)
  }
  scheduleEncode()
}

// ---------------------------------------------------------------------------
// Encodage
// ---------------------------------------------------------------------------

const maxOutputWidth = computed(() => Math.max(MIN_OUTPUT_WIDTH, Math.round(crop.w)))
const minOutputWidth = computed(() => Math.min(MIN_OUTPUT_WIDTH, maxOutputWidth.value))

const mimeOf: Record<Format, string> = {
  jpeg: 'image/jpeg',
  webp: 'image/webp',
  png: 'image/png',
}

let encodeTimer: ReturnType<typeof setTimeout> | null = null

/**
 * L'ENCODAGE EST DIFFÉRÉ, ET C'EST CE QUI REND LE GLISSÉ FLUIDE. Réencoder à
 * chaque pixel parcouru bloquerait le fil principal une dizaine de fois par
 * seconde sur une photographie de vingt mégapixels.
 */
function scheduleEncode(): void {
  if (encodeTimer) clearTimeout(encodeTimer)
  encoding.value = true
  encodeTimer = setTimeout(() => void encode(), 160)
}

async function encode(): Promise<void> {
  const source = image.value
  encoding.value = false
  if (!source) return
  encodeFailed.value = false

  const width = Math.max(1, Math.round(Math.min(outputWidth.value, crop.w)))
  const height = Math.max(1, Math.round(width / (crop.w / crop.h)))

  const target = document.createElement('canvas')
  target.width = width
  target.height = height
  const ctx = target.getContext('2d')
  if (!ctx) {
    encodeFailed.value = true
    return
  }

  // LE FOND BLANC N'EST PAS DÉCORATIF : un PNG transparent aplati en JPEG donne
  // du NOIR là où il n'y avait rien, et le défaut ne se voit qu'après le dépôt.
  if (format.value !== 'png') {
    ctx.fillStyle = '#ffffff'
    ctx.fillRect(0, 0, width, height)
  }
  ctx.imageSmoothingQuality = 'high'

  // `drawImage` LÈVE sur une sélection de largeur ou de hauteur nulle, et la
  // gravure d'un format peut manquer au navigateur — `toBlob` rend alors `null`.
  // Les deux se disent, plutôt que de laisser le bouton fermé sans raison.
  let blob: Blob | null = null
  try {
    ctx.drawImage(source, crop.x, crop.y, crop.w, crop.h, 0, 0, width, height)
    blob = await new Promise<Blob | null>((resolve) => {
      target.toBlob(
        resolve,
        mimeOf[format.value],
        format.value === 'png' ? undefined : quality.value / 100,
      )
    })
  } catch {
    blob = null
  }
  if (!blob) {
    encodeFailed.value = true
    return
  }

  if (outputUrl.value) URL.revokeObjectURL(outputUrl.value)
  output.value = { blob, width, height }
  outputUrl.value = URL.createObjectURL(blob)
}

watch([format, quality, outputWidth], scheduleEncode)
watch(maxOutputWidth, (max) => {
  if (outputWidth.value > max) outputWidth.value = max
})

// ---------------------------------------------------------------------------
// Ce que l'écran annonce
// ---------------------------------------------------------------------------

const tooHeavy = computed(
  () => props.maxByteSize !== null && !!output.value && output.value.blob.size > props.maxByteSize,
)

const sizeLabel = computed(() =>
  output.value ? formatByteSize(output.value.blob.size, locale.value) : '—',
)

const shapeLine = computed(() => {
  if (!props.aspectRatio) return t('image-editor.ratio.free')
  const written =
    props.ratioLabel ??
    new Intl.NumberFormat(locale.value, { maximumFractionDigits: 2 }).format(props.aspectRatio)
  return t('image-editor.ratio.locked', { ratio: written })
})

/** Le nom du fichier suit son format : un `.png` encodé en JPEG serait un piège. */
const outputFilename = computed(() => {
  const base = props.file.name.replace(/\.[^.]+$/, '') || 'image'
  const extension = format.value === 'jpeg' ? 'jpg' : format.value
  return `${base}.${extension}`
})

/**
 * CE QUI RETIENT L'ENVOI, en une phrase — vide quand rien ne le retient.
 *
 * L'ordre suit la correction à faire : ce que l'appelant attend d'abord, puis le
 * poids, que l'on règle sur place. L'encodage en cours ne se dit pas — il dure
 * moins de deux dixièmes de seconde, et l'annoncer ferait clignoter le pied.
 */
const blockedBecause = computed<string | null>(() => {
  if (loadFailed.value) return t('image-editor.errors.unreadable')
  if (encodeFailed.value) return t('image-editor.errors.encoding')
  if (tooHeavy.value) return t('image-editor.weight.tooHeavy')
  if (props.applyDisabled && props.applyHint) return props.applyHint
  // Il n'y a pas encore de fichier : c'est passager, et le dire vaut mieux que
  // de laisser croire à un refus.
  if (!output.value) return t('image-editor.preparing')
  return null
})

function apply(): void {
  const result = output.value
  if (!result || tooHeavy.value) return
  emit('apply', {
    blob: result.blob,
    width: result.width,
    height: result.height,
    mimeType: mimeOf[format.value],
    filename: outputFilename.value,
  })
}
</script>

<template>
  <UiModal
    :open="true"
    :title="props.title"
    :description="props.description"
    size="xl"
    :dismissible="!props.busy"
    @update:open="emit('cancel')"
  >
    <UiAlert
      v-if="loadFailed"
      intent="danger"
      :message="t('image-editor.errors.unreadable')"
    />

    <div v-else class="flex flex-col gap-5 lg:flex-row">
      <!-- LE CADRE DE TRAVAIL. Sa hauteur est FIXE et ne suit pas la forme de
           l'image : une hauteur dérivée du fichier ferait sauter la boîte de
           dialogue d'un fichier à l'autre, et un panoramique la réduirait à une
           bande de quarante pixels. -->
      <div class="min-w-0 flex-1">
        <div
          ref="frame"
          class="h-[18rem] w-full overflow-hidden rounded-md border border-border sm:h-[22rem] lg:h-[26rem]"
        >
          <canvas
            ref="canvas"
            class="size-full touch-none outline-none focus-visible:ring-2 focus-visible:ring-accent"
            tabindex="0"
            role="application"
            :aria-label="t('image-editor.canvasLabel')"
            :style="{ cursor }"
            @pointerdown="onPointerDown"
            @pointermove="onPointerMove"
            @pointerup="onPointerUp"
            @pointercancel="onPointerUp"
            @keydown="onKeyDown"
          />
        </div>

        <p class="mt-2 text-xs text-text-subtle">{{ t('image-editor.hint') }}</p>
      </div>

      <!-- LES RÉGLAGES. Peu, et chacun dit ce qu'il coûte.

           L'ORDRE N'EST PAS COSMÉTIQUE : ce que l'appelant EXIGE vient d'abord,
           les réglages facultatifs ensuite. Rangé en bas, le seul champ
           obligatoire se retrouvait à huit cents pixels sous le pli, et le
           bouton d'envoi restait fermé sans que rien ne le dise. -->
      <div class="w-full space-y-5 lg:w-72 lg:shrink-0">
        <section>
          <h3 class="text-sm font-semibold">{{ t('image-editor.preview') }}</h3>
          <p class="mt-0.5 text-xs text-text-subtle">{{ shapeLine }}</p>
          <div
            class="mt-2 flex min-h-24 items-center justify-center rounded-md border border-border-subtle bg-surface-sunken p-2"
          >
            <img
              v-if="outputUrl"
              :src="outputUrl"
              :alt="t('image-editor.previewAlt')"
              class="max-h-32 max-w-full rounded-sm"
            >
            <UiSpinner v-else size="1.25rem" />
          </div>
        </section>

        <!-- Ce que l'appelant exige : le texte alternatif du dépôt vit là. -->
        <slot name="aside" />

        <section>
          <h3 class="text-sm font-semibold">{{ t('image-editor.format.label') }}</h3>
          <div class="mt-2 flex gap-2">
            <button
              v-for="candidate in FORMATS"
              :key="candidate"
              type="button"
              class="min-h-10 flex-1 cursor-pointer rounded-md border px-2 text-xs font-semibold transition-colors"
              :class="
                format === candidate
                  ? 'border-accent-border bg-accent-surface text-accent'
                  : 'border-border bg-surface text-text-muted hover:bg-surface-hover'
              "
              :aria-pressed="format === candidate"
              @click="format = candidate"
            >
              {{ t(`image-editor.format.${candidate}`) }}
            </button>
          </div>
          <p class="mt-1 text-xs text-text-subtle">{{ t(`image-editor.format.${format}Hint`) }}</p>
        </section>

        <section v-if="format !== 'png'">
          <div class="flex items-baseline justify-between">
            <label for="image-editor-quality" class="text-sm font-semibold">
              {{ t('image-editor.quality.label') }}
            </label>
            <span class="font-mono text-sm text-text-muted">{{ quality }}%</span>
          </div>
          <input
            id="image-editor-quality"
            v-model.number="quality"
            type="range"
            min="30"
            max="100"
            step="1"
            class="mt-2 w-full accent-accent-solid"
          >
        </section>

        <section>
          <div class="flex items-baseline justify-between">
            <label for="image-editor-width" class="text-sm font-semibold">
              {{ t('image-editor.width.label') }}
            </label>
            <span class="font-mono text-sm text-text-muted">
              {{ output ? `${output.width} × ${output.height}` : '—' }}
            </span>
          </div>
          <input
            id="image-editor-width"
            v-model.number="outputWidth"
            type="range"
            :min="minOutputWidth"
            :max="maxOutputWidth"
            step="20"
            class="mt-2 w-full accent-accent-solid"
          >
          <p class="mt-1 text-xs text-text-subtle">{{ t('image-editor.width.hint') }}</p>
        </section>

        <section class="rounded-md border border-border-subtle bg-surface p-3">
          <div class="flex items-baseline justify-between text-sm">
            <span class="font-semibold">{{ t('image-editor.weight.label') }}</span>
            <span
              class="font-mono"
              :class="tooHeavy ? 'text-danger' : 'text-text-muted'"
            >{{ encoding ? '…' : sizeLabel }}</span>
          </div>
          <p v-if="props.maxByteSize" class="mt-1 text-xs text-text-subtle">
            {{ t('image-editor.weight.max', { max: formatByteSize(props.maxByteSize, locale) }) }}
          </p>
        </section>

        <UiAlert
          v-if="tooHeavy"
          intent="warning"
          :message="t('image-editor.weight.tooHeavy')"
        />

      </div>
    </div>

    <template #footer>
      <p v-if="blockedBecause" class="me-auto max-w-80 text-sm text-text-muted">
        {{ blockedBecause }}
      </p>
      <UiButton variant="ghost" :disabled="props.busy" @click="emit('cancel')">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton
        icon="upload"
        :disabled="!output || tooHeavy || encoding || props.applyDisabled || loadFailed || encodeFailed"
        :loading="props.busy"
        @click="apply"
      >
        {{ props.applyLabel ?? t('image-editor.apply') }}
      </UiButton>
    </template>
  </UiModal>
</template>
