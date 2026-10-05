<script setup lang="ts">
import type { GlossaryEntry } from '~/types/negotiation-savoir'

/**
 * Un terme trouvé, tel que la maquette 03 le dessine : le terme anglais sous sa marque
 * « EN », la traduction en grand à l'accent, la définition, le favori et la copie, la
 * source. La recherche du lexique et la fiche d'un terme le posent toutes deux.
 *
 * Le lexique ne porte pas de date de vérification (contrairement à la FAQ) : la ligne
 * de source dit la date de mise à jour, jamais une vérification que le modèle ignore.
 */
const props = withDefaults(defineProps<{ entree: GlossaryEntry; titre?: 'h1' | 'h2' }>(), { titre: 'h2' })

const { t, locale } = useI18n()
const { favoris, assurer, basculer } = useGnFavorisLexique()
onMounted(() => void assurer())

const estFavori = computed(() => favoris.value.has(props.entree.id))
const traduction = computed(() =>
  props.entree.acronym
    ? t('gn-carte-terme.avec-sigle', { traduction: props.entree.translation, sigle: props.entree.acronym })
    : props.entree.translation,
)
const source = computed(() => {
  const titres = props.entree.sources.flatMap((s) => s.document_title ?? s.external_title ?? [])
  const jour = new Intl.DateTimeFormat(String(locale.value), { day: 'numeric', month: 'long', timeZone: 'UTC' }).format(
    new Date(props.entree.updated_at),
  )
  return [...new Set(titres)].concat(t('gn-carte-terme.mis-a-jour', { jour })).join(' · ')
})

const message = ref<{ texte: string; rang: number } | null>(null)

async function copier(): Promise<void> {
  const texte = `${props.entree.term} : ${props.entree.translation}`
  let annonce = t('gn-carte-terme.copie')
  try {
    await navigator.clipboard.writeText(texte)
  } catch {
    annonce = t('gn-carte-terme.copie-impossible')
  }
  message.value = { texte: annonce, rang: (message.value?.rang ?? 0) + 1 }
}
</script>

<template>
  <article class="gn-carte-terme">
    <p class="gn-carte-terme__anglais">
      <span class="gn-carte-terme__langue" aria-hidden="true">{{ t('gn-carte-terme.langue') }}</span>
      <span lang="en">{{ entree.term }}</span>
    </p>
    <component :is="titre" class="gn-carte-terme__traduction">{{ traduction }}</component>
    <p class="gn-carte-terme__definition">{{ entree.definition }}</p>
    <div class="gn-carte-terme__actions">
      <GnBouton
        class="gn-carte-terme__favori"
        :picto="estFavori ? 'star-fill' : 'star'"
        :actif="estFavori"
        @clic="basculer(entree.id)"
      >
        {{ estFavori ? t('gn-carte-terme.favori-oui') : t('gn-carte-terme.favori-non') }}
      </GnBouton>
      <GnBouton
        variante="secondaire"
        largeur="demie"
        picto="copy"
        class="gn-carte-terme__copier"
        :aria-label="t('gn-carte-terme.copier')"
        @clic="copier"
      />
      <slot name="actions" />
    </div>
    <p class="gn-carte-terme__source">{{ source }}</p>
    <GnMessageEphemere v-if="message" :key="message.rang" :texte="message.texte" @fini="message = null" />
  </article>
</template>

<style>
[data-app="guide-nego"] .gn-carte-terme {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
  padding-block: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-carte-terme__anglais {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-18);
  line-height: normal;
}

[data-app="guide-nego"] .gn-carte-terme__langue {
  flex: none;
  padding: 4px 8px;
  border-radius: var(--gn-rayon-6);
  background: var(--gn-bloc-releve);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-11);
  line-height: var(--gn-interligne-11);
  font-weight: var(--gn-graisse-extra-gras);
  letter-spacing: var(--gn-approche-11);
}

[data-app="guide-nego"] .gn-carte-terme__traduction {
  color: var(--gn-accent);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-44);
  line-height: var(--gn-interligne-44);
  letter-spacing: var(--gn-approche-44);
  font-weight: var(--gn-graisse-extra-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-carte-terme__definition {
  color: var(--gn-texte-lecture);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-carte-terme__actions {
  display: flex;
  gap: var(--gn-espace-8);
}

/* La maquette : 50 de haut et texte 15 ici, un cran sous le bouton principal d'écran. */
[data-app="guide-nego"] .gn-carte-terme__actions .gn-bouton {
  min-height: 50px;
  font-size: var(--gn-taille-15);
}

[data-app="guide-nego"] .gn-carte-terme__favori {
  flex: 1;
  width: auto;
}

/* « Dans mes favoris » reste plein : l'aplat d'accent dit déjà l'état. */
[data-app="guide-nego"] .gn-carte-terme__favori.gn-bouton--actif {
  background: var(--gn-accent);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-carte-terme__favori .gn-picto {
  width: 18px;
  height: 18px;
}

[data-app="guide-nego"] .gn-carte-terme__copier {
  flex: none;
  width: 50px;
  padding: 0;
}

[data-app="guide-nego"] .gn-carte-terme__source {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}
</style>
