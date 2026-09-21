<script setup lang="ts">
import { APRES_LE_COMPTE } from '~/utils/guide-nego/parcours'
import { MIN_PASSWORD_LENGTH, missingRequirements } from '~/utils/password-strength'
import { resolveI18nText } from '~/utils/i18n-text'
import type { CountryId } from '~/types/shared'

/**
 * Écran 03a — créer son compte, étape 1 sur 3.
 *
 * **Un seul compte pour Guide Négo et l'ePavillon**, et l'écran le dit : une
 * personne qui a déjà déposé une activité n'a pas à s'en créer un second. La
 * réponse de l'API ne distingue pas l'adresse libre de l'adresse déjà prise —
 * c'est le courriel qui diffère —, donc l'écran affiche la même attente dans les
 * deux cas et ne devient jamais l'annuaire des comptes.
 *
 * **L'attente de confirmation vit ici**, dans le même écran : « J'ai confirmé mon
 * adresse » relit l'état, « Renvoyer le courriel » redemande le lien. Sur iPhone,
 * le lien s'ouvre dans une autre session que celle de l'application, et c'est ce
 * retour au premier plan — pas un bouton — qui fait passer à la suite.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const api = useApi()
const session = useGnSession()

const fiche = reactive({
  prenom: '',
  nom: '',
  adresse: '',
  pays: '' as CountryId | '',
  motDePasse: '',
})

type Champ = 'prenom' | 'nom' | 'adresse' | 'pays' | 'motDePasse'
const fautes = reactive<Partial<Record<Champ, string>>>({})

const envoi = ref(false)
const panne = ref<string | null>(null)
/** L'adresse à laquelle le courriel est parti : l'écran d'attente la rappelle. */
const attente = ref<string | null>(null)
const renvoiFait = ref(false)

const pays = ref<{ id: CountryId; libelle: string }[]>([])

onMounted(async () => {
  session.relireAuRetourAuPremierPlan()
  try {
    const liste = await api.reference.countries()
    pays.value = liste
      .map((p) => ({ id: p.id, libelle: resolveI18nText(p.name) }))
      .sort((a, b) => a.libelle.localeCompare(b.libelle, 'fr'))
  } catch {
    // Le pays n'empêche pas de lire l'écran : la liste reste vide et le champ
    // se voit refusé à l'envoi, avec son message.
  }
})

/**
 * **L'inscription n'ouvre aucune session**, et c'est ce qui borne ce que cet
 * écran peut promettre. Tant que la personne ne s'est pas connectée, il n'y a
 * rien à relire : `/auth/me` ne rend rien, et aucune route ne dit « cette
 * adresse est-elle confirmée ? » — elle ne pourrait le dire qu'en révélant
 * qu'un compte existe.
 *
 * La suite après confirmation est donc **la connexion**, adresse reportée. Une
 * personne déjà connectée et confirmée, elle, passe directement.
 */
watch(
  () => session.compte.value.connectee && session.compte.value.adresseConfirmee,
  (prete) => {
    if (prete) void navigateTo(APRES_LE_COMPTE)
  },
)

async function apresConfirmation(): Promise<void> {
  await navigateTo({
    path: '/guide-nego/connexion',
    query: attente.value ? { adresse: attente.value } : undefined,
  })
}

function valider(): boolean {
  for (const champ of Object.keys(fautes) as Champ[]) delete fautes[champ]

  if (!fiche.prenom.trim()) fautes.prenom = t('guide-nego.compte.faute.prenom')
  if (!fiche.nom.trim()) fautes.nom = t('guide-nego.compte.faute.nom')
  if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(fiche.adresse.trim()))
    fautes.adresse = t('guide-nego.compte.faute.adresse')
  if (!fiche.pays) fautes.pays = t('guide-nego.compte.faute.pays')
  if (missingRequirements(fiche.motDePasse).length > 0)
    fautes.motDePasse = t('guide-nego.compte.faute.mot-de-passe', { min: MIN_PASSWORD_LENGTH })

  return Object.keys(fautes).length === 0
}

async function envoyer(): Promise<void> {
  panne.value = null
  if (!valider() || envoi.value) return

  envoi.value = true
  try {
    const reponse = await api.guideNego.inscription({
      first_name: fiche.prenom.trim(),
      last_name: fiche.nom.trim(),
      email: fiche.adresse.trim(),
      country_id: fiche.pays as CountryId,
      password: fiche.motDePasse,
      preferred_locale: 'fr',
      timezone: fuseauDuTelephone(),
    })
    attente.value = reponse.email
  } catch (erreur) {
    // Un message de l'API s'affiche TEL QUEL ; le nôtre ne parle que lorsque
    // l'API s'est tue.
    panne.value = messageDePanne(erreur)
  } finally {
    envoi.value = false
  }
}

async function renvoyer(): Promise<void> {
  if (!attente.value) return
  try {
    await api.guideNego.renvoyerLeCourriel(attente.value)
    renvoiFait.value = true
  } catch (erreur) {
    panne.value = messageDePanne(erreur)
  }
}

/**
 * « J'ai confirmé mon adresse ». On relit d'abord — une personne déjà connectée
 * n'a rien à ressaisir — puis on l'emmène se connecter, ce que l'inscription
 * seule ne fait pas.
 */
async function jaiConfirme(): Promise<void> {
  await session.rafraichir()
  if (session.compte.value.connectee && session.compte.value.adresseConfirmee) return
  await apresConfirmation()
}

function fuseauDuTelephone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
  } catch {
    return 'UTC'
  }
}

function messageDePanne(erreur: unknown): string {
  const message = erreur instanceof Error ? erreur.message : ''
  return message || t('api.unreachable.network')
}

useHead({ title: t('guide-nego.compte.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.compte.titre')"
    :sous-titre="t('guide-nego.compte.sous-titre')"
    retour="/guide-nego"
    :onglets="false"
  >
    <!-- Attente de confirmation : le même écran, une fois le courriel parti. -->
    <template v-if="attente">
      <GnEnteteGroupe :titre="t('guide-nego.compte.attente.titre')" picto="clock" />
      <p class="gn-compte__propos">{{ t('guide-nego.compte.attente.propos', { adresse: attente }) }}</p>
      <p class="gn-compte__propos gn-compte__propos--discret">
        {{ t('guide-nego.compte.attente.duree') }}
      </p>

      <div class="gn-compte__sorties">
        <GnBouton variante="principal" @clic="jaiConfirme">
          {{ t('guide-nego.compte.attente.jai-confirme') }}
        </GnBouton>
        <GnBouton variante="secondaire" :desactive="renvoiFait" @clic="renvoyer">
          {{ renvoiFait ? t('guide-nego.compte.attente.renvoye') : t('guide-nego.compte.attente.renvoyer') }}
        </GnBouton>
      </div>

      <p v-if="panne" class="gn-compte__panne" role="alert">{{ panne }}</p>
    </template>

    <!-- Le formulaire -->
    <template v-else>
      <GnEtapes :courante="1" :libelle="t('guide-nego.compte.etape')" />

      <form class="gn-compte__formulaire" novalidate @submit.prevent="envoyer">
        <GnChamp
          v-model="fiche.prenom"
          :libelle="t('guide-nego.compte.champ.prenom')"
          :erreur="fautes.prenom"
          autocomplete="given-name"
        />
        <GnChamp
          v-model="fiche.nom"
          :libelle="t('guide-nego.compte.champ.nom')"
          :erreur="fautes.nom"
          autocomplete="family-name"
        />
        <GnChamp
          v-model="fiche.adresse"
          type="email"
          :libelle="t('guide-nego.compte.champ.adresse')"
          :aide="t('guide-nego.compte.champ.adresse-aide')"
          :erreur="fautes.adresse"
          autocomplete="email"
          inputmode="email"
        />

        <!-- Le pays passe par le créneau de `GnChamp` : libellé, aide et erreur
             restent les siens, et la liste n'a pas de composant à elle. -->
        <GnChamp
          :model-value="String(fiche.pays)"
          :libelle="t('guide-nego.compte.champ.pays')"
          :erreur="fautes.pays"
        >
          <template #default="{ idSaisie, decritPar, invalide }">
            <select
              :id="idSaisie"
              v-model="fiche.pays"
              class="gn-compte__liste"
              :aria-describedby="decritPar"
              :aria-invalid="invalide"
              autocomplete="country"
            >
              <option value="" disabled>{{ t('guide-nego.compte.champ.pays-vide') }}</option>
              <option v-for="p in pays" :key="p.id" :value="p.id">{{ p.libelle }}</option>
            </select>
          </template>
        </GnChamp>

        <GnChamp
          v-model="fiche.motDePasse"
          type="password"
          :libelle="t('guide-nego.compte.champ.mot-de-passe')"
          :aide="t('guide-nego.compte.champ.mot-de-passe-aide')"
          :erreur="fautes.motDePasse"
          autocomplete="new-password"
        />

        <p v-if="panne" class="gn-compte__panne" role="alert">{{ panne }}</p>

        <div class="gn-compte__sorties">
          <GnBouton type="submit" variante="principal" :chargement="envoi">
            {{ t('guide-nego.compte.creer') }}
          </GnBouton>
          <GnBouton variante="discret" vers="/guide-nego/connexion">
            {{ t('guide-nego.compte.deja-un-compte') }}
          </GnBouton>
        </div>
      </form>
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-compte__formulaire {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-compte__propos {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-compte__propos--discret {
  font-size: var(--gn-taille-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-compte__panne {
  font-size: var(--gn-taille-15);
  color: var(--gn-danger);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-compte__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-compte__liste {
  min-height: var(--gn-champ);
  width: 100%;
  padding-inline: var(--gn-espace-12);
  font-family: var(--gn-police);
  font-size: var(--gn-taille-17);
  color: var(--gn-texte);
  background: var(--gn-fond);
  border: var(--gn-filet-2) solid var(--gn-filet);
  border-radius: var(--gn-rayon-4);
}

[data-app="guide-nego"] .gn-compte__liste:focus-visible {
  outline: var(--gn-focus-anneau) solid var(--gn-focus);
  outline-offset: var(--gn-focus-decalage);
  border-color: var(--gn-filet-fort);
}
</style>
