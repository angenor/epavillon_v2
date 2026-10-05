<script setup lang="ts">
import type { QuestionEnFile } from '~/composables/guide-nego/useGnQuestions'
import { moduleOuvert } from '~/utils/guide-nego/verrou'

/**
 * Poser une question à un expert — maquette 05, écrans 03, 03b et 03c. Réservé à
 * l'accès négociateur : sans lui, le verrou de 0b. La question part par la file :
 * sans réseau, l'écran « Envoyé » dit qu'elle partira au retour. Aucun délai promis
 * (FR-022).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const acces = useGnAcces()
const session = useGnSession()
const savoir = useGnSavoir()
const thematiques = useGnThematiques()
const questions = useGnQuestions()
const { edition, rafraichir: lireLEdition } = useGnEdition()
const { dayLong, timeWithZone } = useDateTime()

const MAXIMUM = 600
const theme = ref('')
const texte = ref('')
const consentement = ref(true)
const envoi = ref(false)
const envoyee = ref<QuestionEnFile | null>(null)

onMounted(async () => {
  void lireLEdition()
  await session.assurer()
  await acces.assurer()
  await Promise.all([
    session.connectee.value ? thematiques.assurer() : thematiques.assurerLeVocabulaire(),
    savoir.assurer(),
  ])
  if (ouvert.value) questions.assurer()
  if (!theme.value) theme.value = thematiques.mesCodes.value[0] ?? ''
})

const pret = computed(() => session.pret.value && acces.pret.value)
// Après une déconnexion hors connexion, l'accès gardé dit encore « ouvert » : le compte tranche.
const ouvert = computed(() => session.connectee.value && moduleOuvert(acces.acces.value))
const reseau = computed(() => acces.reseaux.value[0]?.label ?? null)
const choix = computed(() =>
  thematiques.thematiques.value.map((th) => ({ code: th.code, libelle: thematiques.nomDe(th.code) ?? th.code })),
)
const envoyable = computed(() => theme.value !== '' && texte.value.trim() !== '' && [...texte.value].length <= MAXIMUM)

async function envoyer(): Promise<void> {
  if (!envoyable.value || envoi.value) return
  envoi.value = true
  try {
    envoyee.value = await questions.poser(theme.value, texte.value, consentement.value)
  } finally {
    envoi.value = false
  }
}

function uneAutre(): void {
  envoyee.value = null
  texte.value = ''
  consentement.value = true
}

const partie = computed(() => (envoyee.value ? questions.partie(envoyee.value.entree.client_ref) : null))
const fuseau = computed(() => edition.value?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone)
const quand = computed(() => {
  const instant = partie.value?.created_at ?? envoyee.value?.prise_a
  if (!instant) return null
  return { jour: dayLong(instant, fuseau.value), heure: timeWithZone(instant, fuseau.value, edition.value?.city ?? undefined) }
})
const nomDuTheme = computed(() => (envoyee.value ? thematiques.nomDe(envoyee.value.entree.theme_code) : null))

watch(locale, () => void thematiques.assurerLeVocabulaire())

useHead({ title: t('guide-nego.question.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.question.titre')"
    :sous-titre="t('guide-nego.question.sous-titre')"
    retour="/guide-nego/ressources/faq"
  >
    <GnChargement v-if="!pret" forme="squelette" :lignes="4" :libelle="t('guide-nego.question.chargement')" />

    <GnVerrou
      v-else-if="!ouvert"
      :propos="t('guide-nego.question.verrou.propos')"
      :reste-ouvert="t('guide-nego.question.verrou.reste', { count: savoir.faq.value.length }, savoir.faq.value.length)"
      :retour="t('guide-nego.question.revenir')"
      retour-vers="/guide-nego/ressources/faq"
    />

    <div v-else-if="envoyee" class="gn-question gn-question--envoyee">
      <GnPicto :nom="partie ? 'send' : 'clock'" :taille="40" class="gn-question__signe" />
      <h2 class="gn-question__annonce" role="status">
        {{ partie ? t('guide-nego.question.envoyee.titre') : t('guide-nego.question.envoyee.titre-attente') }}
      </h2>
      <dl class="gn-question__recap">
        <div class="gn-question__ligne gn-question__ligne--pile">
          <dt>{{ t('guide-nego.question.envoyee.question') }}</dt>
          <dd>{{ envoyee.entree.body }}</dd>
        </div>
        <div class="gn-question__ligne">
          <dt>{{ t('guide-nego.question.envoyee.thematique') }}</dt>
          <dd>{{ partie?.theme_label ?? nomDuTheme ?? envoyee.entree.theme_code }}</dd>
        </div>
        <div v-if="quand" class="gn-question__ligne">
          <dt>{{ partie ? t('guide-nego.question.envoyee.envoyee') : t('guide-nego.question.envoyee.ecrite') }}</dt>
          <dd>{{ quand.jour }}<br><span class="gn-question__zone">{{ quand.heure }}</span></dd>
        </div>
        <div class="gn-question__ligne">
          <dt>{{ t('guide-nego.question.envoyee.etat') }}</dt>
          <dd><GnMarqueEtat :etat="partie ? 'envoye' : 'a-envoyer'" /></dd>
        </div>
      </dl>
      <p class="gn-question__aide">
        {{ envoyee.entree.consent_to_faq ? t('guide-nego.question.envoyee.suite-faq') : t('guide-nego.question.envoyee.suite') }}
      </p>
      <div class="gn-question__sorties">
        <GnBouton variante="principal" vers="/guide-nego/ressources/faq">{{ t('guide-nego.question.revenir') }}</GnBouton>
        <GnBouton variante="discret" vers="/guide-nego/ressources/faq/mes-questions">
          {{ t('guide-nego.question.mes-questions') }}
        </GnBouton>
        <GnBouton variante="discret" @clic="uneAutre">{{ t('guide-nego.question.une-autre') }}</GnBouton>
      </div>
    </div>

    <form v-else class="gn-question" @submit.prevent="envoyer">
      <p class="gn-question__badge">
        <GnPicto nom="shield-check" :taille="16" />
        {{ reseau ? t('guide-nego.question.badge-reseau', { reseau }) : t('guide-nego.question.badge') }}
      </p>
      <p class="gn-question__aide">{{ t('guide-nego.question.propos') }}</p>

      <GnChamp :model-value="theme" :libelle="t('guide-nego.question.thematique')">
        <template #default="{ idSaisie, decritPar, invalide }">
          <select
            :id="idSaisie"
            v-model="theme"
            class="gn-champ__saisie gn-question__liste"
            :aria-describedby="decritPar"
            :aria-invalid="invalide"
          >
            <option value="" disabled>{{ t('guide-nego.question.thematique-vide') }}</option>
            <option v-for="c in choix" :key="c.code" :value="c.code">{{ c.libelle }}</option>
          </select>
        </template>
      </GnChamp>

      <GnZoneTexte
        v-model="texte"
        :libelle="t('guide-nego.question.champ')"
        :indication="t('guide-nego.question.indication')"
        :maximum="MAXIMUM"
      />

      <GnCase v-model="consentement" :libelle="t('guide-nego.question.consentement')" derniere />

      <div class="gn-question__pied">
        <GnBouton type="submit" picto="send" :desactive="!envoyable" :chargement="envoi">
          {{ t('guide-nego.question.envoyer') }}
        </GnBouton>
        <p class="gn-question__note">{{ t('guide-nego.question.sans-reseau') }}</p>
        <GnBouton variante="discret" vers="/guide-nego/ressources/faq/mes-questions">
          {{ t('guide-nego.question.mes-questions') }}
        </GnBouton>
      </div>
    </form>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-question {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-20);
}

[data-app="guide-nego"] .gn-question__badge {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-etat-verifie);
}

[data-app="guide-nego"] .gn-question__badge + .gn-question__aide {
  margin-top: calc(-1 * var(--gn-espace-12));
}

[data-app="guide-nego"] .gn-question__aide,
[data-app="guide-nego"] .gn-question__note {
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-question__note {
  text-align: center;
}

/* La liste prend le dessin du champ (gn-champ__saisie) ; la flèche reste celle du système. */
[data-app="guide-nego"] .gn-question__liste {
  cursor: pointer;
}

[data-app="guide-nego"] .gn-question__pied,
[data-app="guide-nego"] .gn-question__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-question--envoyee {
  align-items: center;
  padding-top: var(--gn-espace-12);
  text-align: center;
}

[data-app="guide-nego"] .gn-question__signe {
  color: var(--gn-etat-valide);
}

[data-app="guide-nego"] .gn-question__annonce {
  margin-top: calc(-1 * var(--gn-espace-8));
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

/* Le récapitulatif : une carte, ses lignes séparées d'un filet doux. */
[data-app="guide-nego"] .gn-question__recap {
  align-self: stretch;
  padding-inline: var(--gn-espace-16);
  border-radius: var(--gn-rayon-20);
  background: var(--gn-fond-2);
  text-align: start;
}

[data-app="guide-nego"] .gn-question__ligne {
  display: flex;
  justify-content: space-between;
  gap: var(--gn-espace-12);
  padding-block: 14px;
}

[data-app="guide-nego"] .gn-question__ligne + .gn-question__ligne {
  border-top: var(--gn-filet-1) solid var(--gn-filet-doux);
}

[data-app="guide-nego"] .gn-question__ligne--pile {
  flex-direction: column;
  gap: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-question__ligne dt,
[data-app="guide-nego"] .gn-question__zone {
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  font-weight: var(--gn-graisse-regulier);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-question__ligne dd {
  margin: 0;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  overflow-wrap: anywhere;
  text-align: end;
}

[data-app="guide-nego"] .gn-question__ligne--pile dd {
  text-align: start;
}

[data-app="guide-nego"] .gn-question__sorties {
  align-self: stretch;
}
</style>
