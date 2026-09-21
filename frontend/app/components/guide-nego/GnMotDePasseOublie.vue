<script setup lang="ts">
/**
 * Demander un nouveau mot de passe. **Un seul formulaire, deux hôtes** : la
 * feuille basse de l'écran de connexion et l'écran qu'on atteint par un lien
 * direct. Deux implémentations du même geste, c'est la seconde qui ne recevrait
 * jamais les corrections de la première.
 *
 * **La réponse est invariable** : adresse connue ou non, l'écran affiche la même
 * chose. Seul le courriel diffère, et il n'arrive que si le compte existe.
 */
const { adresse = '' } = defineProps<{ adresse?: string }>()
const emit = defineEmits<{ annuler: [] }>()

const { t } = useI18n()
const api = useApi()

const saisie = ref(adresse)
const envoi = ref(false)
const envoye = ref(false)
const panne = ref<string | null>(null)

watch(
  () => adresse,
  (valeur) => {
    if (!envoye.value) saisie.value = valeur
  },
)

async function envoyer(): Promise<void> {
  panne.value = null
  if (envoi.value || !saisie.value.trim()) return

  envoi.value = true
  try {
    await api.guideNego.motDePasseOublie(saisie.value.trim())
    envoye.value = true
  } catch (erreur) {
    panne.value =
      erreur instanceof Error && erreur.message ? erreur.message : t('api.unreachable.network')
  } finally {
    envoi.value = false
  }
}
</script>

<template>
  <div class="gn-oubli">
    <template v-if="envoye">
      <p class="gn-oubli__propos">{{ t('gn-mot-de-passe-oublie.envoye', { adresse: saisie }) }}</p>
      <GnBouton variante="secondaire" @clic="emit('annuler')">
        {{ t('gn-mot-de-passe-oublie.fermer') }}
      </GnBouton>
    </template>

    <template v-else>
      <p class="gn-oubli__propos">{{ t('gn-mot-de-passe-oublie.propos') }}</p>

      <form novalidate @submit.prevent="envoyer">
        <GnChamp
          v-model="saisie"
          type="email"
          :libelle="t('gn-mot-de-passe-oublie.champ')"
          autocomplete="email"
          inputmode="email"
        />

        <p v-if="panne" class="gn-oubli__panne" role="alert">{{ panne }}</p>

        <div class="gn-oubli__sorties">
          <GnBouton type="submit" variante="principal" picto="send" :chargement="envoi">
            {{ t('gn-mot-de-passe-oublie.envoyer') }}
          </GnBouton>
          <GnBouton variante="discret" @clic="emit('annuler')">
            {{ t('gn-mot-de-passe-oublie.annuler') }}
          </GnBouton>
        </div>
      </form>
    </template>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-oubli {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-oubli__propos {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-oubli__panne {
  font-size: var(--gn-taille-15);
  color: var(--gn-danger);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-oubli__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-16);
}
</style>
