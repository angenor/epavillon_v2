<script setup lang="ts">
import { APRES_LE_COMPTE } from '~/utils/guide-nego/parcours'

/**
 * Écran 03b — se connecter.
 *
 * **Pas de case « se souvenir de moi », et c'est une décision** : la session
 * ouverte depuis l'application dure quatre-vingt-dix jours glissants, d'office.
 * Douze heures déconnecteraient une négociatrice en salle, là où aucun réseau ne
 * permet de se reconnecter — le pire moment possible.
 *
 * Les six issues de la connexion sortent en 200 avec leur discriminant ; chacune
 * laisse une action possible. Seule `invalid_credentials` ne dit rien de plus :
 * adresse inconnue et mot de passe faux sont indiscernables, et c'est voulu.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const api = useApi()
const route = useRoute()
const session = useGnSession()

// L'adresse est reportée depuis l'écran de compte ou depuis le retour du
// courriel : personne ne doit la ressaisir juste après l'avoir confirmée.
const adresse = ref(typeof route.query.adresse === 'string' ? route.query.adresse : '')
const motDePasse = ref('')
const envoi = ref(false)
/** Ce que l'API a répondu, affiché **tel quel** quand elle parle. */
const refus = ref<string | null>(null)
const adresseAConfirmer = ref<string | null>(null)
const feuilleOuverte = ref(false)

async function envoyer(): Promise<void> {
  refus.value = null
  adresseAConfirmer.value = null
  if (envoi.value) return

  envoi.value = true
  try {
    const issue = await session.connecter(adresse.value.trim(), motDePasse.value)

    switch (issue.status) {
      case 'authenticated':
        await navigateTo(APRES_LE_COMPTE)
        break
      case 'email_unverified':
        adresseAConfirmer.value = issue.email
        break
      case 'invalid_credentials':
        refus.value = t('guide-nego.connexion.refus.identifiants')
        break
      case 'locked':
        refus.value = t('guide-nego.connexion.refus.verrouille')
        break
      case 'suspended':
        refus.value = t('guide-nego.connexion.refus.suspendu')
        break
      case 'mfa_required':
        refus.value = t('guide-nego.connexion.refus.second-facteur')
        break
    }
  } catch (erreur) {
    refus.value = erreur instanceof Error && erreur.message ? erreur.message : t('api.unreachable.network')
  } finally {
    envoi.value = false
  }
}

async function renvoyerLeCourriel(): Promise<void> {
  if (!adresseAConfirmer.value) return
  await api.guideNego.renvoyerLeCourriel(adresseAConfirmer.value)
}

useHead({ title: t('guide-nego.connexion.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.connexion.titre')"
    :sous-titre="t('guide-nego.connexion.sous-titre')"
    retour="/guide-nego"
    :onglets="false"
  >
    <form class="gn-connexion__formulaire" novalidate @submit.prevent="envoyer">
      <GnChamp
        v-model="adresse"
        type="email"
        :libelle="t('guide-nego.connexion.champ.adresse')"
        autocomplete="email"
        inputmode="email"
      />
      <GnChamp
        v-model="motDePasse"
        type="password"
        :libelle="t('guide-nego.connexion.champ.mot-de-passe')"
        autocomplete="current-password"
      />

      <GnBouton variante="discret" largeur="demie" @clic="feuilleOuverte = true">
        {{ t('guide-nego.connexion.mot-de-passe-oublie') }}
      </GnBouton>

      <p v-if="refus" class="gn-connexion__refus" role="alert">{{ refus }}</p>

      <!-- Une adresse non confirmée n'est pas un échec : c'est une étape, et
           l'écran donne la seule action qui la débloque. -->
      <template v-if="adresseAConfirmer">
        <p class="gn-connexion__refus" role="alert">
          {{ t('guide-nego.connexion.refus.adresse-non-confirmee', { adresse: adresseAConfirmer }) }}
        </p>
        <GnBouton variante="secondaire" @clic="renvoyerLeCourriel">
          {{ t('guide-nego.connexion.renvoyer') }}
        </GnBouton>
      </template>

      <div class="gn-connexion__sorties">
        <GnBouton type="submit" variante="principal" :chargement="envoi">
          {{ t('guide-nego.connexion.entrer') }}
        </GnBouton>
        <GnBouton variante="discret" vers="/guide-nego/compte">
          {{ t('guide-nego.connexion.creer-un-compte') }}
        </GnBouton>
      </div>
    </form>

    <GnFeuilleBasse
      v-model="feuilleOuverte"
      :titre="t('guide-nego.connexion.oubli.titre')"
    >
      <GnMotDePasseOublie :adresse="adresse" @annuler="feuilleOuverte = false" />
    </GnFeuilleBasse>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-connexion__formulaire {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-connexion__refus {
  font-size: var(--gn-taille-15);
  color: var(--gn-danger);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-connexion__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}
</style>
