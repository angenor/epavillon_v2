<script setup lang="ts">
/**
 * Le retour depuis le courriel de vérification.
 *
 * **Cet écran n'est pas dans la maquette, et c'est un écart assumé** : elle n'a
 * pas prévu ce moment. Sans lui, une personne qui crée son compte depuis
 * l'application sort de l'application pour n'y jamais revenir — le lien la
 * déposait sur l'ePavillon, autre apparence, autre logique, aucun chemin de
 * retour.
 *
 * **Les deux téléphones ne se comportent pas pareil.** Android laisse
 * l'application ouvrir le lien elle-même : on enchaîne. iPhone ouvre le lien
 * dans Safari, **dans une session qui n'est pas celle de l'application** : on ne
 * peut donc pas continuer le parcours ici. L'écran confirme et renvoie ; c'est
 * l'application, à son retour au premier plan, qui relit l'état et passe à la
 * suite — sans rien demander à personne.
 */
import { plateformeDe } from '~/utils/guide-nego/appareil'
import { APRES_LE_COMPTE } from '~/utils/guide-nego/parcours'

definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const api = useApi()
const route = useRoute()
const session = useGnSession()

type Issue = 'attente' | 'confirmee' | 'perimee' | 'deja-utilisee' | 'inconnue' | 'panne'

const issue = ref<Issue>('attente')
const adresse = ref<string | null>(null)
const renvoiFait = ref(false)

/**
 * Sur iPhone, l'application ne partage pas le stockage de Safari : cette page
 * n'est pas dans la session de l'application, et il n'y a rien à enchaîner.
 */
const surIphone = computed(() =>
  typeof navigator === 'undefined' ? false : plateformeDe(navigator.userAgent) === 'ios',
)

const jeton = computed(() => {
  const valeur = route.query.token
  return typeof valeur === 'string' ? valeur : null
})

onMounted(async () => {
  if (!jeton.value) {
    issue.value = 'inconnue'
    return
  }

  try {
    const reponse = await api.auth.verifyEmail(jeton.value)
    if (reponse.status === 'verified') {
      adresse.value = reponse.email
      issue.value = 'confirmee'
      // Android : la page est dans la session de l'application, on enchaîne.
      //
      // **Vers la connexion, et non vers la suite du parcours** : confirmer une
      // adresse n'ouvre pas de session — l'inscription n'en ouvre aucune, et
      // c'est voulu. Une personne déjà connectée, elle, passe directement.
      if (!surIphone.value) {
        await session.rafraichir()
        if (session.compte.value.connectee) void navigateTo(APRES_LE_COMPTE)
        else void navigateTo({ path: '/guide-nego/connexion', query: { adresse: reponse.email } })
      }
      return
    }

    issue.value =
      reponse.reason === 'expired'
        ? 'perimee'
        : reponse.reason === 'already_used'
          ? 'deja-utilisee'
          : 'inconnue'
  } catch {
    issue.value = 'panne'
  }
})

async function renvoyer(): Promise<void> {
  if (!adresse.value) return
  await api.guideNego.renvoyerLeCourriel(adresse.value)
  renvoiFait.value = true
}

useHead({ title: t('guide-nego.verification-adresse.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.verification-adresse.titre')" :onglets="false">
    <GnChargement
      v-if="issue === 'attente'"
      forme="arc"
      :libelle="t('guide-nego.verification-adresse.attente')"
    />

    <template v-else-if="issue === 'confirmee'">
      <GnEtatVide
        picto="check"
        :titre="t('guide-nego.verification-adresse.confirmee.titre')"
        :texte="
          surIphone
            ? t('guide-nego.verification-adresse.confirmee.retournez')
            : t('guide-nego.verification-adresse.confirmee.suite')
        "
      />
    </template>

    <!-- Un jeton périmé se redemande : c'est la seule sortie utile. -->
    <template v-else-if="issue === 'perimee'">
      <GnEtatErreur
        :titre="t('guide-nego.verification-adresse.perimee.titre')"
        :texte="t('guide-nego.verification-adresse.perimee.texte')"
        :sortie="renvoiFait ? undefined : t('guide-nego.verification-adresse.renvoyer')"
        :sortie-secondaire="t('guide-nego.verification-adresse.vers-connexion')"
        sortie-secondaire-vers="/guide-nego/connexion"
        @sortie="renvoyer"
      />
    </template>

    <!-- Déjà utilisé : le travail est fait, il n'y a rien à refaire. -->
    <GnEtatVide
      v-else-if="issue === 'deja-utilisee'"
      picto="check"
      :titre="t('guide-nego.verification-adresse.deja-utilisee.titre')"
      :texte="t('guide-nego.verification-adresse.deja-utilisee.texte')"
      :sortie="t('guide-nego.verification-adresse.vers-connexion')"
      sortie-vers="/guide-nego/connexion"
    />

    <GnEtatErreur
      v-else-if="issue === 'inconnue'"
      :titre="t('guide-nego.verification-adresse.inconnue.titre')"
      :texte="t('guide-nego.verification-adresse.inconnue.texte')"
      :sortie="t('guide-nego.verification-adresse.vers-connexion')"
      sortie-vers="/guide-nego/connexion"
    />

    <GnEtatErreur
      v-else
      :titre="t('guide-nego.verification-adresse.panne.titre')"
      :texte="t('api.unreachable.network')"
      :sortie="t('guide-nego.verification-adresse.vers-connexion')"
      sortie-vers="/guide-nego/connexion"
    />
  </GnEcran>
</template>
