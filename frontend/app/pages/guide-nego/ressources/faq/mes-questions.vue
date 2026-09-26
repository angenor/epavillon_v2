<script setup lang="ts">
import type { NomDEtat } from '~/utils/guide-nego/etats'
import type { ExpertQuestionStatus } from '~/types/negotiation-savoir'
import { moduleOuvert } from '~/utils/guide-nego/verrou'

/**
 * « Mes questions » (FR-023) : les questions de la personne, leur état et, une fois
 * répondues, la réponse signée et datée. Lisible sans réseau une fois lue ; celles
 * qui attendent le réseau s'affichent en tête. Pas maquetté : composé sur « 03b ».
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const acces = useGnAcces()
const session = useGnSession()
const connexion = useGnConnexion()
const questions = useGnQuestions()
const { edition, rafraichir: lireLEdition } = useGnEdition()
const { dayLong, timeWithZone } = useDateTime()

onMounted(async () => {
  void lireLEdition()
  await session.assurer()
  await acces.assurer()
  if (ouvert.value) questions.assurer()
})

const pret = computed(() => session.pret.value && acces.pret.value)
// Après une déconnexion hors connexion, l'accès gardé dit encore « ouvert » : le compte tranche.
const ouvert = computed(() => session.connectee.value && moduleOuvert(acces.acces.value))
const liste = computed(() => questions.questions.value)
const enFile = computed(() => questions.enFile.value)
const total = computed(() => liste.value.length + enFile.value.length)
const chargement = computed(() => !questions.pret.value && !questions.connues.value)
const jamaisLue = computed(() => questions.pret.value && !questions.connues.value && enFile.value.length === 0)

const ETATS: Record<ExpertQuestionStatus, NomDEtat> = {
  pending: 'envoye',
  answered: 'repondue',
  added_to_faq: 'ajoutee-faq',
}

const fuseau = computed(() => edition.value?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone)
const moment = (instant: string) =>
  `${dayLong(instant, fuseau.value)}, ${timeWithZone(instant, fuseau.value, edition.value?.city ?? undefined)}`

useHead({ title: t('guide-nego.mes-questions.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.mes-questions.titre')"
    :sous-titre="ouvert && total ? t('guide-nego.mes-questions.compte', { count: total }, total) : undefined"
    retour="/guide-nego/ressources/faq"
  >
    <template v-if="ouvert" #connexion>
      <GnLigneConnexion :en-ligne="connexion.etat.value.enLigne" :lu-a="questions.luA.value" />
    </template>

    <GnChargement v-if="!pret" forme="squelette" :lignes="4" :libelle="t('guide-nego.mes-questions.chargement')" />

    <GnVerrou
      v-else-if="!ouvert"
      :propos="t('guide-nego.question.verrou.propos')"
      :reste-ouvert="t('guide-nego.mes-questions.reste')"
      :retour="t('guide-nego.question.revenir')"
      retour-vers="/guide-nego/ressources/faq"
    />

    <GnChargement v-else-if="chargement" forme="squelette" :lignes="4" :libelle="t('guide-nego.mes-questions.chargement')" />

    <GnEtatErreur
      v-else-if="jamaisLue"
      :titre="t('guide-nego.mes-questions.erreur.titre')"
      :texte="t('guide-nego.mes-questions.erreur.texte')"
    />

    <div v-else class="gn-mes-questions">
      <template v-if="total === 0">
        <GnEtatVide
          picto="quiz"
          :titre="t('guide-nego.mes-questions.vide.titre')"
          :texte="t('guide-nego.mes-questions.vide.texte')"
        />
      </template>

      <ul v-else role="list" class="gn-mes-questions__liste">
        <li v-for="q in enFile" :key="q.entree.client_ref" class="gn-mes-questions__carte">
          <div class="gn-mes-questions__tete">
            <GnMarqueEtat etat="a-envoyer" />
            <span class="gn-mes-questions__quand">{{ moment(q.prise_a) }}</span>
          </div>
          <p class="gn-mes-questions__question">{{ q.entree.body }}</p>
          <p class="gn-mes-questions__note">{{ t('guide-nego.mes-questions.attente-reseau') }}</p>
        </li>

        <li v-for="q in liste" :key="q.id" class="gn-mes-questions__carte">
          <div class="gn-mes-questions__tete">
            <GnMarqueEtat :etat="ETATS[q.status]" />
            <span class="gn-mes-questions__quand">{{ q.theme_label }} · {{ moment(q.created_at) }}</span>
          </div>
          <p class="gn-mes-questions__question">{{ q.body }}</p>
          <div v-if="q.answer" class="gn-mes-questions__reponse">
            <h3 class="gn-mes-questions__intitule">{{ t('guide-nego.mes-questions.reponse') }}</h3>
            <p class="gn-mes-questions__texte">{{ q.answer }}</p>
            <p class="gn-mes-questions__signature">
              <GnPicto nom="shield-check" :taille="16" />
              <span>
                {{ q.answered_by_name ? t('guide-nego.mes-questions.signature', { nom: q.answered_by_name }) : t('guide-nego.mes-questions.signature-sans-nom') }}
                <template v-if="q.answered_at"> — {{ moment(q.answered_at) }}</template>
              </span>
            </p>
          </div>
        </li>
      </ul>

      <div class="gn-mes-questions__pied">
        <GnBouton variante="secondaire" picto="send" vers="/guide-nego/ressources/faq/question">
          {{ t('guide-nego.faq.expert') }}
        </GnBouton>
      </div>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-mes-questions {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-mes-questions__liste {
  display: flex;
  flex-direction: column;
  list-style: none;
}

[data-app="guide-nego"] .gn-mes-questions__carte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-block: var(--gn-espace-16);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-mes-questions__tete {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-mes-questions__quand,
[data-app="guide-nego"] .gn-mes-questions__note,
[data-app="guide-nego"] .gn-mes-questions__signature {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-mes-questions__question {
  font-weight: var(--gn-graisse-demi-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-mes-questions__reponse {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-inline-start: var(--gn-espace-12);
  border-inline-start: var(--gn-filet-3) solid var(--gn-etat-repondue);
}

[data-app="guide-nego"] .gn-mes-questions__intitule {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-mes-questions__texte {
  white-space: pre-line;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-mes-questions__signature {
  display: flex;
  gap: var(--gn-espace-8);
  align-items: flex-start;
}

[data-app="guide-nego"] .gn-mes-questions__signature .gn-picto {
  flex-shrink: 0;
  margin-top: 2px;
  color: var(--gn-etat-expert);
}
</style>
