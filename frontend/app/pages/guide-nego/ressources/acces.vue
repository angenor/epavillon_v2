<script setup lang="ts">
import type { NomDEtat } from '~/utils/guide-nego/etats'
import type { AccessState } from '~/types/negotiation'

/**
 * « Mon accès » — les cinq états, et ce que l'accès ouvre.
 *
 * **LA MAQUETTE NE DESSINE PAS CET ÉCRAN.** Elle en dessine la ligne de menu
 * qui y mène, et un seul de ses états — « 05 Demande en attente », dont la
 * table de faits bordée sert ici de patron. Les quatre autres se composent avec
 * les composants de 0a, sans rien redessiner.
 *
 * **RIEN ICI NE NOMME NI NE SUPPOSE UN GENRE** (SC-006). La maquette écrit
 * « Négociatrice — réseau » sous la ligne de menu : cette formule ne se reprend
 * pas. L'appartenance à un réseau vient du code utilisé, et de rien d'autre ;
 * son libellé vient de la taxonomie, jamais d'un fichier de traduction.
 *
 * **HORS CONNEXION, L'ÉTAT LU S'AFFICHE AVEC SON HEURE** (FR-034, principe XI).
 * Jamais un écran vide, et jamais la prétention d'un accès : `state` reste ce
 * que la dernière lecture disait. L'heure est celle du téléphone, sans fuseau
 * (écart 32) — elle se juge contre l'horloge affichée au-dessus.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const acces = useGnAcces()
const session = useGnSession()
const connexion = useGnConnexion()
const { momentLisible } = useGnMomentLecture()
const { date } = useDateTime()

onMounted(async () => {
  acces.relireAuRetourAuPremierPlan()
  await session.assurer()
  await acces.assurer()
})

useHead({ title: t('guide-nego.acces.titre') })

const etat = computed<AccessState>(() => acces.acces.value.state)
const accorde = computed(() => acces.acces.value.granted)
const demande = computed(() => acces.demande.value)
const luA = computed(() => momentLisible(acces.luA.value))

/**
 * La marque d'état, prise dans la table des états de 0a — un état = un
 * pictogramme, un mot, une couleur. Rien n'est inventé ici.
 */
const MARQUE: Record<AccessState, NomDEtat> = {
  visitor: 'acces-limite',
  pending: 'en-attente',
  granted: 'ouverte',
  rejected: 'non-retenu',
  revoked: 'acces-limite',
}

/**
 * Ce que l'accès ouvre : la COP nommée, ou Guide Négo en entier. **Le nom vient
 * de l'API**, qui l'a résolu dans la langue de la lecture — l'écran ne connaît
 * aucun nom d'espace.
 */
const ouvre = computed(() => {
  const portee = accorde.value?.scope
  if (!portee) return null
  return portee.type === 'global' ? t('guide-nego.acces.ouvre.tout') : portee.name
})

/** La date de l'admission, dans le fuseau du téléphone : c'est un fait du compte,
 *  pas un créneau d'événement — il n'a pas de fuseau propre à porter. */
const fuseau = computed(
  () => (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC') as never,
)
const admiseLe = computed(() =>
  accorde.value ? date(accorde.value.granted_at, fuseau.value) : null,
)
const trancheeLe = computed(() =>
  demande.value?.decided_at ? date(demande.value.decided_at, fuseau.value) : null,
)
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.acces.titre')"
    retour="/guide-nego/ressources/reglages"
    :onglets="false"
  >
    <GnChargement v-if="!acces.pret.value" :libelle="t('guide-nego.acces.chargement')" />

    <template v-else>
      <!-- L'état, d'abord et en un coup d'œil. -->
      <div class="gn-acces__etat">
        <GnMarqueEtat :etat="MARQUE[etat]" :libelle="t(`guide-nego.acces.etat.${etat}`)" />
      </div>

      <p class="gn-acces__propos">{{ t(`guide-nego.acces.propos.${etat}`) }}</p>

      <!-- Hors connexion, on dit CE QU'ON SAIT et QUAND on l'a su. -->
      <GnLigneInformation
        v-if="!connexion.etat.value.enLigne && luA"
        :texte="t('guide-nego.acces.hors-connexion', { moment: luA })"
      />

      <!-- Ce que l'accès ouvre. Une COP nommée n'est pas tout Guide Négo, et la
           personne doit savoir lequel des deux elle détient. -->
      <template v-if="etat === 'granted' && accorde">
        <GnEnteteGroupe :titre="t('guide-nego.acces.ouvre.titre')" picto="unlock" />
        <GnLigneReglage :libelle="ouvre ?? t('guide-nego.acces.ouvre.tout')" picto="unlock" />
        <GnLigneReglage
          v-if="admiseLe"
          :libelle="t('guide-nego.acces.admise-le')"
          :valeur="admiseLe"
          picto="calendar"
        />
        <GnLigneReglage
          v-if="accorde.source_code_label"
          :libelle="t('guide-nego.acces.par-le-code')"
          :valeur="accorde.source_code_label"
          picto="lock"
          derniere
        />
      </template>

      <!-- L'appartenance de réseau. Elle n'ouvre aucun droit à cette étape, et
           l'écran le dit plutôt que de laisser croire le contraire. -->
      <template v-if="acces.reseaux.value.length > 0">
        <GnEnteteGroupe :titre="t('guide-nego.acces.reseaux.titre')" picto="user" />
        <GnLigneReglage
          v-for="(reseau, index) in acces.reseaux.value"
          :key="reseau.code"
          :libelle="reseau.label"
          picto="check-circle"
          :derniere="index === acces.reseaux.value.length - 1"
        />
        <p class="gn-acces__aide">{{ t('guide-nego.acces.reseaux.aide') }}</p>
      </template>

      <!-- La demande, quelle que soit son issue : l'attente s'y lit, et le
           motif d'un refus aussi. -->
      <template v-if="demande">
        <GnEnteteGroupe :titre="t('guide-nego.acces.demande.titre')" picto="send" />
        <GnLigneReglage
          :libelle="t('guide-nego.acces.demande.envoyee')"
          :valeur="momentLisible(demande.submitted_at) ?? undefined"
          picto="clock"
        />
        <GnLigneReglage
          v-if="trancheeLe"
          :libelle="t(`guide-nego.acces.demande.${demande.status}`)"
          :valeur="trancheeLe"
          picto="check"
          :derniere="!demande.decision_reason"
        />
        <GnLigneReglage
          v-if="demande.decision_reason"
          :libelle="t('guide-nego.acces.demande.motif')"
          :valeur="demande.decision_reason"
          picto="info"
          derniere
        />
      </template>

      <!-- Une suite, toujours : aucun des cinq états ne laisse l'écran sans
           action possible. -->
      <div class="gn-acces__sorties">
        <GnBouton
          v-if="etat !== 'granted' && etat !== 'pending' && acces.codeOffert.value"
          variante="principal"
          picto="lock"
          vers="/guide-nego/code"
        >
          {{ t('guide-nego.acces.saisir') }}
        </GnBouton>
        <GnBouton
          v-if="etat === 'pending' || acces.demandePossibleMaintenant.value"
          :variante="acces.codeOffert.value ? 'secondaire' : 'principal'"
          picto="send"
          vers="/guide-nego/demande"
        >
          {{
            etat === 'pending'
              ? t('guide-nego.acces.voir-ma-demande')
              : t('guide-nego.acces.demander')
          }}
        </GnBouton>
        <GnBouton variante="discret" vers="/guide-nego">
          {{ t('guide-nego.acces.ma-journee') }}
        </GnBouton>
      </div>
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-acces__etat {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-acces__propos {
  padding-top: var(--gn-espace-8);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-acces__aide {
  padding-top: var(--gn-espace-8);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-acces__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}
</style>
