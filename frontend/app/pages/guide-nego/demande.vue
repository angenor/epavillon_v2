<script setup lang="ts">
import { APRES_LE_CODE } from '~/utils/guide-nego/parcours'

/**
 * Demander l'accès à l'IFDD — étape 2 sur 3, quand le mode l'exige.
 *
 * **Quatre écrans en un**, et l'état d'accès seul décide lequel : le formulaire
 * de demande, l'attente, le refus avec son motif, et le cas d'une personne qui
 * détient déjà l'accès. Un écran par état obligerait chaque page à connaître
 * l'ordre de priorité des cinq états, et la première qui l'oublierait montrerait
 * un formulaire à quelqu'un qui attend déjà une réponse.
 *
 * **L'ÉCRAN D'ATTENTE NE PROMET RIEN D'AUTRE QUE CE QU'IL SAIT** (FR-025). Il
 * dit le nom, l'heure d'envoi, que la réponse arrive par courriel, et ce qui
 * reste lisible en attendant — les trois agendas, le lexique, la
 * Francophonie. Il offre deux sorties : aller lire, ou saisir un code si on en
 * reçoit un entre-temps.
 *
 * **« ANNULÉE », JAMAIS « RÉVOQUÉE »** (FR-026). Retirer sa demande est son
 * propre fait ; « révoqué » qualifie un accès qu'un administrateur retire.
 *
 * **Hors connexion, l'envoi se refuse et le dit** (FR-019) : un accès n'est pas
 * un signalement, on ne peut pas l'annoncer avant de l'avoir obtenu.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const acces = useGnAcces()
const session = useGnSession()
const connexion = useGnConnexion()
const { momentLisible } = useGnMomentLecture()

const message = ref('')
const envoi = ref(false)
const panne = ref<string | null>(null)
const confirmation = ref(false)

onMounted(async () => {
  acces.relireAuRetourAuPremierPlan()
  await session.assurer()
  await acces.assurer()
})

useHead({ title: t('guide-nego.demande.titre') })

/** Sans compte, le parcours passe d'abord par le compte (FR-031). */
const compteManquant = computed(() => session.pret.value && !session.connectee.value)

const demande = computed(() => acces.demande.value)
const etat = computed(() => acces.acces.value.state)

const nomComplet = computed(() => {
  const { prenom, nom } = session.compte.value
  return [prenom, nom].filter(Boolean).join(' ')
})

async function envoyer(): Promise<void> {
  panne.value = null
  if (envoi.value) return

  envoi.value = true
  try {
    await acces.demanderLAcces(message.value)
  } catch (erreur) {
    // Une demande déjà en attente, un accès déjà détenu : l'API le dit en
    // français, et son message s'affiche tel quel.
    panne.value =
      erreur instanceof Error && erreur.message ? erreur.message : t('api.unreachable.network')
  } finally {
    envoi.value = false
  }
}

async function retirer(): Promise<void> {
  const enCours = demande.value
  if (!enCours || envoi.value) return

  panne.value = null
  envoi.value = true
  try {
    await acces.annulerSaDemande(enCours.id)
    confirmation.value = false
  } catch (erreur) {
    panne.value =
      erreur instanceof Error && erreur.message ? erreur.message : t('api.unreachable.network')
  } finally {
    envoi.value = false
  }
}
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.demande.titre')"
    :sous-titre="t('guide-nego.demande.sous-titre')"
    retour="/guide-nego"
    :onglets="false"
  >
    <GnEtapes :courante="2" :libelle="t('guide-nego.demande.etape')" />

    <!-- Sans compte, le parcours passe d'abord par le compte. -->
    <template v-if="compteManquant">
      <GnLigneInformation :texte="t('guide-nego.demande.sans-compte')" />
      <div class="gn-demande__sorties">
        <GnBouton variante="principal" vers="/guide-nego/compte">
          {{ t('guide-nego.demande.creer-mon-compte') }}
        </GnBouton>
        <GnBouton variante="discret" vers="/guide-nego/connexion">
          {{ t('guide-nego.demande.me-connecter') }}
        </GnBouton>
      </div>
    </template>

    <!-- L'accès est déjà ouvert : il n'y a rien à demander. -->
    <template v-else-if="etat === 'granted'">
      <GnEnteteGroupe :titre="t('guide-nego.demande.deja-admise.titre')" picto="check-circle" />
      <p class="gn-demande__propos">{{ t('guide-nego.demande.deja-admise.propos') }}</p>
      <div class="gn-demande__sorties">
        <GnBouton variante="principal" :vers="APRES_LE_CODE">
          {{ t('guide-nego.demande.deja-admise.continuer') }}
        </GnBouton>
      </div>
    </template>

    <!-- En attente : ce qu'on sait, et deux sorties. -->
    <template v-else-if="demande && demande.status === 'pending'">
      <GnEnteteGroupe :titre="t('guide-nego.demande.attente.titre')" picto="clock" />

      <GnLigneInformation
        v-if="nomComplet"
        :texte="t('guide-nego.demande.attente.nom', { nom: nomComplet })"
      />
      <GnLigneInformation
        :texte="
          t('guide-nego.demande.attente.envoyee', {
            moment: momentLisible(demande.submitted_at) ?? '',
          })
        "
      />

      <p class="gn-demande__propos">{{ t('guide-nego.demande.attente.courriel') }}</p>
      <p class="gn-demande__propos gn-demande__propos--discret">
        {{ t('guide-nego.demande.attente.reste-lisible') }}
      </p>

      <p v-if="panne" class="gn-demande__panne" role="alert">
        <GnPicto nom="warn" :taille="20" />
        <span>{{ panne }}</span>
      </p>

      <div class="gn-demande__sorties">
        <GnBouton variante="principal" :vers="APRES_LE_CODE">
          {{ t('guide-nego.demande.attente.ma-journee') }}
        </GnBouton>
        <GnBouton v-if="acces.codeOffert.value" variante="secondaire" vers="/guide-nego/code">
          {{ t('guide-nego.demande.attente.jai-un-code') }}
        </GnBouton>
        <GnBouton variante="discret" @clic="confirmation = true">
          {{ t('guide-nego.demande.attente.annuler') }}
        </GnBouton>
      </div>

      <GnConfirmation
        v-model="confirmation"
        :question="t('guide-nego.demande.annulation.question')"
        :phrase="t('guide-nego.demande.annulation.phrase')"
        :action="t('guide-nego.demande.annulation.action')"
        :retour="t('guide-nego.demande.annulation.retour')"
        picto="warn"
        @confirmer="retirer"
      />
    </template>

    <!-- Refusée : le motif tel que l'administrateur l'a écrit, et ce qui reste. -->
    <template v-else-if="etat === 'rejected'">
      <GnEnteteGroupe :titre="t('guide-nego.demande.refusee.titre')" picto="x-circle" />
      <p class="gn-demande__propos">
        {{
          demande?.decision_reason
            ? t('guide-nego.demande.refusee.motif', { motif: demande.decision_reason })
            : t('guide-nego.demande.refusee.sans-motif')
        }}
      </p>
      <p class="gn-demande__propos gn-demande__propos--discret">
        {{ t('guide-nego.demande.refusee.suite') }}
      </p>

      <div class="gn-demande__sorties">
        <GnBouton v-if="acces.codeOffert.value" variante="principal" vers="/guide-nego/code">
          {{ t('guide-nego.demande.refusee.jai-un-code') }}
        </GnBouton>
        <GnBouton
          variante="secondaire"
          :chargement="envoi"
          :desactive="!acces.demandePossibleMaintenant.value"
          @clic="envoyer"
        >
          {{ t('guide-nego.demande.refusee.redemander') }}
        </GnBouton>
        <GnBouton variante="discret" :vers="APRES_LE_CODE">
          {{ t('guide-nego.demande.attente.ma-journee') }}
        </GnBouton>
      </div>
    </template>

    <!-- Le formulaire : le premier envoi. -->
    <template v-else>
      <p class="gn-demande__propos">{{ t('guide-nego.demande.propos') }}</p>

      <form novalidate @submit.prevent="envoyer">
        <GnZoneTexte
          v-model="message"
          :libelle="t('guide-nego.demande.message.libelle')"
          :aide="t('guide-nego.demande.message.aide')"
          :indication="t('guide-nego.demande.message.indication')"
          :desactive="!connexion.etat.value.enLigne"
        />

        <p v-if="panne" class="gn-demande__panne" role="alert">
          <GnPicto nom="warn" :taille="20" />
          <span>{{ panne }}</span>
        </p>

        <GnLigneInformation
          v-if="!connexion.etat.value.enLigne"
          :texte="t('guide-nego.demande.hors-connexion')"
        />

        <div class="gn-demande__sorties">
          <GnBouton
            type="submit"
            variante="principal"
            :chargement="envoi"
            :desactive="!connexion.etat.value.enLigne"
          >
            {{ t('guide-nego.demande.envoyer') }}
          </GnBouton>
          <GnBouton v-if="acces.codeOffert.value" variante="discret" vers="/guide-nego/code">
            {{ t('guide-nego.demande.attente.jai-un-code') }}
          </GnBouton>
        </div>
      </form>
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-demande__propos {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-demande__propos--discret {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-demande__panne {
  display: flex;
  gap: var(--gn-espace-8);
  align-items: flex-start;
  padding-top: var(--gn-espace-8);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-demande__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}
</style>
