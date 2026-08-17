<script setup lang="ts">
import type { AttachedImage } from '~/types/media'

/**
 * IMAGE SERVIE PAR LE MODULE MÉDIA.
 *
 * Il rend une ligne de `media.attached_image()` — telle que l'exposent les vues,
 * couverture d'une séance en tête. Trois règles du modèle sont tenues ici, et
 * chacune corrige un défaut de la v1 :
 *
 * · IL NE FABRIQUE JAMAIS D'URL. La v1 stockait quinze colonnes `*_url` en base,
 *   et changer de fournisseur de stockage demandait une migration. Le modèle ne
 *   stocke plus que `(bucket, object_key)` et compose l'adresse par
 *   `media.object_url()`, depuis un réglage. Une URL fabriquée côté client
 *   réintroduirait exactement ce qu'on a supprimé.
 *
 * · LE TEXTE ALTERNATIF EST OBLIGATOIRE, et ce n'est pas une politesse :
 *   `ck_assets_alt_text_required` interdit à une image d'atteindre l'état `ready`
 *   sans lui, et seul `ready` est servi. Toute image affichable en a donc un.
 *   D'où un type non nullable — pas de repli sur `alt=""`, pas de repli sur le
 *   nom de fichier. C'est une DONNÉE multilingue, résolue comme telle.
 *
 * · AUCUN RATIO N'EST PRÉSUMÉ. Les variantes vivent dans la configuration du
 *   worker, pas dans le schéma : le commentaire de `renditions.variant_code` est
 *   explicite. Le composant consomme donc `sources` s'il y en a, et se rabat
 *   toujours sur `url`, qui porte l'original. Entre le téléversement et le
 *   passage du worker, `sources` est vide alors que l'image est valide — sans ce
 *   repli, la carte resterait trouée pendant ce délai.
 *
 * L'ABSENCE D'IMAGE N'EST PAS UN DÉFAUT. Une activité sans couverture doit rester
 * une carte lisible : le composant ne rend RIEN plutôt qu'une image de
 * remplacement, un dégradé ou un pictogramme inventé. C'est à l'appelant de
 * décider de la mise en page dans ce cas — voir `UiSessionCard`.
 */

interface Props {
  /** Ce que rend `media.attached_image()`. `null` : le composant ne rend rien. */
  image: AttachedImage | null | undefined
  /**
   * Proportions imposées au cadre, en notation CSS (`16 / 9`, `1 / 1`).
   * Le cadre est recadré par `object-fit: cover` : c'est la mise en page qui
   * décide du format, jamais le fichier téléversé.
   *
   * `'auto'` n'impose RIEN, et c'est le seul moyen d'obtenir des proportions qui
   * changent avec la largeur : le ratio est posé en style *inline*, qu'aucune
   * classe utilitaire ne peut redéfinir. Un appelant qui veut « 16:9 en bandeau,
   * pleine hauteur en colonne » passe donc `ratio="auto"` et décrit les deux cas
   * dans `frameClass`.
   */
  ratio?: string
  /** Chargement différé. Immédiat seulement pour ce qui est au-dessus de la ligne de flottaison. */
  loading?: 'lazy' | 'eager'
  /** Attribut `sizes` du `<picture>`, quand la largeur rendue est connue. */
  sizes?: string
  /** Coins arrondis du cadre. */
  rounded?: string
  /**
   * Classes posées sur le CADRE — celui qui porte les proportions et le
   * recadrage — et non sur la racine. Sans elles, un appelant ne peut pas
   * atteindre l'élément qui décide de la hauteur de l'image, et l'en-tête est
   * pourtant formel : c'est à lui de décider de la mise en page.
   */
  frameClass?: string
  /** Affiche la légende et le crédit sous l'image, dans un `<figure>`. */
  showCaption?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  ratio: '16 / 9',
  loading: 'lazy',
})

const { tr } = useI18nText()

/**
 * Variantes triées par largeur croissante, pour un `srcset` exploitable.
 * Les clés sont `<variant_code>_<format>` ; on ne retient que les entrées dont
 * la largeur est connue, sans quoi le descripteur `w` serait faux.
 */
const srcset = computed(() => {
  const sources = props.image?.sources
  if (!sources) return undefined
  const entries = Object.values(sources)
    .filter((source) => typeof source.width === 'number' && source.width > 0)
    .sort((a, b) => (a.width ?? 0) - (b.width ?? 0))
  if (entries.length === 0) return undefined
  return entries.map((source) => `${source.url} ${source.width}w`).join(', ')
})

/** Le texte alternatif résolu. Toujours renseigné — voir l'en-tête. */
const alt = computed(() => tr(props.image?.alt_text))
const caption = computed(() => tr(props.image?.caption))
</script>

<template>
  <component
    :is="props.showCaption && (caption || props.image?.credit) ? 'figure' : 'div'"
    v-if="props.image"
    class="m-0"
  >
    <div
      class="overflow-hidden bg-surface-sunken"
      :class="[props.rounded, props.frameClass]"
      :style="props.ratio === 'auto' ? undefined : { aspectRatio: props.ratio }"
    >
      <img
        :src="props.image.url"
        :srcset="srcset"
        :sizes="props.sizes"
        :alt="alt"
        :width="props.image.width ?? undefined"
        :height="props.image.height ?? undefined"
        :loading="props.loading"
        decoding="async"
        class="size-full object-cover"
      >
    </div>

    <figcaption
      v-if="props.showCaption && (caption || props.image.credit)"
      class="mt-1.5 text-xs text-text-subtle"
    >
      {{ caption }}
      <span v-if="props.image.credit" class="italic">{{ props.image.credit }}</span>
    </figcaption>
  </component>
</template>
