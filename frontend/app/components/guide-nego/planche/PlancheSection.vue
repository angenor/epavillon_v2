<script lang="ts">
import { onBeforeUnmount, onMounted, ref, type ShallowRef } from 'vue'

type Hote = Readonly<ShallowRef<HTMLElement | null>>

/**
 * Lecture des jetons à l'exécution.
 *
 * Une planche montre la valeur que le navigateur applique vraiment : recopier les
 * hexadécimaux du thème donnerait une planche qui ment dès la première correction.
 * La sonde vit DANS la planche, et non dans `<body>` : elle hérite ainsi du thème du
 * cadre qui l'entoure — la page peut en afficher deux côte à côte.
 */
export function useJetonsLus(
  hote: Hote,
  couleurs: readonly string[] = [],
  mesures: readonly string[] = [],
) {
  const lus = ref<Record<string, string>>({})

  function enHexa(calculee: string): string {
    const canaux = calculee.match(/[\d.]+/g)
    if (!canaux || canaux.length < 3) return calculee
    return `#${canaux
      .slice(0, 3)
      .map((canal) => Number(canal).toString(16).padStart(2, '0'))
      .join('')
      .toUpperCase()}`
  }

  function relire() {
    const racine = hote.value
    if (!racine) return
    const sonde = document.createElement('span')
    sonde.setAttribute('aria-hidden', 'true')
    sonde.style.cssText = 'position:absolute;width:1px;height:1px;opacity:0;pointer-events:none'
    racine.appendChild(sonde)
    const calcul = getComputedStyle(sonde)
    const releve: Record<string, string> = {}
    for (const nom of couleurs) {
      sonde.style.setProperty('color', `var(${nom})`)
      releve[nom] = enHexa(calcul.color)
    }
    for (const nom of mesures) releve[nom] = calcul.getPropertyValue(nom).trim()
    sonde.remove()
    lus.value = releve
  }

  let observateur: MutationObserver | undefined
  let mouvement: MediaQueryList | undefined

  onMounted(() => {
    relire()
    // Le thème peut changer ailleurs que sur la racine de l'application : la page des
    // composants en force un par colonne quand elle montre clair et sombre ensemble.
    observateur = new MutationObserver(relire)
    observateur.observe(document.documentElement, {
      subtree: true,
      attributes: true,
      attributeFilter: ['data-theme'],
    })
    // L'autre condition qui change la valeur d'un jeton sans toucher au document :
    // « réduire les animations » met les durées à zéro.
    mouvement = window.matchMedia('(prefers-reduced-motion: reduce)')
    mouvement.addEventListener('change', relire)
  })

  onBeforeUnmount(() => {
    observateur?.disconnect()
    mouvement?.removeEventListener('change', relire)
  })

  return lus
}
</script>

<script setup lang="ts">
/**
 * Cadre commun des sept planches : un titre numéroté, une phrase de rôle, le contenu.
 * Sans `numero`, le même composant sert de sous-bloc titré — un seul dessin pour les
 * deux niveaux, et une planche qui se découpe sans rien réécrire.
 *
 * Il porte aussi les classes partagées `gn-planche-*` : échantillon, légende, ligne,
 * grille. C'est le motif qui revient d'une planche à l'autre, dessiné une seule fois.
 */
withDefaults(defineProps<{ titre: string; numero?: string; propos?: string }>(), {
  numero: undefined,
  propos: undefined,
})
</script>

<template>
  <component
    :is="numero ? 'section' : 'div'"
    class="gn-planche-section"
    :class="{ 'gn-planche-section--groupe': !numero }"
  >
    <component :is="numero ? 'h2' : 'h3'" class="gn-planche-section__titre">
      <span v-if="numero" class="gn-planche-section__numero">{{ numero }}</span>
      <span>{{ titre }}</span>
    </component>
    <p v-if="propos" class="gn-planche-section__propos">{{ propos }}</p>
    <div class="gn-planche-section__corps"><slot /></div>
  </component>
</template>

<style>
[data-app="guide-nego"] .gn-planche-section {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-planche-section__titre {
  display: flex;
  align-items: baseline;
  gap: var(--gn-espace-8);
  padding-bottom: var(--gn-espace-8);
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
  color: var(--gn-titre);
  font-size: var(--gn-taille-24);
  font-weight: var(--gn-graisse-gras);
  line-height: var(--gn-interligne-24);
}

[data-app="guide-nego"] .gn-planche-section__numero {
  flex: none;
  font-size: var(--gn-taille-17);
}

[data-app="guide-nego"] .gn-planche-section--groupe {
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-planche-section--groupe .gn-planche-section__titre {
  border-bottom-width: var(--gn-filet-1);
  border-bottom-color: var(--gn-filet);
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
}

[data-app="guide-nego"] .gn-planche-section__propos {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-planche-section__corps {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-planche-section--groupe .gn-planche-section__corps {
  gap: var(--gn-espace-12);
}

/* Motifs partagés par les sept planches. */

[data-app="guide-nego"] .gn-planche-grille {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-planche-grille--large {
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
}

[data-app="guide-nego"] .gn-planche-grille--serree {
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
}

[data-app="guide-nego"] .gn-planche-echantillon {
  display: flex;
  flex-direction: column;
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-0);
}

[data-app="guide-nego"] .gn-planche-echantillon__aplat {
  min-height: var(--gn-espace-48);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-legende {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  padding: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-planche-jeton {
  color: var(--gn-texte);
  font-size: var(--gn-taille-15);
  font-weight: var(--gn-graisse-gras);
  line-height: var(--gn-interligne-15);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-planche-valeur,
[data-app="guide-nego"] .gn-planche-note {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-planche-note {
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-planche-liste {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-planche-ligne {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  min-height: var(--gn-cible);
  padding-block: var(--gn-ligne-air);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-liste > .gn-planche-ligne:first-child {
  border-top: none;
}

[data-app="guide-nego"] .gn-planche-ligne__nom {
  flex: 1;
  min-width: 0;
}

/* Une mesure plus large que la colonne de lecture se montre à sa taille réelle ;
   c'est la bande qui défile, jamais la page. */
[data-app="guide-nego"] .gn-planche-bande {
  overflow-x: auto;
  max-width: 100%;
}
</style>
