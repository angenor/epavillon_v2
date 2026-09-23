<script setup lang="ts">
import type { AvatarDEntete } from '~/components/guide-nego/GnEntete.vue'
/**
 * Le cadre commun : en-tête, contenu, et la barre d'onglets quand l'écran en porte une.
 * Un écran secondaire — lexique, réglages — donne un `retour` et se passe d'onglets.
 *
 * Les zones sûres et les marges viennent de `.gn-ecran` (mesures.css) ; rien ici ne
 * les redit.
 */
withDefaults(
  defineProps<{
    titre: string
    sousTitre?: string
    retour?: string
    /** Relayé à l'en-tête : aucune page n'instancie `GnEntete` directement. */
    avatar?: AvatarDEntete
    avatarDuTitre?: AvatarDEntete
    onglets?: boolean
    lexiqueOuvert?: boolean
    /** Relayé au bandeau hors connexion, pour un écran qui n'est pas tout entier lisible sans réseau. */
    ceQuiSeLit?: string
    /** Relayé à l'en-tête : le lecteur n'a qu'une ligne d'en-tête. */
    compact?: boolean
  }>(),
  {
    sousTitre: undefined,
    retour: undefined,
    avatar: undefined,
    avatarDuTitre: undefined,
    onglets: true,
    lexiqueOuvert: false,
    ceQuiSeLit: undefined,
    compact: false,
  },
)

const { echangesOuverts } = useGnDrapeaux()
const { etat, marquerBandeauVu } = useGnConnexion()

// Le bandeau se montre une fois par épisode : l'écran qui le montre le marque vu et le
// garde affiché, les écrans suivants n'en portent plus que le rappel de l'en-tête.
const bandeau = ref(false)
watch(
  [() => etat.value.enLigne, () => etat.value.bandeauVu],
  ([enLigne, vu]) => {
    if (enLigne) return (bandeau.value = false)
    if (!vu) {
      bandeau.value = true
      marquerBandeauVu()
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="gn-ecran" :class="{ 'gn-ecran--sans-onglets': !onglets }">
    <GnEntete
      :titre="titre"
      :sous-titre="sousTitre"
      :retour="retour"
      :avatar="avatar"
      :avatar-du-titre="avatarDuTitre"
      :lexique-ouvert="lexiqueOuvert"
      :compact="compact"
    >
      <template #connexion>
        <slot name="connexion">
          <GnLigneConnexion :en-ligne="etat.enLigne" :lu-a="etat.luA" />
        </slot>
      </template>
    </GnEntete>
    <GnBandeauConnexion v-if="bandeau" :lu-a="etat.luA" :ce-qui-se-lit="ceQuiSeLit" />
    <main class="gn-ecran__contenu">
      <slot />
    </main>
    <GnBarreOnglets v-if="onglets" :echanges-ouverts="echangesOuverts" />
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-ecran {
  flex: 1;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-ecran__contenu {
  flex: 1;
}

/* Le bandeau est pleine largeur : il sort des marges de l'écran. */
[data-app="guide-nego"] .gn-ecran > .gn-bandeau {
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  margin-top: var(--gn-espace-12);
}
</style>
