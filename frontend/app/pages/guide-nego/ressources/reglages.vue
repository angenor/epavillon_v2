<script setup lang="ts">
import type { SegmentDeChoix } from '~/components/guide-nego/GnSegmente.vue'
import type { ChoixDeTheme } from '~/utils/guide-nego/theme'

/**
 * Le thème, le compte et la déconnexion. « Mon accès » s'y ajoute avec le récit
 * du verrou (US5) ; 0c apportera le reste.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { choix, choisir } = useGnTheme()
const session = useGnSession()
const { momentLisible } = useGnMomentLecture()

const confirmation = ref(false)

onMounted(() => {
  session.relireAuRetourAuPremierPlan()
  void session.assurer()
})

/**
 * Ce que la personne lit de son compte. **Hors connexion, ce qui a été lu avec
 * l'heure de sa lecture** — jamais un écran vide, jamais une reconnexion
 * réclamée sans réseau (FR-006 ter).
 */
const appareil = computed(() => session.compte.value.appareil)
const luA = computed(() => momentLisible(session.luA.value))

async function deconnecter(): Promise<void> {
  await session.deconnecter()
  await navigateTo('/guide-nego')
}

const segments = computed<SegmentDeChoix[]>(() => [
  { valeur: 'clair', libelle: t('guide-nego.reglages.theme.clair'), picto: 'sun' },
  { valeur: 'sombre', libelle: t('guide-nego.reglages.theme.sombre'), picto: 'moon' },
  { valeur: 'systeme', libelle: t('guide-nego.reglages.theme.systeme') },
])

const theme = computed({
  get: () => choix.value as string,
  set: (valeur: string) => choisir(valeur as ChoixDeTheme),
})

useHead({ title: t('guide-nego.reglages.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.reglages.titre')"
    retour="/guide-nego/ressources"
    :onglets="false"
  >
    <GnEnteteGroupe :titre="t('guide-nego.reglages.affichage')" />
    <section class="gn-reglages__bloc">
      <h3 class="gn-reglages__libelle">{{ t('guide-nego.reglages.theme.libelle') }}</h3>
      <GnSegmente
        v-model="theme"
        :segments="segments"
        :libelle="t('guide-nego.reglages.theme.libelle')"
      />
      <p class="gn-reglages__aide">{{ t('guide-nego.reglages.theme.aide') }}</p>
    </section>

    <!-- Le compte. Rien ici ne s'affiche quand personne n'est connecté : ce
         serait proposer de se déconnecter d'un compte qu'on n'a pas. -->
    <template v-if="session.connectee.value">
      <GnEnteteGroupe :titre="t('guide-nego.reglages.compte.titre')" picto="user" />
      <GnLigneReglage
        :libelle="session.compte.value.adresse ?? t('guide-nego.reglages.compte.sans-adresse')"
        :valeur="appareil ?? undefined"
        picto="user"
      />
      <p v-if="luA" class="gn-reglages__aide">
        {{ t('guide-nego.reglages.compte.lu-a', { moment: luA }) }}
      </p>

      <div class="gn-reglages__sorties">
        <GnBouton variante="secondaire" picto="logout" @clic="confirmation = true">
          {{ t('guide-nego.reglages.compte.deconnexion') }}
        </GnBouton>
      </div>

      <GnConfirmation
        v-model="confirmation"
        :question="t('guide-nego.reglages.compte.confirmation.question')"
        :phrase="t('guide-nego.reglages.compte.confirmation.phrase')"
        :action="t('guide-nego.reglages.compte.confirmation.action')"
        :retour="t('guide-nego.reglages.compte.confirmation.retour')"
        picto="logout"
        @confirmer="deconnecter"
      />
    </template>

    <template v-else>
      <GnEnteteGroupe :titre="t('guide-nego.reglages.compte.titre')" picto="user" />
      <div class="gn-reglages__sorties">
        <GnBouton variante="principal" vers="/guide-nego/compte">
          {{ t('guide-nego.reglages.compte.creer') }}
        </GnBouton>
        <GnBouton variante="discret" vers="/guide-nego/connexion">
          {{ t('guide-nego.reglages.compte.se-connecter') }}
        </GnBouton>
      </div>
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-reglages__bloc {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-reglages__libelle {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-reglages__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-reglages__aide {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}
</style>
