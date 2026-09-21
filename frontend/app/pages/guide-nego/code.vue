<script setup lang="ts">
import { APRES_LE_CODE } from '~/utils/guide-nego/parcours'
import type { RedeemResult } from '~/types/negotiation'

/**
 * Écrans 04a, 04b et 04c — saisir le code d'invitation, étape 2 sur 3.
 *
 * **Les neuf issues s'affichent telles que l'API les formule** (FR-020). Elle
 * seule connaît la date de révocation et le temps d'attente restant ; un second
 * catalogue ici donnerait deux textes pour un même refus, et le second se
 * périmerait au premier changement. L'écran choisit seulement le ton — réussi,
 * à corriger, refusé — et les sorties.
 *
 * **Aucune issue ne laisse l'écran sans suite** (FR-015) : il reste toujours au
 * moins « Continuer » ou « Plus tard », et le champ se ressaisit.
 *
 * **Hors connexion, la saisie se refuse et le dit** (FR-019). Rien n'est mis en
 * file : un accès n'est pas un signalement, on ne peut pas l'annoncer avant de
 * l'avoir obtenu.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const acces = useGnAcces()
const session = useGnSession()
const connexion = useGnConnexion()

const saisie = ref('')
const envoi = ref(false)
const resultat = ref<RedeemResult | null>(null)
const panne = ref<string | null>(null)

onMounted(async () => {
  acces.relireAuRetourAuPremierPlan()
  await session.assurer()
  await acces.assurer()
})

/**
 * Le verrou vu sans compte mène d'abord à la création de compte ou à la
 * connexion (FR-031) : saisir un code sans session n'ouvrirait rien, et le
 * refus viendrait de l'API sans que l'écran ait rien expliqué.
 */
const compteManquant = computed(() => session.pret.value && !session.connectee.value)

/** Le ton du message, et lui seul : le texte vient de l'API. */
const ton = computed(() => {
  if (!resultat.value) return null
  switch (resultat.value.issue) {
    case 'accepted':
    case 'already_granted':
      return 'succes' as const
    case 'pending_approval':
      return 'attente' as const
    case 'unknown':
    case 'not_yet_valid':
      return 'attention' as const
    default:
      return 'refus' as const
  }
})

const picto = computed(() => {
  switch (ton.value) {
    case 'succes':
      return 'check-circle' as const
    case 'attente':
      return 'clock' as const
    case 'attention':
      return 'warn' as const
    default:
      return 'x-circle' as const
  }
})

/** L'accès est ouvert : la seule issue qui fait avancer le parcours. */
const entree = computed(
  () => resultat.value?.issue === 'accepted' || resultat.value?.issue === 'already_granted',
)

async function envoyer(): Promise<void> {
  panne.value = null
  if (envoi.value || !saisie.value.trim()) return

  envoi.value = true
  try {
    resultat.value = await acces.saisirLeCode(saisie.value)
  } catch (erreur) {
    // Une vraie panne — API injoignable, délai dépassé. Les refus du parcours,
    // eux, arrivent en réponse et non en exception.
    panne.value = erreur instanceof Error && erreur.message ? erreur.message : t('api.unreachable.network')
  } finally {
    envoi.value = false
  }
}

async function continuer(): Promise<void> {
  await navigateTo(APRES_LE_CODE)
}

useHead({ title: t('guide-nego.code.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.code.titre')"
    :sous-titre="t('guide-nego.code.sous-titre')"
    retour="/guide-nego"
    :onglets="false"
  >
    <GnEtapes :courante="2" :libelle="t('guide-nego.code.etape')" />

    <p class="gn-code__propos">{{ t('guide-nego.code.propos') }}</p>

    <!-- Sans compte, le parcours passe d'abord par le compte. -->
    <template v-if="compteManquant">
      <GnLigneInformation :texte="t('guide-nego.code.sans-compte')" />
      <div class="gn-code__sorties">
        <GnBouton variante="principal" vers="/guide-nego/compte">
          {{ t('guide-nego.code.creer-mon-compte') }}
        </GnBouton>
        <GnBouton variante="discret" vers="/guide-nego/connexion">
          {{ t('guide-nego.code.me-connecter') }}
        </GnBouton>
      </div>
    </template>

    <template v-else>
      <form novalidate @submit.prevent="envoyer">
        <GnChamp
          v-model="saisie"
          :libelle="t('guide-nego.code.champ.libelle')"
          :aide="t('guide-nego.code.champ.aide')"
          :indication="t('guide-nego.code.champ.indication')"
          :desactive="!acces.saisieDeCodePossible.value"
          autocomplete="off"
          autocapitalize="characters"
          spellcheck="false"
        />

        <!-- Le message de l'API, tel quel. Le ton et le pictogramme sont à nous. -->
        <p
          v-if="resultat"
          class="gn-code__issue"
          :class="`gn-code__issue--${ton}`"
          role="status"
        >
          <GnPicto :nom="picto" :taille="20" />
          <span>{{ resultat.message }}</span>
        </p>

        <p v-if="panne" class="gn-code__issue gn-code__issue--refus" role="alert">
          <GnPicto nom="warn" :taille="20" />
          <span>{{ panne }}</span>
        </p>

        <!-- Hors connexion : la saisie demande le réseau, et RIEN n'est promis. -->
        <GnLigneInformation
          v-if="!connexion.etat.value.enLigne"
          :texte="t('guide-nego.code.hors-connexion')"
        />

        <div class="gn-code__sorties">
          <GnBouton
            v-if="entree"
            variante="principal"
            @clic="continuer"
          >
            {{ t('guide-nego.code.continuer') }}
          </GnBouton>
          <GnBouton
            v-else
            type="submit"
            variante="principal"
            :chargement="envoi"
            :desactive="!acces.saisieDeCodePossible.value || !saisie.trim()"
          >
            {{ t('guide-nego.code.valider') }}
          </GnBouton>

          <GnBouton variante="discret" :vers="APRES_LE_CODE">
            {{ t('guide-nego.code.plus-tard') }}
          </GnBouton>
        </div>
      </form>

      <!-- « Vous n'avez pas de code ? » — la demande à l'IFDD arrive avec US4
           (T093) ; l'aide dit dès maintenant où le code se trouve. -->
      <GnEnteteGroupe :titre="t('guide-nego.code.pas-de-code.titre')" picto="info" />
      <p class="gn-code__propos gn-code__propos--discret">
        {{ t('guide-nego.code.pas-de-code.propos') }}
      </p>
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-code__propos {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-code__propos--discret {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-code__issue {
  display: flex;
  gap: var(--gn-espace-8);
  align-items: flex-start;
  padding-top: var(--gn-espace-8);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-code__issue--succes {
  color: var(--gn-succes);
}

[data-app="guide-nego"] .gn-code__issue--attente {
  color: var(--gn-information);
}

[data-app="guide-nego"] .gn-code__issue--attention {
  color: var(--gn-attention-aplat-texte);
}

[data-app="guide-nego"] .gn-code__issue--refus {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-code__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}
</style>
