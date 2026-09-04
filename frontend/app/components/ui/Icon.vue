<script setup lang="ts">
/**
 * Jeu d'icônes de la plateforme.
 *
 * DIRECTION ARTISTIQUE — « Emoji en guise d'icônes fonctionnelles » figure dans
 * les interdits du projet, et « les titres portent le sens, pas les icônes » en
 * est le pendant positif. Une icône accompagne un libellé, elle ne le remplace
 * pas : c'est pourquoi ce composant est TOUJOURS `aria-hidden`. Quand une icône
 * est seule dans un bouton, c'est au bouton de porter son nom accessible
 * (`aria-label`), jamais à l'icône.
 *
 * Dessin : trait de 1,7 px sur une grille de 24, extrémités arrondies, aucun
 * aplat. Les icônes héritent de la couleur du texte (`currentColor`) et de sa
 * taille (`1em`) : elles suivent donc le thème sans réglage.
 *
 * Un jeu embarqué plutôt qu'une bibliothèque : trente pictogrammes suffisent au
 * jalon, et une dépendance de plus se paierait en poids de paquet et en dérive
 * de style.
 */

interface Props {
  /** Nom du pictogramme. Un nom inconnu ne rend rien plutôt qu'un carré vide. */
  name: string
  /** Taille CSS ; par défaut la taille de police du contexte. */
  size?: string
  /** Épaisseur du trait. À monter légèrement sur les très petites tailles. */
  strokeWidth?: number
}

const props = withDefaults(defineProps<Props>(), {
  size: '1em',
  strokeWidth: 1.7,
})

/**
 * Chemins SVG, grille 24 × 24. Les icônes d'état (`info`, `warning`, `error`,
 * `success`) sont volontairement distinctes DE FORME et pas seulement de
 * couleur : une information portée par la seule couleur est perdue pour une
 * personne daltonienne.
 */
const PATHS: Record<string, string> = {
  // — Navigation
  'chevron-down': 'M6 9.5 12 15.5 18 9.5',
  'chevron-up': 'M6 14.5 12 8.5 18 14.5',
  'chevron-left': 'M14.5 6 8.5 12 14.5 18',
  'chevron-right': 'M9.5 6 15.5 12 9.5 18',
  'arrow-left': 'M19 12H5M11 6 5 12l6 6',
  'arrow-right': 'M5 12h14M13 6l6 6-6 6',
  'arrow-up-right': 'M8 16 16 8M9 8h7v7',
  'external-link': 'M14 4h6v6M20 4l-8.5 8.5M18 14v4.5A1.5 1.5 0 0 1 16.5 20h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H10',
  menu: 'M4 7h16M4 12h16M4 17h16',
  close: 'M6 6l12 12M18 6 6 18',
  'more-vertical': 'M12 5.5v.01M12 12v.01M12 18.5v.01',
  'more-horizontal': 'M5.5 12h.01M12 12h.01M18.5 12h.01',

  // — État et retour d'information
  check: 'M4.5 12.5 9.5 17.5 19.5 7',
  'check-circle': 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM8 12.2l2.8 2.8L16 9.8',
  info: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM12 11v5.5M12 7.6v.01',
  warning: 'M12 3.8 2.9 19.4h18.2L12 3.8ZM12 10v4.2M12 17.1v.01',
  error: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM9 9l6 6M15 9l-6 6',
  ban: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM5.6 5.6l12.8 12.8',
  lock: 'M7 10.5V8a5 5 0 0 1 10 0v2.5M5.8 10.5h12.4a.8.8 0 0 1 .8.8v7.4a.8.8 0 0 1-.8.8H5.8a.8.8 0 0 1-.8-.8v-7.4a.8.8 0 0 1 .8-.8Z',
  'shield-check': 'M12 3.2 5 6v6c0 4 3 7.2 7 8.8 4-1.6 7-4.8 7-8.8V6l-7-2.8ZM9 12l2.2 2.2L15.2 10',

  // — Objets de la plateforme
  calendar: 'M5 6.5h14a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-11a1 1 0 0 1 1-1ZM4 10.5h16M8.5 4v4M15.5 4v4',
  clock: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM12 7.2V12l3.2 2',
  'map-pin': 'M12 21c3.6-4.2 6-7.2 6-10a6 6 0 1 0-12 0c0 2.8 2.4 5.8 6 10ZM12 13.2a2.2 2.2 0 1 0 0-4.4 2.2 2.2 0 0 0 0 4.4Z',
  globe: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM3.4 9.4h17.2M3.4 14.6h17.2M12 3a14 14 0 0 1 0 18 14 14 0 0 1 0-18Z',
  users: 'M9 11.4a3.4 3.4 0 1 0 0-6.8 3.4 3.4 0 0 0 0 6.8ZM2.8 19.6c0-3.1 2.8-5.2 6.2-5.2s6.2 2.1 6.2 5.2M16.4 5.1a3.2 3.2 0 0 1 0 6.1M17.6 14.7c2.2.5 3.6 2.1 3.6 4.2',
  building: 'M4.5 20.5h15M6 20.5V4.8a.8.8 0 0 1 .8-.8h7.4a.8.8 0 0 1 .8.8v15.7M14.2 9h3a.8.8 0 0 1 .8.8v10.7M9 8h2.5M9 12h2.5M9 16h2.5',
  document: 'M6 3.5h7l5 5v12a.8.8 0 0 1-.8.8H6a.8.8 0 0 1-.8-.8V4.3a.8.8 0 0 1 .8-.8ZM13 3.5V9h5M8.5 13.5h7M8.5 17h4.5',
  broadcast: 'M12 13.6a1.6 1.6 0 1 0 0-3.2 1.6 1.6 0 0 0 0 3.2ZM8.4 8.4a5 5 0 0 0 0 7.2M15.6 8.4a5 5 0 0 1 0 7.2M5.6 5.6a9 9 0 0 0 0 12.8M18.4 5.6a9 9 0 0 1 0 12.8',
  monitor: 'M3.5 5h17a.8.8 0 0 1 .8.8v9.4a.8.8 0 0 1-.8.8h-17a.8.8 0 0 1-.8-.8V5.8A.8.8 0 0 1 3.5 5ZM8.5 20h7M12 16v4',
  // Caméra : diffusion, rejoindre un direct, revoir un enregistrement. À ne pas
  // confondre avec `broadcast`, qui figure le SIGNAL, pas l'appareil.
  video: 'M3.8 6.5h10.4a.8.8 0 0 1 .8.8v9.4a.8.8 0 0 1-.8.8H3.8a.8.8 0 0 1-.8-.8V7.3a.8.8 0 0 1 .8-.8ZM15 10.6l5-2.8v8.4l-5-2.8',
  home: 'M4 10.5 12 4l8 6.5V19a.8.8 0 0 1-.8.8h-4.4v-6h-5.6v6H4.8A.8.8 0 0 1 4 19v-8.5Z',
  grid: 'M4.5 4.5h5.5v5.5H4.5ZM14 4.5h5.5v5.5H14ZM4.5 14h5.5v5.5H4.5ZM14 14h5.5v5.5H14Z',
  chart: 'M4 20V4M4 20h16M8 16.5V11M12.5 16.5V7M17 16.5v-3.5',
  inbox: 'M4 12.5 6.2 5.3a.8.8 0 0 1 .8-.6h10a.8.8 0 0 1 .8.6L20 12.5v6.2a.8.8 0 0 1-.8.8H4.8a.8.8 0 0 1-.8-.8v-6.2ZM4 12.5h4l1.2 2.4h5.6L16 12.5h4',
  // Enveloppe : un courriel envoyé — vérification d'adresse, lien de
  // réinitialisation. À ne pas confondre avec `inbox`, qui figure une CORBEILLE
  // de travail à traiter, pas un message en transit.
  mail: 'M4 6.5h16a.8.8 0 0 1 .8.8v9.4a.8.8 0 0 1-.8.8H4a.8.8 0 0 1-.8-.8V7.3a.8.8 0 0 1 .8-.8ZM3.5 7.6 12 13.2l8.5-5.6',

  // — Thème d'affichage
  sun: 'M12 16.2a4.2 4.2 0 1 0 0-8.4 4.2 4.2 0 0 0 0 8.4ZM12 2.6v2.2M12 19.2v2.2M4.2 12H2M22 12h-2.2M6.3 6.3 4.8 4.8M19.2 19.2l-1.5-1.5M17.7 6.3l1.5-1.5M4.8 19.2l1.5-1.5',
  moon: 'M20 14.2A8.4 8.4 0 0 1 9.8 4a8.4 8.4 0 1 0 10.2 10.2Z',

  // — Actions
  search: 'M10.8 17.6a6.8 6.8 0 1 0 0-13.6 6.8 6.8 0 0 0 0 13.6ZM15.8 15.8 20.5 20.5',
  filter: 'M4 6h16l-6.2 7.2v5.4l-3.6 1.8v-7.2L4 6Z',
  // Curseurs de réglage : « Paramètres ». Un engrenage à 16 px, tracé à 1,7,
  // se referme en tache — ces trois glissières restent lisibles à la taille
  // d'une entrée de navigation.
  sliders: 'M4 7h6M13.4 7H20M4 17h9.6M16.8 17H20M11.8 8.6a1.6 1.6 0 1 0 0-3.2 1.6 1.6 0 0 0 0 3.2ZM15.2 18.6a1.6 1.6 0 1 0 0-3.2 1.6 1.6 0 0 0 0 3.2Z',
  // Porte ouverte, flèche qui sort : la déconnexion. À ne pas confondre avec
  // `arrow-right`, qui fait avancer dans un parcours.
  'log-out': 'M14 4h4.2a.8.8 0 0 1 .8.8v14.4a.8.8 0 0 1-.8.8H14M9.5 8.2 13.3 12l-3.8 3.8M13.3 12H3.5',
  plus: 'M12 5v14M5 12h14',
  minus: 'M5 12h14',
  edit: 'M4.5 19.5h4l10-10a2.1 2.1 0 0 0-3-3l-10 10v3ZM14.5 6.5l3 3',
  trash: 'M4.5 7h15M9.5 7V4.8a.8.8 0 0 1 .8-.8h3.4a.8.8 0 0 1 .8.8V7M6.5 7l.8 12.2a.8.8 0 0 0 .8.8h7.8a.8.8 0 0 0 .8-.8L17.5 7M10.5 11v6M13.5 11v6',
  download: 'M12 4v11M7.5 10.5 12 15l4.5-4.5M4.5 19.5h15',
  upload: 'M12 15V4M7.5 8.5 12 4l4.5 4.5M4.5 19.5h15',
  refresh: 'M20 12a8 8 0 1 1-2.6-5.9M20 4v4.5h-4.5',
  eye: 'M12 5.5c4.4 0 8 3.3 9.2 6.5-1.2 3.2-4.8 6.5-9.2 6.5S4 15.2 2.8 12C4 8.8 7.6 5.5 12 5.5ZM12 14.8a2.8 2.8 0 1 0 0-5.6 2.8 2.8 0 0 0 0 5.6Z',
  // Œil barré : masquer un mot de passe révélé. La barre traverse tout le
  // glyphe — un œil « fermé » dessiné en trait fin se confond avec l'œil ouvert
  // à 16 px, et le bouton devient impossible à lire d'un coup d'œil.
  'eye-off':
    'M6.5 6.7C4.7 8 3.5 9.9 2.8 12c1.2 3.2 4.8 6.5 9.2 6.5 1.4 0 2.7-.3 3.8-.9M9.9 9.9a2.8 2.8 0 0 0 3.9 3.9M10.2 5.7A9.9 9.9 0 0 1 12 5.5c4.4 0 8 3.3 9.2 6.5-.6 1.5-1.6 3-3 4.2M4 4l16 16',
  copy: 'M9 9.8a.8.8 0 0 1 .8-.8h9.4a.8.8 0 0 1 .8.8v9.4a.8.8 0 0 1-.8.8H9.8a.8.8 0 0 1-.8-.8V9.8ZM6 15H4.8a.8.8 0 0 1-.8-.8V4.8a.8.8 0 0 1 .8-.8h9.4a.8.8 0 0 1 .8.8V6',
  drag: 'M9 6.5h.01M15 6.5h.01M9 12h.01M15 12h.01M9 17.5h.01M15 17.5h.01',
  // — Mise en forme du texte riche (UiRichText). Aucune icône de couleur ni de
  // police : ces commandes n'existent pas, la charte décide de l'apparence.
  bold: 'M7 5h6.5a3.5 3.5 0 0 1 0 7H7V5ZM7 12h7.5a3.5 3.5 0 0 1 0 7H7v-7Z',
  italic: 'M15 5h-5M14 19H9M13.5 5 10.5 19',
  list: 'M9 6.5h11M9 12h11M9 17.5h11M4.5 6.5h.01M4.5 12h.01M4.5 17.5h.01',
  'list-ordered': 'M9 6.5h11M9 12h11M9 17.5h11M3.6 4.6h1.2v3.8M3.4 8.4h2M3.4 10.8h2.2l-2.2 2.8h2.2M3.4 15.8h2.2v1.4H4.2v.9h1.4v1.5H3.4',
  quote: 'M9.5 7.5C7 8.5 5.5 10.5 5.5 13v3.5h4.2V13H7.6c0-1.6.8-2.8 2.4-3.6l-.5-1.9ZM18 7.5c-2.5 1-4 3-4 5.5v3.5h4.2V13h-2.1c0-1.6.8-2.8 2.4-3.6L18 7.5Z',

  'sort-asc': 'M8 18V6M4.5 9.5 8 6l3.5 3.5M14 8h6M14 12h4.5M14 16h3',
  'sort-desc': 'M8 6v12M4.5 14.5 8 18l3.5-3.5M14 8h6M14 12h4.5M14 16h3',
}

const path = computed(() => PATHS[props.name] ?? null)

// Les points isolés d'un chemin (`M12 11v.01`) ne se voient qu'avec des
// extrémités rondes : le réglage vaut pour tout le jeu.
</script>

<template>
  <svg
    v-if="path"
    :width="props.size"
    :height="props.size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="props.strokeWidth"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
    class="inline-block shrink-0"
  >
    <path :d="path" />
  </svg>
</template>
