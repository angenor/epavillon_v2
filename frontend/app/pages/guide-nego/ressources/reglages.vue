<script setup lang="ts">
import type { SegmentDeChoix } from '~/components/guide-nego/GnSegmente.vue'
import type { ChoixDeTheme } from '~/utils/guide-nego/theme'
import { momentDeLecture } from '~/utils/guide-nego/connexion'
import { placeDesCopies } from '~/utils/guide-nego/mes-documents'
import { tailleLisible } from '~/utils/guide-nego/place'

/**
 * Écran 11 — « Profil et réglages ». En tête, le nom et le pays avec l'avatar ;
 * puis « Mon suivi », « Affichage », « Application », et le compte avec sa
 * déconnexion, tels que 0b les a posés ; « Notifications par thématique »
 * depuis 3b — une ligne allumée élargit, elle ne coupe jamais l'agenda (FR-031).
 *
 * LA LIGNE « MON ACCÈS » PORTE L'ÉTAT, JAMAIS UN RÔLE. La maquette écrit
 * « Négociatrice — réseau » en second rang : cette formule est genrée et ne se
 * reprend pas (SC-006). Ce qui s'affiche est l'état d'accès, le même mot que
 * l'écran vers lequel la ligne mène.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const { choix, choisir } = useGnTheme()
const session = useGnSession()
const acces = useGnAcces()
const thematiques = useGnThematiques()
const pays = useGnPays()
const connexion = useGnConnexion()
const copies = useGnCopies()
const reglageNotifications = useGnReglageNotifications()
const { momentLisible } = useGnMomentLecture()

const confirmation = ref(false)

/** Les suivies dont le nom est connu : jamais un code brut (FR-022). */
const thematiquesNotifiables = computed(() =>
  thematiques.mesCodes.value.flatMap((code) => {
    const nom = thematiques.nomDe(code)
    return nom ? [{ code, nom }] : []
  }),
)

function notifiee(code: string): boolean {
  return reglageNotifications.notifiees.value.includes(code)
}

function basculer(code: string, allumee: boolean): void {
  const autres = reglageNotifications.notifiees.value.filter((c) => c !== code)
  void reglageNotifications.notifierDesThematiques(allumee ? [...autres, code] : autres)
}

onMounted(async () => {
  session.relireAuRetourAuPremierPlan()
  void copies.recharger().catch(() => undefined)
  await session.assurer()
  if (!session.connectee.value) return
  void acces.assurer()
  void thematiques.assurer()
  void pays.assurer()
  reglageNotifications.assurer()
})

const compte = session.compte
const nomComplet = computed(() => [compte.value.prenom, compte.value.nom].filter(Boolean).join(' '))
const titre = computed(() =>
  session.connectee.value && nomComplet.value ? nomComplet.value : t('guide-nego.reglages.titre'),
)
const sousTitre = computed(() =>
  session.connectee.value ? (pays.nomDuPays(compte.value.paysId) ?? undefined) : undefined,
)
const avatarDuTitre = computed(() =>
  session.connectee.value ? { prenom: compte.value.prenom, nom: compte.value.nom } : undefined,
)

/**
 * Les noms croisés avec le vocabulaire gardé ; **le nombre** quand le vocabulaire
 * n'a jamais été lu — jamais une liste de codes bruts (FR-022).
 */
const valeurDesThematiques = computed(() => {
  const codes = thematiques.mesCodes.value
  if (codes.length === 0) return t('guide-nego.reglages.suivi.aucune-thematique')
  const { noms } = thematiques.resumeDe(codes)
  if (noms.length === codes.length) return noms.join(', ')
  return t('guide-nego.reglages.suivi.nombre-de-thematiques', { count: codes.length }, codes.length)
})

const valeurDesTelechargements = computed(() => {
  const n = copies.copies.value.length
  if (!n) return t('guide-nego.reglages.suivi.telechargements-vide')
  const place = tailleLisible(placeDesCopies(copies.copies.value), locale.value)
  return t('guide-nego.reglages.suivi.telechargements-place', { count: n, place }, n)
})

/** L'heure du téléphone, sans fuseau (écart 32) : une information, pas une action. */
const derniereSynchronisation = computed(() => {
  const luA = connexion.etat.value.luA
  if (!luA) return t('guide-nego.reglages.application.jamais')
  const moment = momentDeLecture(luA, new Date(), locale.value)
  return t(`guide-nego.reglages.application.moment.${moment.quand}`, {
    heure: moment.heure,
    jour: moment.quand === 'avant' ? moment.jour : '',
  })
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
    :titre="titre"
    :sous-titre="sousTitre"
    :avatar-du-titre="avatarDuTitre"
    retour="/guide-nego/ressources"
    :onglets="false"
  >
    <template v-if="session.connectee.value">
      <GnEnteteGroupe :titre="t('guide-nego.reglages.suivi.titre')" />
      <GnLigneReglage
        :libelle="t('guide-nego.reglages.suivi.thematiques')"
        :valeur="valeurDesThematiques"
        picto="filter"
        vers="/guide-nego/thematiques"
      />
      <GnLigneReglage
        :libelle="t('guide-nego.reglages.suivi.telechargements')"
        :valeur="valeurDesTelechargements"
        picto="download"
        vers="/guide-nego/ressources/mes-documents"
      />
      <GnLigneReglage
        :libelle="t('guide-nego.reglages.suivi.signalements')"
        picto="flag"
        vers="/guide-nego/negociations/signalements"
      />
      <!-- L'état, et non un rôle : « Négociatrice — réseau » de la maquette est
           genré et ne se reprend pas (SC-006). -->
      <GnLigneReglage
        :libelle="t('guide-nego.reglages.compte.acces')"
        :valeur="t(`guide-nego.acces.etat.${acces.acces.value.state}`)"
        picto="lock"
        vers="/guide-nego/ressources/acces"
        derniere
      />
    </template>

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
    <section class="gn-reglages__bloc">
      <h3 class="gn-reglages__libelle">{{ t('guide-nego.reglages.taille.libelle') }}</h3>
      <GnChoixTailleLecture :libelle="t('guide-nego.reglages.taille.libelle')" />
      <p class="gn-reglages__aide">{{ t('guide-nego.reglages.taille.aide') }}</p>
    </section>

    <template v-if="session.connectee.value">
      <GnEnteteGroupe :titre="t('guide-nego.reglages.notifications.titre')" />
      <p class="gn-reglages__aide gn-reglages__aide--espacee">{{ t('guide-nego.reglages.notifications.agenda') }}</p>
      <template v-if="thematiquesNotifiables.length">
        <GnInterrupteur
          v-for="(th, i) in thematiquesNotifiables"
          :key="th.code"
          :model-value="notifiee(th.code)"
          :libelle="th.nom"
          :detail="t('guide-nego.reglages.notifications.detail')"
          :derniere="i === thematiquesNotifiables.length - 1"
          @update:model-value="basculer(th.code, $event)"
        />
      </template>
      <GnLigneReglage
        v-else
        :libelle="t('guide-nego.reglages.notifications.choisir')"
        :valeur="t('guide-nego.reglages.notifications.choisir-detail')"
        picto="filter"
        vers="/guide-nego/thematiques"
        derniere
      />
    </template>

    <GnEnteteGroupe :titre="t('guide-nego.reglages.application.titre')" />
    <GnLigneReglage
      :libelle="t('guide-nego.reglages.application.a-propos')"
      picto="info"
      vers="/guide-nego/ressources/a-propos"
    />
    <GnLigneReglage
      :libelle="t('guide-nego.reglages.application.synchronisation')"
      :valeur="derniereSynchronisation"
      picto="sync"
      derniere
    />

    <!-- Le compte. Rien ici ne s'affiche quand personne n'est connecté : ce
         serait proposer de se déconnecter d'un compte qu'on n'a pas. -->
    <template v-if="session.connectee.value">
      <GnEnteteGroupe :titre="t('guide-nego.reglages.compte.titre')" picto="user" />
      <GnLigneReglage
        :libelle="session.compte.value.adresse ?? t('guide-nego.reglages.compte.sans-adresse')"
        :valeur="appareil ?? undefined"
        picto="user"
        derniere
      />
      <p v-if="luA" class="gn-reglages__aide">
        {{ t('guide-nego.reglages.compte.lu-a', { moment: luA }) }}
      </p>

      <div class="gn-reglages__sorties">
        <GnBouton variante="secondaire" picto="logout" @clic="confirmation = true">
          {{ t('guide-nego.reglages.compte.deconnexion') }}
        </GnBouton>
        <p class="gn-reglages__aide gn-reglages__aide--centree">
          {{ t('guide-nego.reglages.compte.telechargements-restent') }}
        </p>
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

[data-app="guide-nego"] .gn-reglages__aide--espacee {
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-reglages__aide--centree {
  text-align: center;
}
</style>
