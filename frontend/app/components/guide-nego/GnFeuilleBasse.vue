<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
/**
 * La feuille qui monte du bas : menus du « ⋯ » et du long-appui, motifs de signalement,
 * filtres, réglages de lecture, export.
 *
 * Deux façons de la remplir, parce que ses usages sont de deux sortes. Une liste
 * d'options couvre le cas courant — des actions qu'on choisit une fois, et dont le
 * dessin doit être le même partout ; le `<slot>` couvre les feuilles qui portent autre
 * chose que des choix uniques (cases à cocher d'un filtre, deux sélecteurs segmentés de
 * la lecture). Les deux se cumulent.
 *
 * Une option est une phrase : « Signaler ce message », jamais « Signaler ».
 * Elle se téléporte dans `#gn-portail` — dans `<body>`, elle n'aurait ni jetons ni thème.
 */
export interface OptionDeFeuille {
  valeur: string
  libelle: string
  picto?: NomDePicto
  /** Une option destructrice : rouge, et toujours suivie d'une boîte de confirmation. */
  dangereuse?: boolean
}

const props = withDefaults(
  defineProps<{
    titre: string
    sousTitre?: string
    options?: OptionDeFeuille[]
    /** Le bouton qui referme : « Annuler » par défaut ; « Fermer » quand rien n'est à annuler. */
    fermeture?: string
  }>(),
  { sousTitre: undefined, options: () => [], fermeture: undefined },
)

const emit = defineEmits<{ choisir: [OptionDeFeuille] }>()

const ouverte = defineModel<boolean>({ required: true })

const { t } = useI18n()
const idTitre = useId()
const feuille = useTemplateRef<HTMLElement>('feuille')

useGnPiegeFocus(ouverte, () => feuille.value, { fermer })

function fermer() {
  ouverte.value = false
}

function choisir(option: OptionDeFeuille) {
  emit('choisir', option)
  fermer()
}

const derniere = computed(() => props.options.length - 1)
</script>

<template>
  <Teleport to="#gn-portail">
    <Transition name="gn-feuille">
      <div v-if="ouverte" class="gn-feuille-basse">
        <div class="gn-feuille-basse__voile" aria-hidden="true" @click="fermer" />
        <div
          ref="feuille"
          class="gn-feuille-basse__feuille"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="idTitre"
        >
          <span class="gn-feuille-basse__poignee" aria-hidden="true" />
          <h2
            :id="idTitre"
            class="gn-feuille-basse__titre"
            :class="{ 'gn-feuille-basse__titre--avec-sous-titre': sousTitre }"
          >
            {{ titre }}
          </h2>
          <p v-if="sousTitre" class="gn-feuille-basse__sous-titre">{{ sousTitre }}</p>

          <ul v-if="options.length" class="gn-feuille-basse__options">
            <li v-for="(option, rang) in options" :key="option.valeur">
              <button
                type="button"
                class="gn-feuille-basse__option"
                :class="{
                  'gn-feuille-basse__option--dangereuse': option.dangereuse,
                  'gn-feuille-basse__option--derniere': rang === derniere,
                }"
                @click="choisir(option)"
              >
                <GnPicto v-if="option.picto" :nom="option.picto" :taille="24" />
                {{ option.libelle }}
              </button>
            </li>
          </ul>

          <slot />

          <GnBouton variante="secondaire" @clic="fermer">
            {{ fermeture ?? t('gn-feuille-basse.annuler') }}
          </GnBouton>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style>
/* `#gn-portail` est fixe et couvre l'écran : la feuille s'y cale en absolu et se pose en bas. */
[data-app="guide-nego"] .gn-feuille-basse {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
}

[data-app="guide-nego"] .gn-feuille-basse__voile {
  position: absolute;
  inset: 0;
  background: var(--gn-voile);
  opacity: var(--gn-voile-opacite);
}

[data-app="guide-nego"] .gn-feuille-basse__feuille {
  position: relative;
  max-height: 100%;
  overflow-y: auto;
  padding: var(--gn-espace-8) var(--gn-marge-ecran)
    calc(var(--gn-espace-16) + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  border-top: var(--gn-filet-3) solid var(--gn-filet-fort);
  background: var(--gn-fond);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-feuille-basse__poignee {
  flex: none;
  align-self: center;
  width: var(--gn-feuille-poignee-largeur);
  height: var(--gn-feuille-poignee-hauteur);
  background: var(--gn-filet);
}

[data-app="guide-nego"] .gn-feuille-basse__titre {
  color: var(--gn-titre);
  font-size: var(--gn-taille-24);
  line-height: var(--gn-interligne-24);
  font-weight: var(--gn-graisse-gras);
}

/* Avec un sous-titre, le titre descend d'un cran : les deux lignes forment un bloc. */
[data-app="guide-nego"] .gn-feuille-basse__titre--avec-sous-titre {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
}

[data-app="guide-nego"] .gn-feuille-basse__sous-titre {
  margin-top: calc(-1 * var(--gn-espace-8));
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-feuille-basse__options {
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-feuille-basse__option {
  width: 100%;
  min-height: var(--gn-ligne-reglage);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border: none;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  background: none;
  color: var(--gn-texte);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  text-align: left;
}

[data-app="guide-nego"] .gn-feuille-basse__option--derniere {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-feuille-basse__option > .gn-picto {
  color: var(--gn-picto);
}

/* Écart 33 : un texte et un pictogramme rouges gardent la couleur de charte ; seul l'aplat s'assombrit. */
[data-app="guide-nego"] .gn-feuille-basse__option--dangereuse,
[data-app="guide-nego"] .gn-feuille-basse__option--dangereuse > .gn-picto {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-feuille-basse__option:active {
  background: var(--gn-presse);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-feuille-basse__option:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-feuille-enter-active,
[data-app="guide-nego"] .gn-feuille-leave-active {
  transition: opacity var(--gn-duree-standard) var(--gn-courbe-sortie);
}

[data-app="guide-nego"] .gn-feuille-enter-active .gn-feuille-basse__feuille,
[data-app="guide-nego"] .gn-feuille-leave-active .gn-feuille-basse__feuille {
  transition: transform var(--gn-duree-standard) var(--gn-courbe-sortie);
}

[data-app="guide-nego"] .gn-feuille-enter-from,
[data-app="guide-nego"] .gn-feuille-leave-to {
  opacity: 0;
}

[data-app="guide-nego"] .gn-feuille-enter-from .gn-feuille-basse__feuille,
[data-app="guide-nego"] .gn-feuille-leave-to .gn-feuille-basse__feuille {
  transform: translateY(100%);
}
</style>
